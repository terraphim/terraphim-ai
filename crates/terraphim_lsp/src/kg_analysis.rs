//! Knowledge-graph analysis helpers for Terraphim LSP.
//!
//! Term matching, concept ids and the annotation-block boundary come from
//! [`terraphim_lsp_core`]; this module adds the server-only list of
//! unrecognised words used for the opt-in unknown-term diagnostics.

use std::ops::Range;

use terraphim_lsp_core::{Diagnostic, KgEngine};

pub use terraphim_lsp_core::TermMatch;

/// Result of analysing a document against a knowledge graph.
#[derive(Debug, Clone, PartialEq)]
pub struct KgAnalysis {
    /// Terms from the thesaurus found in the document body, with concept id,
    /// normalised concept name and byte plus UTF-16 ranges.
    pub matched_terms: Vec<TermMatch>,
    /// Every occurrence of a word in the body that is not part of any
    /// thesaurus match, in document order, each with its own byte range.
    pub unknown_terms: Vec<UnknownTerm>,
    /// Problems found by the core, such as a malformed annotation block.
    pub diagnostics: Vec<Diagnostic>,
    /// One faded (`Unnecessary`) hint per ghosted span of the annotation
    /// block. Published only when the `ghostDiagnostics` setting is on.
    pub ghosts: Vec<Diagnostic>,
}

/// One occurrence of a word that is not part of any knowledge-graph match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTerm {
    /// The word as written, without surrounding punctuation.
    pub text: String,
    /// Byte range of this occurrence in the analysed document. Convert it
    /// with [`terraphim_lsp_core::LineIndex`] (UTF-16 columns) for LSP.
    pub bytes: Range<usize>,
}

impl KgAnalysis {
    /// Create an empty analysis result.
    pub fn empty() -> Self {
        Self {
            matched_terms: Vec::new(),
            unknown_terms: Vec::new(),
            diagnostics: Vec::new(),
            ghosts: Vec::new(),
        }
    }

    /// Returns true if nothing was matched, no unknown terms were found and
    /// there are no diagnostics.
    pub fn is_empty(&self) -> bool {
        self.matched_terms.is_empty()
            && self.unknown_terms.is_empty()
            && self.diagnostics.is_empty()
            && self.ghosts.is_empty()
    }
}

/// Analyse a markdown document with a knowledge-graph engine.
///
/// Matching is delegated to [`KgEngine::analyse`], which excludes a trailing
/// `terraphim-alternatives` annotation block. Unknown terms are the
/// whitespace-separated words of the body that were not part of any match.
pub fn analyse_kg_document(text: &str, engine: &KgEngine) -> KgAnalysis {
    if text.trim().is_empty() {
        return KgAnalysis::empty();
    }

    let analysis = engine.analyse(text);
    let ghosts = analysis
        .block
        .as_ref()
        .map(|block| block.ghost_diagnostics())
        .unwrap_or_default();
    if engine.concept_index().is_empty() {
        // No knowledge graph loaded: every word would be "unknown", which is
        // noise. Block diagnostics and ghosts do not depend on the graph, so
        // keep them.
        return KgAnalysis {
            diagnostics: analysis.diagnostics,
            ghosts,
            ..KgAnalysis::empty()
        };
    }

    let body = &text[..analysis.body_end.byte];
    let matched: Vec<Range<usize>> = analysis.matches.iter().map(|m| m.range.bytes()).collect();
    // A word is unknown when its range overlaps no match. Multi-word
    // unknown phrases are not reconstructed: each word is reported alone.
    let unknown_terms = words(body)
        .filter(|word| {
            !matched
                .iter()
                .any(|range| range.start < word.end && word.start < range.end)
        })
        .map(|bytes| UnknownTerm {
            text: body[bytes.clone()].to_string(),
            bytes,
        })
        .collect();

    KgAnalysis {
        matched_terms: analysis.matches,
        unknown_terms,
        diagnostics: analysis.diagnostics,
        ghosts,
    }
}

