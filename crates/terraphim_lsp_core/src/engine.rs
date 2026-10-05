//! The KG engine: term analysis and synonym alternatives over one thesaurus.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{article_for, preceding_article, respell};
use terraphim_automata::{
    CompiledMatcher, ConceptIndex, Matched, MatcherBuilder, MatcherOptions, TerraphimAutomataError,
    alternatives_in, load_thesaurus_from_json,
};
use terraphim_types::Thesaurus;

use crate::block::{AnnotationBlock, split_annotation_block};
use crate::case::Capitalisation;
use crate::diagnostic::Diagnostic;
use crate::offset::{TextOffset, TextRange, Utf16Cursor};

/// Errors from building a [`KgEngine`].
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The thesaurus JSON could not be read.
    #[error("thesaurus JSON could not be read: {0}")]
    Thesaurus(#[source] TerraphimAutomataError),
    /// The matcher could not be compiled from the thesaurus.
    #[error("KG matcher could not be built: {0}")]
    Matcher(#[source] TerraphimAutomataError),
}

/// A knowledge-graph term found in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMatch {
    /// The thesaurus key that matched (normalised: trimmed, lowercase).
    pub term: String,
    /// The matched text exactly as it appears in the document.
    pub text: String,
    /// Normalised name of the concept the term belongs to.
    pub nterm: String,
    /// Id of the concept. Every synonym of a concept shares it.
    ///
    /// Thesaurus ids are small sequence numbers in practice, well inside the
    /// 2^53 range a JavaScript number represents exactly.
    pub concept_id: u64,
    /// Where the term is.
    pub range: TextRange,
    /// Display form of the concept (`NormalizedTerm::display`), if non-empty.
    pub description: Option<String>,
    /// The concept's URL, if the thesaurus has one.
    pub url: Option<String>,
}

/// The result of analysing a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    /// KG terms found in the body, in document order, never overlapping.
    pub matches: Vec<TermMatch>,
    /// End of the analysed body: the start of the annotation block, or the
    /// end of the text when there is none.
    pub body_end: TextOffset,
    /// The trailing annotation block, if any. It is never analysed.
    pub block: Option<AnnotationBlock>,
    /// Problems found. At most one per document today: a malformed
    /// annotation block.
    pub diagnostics: Vec<Diagnostic>,
}

/// One replacement of a document range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    /// The range to replace, in the text the edit was computed for.
    pub range: TextRange,
    /// The text to put there.
    pub new_text: String,
}

/// Replacing a matched term with one of its synonyms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    /// The synonym, with the original's capitalisation applied.
    pub text: String,
    /// The edits that perform the replacement, sorted by position and
    /// non-overlapping: an `a`/`an` fix-up (only when the article changes)
    /// followed by the term itself. Apply them together, as one edit.
    pub edits: Vec<TextEdit>,
}

/// The alternatives for one matched term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeSet {
    /// The matched term the alternatives replace.
    pub term: TermMatch,
    /// The capitalisation pattern of the matched text, applied to every
    /// replacement.
    pub capitalisation: Capitalisation,
    /// One entry per other term of the concept, current form excluded, in
    /// [`ConceptIndex::synonyms_of`] order (concept name first, then the
    /// remaining synonyms sorted). Empty for a single-term concept.
    pub replacements: Vec<Replacement>,
}

/// Term analysis and synonym alternatives for one thesaurus.
///
/// Build it once per thesaurus load: it compiles the matcher and inverts the
/// thesaurus into a [`ConceptIndex`], both reused for every call. All methods
/// take the document text and return offsets relative to it, so a caller may
/// pass the whole file (the LSP server) or just the body (the editor).
///
/// ```
/// use terraphim_lsp_core::KgEngine;
///
/// let thesaurus = r#"{"name": "demo", "data": {
///     "eraser": {"id": 1, "nterm": "eraser"},
///     "rubber": {"id": 1, "nterm": "eraser"}
/// }}"#;
/// let engine = KgEngine::from_json(thesaurus)?;
/// let text = "Pass me an eraser.";
/// let set = engine.alternatives_at(text, 12).expect("eraser is a KG term");
/// assert_eq!(set.term.concept_id, 1);
/// let rubber = &set.replacements[0];
/// assert_eq!(rubber.text, "rubber");
/// // "an" becomes "a" in the same replacement.
/// assert_eq!(rubber.edits[0].new_text, "a");
/// assert_eq!(rubber.edits[1].new_text, "rubber");
/// # Ok::<(), terraphim_lsp_core::CoreError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct KgEngine {
    /// `None` when the thesaurus has no usable pattern.
    matcher: Option<CompiledMatcher>,
    index: ConceptIndex,
    skipped_patterns: Vec<String>,
}

