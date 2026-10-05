//! The trailing `terraphim-alternatives` annotation block.
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
//! KG analysis must never see that block, or stored alternatives would show
//! up as matches. [`split_annotation_block`] finds where the body ends so
//! analysis can stop there.
//!
//! # One parser
//!
//! Locating and validating the block is delegated entirely to the editor's
//! own parser, `terraphim_alternatives::parse`, so the editor and every LSP
//! client agree byte for byte: [`BlockSplit::body_end`] is the length of the
//! body that parser returns (`body + block == text`), and any
//! `terraphim_alternatives::BlockError` (fences, JSON, schema version, field
//! types, duplicate ids, overlapping spans or ghosts, invalid anchors)
//! yields exactly one [`Diagnostic`], with one [`DiagnosticCode`] per
//! `BlockErrorKind` and the parser's message. A malformed block is still
//! excluded from analysis and the body is left unchanged: this crate never
//! edits or rewrites the block.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::parse;

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::offset::{TextOffset, TextRange};

pub use terraphim_alternatives::FENCE_INFO;

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
    /// Why the editor's parser rejects the block, or `None` when it accepts
    /// it.
    pub problem: Option<Diagnostic>,
}

/// Split `text` into its analysable body and trailing annotation block.
///
/// A text with no opening fence line is all body.
///
/// ```
/// use terraphim_lsp_core::split_annotation_block;
///
/// let text = "A choice.\n\n```terraphim-alternatives\n\
///             {\"version\": 1, \"spans\": [], \"ghosts\": [], \"overflow\": \"notes\"}\n```\n";
/// let split = split_annotation_block(text);
/// assert_eq!(&text[..split.body_end.byte], "A choice.");
/// assert!(split.block.unwrap().problem.is_none());
/// ```
pub fn split_annotation_block(text: &str) -> BlockSplit {
    let (body_len, error) = match parse(text) {
        Ok(document) => (document.body.len(), None),
        Err(error) => (error.body.len(), Some(error)),
    };
    if body_len >= text.len() && error.is_none() {
        // The parser found no block: the whole text is body.
        return BlockSplit {
            body_end: TextOffset::from_byte(text, text.len()),
            block: None,
        };
    }

    // The block starts with the separator newlines the parser stripped from
    // the body; the opening fence is its first non-blank line.
    let raw_block = &text[body_len..];
    let fence_start =
        body_len + (raw_block.len() - raw_block.trim_start_matches(['\r', '\n']).len());
    let fence_line = text[fence_start..]
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r');
    let opening_fence = TextRange::from_bytes(text, fence_start, fence_start + fence_line.len());
    let problem = error.map(|error| Diagnostic {
        range: opening_fence,
        severity: Severity::Warning,
        code: DiagnosticCode::for_block_error(&error.kind),
        message: error.kind.to_string(),
    });

    BlockSplit {
        body_end: TextOffset::from_byte(text, body_len),
        block: Some(AnnotationBlock {
            range: TextRange::from_bytes(text, body_len, text.len()),
            opening_fence,
            problem,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: &str = "```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"ghosts\": [], \"overflow\": \"stash\"}\n```\n";

    fn problem_code(text: &str) -> DiagnosticCode {
        split_annotation_block(text)
            .block
            .expect("block located")
            .problem
            .expect("block rejected")
            .code
    }

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
        let text = "body\r\n\r\n```terraphim-alternatives  \r\n{\"version\": 1, \"spans\": [], \"overflow\": \"x\"}\r\n```\r\n  \n";
        let split = split_annotation_block(text);
        assert_eq!(&text[..split.body_end.byte], "body");
        let block = split.block.unwrap();
        assert_eq!(block.problem, None);
        assert_eq!(
            &text[block.opening_fence.bytes()],
            "```terraphim-alternatives  "
        );
    }

    #[test]
    fn an_empty_example_block_is_body_text() {
        // The editor's guard rule: a block with no annotations that the
        // writer did not produce is documentation, not annotations.
        let text = "Docs:\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"ghosts\": []}\n```\n";
        let split = split_annotation_block(text);
        assert_eq!(split.body_end.byte, text.len());
        assert!(split.block.is_none());
    }

    #[test]
    fn body_matches_the_editor_parser_even_when_malformed() {
        let text = format!("body\n\n{BLOCK}trailing words\n");
        let error = terraphim_alternatives::parse(&text).unwrap_err();
        let split = split_annotation_block(&text);
        assert_eq!(split.body_end.byte, error.body.len());
        assert_eq!(&text[split.body_end.byte..], error.raw_block);
    }

    #[test]
    fn structural_problems() {
        let truncated = "body\n\n```terraphim-alternatives\n{\"version\": 1}\n";
        assert_eq!(
            problem_code(truncated),
            DiagnosticCode::AnnotationBlockTruncated
        );
        let fence_only = "body\n```terraphim-alternatives";
        assert_eq!(
            problem_code(fence_only),
            DiagnosticCode::AnnotationBlockTruncated
        );
        let trailing = format!("body\n\n{BLOCK}trailing words\n");
        assert_eq!(
            problem_code(&trailing),
            DiagnosticCode::AnnotationBlockNotTrailing
        );
        let bad_json = "body\n\n```terraphim-alternatives\n{\"version\": 1,\n```\n";
        assert_eq!(
            problem_code(bad_json),
            DiagnosticCode::AnnotationBlockInvalidJson
        );
    }

    #[test]
    fn schema_problems_the_json_check_alone_missed() {
        let block = |json: &str| format!("body\n\n```terraphim-alternatives\n{json}\n```\n");
        assert_eq!(
            problem_code(&block("{\"spans\": []}")),
            DiagnosticCode::AnnotationBlockMissingVersion
        );
        assert_eq!(
            problem_code(&block("{\"version\": 99, \"spans\": []}")),
            DiagnosticCode::AnnotationBlockUnknownVersion
        );
        assert_eq!(
            problem_code(&block("{\"version\": 1, \"spans\": 3}")),
            DiagnosticCode::AnnotationBlockInvalidSchema
        );
    }

    #[test]
    fn diagnostic_sits_on_the_opening_fence_with_the_parser_message() {
        let text = "body\n\n```terraphim-alternatives\n{\"spans\": []}\n```\n";
        let problem = split_annotation_block(text).block.unwrap().problem.unwrap();
        assert_eq!(&text[problem.range.bytes()], "```terraphim-alternatives");
        assert_eq!(problem.severity, Severity::Warning);
        assert!(problem.message.contains("version"), "{}", problem.message);
    }
}
