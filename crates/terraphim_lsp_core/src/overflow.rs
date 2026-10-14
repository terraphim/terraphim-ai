//! The overflow stash (R-6.3, R-6.4, R-6.5).
//!
//! Overflow is free text stored in the annotation block, never exported.
//! [`stash_range`] moves a selection out of the body to the end of it, and
//! [`pull_back`] copies one paragraph of it into the body at a position, as
//! the editor does: the overflow keeps the text, so it can be used again.
//! Both go through the editor's span model (`Document::apply_edit`), so
//! spans after the edit move exactly as the editor moves them, and through
//! its writer, so the rest of the block (spans, ghosts, the other overflow
//! text) is preserved. A pull back changes the block only when it moves a
//! span or ghost; otherwise its only edit is the body's.
//!
//! Overflow is a list of paragraphs: runs of non-blank lines, separated by
//! blank lines. A stash appends after a blank line. Text the writer typed
//! into overflow by hand keeps its own layout.
//!
//! The stashed text is the selection exactly (its surrounding line breaks
//! excepted): indentation and trailing spaces are the writer's.
//!
//! An edit that would change the text of a span is refused rather than
//! detaching the span.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::{Document, EditError, parse, write};

use crate::diagnostic::DiagnosticCode;
use crate::engine::TextEdit;
use crate::offset::TextRange;
use crate::reanchor::block_edit;

/// Why a stash or pull back could not be made. The document is never
/// changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverflowError {
    /// The annotation block is malformed, so it cannot be rewritten without
    /// losing what it holds.
    #[error("the annotation block is malformed ({}): {message}", code.as_str())]
    MalformedBlock {
        /// The block diagnostic's code.
        code: DiagnosticCode,
        /// The parser's message.
        message: String,
    },
    /// The range is empty once cut back to the body, or the position is
    /// outside it.
    #[error("the range must be non-empty text of the body, before the annotation block")]
    OutsideBody,
    /// The edit would change the text of an alternatives span.
    #[error("the edit would change a span with alternatives; clear its alternatives first")]
    TouchesSpan,
    /// Some stored spans or ghosts no longer match the body and cannot be
    /// re-anchored, so rewriting the block would drop them.
    #[error("{0} stored span(s) or ghost(s) no longer match the body; resolve them first")]
    UnresolvedAnchors(usize),
    /// There is no overflow paragraph with that index.
    #[error("there is no overflow paragraph {0}")]
    NoSuchParagraph(usize),
    /// The span model refused the edit.
    #[error("the span model refused the edit: {0}")]
    Refused(String),
}

/// The edits of a stash or a pull back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverflowEdited {
    /// Non-overlapping edits, in text order, with offsets into the text
    /// passed in: the body change and the block rewrite, merged into one
    /// when they touch.
    pub edits: Vec<TextEdit>,
}

/// One paragraph of overflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverflowParagraph {
    /// Position in the overflow, from 0; the index [`pull_back`] takes.
    pub index: usize,
    /// The paragraph's text, without its blank-line separators.
    pub text: String,
}

/// The paragraphs of `text`'s overflow, in order. Empty for a text with no
/// block, an empty overflow or a malformed block.
///
/// ```
/// use terraphim_lsp_core::overflow_paragraphs;
///
/// let text = "Body.\n\n```terraphim-alternatives\n\
///     {\"version\": 1, \"spans\": [], \"overflow\": \"One.\\n\\nTwo\\nlines.\"}\n```\n";
/// let paragraphs = overflow_paragraphs(text);
/// assert_eq!(paragraphs[1].text, "Two\nlines.");
/// ```
pub fn overflow_paragraphs(text: &str) -> Vec<OverflowParagraph> {
    let Ok(document) = parse(text) else {
        return Vec::new();
    };
    paragraph_ranges(&document.annotations.overflow)
        .into_iter()
        .enumerate()
        .map(|(index, (start, end))| OverflowParagraph {
            index,
            text: document.annotations.overflow[start..end].to_string(),
        })
        .collect()
}