impl KgEngine {
    /// Build an engine from a thesaurus.
    ///
    /// Thesaurus keys the matcher cannot use (blank, or shorter than the
    /// matcher's minimum length) are skipped rather than failing the whole
    /// load, as `terraphim_automata::find_matches` does; they are listed by
    /// [`KgEngine::skipped_patterns`] and may still be offered as
    /// alternatives.
    pub fn new(thesaurus: &Thesaurus) -> Result<Self, CoreError> {
        let mut builder = MatcherBuilder::new(MatcherOptions::default());
        let mut skipped_patterns = Vec::new();
        for (key, term) in thesaurus {
            match builder.insert(key.to_string(), term.clone()) {
                Ok(_) => {}
                Err(TerraphimAutomataError::InvalidPattern { pattern, .. }) => {
                    skipped_patterns.push(pattern);
                }
                Err(error) => return Err(CoreError::Matcher(error)),
            }
        }
        skipped_patterns.sort_unstable();
        let matcher = if builder.is_empty() {
            None
        } else {
            Some(builder.build().map_err(CoreError::Matcher)?)
        };
        Ok(Self {
            matcher,
            index: ConceptIndex::from_thesaurus(thesaurus),
            skipped_patterns,
        })
    }

    /// An engine with no thesaurus: it finds no terms and offers no
    /// alternatives. Useful as a fallback before a thesaurus is loaded.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build an engine from a serialised [`Thesaurus`] (the JSON written by
    /// terraphim's thesaurus builders).
    pub fn from_json(json: &str) -> Result<Self, CoreError> {
        let thesaurus = load_thesaurus_from_json(json).map_err(CoreError::Thesaurus)?;
        Self::new(&thesaurus)
    }

    /// Thesaurus keys that were not compiled into the matcher, sorted.
    pub fn skipped_patterns(&self) -> &[String] {
        &self.skipped_patterns
    }

    /// The concept index, for callers that need every term of a concept.
    pub fn concept_index(&self) -> &ConceptIndex {
        &self.index
    }

    /// Find the KG terms in `text`.
    ///
    /// A trailing `terraphim-alternatives` annotation block is excluded; if it
    /// is malformed, [`Analysis::diagnostics`] holds exactly one diagnostic
    /// for it. The text is never modified.
    pub fn analyse(&self, text: &str) -> Analysis {
        let split = split_annotation_block(text);
        let body = &text[..split.body_end.byte];
        let mut cursor = Utf16Cursor::new(text);
        let matches = self
            .scan(body)
            .iter()
            .filter_map(|matched| self.term_match(text, matched, &mut cursor))
            .collect();
        let diagnostics = split
            .block
            .as_ref()
            .and_then(|block| block.problem.clone())
            .into_iter()
            .collect();
        Analysis {
            matches,
            body_end: split.body_end,
            block: split.block,
            diagnostics,
        }
    }

    /// The alternatives for the KG term covering byte offset `byte`.
    ///
    /// A term covers the offsets from its start to its end inclusive, so a
    /// cursor just after a word still finds it. Returns `None` when no term
    /// covers the offset, when the offset lies in the annotation block, or
    /// when the matched term is not in the concept index.
    pub fn alternatives_at(&self, text: &str, byte: usize) -> Option<AlternativeSet> {
        let body_end = split_annotation_block(text).body_end.byte;
        if byte > body_end {
            return None;
        }
        let matches = self.scan(&text[..body_end]);
        let found = alternatives_in(&matches, byte, &self.index)?;
        let matched = matches
            .iter()
            .find(|matched| matched.pos == Some((found.range.start, found.range.end)))?;
        let term = self.term_match(text, matched, &mut Utf16Cursor::new(text))?;
        Some(self.alternative_set(text, term, &found.alternatives))
    }

