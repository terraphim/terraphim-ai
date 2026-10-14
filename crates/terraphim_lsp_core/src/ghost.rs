//! Ghosting and reviving body text (R-5.1, R-5.2).
//!
//! A ghost dims a range without deleting it. Ghosts are an independent
//! layer, persisted in the trailing `terraphim-alternatives` block next to
//! the spans. [`ghost_range`] and [`revive_range`] return one [`TextEdit`]
//! that rewrites only that block, with the editor's own span model
//! ([`terraphim_alternatives::Document::ghost`] and
//! [`terraphim_alternatives::Document::revive`]) and writer, so merging
//! (overlapping or touching ghosts become one), splitting and trimming are
//! exactly the editor's. The body is never touched, and span alternatives
//! and overflow stay as they were.
//!
//! [`ghost_availability`] says which of the two actions apply to a
//! selection, without building an edit.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{Document, EditError, parse, write};

use crate::diagnostic::DiagnosticCode;
use crate::engine::TextEdit;
use crate::offset::TextRange;

/// Why a ghost or revive edit could not be made. The document is never
/// changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GhostError {
    /// The annotation block is malformed, so it cannot be rewritten without
    /// losing what it holds. Fix the block first.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
    /// The range is empty once cut back to the body.
    #[error("the range must be a non-empty range of the body, before the annotation block")]
    OutsideBody,
    /// The range is already wholly ghosted.
    #[error("the range is already ghosted")]
    AlreadyGhosted,
    /// No ghost overlaps the range.
    #[error("no ghost overlaps the range")]
    NothingToRevive,
    /// Some stored spans or ghosts no longer match the body and cannot be
    /// re-anchored. Rewriting the block now would drop them, so the edit is
    /// refused.
    #[error("{0} stored span(s) or ghost(s) no longer match the body; resolve them first")]
    UnresolvedAnchors(usize),
    /// The span model refused the edit.
    #[error("the span model refused the edit: {0}")]
    Refused(String),
}

/// A ghost or revive edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostEdited {
    /// One edit replacing everything after the body (the old block, or
    /// nothing) with the rewritten block. Offsets are into the text passed
    /// in.
    pub edit: TextEdit,
}

/// Which ghost actions apply to a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhostAvailability {
    /// "Ghost it": the selection is non-empty and not entirely ghosted.
    pub can_ghost: bool,
    /// "Revive": the cursor is in a ghost, or the selection overlaps one.
    pub can_revive: bool,
}

/// Which of "Ghost it" and "Revive" apply to `range` in `text`. Both are
/// `false` for a malformed block or a block with unplaceable anchors, the
/// cases where the edit would be refused.
pub fn ghost_availability(text: &str, range: TextRange) -> GhostAvailability {
    let none = GhostAvailability {
        can_ghost: false,
        can_revive: false,
    };
    let Ok(mut document) = parse(text) else {
        return none;
    };
    if unresolved(&mut document) > 0 {
        return none;
    }
    let body_len = document.body.len();
    let (start, end) = (range.start.byte, range.end.byte.min(body_len));
    let ghosts = ghost_bytes(&document);
    let can_revive = if range.start.byte >= range.end.byte {
        ghosts
            .iter()
            .any(|&(ghost_start, ghost_end)| ghost_start <= start && start <= ghost_end)
    } else {
        ghosts
            .iter()
            .any(|&(ghost_start, ghost_end)| ghost_start < end && start < ghost_end)
    };
    let can_ghost = start < end && !covered(&ghosts, start, end);
    GhostAvailability {
        can_ghost,
        can_revive,
    }
}

/// Ghost `range` of `text`: a ghost overlapping or touching it is merged
/// into it. A range running into the annotation block is cut back to the
/// body.
///
/// ```
/// use terraphim_lsp_core::{TextRange, apply_edits, ghost_range};
///
/// let text = "Keep this. Drop this.";
/// let range = TextRange::from_bytes(text, 11, 21);
/// let ghosted = ghost_range(text, range)?;
/// let saved = apply_edits(text, std::slice::from_ref(&ghosted.edit));
/// assert!(saved.starts_with("Keep this. Drop this.\n\n```terraphim-alternatives\n"));
/// let document = terraphim_alternatives::parse(&saved).unwrap();
/// assert_eq!(document.annotations.ghosts[0].anchor.text, "Drop this.");
/// # Ok::<(), terraphim_lsp_core::GhostError>(())
/// ```
pub fn ghost_range(text: &str, range: TextRange) -> Result<GhostEdited, GhostError> {
    let mut document = open(text)?;
    let (start, end) = utf16_in_body(&document, range).ok_or(GhostError::OutsideBody)?;
    let (byte_start, byte_end) = (range.start.byte, range.end.byte.min(document.body.len()));
    if covered(&ghost_bytes(&document), byte_start, byte_end) {
        return Err(GhostError::AlreadyGhosted);
    }
    document.ghost(start, end).map_err(refused)?;
    Ok(rewritten(text, &document))
}