/// Move `range` of `text` out of the body to the end of the overflow. The
/// selection is trimmed of surrounding line breaks only; one that ends its line
/// also takes the spaces before it, and one covering whole lines takes its
/// line break, so nothing untidy is left behind.
///
/// ```
/// use terraphim_lsp_core::{TextRange, apply_edits, overflow_paragraphs, stash_range};
///
/// let text = "Keep this. Move this.";
/// let range = TextRange::from_bytes(text, 11, 21);
/// let stashed = stash_range(text, range)?;
/// let saved = apply_edits(text, &stashed.edits);
/// assert!(saved.starts_with("Keep this.\n\n```terraphim-alternatives\n"));
/// assert_eq!(overflow_paragraphs(&saved)[0].text, "Move this.");
/// # Ok::<(), terraphim_lsp_core::OverflowError>(())
/// ```
pub fn stash_range(text: &str, range: TextRange) -> Result<OverflowEdited, OverflowError> {
    let mut document = open(text)?;
    let body_len = document.body.len();
    let (mut start, mut end) = (range.start.byte, range.end.byte.min(body_len));
    if start >= end {
        return Err(OverflowError::OutsideBody);
    }
    let body = document.body.clone();
    let selected = &body[start..end];
    let trimmed_start = start + (selected.len() - selected.trim_start_matches(['\r', '\n']).len());
    let trimmed_end = end - (selected.len() - selected.trim_end_matches(['\r', '\n']).len());
    if trimmed_start >= trimmed_end {
        return Err(OverflowError::OutsideBody);
    }
    (start, end) = (trimmed_start, trimmed_end);
    let moved = body[start..end].to_string();
    // A selection that ends its line also takes the spaces before it, and
    // one that starts and ends its line takes its line break too, so no
    // blank line or trailing space is left behind.
    let at_line_end = end == body.len() || body[end..].starts_with(['\r', '\n']);
    if at_line_end {
        start -= body[..start].len() - body[..start].trim_end_matches([' ', '\t']).len();
    }
    let at_line_start = start == 0 || body.as_bytes()[start - 1] == b'\n';
    if at_line_start && body[end..].starts_with("\r\n") {
        end += 2;
    } else if at_line_start && body[end..].starts_with('\n') {
        end += 1;
    } else if at_line_start && end == body.len() && start > 0 {
        // The last line: take the break before it instead.
        start -= if body[..start].ends_with("\r\n") {
            2
        } else {
            1
        };
    }
    let (utf16_start, utf16_end) = (utf16(&body, start), utf16(&body, end));
    let detached = document
        .apply_edit(utf16_start, utf16_end, "")
        .map_err(refused)?;
    if !detached.is_empty() {
        return Err(OverflowError::TouchesSpan);
    }
    append(&mut document.annotations.overflow, &moved);
    Ok(finish(text, body_len, &document, start, end, ""))
}

/// Copy paragraph `index` of the overflow into the body at byte offset `at`
/// (the cursor). The overflow is left as it is.
///
/// ```
/// use terraphim_lsp_core::{TextRange, apply_edits, overflow_paragraphs, pull_back, stash_range};
///
/// let text = "One. Two.";
/// let stashed = stash_range(text, TextRange::from_bytes(text, 5, 9))?;
/// let stashed = apply_edits(text, &stashed.edits);
/// let pulled = pull_back(&stashed, 0, 4)?;
/// let back = apply_edits(&stashed, &pulled.edits);
/// assert!(back.starts_with("One.\n\nTwo."));
/// assert_eq!(overflow_paragraphs(&back).len(), 1, "still in the overflow");
/// # Ok::<(), terraphim_lsp_core::OverflowError>(())
/// ```
pub fn pull_back(text: &str, index: usize, at: usize) -> Result<OverflowEdited, OverflowError> {
    let mut document = open(text)?;
    let body_len = document.body.len();
    if at > body_len || !document.body.is_char_boundary(at) {
        return Err(OverflowError::OutsideBody);
    }
    // Between the \r and the \n of a line break: after the whole break.
    let at =
        if at > 0 && document.body[..at].ends_with('\r') && document.body[at..].starts_with('\n') {
            at + 1
        } else {
            at
        };
    let ranges = paragraph_ranges(&document.annotations.overflow);
    let &(start, end) = ranges
        .get(index)
        .ok_or(OverflowError::NoSuchParagraph(index))?;
    let paragraph = separated(
        &document.body,
        at,
        &document.annotations.overflow[start..end],
    );
    // What the block says now, before re-anchoring refreshed anything.
    let stored = parse(text).ok().map(|stored| stored.annotations);
    let detached = document
        .apply_edit(
            utf16(&document.body, at),
            utf16(&document.body, at),
            &paragraph,
        )
        .map_err(refused)?;
    if !detached.is_empty() {
        return Err(OverflowError::TouchesSpan);
    }
    if stored.as_ref() == Some(&document.annotations) {
        // Nothing in the block moved: the body edit alone.
        return Ok(OverflowEdited {
            edits: vec![TextEdit {
                range: TextRange::from_bytes(text, at, at),
                new_text: paragraph,
            }],
        });
    }
    Ok(finish(text, body_len, &document, at, at, &paragraph))
}

