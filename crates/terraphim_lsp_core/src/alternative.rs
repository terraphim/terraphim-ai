//! Adding a human-written alternative to the annotation block.
//!
//! KG synonyms are never stored: they are derived from the thesaurus. What
//! the block holds is what the knowledge graph cannot derive, such as an
//! alternative the writer typed. [`add_alternative`] records one for a body
//! range and returns a single [`TextEdit`] that rewrites the trailing block.
//! The body is never touched: the text on the page stays the active
//! alternative, exactly as when the editor adds one.
//!
//! Everything is delegated to terraphim-editor's own span model
//! ([`terraphim_alternatives::Document`]) and writer
//! ([`terraphim_alternatives::write`]), so the block written here is byte for
//! byte what the editor would write for the same document.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{EditError, Source, SpanKind, parse, write};

use crate::diagnostic::DiagnosticCode;
use crate::engine::TextEdit;
use crate::offset::{TextRange, utf16_len};

/// Why an alternative could not be added. The document is never changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddAlternativeError {
    /// The annotation block is malformed, so it cannot be rewritten without
    /// losing what it holds. Fix the block first.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
    /// The range is empty or does not lie wholly in the body.
    #[error("the range must be a non-empty range of the body, before the annotation block")]
    OutsideBody,
    /// The alternative text is empty.
    #[error("the alternative text is empty")]
    EmptyText,
    /// The alternative is the text already in the body.
    #[error("the alternative is the current text")]
    SameAsCurrent,
    /// The span over the range already has this alternative.
    #[error("span {span_id:?} already has this alternative (index {index})")]
    AlreadyPresent {
        /// The span's id.
        span_id: String,
        /// The existing alternative's index.
        index: usize,
    },
    /// The range overlaps an existing span without matching it exactly.
    /// Spans never overlap, so the alternative has nowhere to go.
    #[error("the range overlaps span {0:?}; select exactly that span's text")]
    Overlap(String),
    /// Some stored spans or ghosts no longer match the body and cannot be
    /// re-anchored. Rewriting the block now would drop them, so the edit is
    /// refused; open the file in terraphim-editor to resolve them.
    #[error(
        "{0} stored span(s) or ghost(s) no longer match the body; resolve them before adding alternatives"
    )]
    UnresolvedAnchors(usize),
    /// The span model refused the edit for another reason.
    #[error("the span model refused the edit: {0}")]
    Refused(String),
}

/// The result of [`add_alternative`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeAdded {
    /// One edit replacing everything after the body (the old block, or
    /// nothing) with the rewritten block. Offsets are into the text passed
    /// in.
    pub edit: TextEdit,
    /// The id of the span holding the alternative.
    pub span_id: String,
    /// The new alternative's index in the span (0 is the original).
    pub index: usize,
    /// Whether a new span was created for the range.
    pub new_span: bool,
}

/// Record `alternative` as a human-written alternative of `range` in the
/// annotation block of `text`.
///
/// `range` must lie wholly in the body (before the annotation block) and be
/// non-empty. If a stored span covers exactly `range`, the alternative is
/// added to it; otherwise a new span is created, of kind `kind` or, when
/// `None`, [`SpanKind::Word`] for text without whitespace and
/// [`SpanKind::Sentence`] otherwise.
///
/// Stored anchors are first re-anchored against the body, as the editor does
/// when it opens a file; if any cannot be placed the edit is refused
/// ([`AddAlternativeError::UnresolvedAnchors`]) rather than written without
/// them.
///
/// ```
/// use terraphim_lsp_core::{TextRange, add_alternative, apply_edits};
///
/// let text = "Pass me a paperclip.";
/// let range = TextRange::from_bytes(text, 10, 19);
/// let added = add_alternative(text, range, "binder clip", None)?;
/// let saved = apply_edits(text, std::slice::from_ref(&added.edit));
/// assert!(saved.starts_with("Pass me a paperclip.\n\n```terraphim-alternatives\n"));
/// let document = terraphim_alternatives::parse(&saved).unwrap();
/// assert_eq!(document.body, text);
/// assert_eq!(document.annotations.spans[0].alts[1].text, "binder clip");
/// # Ok::<(), terraphim_lsp_core::AddAlternativeError>(())
/// ```
pub fn add_alternative(
    text: &str,
    range: TextRange,
    alternative: &str,
    kind: Option<SpanKind>,
) -> Result<AlternativeAdded, AddAlternativeError> {
    let mut document = parse(text).map_err(|error| AddAlternativeError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    let body_len = document.body.len();
    if range.start.byte >= range.end.byte || range.end.byte > body_len {
        return Err(AddAlternativeError::OutsideBody);
    }
    if alternative.is_empty() {
        return Err(AddAlternativeError::EmptyText);
    }
    let current = &text[range.bytes()];
    if alternative == current {
        return Err(AddAlternativeError::SameAsCurrent);
    }

    let report = document.reanchor();
    let unresolved = report.unresolved.len() + report.unresolved_ghosts.len();
    if unresolved > 0 {
        return Err(AddAlternativeError::UnresolvedAnchors(unresolved));
    }

    // The body is a prefix of `text`, so its UTF-16 offsets are the text's.
    let (start, end) = (range.start.utf16, range.end.utf16);
    let existing = document
        .annotations
        .spans
        .iter()
        .find(|span| span.anchor.start == start && span.anchor.end == end)
        .map(|span| span.id.clone());
    let (span_id, new_span) = match existing {
        Some(id) => (id, false),
        None => {
            let kind = kind.unwrap_or_else(|| default_kind(current));
            let id = document
                .add_span(kind, start, end)
                .map_err(refused_by_span_model)?;
            (id, true)
        }
    };
    let before = document.span(&span_id).map_or(0, |span| span.alts.len());
    let index = document
        .add_alternative(&span_id, alternative, Source::Human, None)
        .map_err(refused_by_span_model)?;
    if index < before {
        return Err(AddAlternativeError::AlreadyPresent { span_id, index });
    }

    let written = write(&document);
    debug_assert!(written.starts_with(&document.body));
    Ok(AlternativeAdded {
        edit: TextEdit {
            range: TextRange::from_bytes(text, body_len, text.len()),
            new_text: written[body_len..].to_string(),
        },
        span_id,
        index,
        new_span,
    })
}

/// [`add_alternative`] for a UTF-16 range, as reported by a browser
/// selection.
pub fn add_alternative_utf16(
    text: &str,
    start: usize,
    end: usize,
    alternative: &str,
    kind: Option<SpanKind>,
) -> Result<AlternativeAdded, AddAlternativeError> {
    let range = TextRange::from_utf16(text, start, end);
    if range.start.utf16 != start || range.end.utf16 != end || end > utf16_len(text) {
        // Offsets that split a character are rejected, never rounded, as in
        // terraphim_alternatives.
        return Err(AddAlternativeError::OutsideBody);
    }
    add_alternative(text, range, alternative, kind)
}

fn default_kind(text: &str) -> SpanKind {
    if text.chars().any(char::is_whitespace) {
        SpanKind::Sentence
    } else {
        SpanKind::Word
    }
}

fn refused_by_span_model(error: EditError) -> AddAlternativeError {
    match error {
        EditError::Overlap(id) => AddAlternativeError::Overlap(id),
        other => AddAlternativeError::Refused(other.to_string()),
    }
}
