//! The alternatives stored in the annotation block (R-2.2, R-3.6, R-4.4).
//!
//! [`stored_span_at`] reads the span under a cursor with every alternative's
//! provenance ([`Source`]) and the active one, for a hover. Making another
//! alternative active is [`select_alternative`]: the editor's own
//! [`terraphim_alternatives::Document::set_active`] (which also fixes a
//! preceding `a`/`an`) and writer, returned as two edits, one for the body
//! and one for the block. [`stored_anchor_ranges`] gives the body ranges of
//! every stored span and ghost: a plain rewrite of text inside one would
//! leave its anchor stale, so callers do not offer one there.
//!
//! As everywhere in this crate, stored anchors are first re-anchored against
//! the body, as the editor does when it opens a file.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{Document, EditError, Source, SpanKind, parse, utf16_to_byte, write};

use crate::diagnostic::DiagnosticCode;
use crate::engine::TextEdit;
use crate::ghost::ghost_bytes;
use crate::offset::TextRange;

/// One alternative of a stored span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAlternative {
    /// Its index in the span; 0 is the original.
    pub index: usize,
    /// The text it puts in the body.
    pub text: String,
    /// Who produced it.
    pub source: Source,
    /// The model of an AI alternative, if known.
    pub model: Option<String>,
}

/// A span stored in the annotation block, placed in the body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSpan {
    /// The span's id in the block.
    pub span_id: String,
    /// Its granularity.
    pub kind: SpanKind,
    /// Where it is in the text passed in (after re-anchoring).
    pub range: TextRange,
    /// The index of the alternative shown in the body.
    pub active: usize,
    /// Every alternative, in block order.
    pub alts: Vec<StoredAlternative>,
}

/// Why a stored alternative could not be made active. The document is never
/// changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SelectAlternativeError {
    /// The annotation block is malformed, so it cannot be rewritten without
    /// losing what it holds. Fix the block first.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
    /// Some stored spans or ghosts no longer match the body and cannot be
    /// re-anchored. Rewriting the block now would drop them, so the edit is
    /// refused.
    #[error("{0} stored span(s) or ghost(s) no longer match the body; resolve them first")]
    UnresolvedAnchors(usize),
    /// No stored span has this id.
    #[error("no stored span with id {0:?}")]
    UnknownSpan(String),
    /// The span has no alternative at this index.
    #[error("span {span_id:?} has no alternative {index}")]
    InvalidIndex {
        /// The span's id.
        span_id: String,
        /// The requested index.
        index: usize,
    },
    /// The alternative is already the active one.
    #[error("alternative {index} of span {span_id:?} is already active")]
    AlreadyActive {
        /// The span's id.
        span_id: String,
        /// The requested index.
        index: usize,
    },
    /// The span model refused the edit for another reason.
    #[error("the span model refused the edit: {0}")]
    Refused(String),
}

/// Making a stored alternative active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeSelected {
    /// Two edits, sorted and non-overlapping: the body change (the span's
    /// text and any `a`/`an` fix-up before it) and the block rewrite. Apply
    /// them together. Offsets are into the text passed in.
    pub edits: Vec<TextEdit>,
}

/// The stored span whose body range touches byte `offset` of `text` (either
/// edge counts, as for a KG term), or `None`: no span there, no block, or a
/// malformed block. Spans that cannot be re-anchored are not reported.
///
/// ```
/// use terraphim_lsp_core::{Source, stored_span_at};
///
/// let text = "Pass me a paperclip.\n\n```terraphim-alternatives\n\
///     {\"version\": 1, \"spans\": [{\"id\": \"s1\", \"kind\": \"word\", \
///     \"anchor\": {\"start\": 10, \"end\": 19, \"text\": \"paperclip\"}, \"active\": 0, \
///     \"alts\": [{\"text\": \"paperclip\", \"source\": \"original\"}, \
///     {\"text\": \"binder clip\", \"source\": \"human\"}]}], \"ghosts\": []}\n```\n";
/// let span = stored_span_at(text, 12).unwrap();
/// assert_eq!(span.alts[1].text, "binder clip");
/// assert_eq!(span.alts[1].source, Source::Human);
/// ```
pub fn stored_span_at(text: &str, offset: usize) -> Option<StoredSpan> {
    let mut document = parse(text).ok()?;
    document.reanchor();
    let body = &document.body;
    if offset > body.len() {
        return None;
    }
    document.annotations.spans.iter().find_map(|span| {
        let start = utf16_to_byte(body, span.anchor.start)?;
        let end = utf16_to_byte(body, span.anchor.end)?;
        if !(start <= offset && offset <= end) {
            return None;
        }
        Some(StoredSpan {
            span_id: span.id.clone(),
            kind: span.kind,
            range: TextRange::from_bytes(text, start, end),
            active: span.active,
            alts: span
                .alts
                .iter()
                .enumerate()
                .map(|(index, alt)| StoredAlternative {
                    index,
                    text: alt.text.clone(),
                    source: alt.source,
                    model: alt.model.clone(),
                })
                .collect(),
        })
    })
}