fn open(text: &str) -> Result<Document, OverflowError> {
    let mut document = parse(text).map_err(|error| OverflowError::MalformedBlock {
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    })?;
    let report = document.reanchor();
    match report.unresolved.len() + report.unresolved_ghosts.len() {
        0 => Ok(document),
        count => Err(OverflowError::UnresolvedAnchors(count)),
    }
}

/// The edits taking `text` to `document`: `old_body_start..old_body_end` of
/// the old body became `inserted`, and the block follows.
fn finish(
    text: &str,
    old_body_len: usize,
    document: &Document,
    old_start: usize,
    old_end: usize,
    inserted: &str,
) -> OverflowEdited {
    let block = block_edit(text, old_body_len, document);
    let body_edit = TextEdit {
        range: TextRange::from_bytes(text, old_start, old_end),
        new_text: inserted.to_string(),
    };
    debug_assert!(write(document).starts_with(&document.body));
    // Edits that touch are merged, so no two edits share a position.
    let edits = if old_end >= old_body_len {
        vec![TextEdit {
            range: TextRange::from_bytes(text, old_start, text.len()),
            new_text: format!("{inserted}{}", block.new_text),
        }]
    } else {
        vec![body_edit, block]
    };
    OverflowEdited { edits }
}

/// `paragraph` as it is inserted into `body` at byte `at`. In the middle of
/// a line it goes in as it is. At the start or end of a line, or on a blank
/// line, it becomes a paragraph of its own: set off from the text before and
/// after it by one blank line (counting the blank lines already there), in
/// the document's line ending.
fn separated(body: &str, at: usize, paragraph: &str) -> String {
    let (before, after) = body.split_at(at);
    let at_start = before.is_empty() || before.ends_with('\n');
    let at_end = after.is_empty() || after.starts_with(['\r', '\n']);
    if !at_start && !at_end {
        return paragraph.to_string();
    }
    let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let lead = match (before.is_empty(), at_start) {
        (true, _) => 0,
        (false, true) => 2usize.saturating_sub(line_breaks(before.chars().rev())),
        (false, false) => 2,
    };
    let trail = match (after.is_empty(), at_end) {
        (true, _) => 0,
        (false, true) => 2usize.saturating_sub(line_breaks(after.chars())),
        (false, false) => 2,
    };
    format!("{}{paragraph}{}", eol.repeat(lead), eol.repeat(trail))
}

/// The line breaks at the start of `chars`, up to two (`\r` is part of a
/// `\r\n` break, not a break of its own).
fn line_breaks(chars: impl Iterator<Item = char>) -> usize {
    let mut breaks = 0;
    for c in chars {
        match c {
            '\n' => breaks += 1,
            '\r' => {}
            _ => break,
        }
        if breaks == 2 {
            break;
        }
    }
    breaks
}

/// Append `moved` to `overflow` after a blank line.
fn append(overflow: &mut String, moved: &str) {
    if !overflow.is_empty() {
        let trailing = overflow.len() - overflow.trim_end_matches(['\r', '\n']).len();
        let newlines = overflow[overflow.len() - trailing..]
            .chars()
            .filter(|&c| c == '\n')
            .count();
        for _ in newlines..2 {
            overflow.push('\n');
        }
    }
    overflow.push_str(moved);
}

/// The byte ranges of the paragraphs of `overflow`: runs of lines that are
/// not blank, without their line breaks at either end.
fn paragraph_ranges(overflow: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut current: Option<(usize, usize)> = None;
    let mut offset = 0;
    for line in overflow.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim().is_empty() {
            ranges.extend(current.take());
        } else {
            let start = current.map_or(offset, |(start, _)| start);
            current = Some((start, offset + content.len()));
        }
        offset += line.len();
    }
    ranges.extend(current);
    ranges
}

fn utf16(body: &str, byte: usize) -> usize {
    body[..byte].encode_utf16().count()
}

fn refused(error: EditError) -> OverflowError {
    OverflowError::Refused(error.to_string())
}
