//! Re-anchoring the annotation block against the body (R-9.2).
//!
//! Spans and ghosts are stored with their offsets and the text around them.
//! When the body was edited outside the editor (or the file was converted
//! between line endings), the stored offsets and context go stale.
//! [`reanchor_block`] finds every anchor again with the editor's own
//! re-anchoring, refreshes the stored context, and returns one edit that
//! rewrites only the block, or nothing when the block already says the
//! same. It is idempotent: applying its edit and running it again reports
//! [`ReanchorOutcome::Unchanged`].
//!
//! It never drops anything: if an anchor cannot be placed, the block is left
//! alone ([`ReanchorOutcome::Unresolved`]) because rewriting it would delete
//! the span or ghost.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{Document, parse, write};

use crate::diagnostic::DiagnosticCode;
use crate::engine::TextEdit;
use crate::offset::TextRange;

/// Why the block could not be re-anchored.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReanchorError {
    /// The annotation block is malformed, so it cannot be rewritten without
    /// losing what it holds.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
}

/// What re-anchoring a document's block found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReanchorOutcome {
    /// The stored offsets and context already match the body (or there is
    /// no block).
    Unchanged,
    /// One edit replacing everything after the body with the refreshed
    /// block. Offsets are into the text passed in.
    Rewritten(TextEdit),
    /// This many stored spans or ghosts cannot be placed in the body. Nothing
    /// is rewritten, so nothing is lost.
    Unresolved(usize),
}

/// Re-anchor the spans and ghosts of `text`'s block against its body and
/// refresh their stored context.
///
/// ```
/// use terraphim_lsp_core::{ReanchorOutcome, apply_edits, reanchor_block};
///
/// // The body gained a sentence at the front, so the stored offsets are stale.
/// let text = "New. Keep. Drop this.\n\n```terraphim-alternatives\n\
///     {\"version\": 1, \"spans\": [], \"ghosts\": [{\"id\": \"g1\", \
///     \"anchor\": {\"start\": 6, \"end\": 16, \"text\": \"Drop this.\"}}]}\n```\n";
/// let ReanchorOutcome::Rewritten(edit) = reanchor_block(text)? else { panic!() };
/// let saved = apply_edits(text, std::slice::from_ref(&edit));
/// assert!(saved.contains("\"start\": 11"));
/// assert_eq!(reanchor_block(&saved)?, ReanchorOutcome::Unchanged);
/// # Ok::<(), terraphim_lsp_core::ReanchorError>(())
/// ```
pub fn reanchor_block(text: &str) -> Result<ReanchorOutcome, ReanchorError> {
    let mut document = parse(text).map_err(|error| ReanchorError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    let before = document.annotations.clone();
    let report = document.reanchor();
    let unresolved = report.unresolved.len() + report.unresolved_ghosts.len();
    if unresolved > 0 {
        return Ok(ReanchorOutcome::Unresolved(unresolved));
    }
    document.refresh_context();
    if document.annotations == before {
        return Ok(ReanchorOutcome::Unchanged);
    }
    let body_len = document.body.len();
    Ok(ReanchorOutcome::Rewritten(block_edit(
        text, body_len, &document,
    )))
}

/// The edit that makes the text after the first `old_body_len` bytes of
/// `text` (its old block) the block `write` produces for `document`, whose
/// body may differ from the old one.
pub(crate) fn block_edit(text: &str, old_body_len: usize, document: &Document) -> TextEdit {
    let written = write(document);
    debug_assert!(written.starts_with(&document.body));
    TextEdit {
        range: TextRange::from_bytes(text, old_body_len, text.len()),
        new_text: written[document.body.len()..].to_string(),
    }
}