/// Byte ranges of the words of `text`: whitespace-separated runs with
/// leading and trailing non-alphanumeric characters trimmed. Runs with no
/// alphanumeric character are skipped.
fn words(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        loop {
            // Skip whitespace up to the next run.
            while chars.next_if(|(_, c)| c.is_whitespace()).is_some() {}
            let &(run_start, _) = chars.peek()?;
            let mut run_end = run_start;
            while let Some((at, c)) = chars.next_if(|(_, c)| !c.is_whitespace()) {
                run_end = at + c.len_utf8();
            }
            let run = &text[run_start..run_end];
            let trimmed_start = run.trim_start_matches(|c: char| !c.is_alphanumeric());
            let word = trimmed_start.trim_end_matches(|c: char| !c.is_alphanumeric());
            if !word.is_empty() {
                let start = run_start + (run.len() - trimmed_start.len());
                return Some(start..start + word.len());
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    fn sample_engine() -> KgEngine {
        let mut thesaurus = Thesaurus::new("programming".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("rust"),
            NormalizedTerm::new(1, NormalizedTermValue::from("rust programming language"))
                .with_url("https://rust-lang.org".to_string()),
        );
        thesaurus.insert(
            NormalizedTermValue::from("async"),
            NormalizedTerm::new(2, NormalizedTermValue::from("asynchronous programming")),
        );
        thesaurus.insert(
            NormalizedTermValue::from("tokio"),
            NormalizedTerm::new(3, NormalizedTermValue::from("tokio async runtime")),
        );
        KgEngine::new(&thesaurus).expect("sample thesaurus compiles")
    }

    #[test]
    fn test_empty_text_returns_empty() {
        let analysis = analyse_kg_document("", &sample_engine());
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_empty_thesaurus_returns_empty() {
        let analysis = analyse_kg_document("rust is great", &KgEngine::empty());
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_matched_terms_found() {
        let analysis = analyse_kg_document("rust and tokio are great", &sample_engine());
        let terms: Vec<String> = analysis
            .matched_terms
            .iter()
            .map(|m| m.term.clone())
            .collect();
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_matches_keep_concept_id_and_nterm() {
        let analysis = analyse_kg_document("tokio rocks", &sample_engine());
        let tokio = &analysis.matched_terms[0];
        assert_eq!(tokio.concept_id, 3);
        assert_eq!(tokio.nterm, "tokio async runtime");
    }

    fn unknown_texts(analysis: &KgAnalysis) -> Vec<&str> {
        analysis
            .unknown_terms
            .iter()
            .map(|term| term.text.as_str())
            .collect()
    }

    #[test]
    fn test_unknown_terms_found() {
        let analysis = analyse_kg_document("rust and xyz are great", &sample_engine());
        assert_eq!(unknown_texts(&analysis), ["and", "xyz", "are", "great"]);
    }

    #[test]
    fn test_every_occurrence_has_its_own_range() {
        let text = "the rust book\nthe end, (the) the.";
        let analysis = analyse_kg_document(text, &sample_engine());
        let the: Vec<Range<usize>> = analysis
            .unknown_terms
            .iter()
            .filter(|term| term.text == "the")
            .map(|term| term.bytes.clone())
            .collect();
        assert_eq!(the, [0..3, 14..17, 24..27, 29..32]);
        for range in the {
            assert_eq!(&text[range], "the");
        }
    }

    #[test]
    fn test_words_inside_matches_are_not_unknown() {
        let analysis = analyse_kg_document("Tokio, rust! async?", &sample_engine());
        assert!(analysis.unknown_terms.is_empty(), "{analysis:?}");
    }

    #[test]
    fn test_words_trim_punctuation_and_skip_symbols() {
        let text = "  \"héllo,\" -- 😀 x\r\nend.";
        let ranges: Vec<&str> = words(text).map(|range| &text[range]).collect();
        assert_eq!(ranges, ["héllo", "x", "end"]);
    }

    #[test]
    fn test_positions_are_populated() {
        let analysis = analyse_kg_document("rust is great", &sample_engine());
        let rust_match = analysis
            .matched_terms
            .iter()
            .find(|m| m.term == "rust")
            .expect("rust should match");
        assert_eq!(rust_match.range.bytes(), 0..4);
    }

    #[test]
    fn test_annotation_block_is_not_analysed() {
        let text = "rust\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"overflow\": \"tokio zzz\"}\n```\n";
        let analysis = analyse_kg_document(text, &sample_engine());
        assert_eq!(analysis.matched_terms.len(), 1);
        assert!(
            analysis.unknown_terms.is_empty(),
            "{:?}",
            analysis.unknown_terms
        );
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn test_malformed_annotation_block_yields_one_diagnostic() {
        let text = "rust\n\n```terraphim-alternatives\n{\"version\": 1}\n";
        let analysis = analyse_kg_document(text, &sample_engine());
        assert_eq!(analysis.diagnostics.len(), 1);
        assert_eq!(analysis.matched_terms.len(), 1);
    }

    #[test]
    fn test_analyse_never_panics_on_arbitrary_input() {
        let engine = sample_engine();
        let inputs = [
            "!@#$%^&*()",
            "rust\n\ntokio\tasync",
            "",
            "RUST",
            "a b c d e f g",
            "😀 rust é",
        ];
        for input in inputs {
            let _ = analyse_kg_document(input, &engine);
        }
    }
}