/// Revive `range` of `text`: ghosts inside it are removed, ghosts partly
/// inside it are trimmed and a ghost containing it is split. An empty range
/// (a cursor) revives the whole ghost it is in or at the edge of.
///
/// ```
/// use terraphim_lsp_core::{TextRange, apply_edits, ghost_range, revive_range};
///
/// let text = "Keep this. Drop this.";
/// let all = ghost_range(text, TextRange::from_bytes(text, 11, 21))?;
/// let ghosted = apply_edits(text, std::slice::from_ref(&all.edit));
/// let revived = revive_range(&ghosted, TextRange::from_bytes(&ghosted, 11, 21))?;
/// let saved = apply_edits(&ghosted, std::slice::from_ref(&revived.edit));
/// let document = terraphim_alternatives::parse(&saved).unwrap();
/// assert!(document.annotations.ghosts.is_empty());
/// # Ok::<(), terraphim_lsp_core::GhostError>(())
/// ```
pub fn revive_range(text: &str, range: TextRange) -> Result<GhostEdited, GhostError> {
    let mut document = open(text)?;
    let body_len = document.body.len();
    let (mut byte_start, mut byte_end) = (range.start.byte, range.end.byte.min(body_len));
    if range.start.byte >= range.end.byte {
        // A cursor revives the ghost it touches.
        let at = range.start.byte;
        let ghost = ghost_bytes(&document)
            .into_iter()
            .find(|&(ghost_start, ghost_end)| ghost_start <= at && at <= ghost_end)
            .ok_or(GhostError::NothingToRevive)?;
        (byte_start, byte_end) = ghost;
    }
    let overlaps = ghost_bytes(&document)
        .iter()
        .any(|&(ghost_start, ghost_end)| ghost_start < byte_end && byte_start < ghost_end);
    if !overlaps {
        return Err(GhostError::NothingToRevive);
    }
    let start = byte_to_utf16(&document.body, byte_start);
    let end = byte_to_utf16(&document.body, byte_end);
    document.revive(start, end).map_err(refused)?;
    Ok(rewritten(text, &document))
}

fn open(text: &str) -> Result<Document, GhostError> {
    let mut document = parse(text).map_err(|error| GhostError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    match unresolved(&mut document) {
        0 => Ok(document),
        count => Err(GhostError::UnresolvedAnchors(count)),
    }
}

/// Re-anchor stored spans and ghosts against the body, as the editor does
/// on opening a file; the count of those that cannot be placed.
fn unresolved(document: &mut Document) -> usize {
    let report = document.reanchor();
    report.unresolved.len() + report.unresolved_ghosts.len()
}

fn rewritten(text: &str, document: &Document) -> GhostEdited {
    let body_len = document.body.len();
    let written = write(document);
    debug_assert!(written.starts_with(&document.body));
    GhostEdited {
        edit: TextEdit {
            range: TextRange::from_bytes(text, body_len, text.len()),
            new_text: written[body_len..].to_string(),
        },
    }
}

/// The range as UTF-16 offsets into the body, cut back to the body; `None`
/// when nothing is left.
fn utf16_in_body(document: &Document, range: TextRange) -> Option<(usize, usize)> {
    let end_byte = range.end.byte.min(document.body.len());
    if range.start.byte >= end_byte {
        return None;
    }
    Some((
        byte_to_utf16(&document.body, range.start.byte),
        byte_to_utf16(&document.body, end_byte),
    ))
}

fn byte_to_utf16(body: &str, byte: usize) -> usize {
    body[..byte].encode_utf16().count()
}

/// The ghosts as byte ranges of the (re-anchored) body, in order.
pub(crate) fn ghost_bytes(document: &Document) -> Vec<(usize, usize)> {
    let body = &document.body;
    let mut bytes: Vec<(usize, usize)> = document
        .annotations
        .ghosts
        .iter()
        .filter_map(|ghost| {
            let start = terraphim_alternatives::utf16_to_byte(body, ghost.anchor.start)?;
            let end = terraphim_alternatives::utf16_to_byte(body, ghost.anchor.end)?;
            (start < end).then_some((start, end))
        })
        .collect();
    bytes.sort_unstable();
    bytes
}

/// Whether the ghosts together cover `start..end`.
fn covered(ghosts: &[(usize, usize)], start: usize, end: usize) -> bool {
    let mut at = start;
    for &(ghost_start, ghost_end) in ghosts {
        if ghost_start <= at && at < ghost_end {
            at = ghost_end;
        }
    }
    at >= end
}

fn refused(error: EditError) -> GhostError {
    GhostError::Refused(error.to_string())
}
