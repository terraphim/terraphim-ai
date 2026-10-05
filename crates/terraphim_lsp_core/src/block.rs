//! Locating the trailing `terraphim-alternatives` annotation block.
//!
//! terraphim-editor persists what the knowledge graph cannot derive (human
//! alternatives, ghost flags, overflow) in a fenced block at the end of the
//! Markdown file:
//!
//! ~~~text
//! <body>
//! <blank line>
//! ```terraphim-alternatives
//! { JSON }
//! ```
//! ~~~
//!
//! KG analysis must never see that block, or JSON keys and stored
//! alternatives would show up as matches. [`split_annotation_block`] finds
//! where the body ends so analysis can stop there.
//!
//! # Boundary with the editor's parser
//!
//! The *location* rules are byte-for-byte those of terraphim-editor's
//! `terraphim_alternatives::block` module (the last line equal to
//! ```` ```terraphim-alternatives ```` after trimming trailing whitespace opens
//! the block; up to two newlines before it are a separator), so
//! [`BlockSplit::body_end`] always equals the length of the body the editor
//! parses, and `body + block == text`.
//!
//! Validation here stops at what any client can report without knowing the
//! schema: a missing closing fence, text after the closing fence, and JSON
//! that does not parse. Schema version, field types, ids and overlapping
//! spans remain the editor parser's job. A malformed block is still excluded
//! from analysis, yields exactly one [`Diagnostic`], and the body is left
//! unchanged: this crate never edits or rewrites the block.

use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::offset::{TextOffset, TextRange};

/// Info string identifying the annotation block's opening fence.
pub const FENCE_INFO: &str = "terraphim-alternatives";

const FENCE: &str = "```";

/// Where the analysable body of a document ends and the annotation block,
/// if any, begins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSplit {
    /// End of the body. Equal to the text length when there is no block.
    pub body_end: TextOffset,
    /// The annotation block, if the text has one (well-formed or not).
    pub block: Option<AnnotationBlock>,
}

/// The trailing annotation block of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationBlock {
    /// The whole block, from the end of the body (including the separator
    /// newlines) to the end of the text. Suitable for folding.
    pub range: TextRange,
    /// The opening fence line, without its line break.
    pub opening_fence: TextRange,
    /// Why the block is malformed, or `None` when it is well-formed (as far
    /// as this crate checks; see the module documentation).
    pub problem: Option<Diagnostic>,
}

/// Split `text` into its analysable body and trailing annotation block.
///
/// A text with no opening fence line is all body.
///
/// ```
/// use terraphim_lsp_core::split_annotation_block;
///
/// let text = "A choice.\n\n```terraphim-alternatives\n{\"version\": 1}\n```\n";
/// let split = split_annotation_block(text);
/// assert_eq!(&text[..split.body_end.byte], "A choice.");
/// assert!(split.block.unwrap().problem.is_none());
/// ```
pub fn split_annotation_block(text: &str) -> BlockSplit {
    let Some(open_start) = find_opening_fence(text) else {
        return BlockSplit {
            body_end: TextOffset::from_byte(text, text.len()),
            block: None,
        };
    };
    let body_end = strip_separator(&text[..open_start]);
    let open_line_end = text[open_start..]
        .find('\n')
        .map_or(text.len(), |newline| open_start + newline);
    let problem = check_block(text, open_start).map(|(code, message)| Diagnostic {
        range: TextRange::from_bytes(text, open_start, open_line_end),
        severity: Severity::Warning,
        code,
        message,
    });

    BlockSplit {
        body_end: TextOffset::from_byte(text, body_end),
        block: Some(AnnotationBlock {
            range: TextRange::from_bytes(text, body_end, text.len()),
            opening_fence: TextRange::from_bytes(text, open_start, open_line_end),
            problem,
        }),
    }
}

/// The first structural problem of the block opening at `open_start`.
fn check_block(text: &str, open_start: usize) -> Option<(DiagnosticCode, String)> {
    let truncated = || {
        Some((
            DiagnosticCode::AnnotationBlockTruncated,
            "annotation block is truncated: no closing fence".to_string(),
        ))
    };
    let Some(newline) = text[open_start..].find('\n') else {
        return truncated();
    };
    let content_start = open_start + newline + 1;

    let mut line_start = content_start;
    let mut close = None;
    for line in text[content_start..].split_inclusive('\n') {
        if is_fence_line(line, FENCE) {
            close = Some((line_start, line_start + line.len()));
            break;
        }
        line_start += line.len();
    }
    let Some((close_start, close_end)) = close else {
        return truncated();
    };
    if !text[close_end..].trim().is_empty() {
        return Some((
            DiagnosticCode::AnnotationBlockNotTrailing,
            "annotation block is not at the end of the document".to_string(),
        ));
    }
    let json = &text[content_start..close_start];
    serde_json::from_str::<serde::de::IgnoredAny>(json)
        .err()
        .map(|error| {
            (
                DiagnosticCode::AnnotationBlockInvalidJson,
                format!(
                    "annotation block is not valid JSON (line {}, column {}): {error}",
                    error.line(),
                    error.column()
                ),
            )
        })
}