/// Register the live authored span with the shared Terraphim KG parser/types.
/// Detached spans and malformed blocks never publish searchable synonyms.
/// Registration is derived from this committed source, without a global cache.
pub fn stored_synonym_group_at(
    text: &str,
    offset: usize,
) -> Option<terraphim_alternatives::SynonymGroup> {
    let span = stored_span_at(text, offset)?;
    let mut document = parse(text).ok()?;
    document.reanchor();
    document
        .synonym_groups()
        .into_iter()
        .find(|group| group.span_id == span.span_id)
}

/// The body ranges of every stored span and ghost that can be placed, in
/// body order. Empty without a block or with a malformed one.
pub fn stored_anchor_ranges(text: &str) -> Vec<TextRange> {
    let Ok(mut document) = parse(text) else {
        return Vec::new();
    };
    document.reanchor();
    let body = &document.body;
    let mut bytes: Vec<(usize, usize)> = document
        .annotations
        .spans
        .iter()
        .filter_map(|span| {
            let start = utf16_to_byte(body, span.anchor.start)?;
            let end = utf16_to_byte(body, span.anchor.end)?;
            (start < end).then_some((start, end))
        })
        .chain(ghost_bytes(&document))
        .collect();
    bytes.sort_unstable();
    bytes
        .into_iter()
        .map(|(start, end)| TextRange::from_bytes(text, start, end))
        .collect()
}

/// Make alternative `index` of stored span `span_id` active: the body shows
/// it (with a preceding `a`/`an` fixed) and the block records it, exactly as
/// the editor does.
pub fn select_alternative(
    text: &str,
    span_id: &str,
    index: usize,
) -> Result<AlternativeSelected, SelectAlternativeError> {
    let mut document = open(text)?;
    let span = document
        .span(span_id)
        .ok_or_else(|| SelectAlternativeError::UnknownSpan(span_id.to_string()))?;
    if index >= span.alts.len() {
        return Err(SelectAlternativeError::InvalidIndex {
            span_id: span_id.to_string(),
            index,
        });
    }
    if index == span.active {
        return Err(SelectAlternativeError::AlreadyActive {
            span_id: span_id.to_string(),
            index,
        });
    }
    let old_body = document.body.clone();
    let span_bytes = utf16_to_byte(&old_body, span.anchor.start)
        .zip(utf16_to_byte(&old_body, span.anchor.end))
        .ok_or_else(|| SelectAlternativeError::Refused(format!("{span_id:?} is stale")))?;
    document.set_active(span_id, index).map_err(refused)?;
    let written = write(&document);
    debug_assert!(written.starts_with(&document.body));
    let new_body = &document.body;

    // The body changes in one place: the span, and an article just before
    // it. The edit replaces the whole span (as a KG replacement replaces the
    // whole term) plus whatever else differs, on character boundaries.
    let prefix = common_prefix(&old_body, new_body);
    let suffix = common_suffix(&old_body[prefix..], &new_body[prefix..]);
    // No difference at all (an alternative with the current text): only
    // the span itself, unchanged.
    let (diff_start, diff_end) = if prefix == old_body.len() && prefix == new_body.len() {
        span_bytes
    } else {
        (prefix, old_body.len() - suffix)
    };
    let start = diff_start.min(span_bytes.0);
    let old_end = diff_end.max(span_bytes.1);
    let new_end = old_end + new_body.len() - old_body.len();
    let body_edit = TextEdit {
        range: TextRange::from_bytes(text, start, old_end),
        new_text: new_body[start..new_end].to_string(),
    };
    let block_edit = TextEdit {
        range: TextRange::from_bytes(text, old_body.len(), text.len()),
        new_text: written[new_body.len()..].to_string(),
    };
    Ok(AlternativeSelected {
        edits: vec![body_edit, block_edit],
    })
}

fn open(text: &str) -> Result<Document, SelectAlternativeError> {
    let mut document = parse(text).map_err(|error| SelectAlternativeError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    let report = document.reanchor();
    match report.unresolved.len() + report.unresolved_ghosts.len() {
        0 => Ok(document),
        count => Err(SelectAlternativeError::UnresolvedAnchors(count)),
    }
}

/// Length in bytes of the longest common prefix, on a character boundary.
fn common_prefix(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((at, _), _)| at)
}

/// Length in bytes of the longest common suffix, on a character boundary.
fn common_suffix(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum()
}

fn refused(error: EditError) -> SelectAlternativeError {
    match error {
        EditError::UnknownSpan(id) => SelectAlternativeError::UnknownSpan(id),
        other => SelectAlternativeError::Refused(other.to_string()),
    }
}