    /// [`KgEngine::alternatives_at`] for a UTF-16 offset, as reported by a
    /// browser selection or an LSP position.
    pub fn alternatives_at_utf16(&self, text: &str, utf16: usize) -> Option<AlternativeSet> {
        self.alternatives_at(text, TextOffset::from_utf16(text, utf16).byte)
    }

    /// The alternatives for a term previously returned by
    /// [`KgEngine::analyse`] on the same `text`.
    ///
    /// Returns `None` if `text` no longer has that term at that range.
    pub fn alternatives_for(&self, text: &str, term: &TermMatch) -> Option<AlternativeSet> {
        self.alternatives_at(text, term.range.start.byte)
            .filter(|set| set.term.range == term.range)
    }

    /// Positioned matches in `body`, in the shape `terraphim_automata`'s
    /// concept-index lookups take.
    fn scan(&self, body: &str) -> Vec<Matched> {
        let Some(matcher) = &self.matcher else {
            return Vec::new();
        };
        matcher
            .find_positions(body)
            .filter_map(|found| {
                Some(Matched {
                    term: matcher.pattern(found.pattern_index)?.to_string(),
                    normalized_term: matcher.term(found.pattern_index)?.clone(),
                    pos: Some((found.start, found.end)),
                })
            })
            .collect()
    }

    fn term_match(
        &self,
        text: &str,
        matched: &Matched,
        cursor: &mut Utf16Cursor<'_>,
    ) -> Option<TermMatch> {
        let (start, end) = matched.pos?;
        let concept = &matched.normalized_term;
        Some(TermMatch {
            term: matched.term.clone(),
            text: text.get(start..end)?.to_string(),
            nterm: concept.value.as_str().to_string(),
            concept_id: concept.id,
            range: cursor.range(start, end),
            description: Some(concept.display().to_string()).filter(|d| !d.is_empty()),
            url: concept.url.clone(),
        })
    }

    fn alternative_set(&self, text: &str, term: TermMatch, synonyms: &[String]) -> AlternativeSet {
        let capitalisation = Capitalisation::of(&term.text);
        let article = preceding_article(text, term.range.start.byte);
        let replacements = synonyms
            .iter()
            .map(|synonym| {
                let replacement = capitalisation.apply(synonym);
                let mut edits = Vec::with_capacity(2);
                if let Some((start, end)) = article {
                    let existing = &text[start..end];
                    if let Some(wanted) = article_for(&replacement) {
                        let respelt = respell(existing, wanted);
                        if respelt != existing {
                            edits.push(TextEdit {
                                range: TextRange::from_bytes(text, start, end),
                                new_text: respelt.to_string(),
                            });
                        }
                    }
                }
                edits.push(TextEdit {
                    range: term.range,
                    new_text: replacement.clone(),
                });
                Replacement {
                    text: replacement,
                    edits,
                }
            })
            .collect();
        AlternativeSet {
            term,
            capitalisation,
            replacements,
        }
    }
}

/// Apply `edits` (sorted, non-overlapping, as in [`Replacement::edits`]) to
/// `text`, returning the new text.
///
/// # Panics
///
/// Panics if the edits are out of order, overlap, or do not lie on character
/// boundaries of `text`, which cannot happen for edits computed on `text`.
///
/// ```
/// use terraphim_lsp_core::{KgEngine, apply_edits};
///
/// let engine = KgEngine::from_json(r#"{"name": "demo", "data": {
///     "eraser": {"id": 1, "nterm": "eraser"},
///     "rubber": {"id": 1, "nterm": "eraser"}
/// }}"#)?;
/// let text = "A rubber, please.";
/// let set = engine.alternatives_at(text, 3).unwrap();
/// assert_eq!(apply_edits(text, &set.replacements[0].edits), "An eraser, please.");
/// # Ok::<(), terraphim_lsp_core::CoreError>(())
/// ```
pub fn apply_edits(text: &str, edits: &[TextEdit]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for edit in edits {
        let range = edit.range.bytes();
        out.push_str(&text[copied..range.start]);
        out.push_str(&edit.new_text);
        copied = range.end;
    }
    out.push_str(&text[copied..]);
    out
}