/// Byte offset of the start of the last opening-fence line, if any.
fn find_opening_fence(text: &str) -> Option<usize> {
    let opening = format!("{FENCE}{FENCE_INFO}");
    let mut found = None;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        if is_fence_line(line, &opening) {
            found = Some(line_start);
        }
        line_start += line.len();
    }
    found
}

fn is_fence_line(line: &str, fence: &str) -> bool {
    line.trim_end() == fence
}

/// Length of `before` once the separator is removed: exactly `"\n\n"` when
/// present (so bodies ending in `\r\n` survive), otherwise up to two
/// hand-written newlines (`\n` or `\r\n`).
fn strip_separator(before: &str) -> usize {
    if let Some(stripped) = before.strip_suffix("\n\n") {
        return stripped.len();
    }
    let mut end = before.len();
    for _ in 0..2 {
        let rest = &before[..end];
        if let Some(stripped) = rest.strip_suffix('\n') {
            end = stripped.strip_suffix('\r').unwrap_or(stripped).len();
        } else {
            break;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "```terraphim-alternatives\n{\"version\": 1, \"spans\": []}\n```\n";

    #[test]
    fn text_without_block_is_all_body() {
        let split = split_annotation_block("just a choice");
        assert_eq!(split.body_end.byte, 13);
        assert!(split.block.is_none());
    }

    #[test]
    fn well_formed_block_is_split_after_the_separator() {
        let text = format!("A choice.\n\n{BLOCK}");
        let split = split_annotation_block(&text);
        assert_eq!(&text[..split.body_end.byte], "A choice.");
        let block = split.block.unwrap();
        assert_eq!(block.problem, None);
        assert_eq!(block.range.start, split.body_end);
        assert_eq!(block.range.end.byte, text.len());
        assert_eq!(
            &text[block.opening_fence.bytes()],
            "```terraphim-alternatives"
        );
    }

    #[test]
    fn crlf_body_and_lenient_fences() {
        let text = "body\r\n\r\n```terraphim-alternatives  \r\n{}\r\n```\r\n  \n";
        let split = split_annotation_block(text);
        assert_eq!(&text[..split.body_end.byte], "body");
        assert_eq!(split.block.unwrap().problem, None);
    }

    #[test]
    fn last_opening_fence_wins() {
        let text = format!("```terraphim-alternatives\nnot it\n```\nbody\n\n{BLOCK}");
        let split = split_annotation_block(&text);
        assert!(text[..split.body_end.byte].ends_with("body"));
    }

    #[test]
    fn truncated_block() {
        let text = "body\n\n```terraphim-alternatives\n{\"version\": 1}\n";
        let block = split_annotation_block(text).block.unwrap();
        let problem = block.problem.unwrap();
        assert_eq!(problem.code, DiagnosticCode::AnnotationBlockTruncated);
        assert_eq!(&text[problem.range.bytes()], "```terraphim-alternatives");
    }

    #[test]
    fn opening_fence_on_last_line_is_truncated() {
        let text = "body\n```terraphim-alternatives";
        let split = split_annotation_block(text);
        assert_eq!(&text[..split.body_end.byte], "body");
        let problem = split.block.unwrap().problem.unwrap();
        assert_eq!(problem.code, DiagnosticCode::AnnotationBlockTruncated);
    }

    #[test]
    fn text_after_the_block() {
        let text = format!("body\n\n{BLOCK}trailing words\n");
        let problem = split_annotation_block(&text)
            .block
            .unwrap()
            .problem
            .unwrap();
        assert_eq!(problem.code, DiagnosticCode::AnnotationBlockNotTrailing);
    }

    #[test]
    fn invalid_json() {
        let text = "body\n\n```terraphim-alternatives\n{\"version\": 1,\n```\n";
        let problem = split_annotation_block(text).block.unwrap().problem.unwrap();
        assert_eq!(problem.code, DiagnosticCode::AnnotationBlockInvalidJson);
        assert!(problem.message.contains("line 2"), "{}", problem.message);
    }

    #[test]
    fn separator_stripping_matches_the_editor() {
        assert_eq!(strip_separator("body\n\n"), 4);
        assert_eq!(strip_separator("body\r\n\n"), 5, "writer separator wins");
        assert_eq!(strip_separator("body\r\n\r\n"), 4);
        assert_eq!(strip_separator("body\n"), 4);
        assert_eq!(strip_separator("body"), 4);
        assert_eq!(strip_separator("body\n\n\n"), 5);
    }
}
