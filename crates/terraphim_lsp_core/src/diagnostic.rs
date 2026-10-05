//! Plain-data diagnostics, independent of any LSP crate.

use serde::{Deserialize, Serialize};
use terraphim_alternatives::BlockErrorKind;

use crate::offset::TextRange;

/// How serious a [`Diagnostic`] is. Mirrors the LSP severities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something is broken.
    Error,
    /// Something is probably wrong.
    Warning,
    /// Worth knowing.
    Information,
    /// A gentle suggestion.
    Hint,
}

/// Stable machine-readable identity of a [`Diagnostic`].
///
/// The annotation-block codes correspond one to one with
/// `terraphim_alternatives::BlockErrorKind`, the editor's parser errors;
/// see [`DiagnosticCode::for_block_error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCode {
    /// The annotation block's opening fence has no closing fence.
    AnnotationBlockTruncated,
    /// Non-whitespace text follows the annotation block's closing fence.
    AnnotationBlockNotTrailing,
    /// The annotation block's content does not parse as JSON.
    AnnotationBlockInvalidJson,
    /// The block's JSON has no integer `version` field.
    AnnotationBlockMissingVersion,
    /// The block's schema version is not one the parser understands.
    AnnotationBlockUnknownVersion,
    /// The block's JSON does not match the schema.
    AnnotationBlockInvalidSchema,
    /// Two spans or ghosts in the block share an id.
    AnnotationBlockDuplicateId,
    /// Two spans in the block overlap.
    AnnotationBlockOverlappingSpans,
    /// Two ghosts in the block overlap.
    AnnotationBlockOverlappingGhosts,
    /// A span in the block breaks a structural rule.
    AnnotationBlockInvalidSpan,
    /// A ghost in the block breaks a structural rule.
    AnnotationBlockInvalidGhost,
}

impl DiagnosticCode {
    /// The code for an annotation-block parse error.
    ///
    /// The match is exhaustive on purpose: a new `BlockErrorKind` in a
    /// future `terraphim_alternatives` fails to compile here until it gets
    /// its own code.
    pub fn for_block_error(kind: &BlockErrorKind) -> Self {
        match kind {
            BlockErrorKind::Truncated => Self::AnnotationBlockTruncated,
            BlockErrorKind::NotTrailing => Self::AnnotationBlockNotTrailing,
            BlockErrorKind::InvalidJson { .. } => Self::AnnotationBlockInvalidJson,
            BlockErrorKind::MissingVersion => Self::AnnotationBlockMissingVersion,
            BlockErrorKind::UnknownVersion { .. } => Self::AnnotationBlockUnknownVersion,
            BlockErrorKind::InvalidSchema { .. } => Self::AnnotationBlockInvalidSchema,
            BlockErrorKind::DuplicateId { .. } => Self::AnnotationBlockDuplicateId,
            BlockErrorKind::OverlappingSpans { .. } => Self::AnnotationBlockOverlappingSpans,
            BlockErrorKind::OverlappingGhosts { .. } => Self::AnnotationBlockOverlappingGhosts,
            BlockErrorKind::InvalidSpan { .. } => Self::AnnotationBlockInvalidSpan,
            BlockErrorKind::InvalidGhost { .. } => Self::AnnotationBlockInvalidGhost,
        }
    }

    /// The code as a string, as used in LSP `Diagnostic.code`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnnotationBlockTruncated => "annotation-block-truncated",
            Self::AnnotationBlockNotTrailing => "annotation-block-not-trailing",
            Self::AnnotationBlockInvalidJson => "annotation-block-invalid-json",
            Self::AnnotationBlockMissingVersion => "annotation-block-missing-version",
            Self::AnnotationBlockUnknownVersion => "annotation-block-unknown-version",
            Self::AnnotationBlockInvalidSchema => "annotation-block-invalid-schema",
            Self::AnnotationBlockDuplicateId => "annotation-block-duplicate-id",
            Self::AnnotationBlockOverlappingSpans => "annotation-block-overlapping-spans",
            Self::AnnotationBlockOverlappingGhosts => "annotation-block-overlapping-ghosts",
            Self::AnnotationBlockInvalidSpan => "annotation-block-invalid-span",
            Self::AnnotationBlockInvalidGhost => "annotation-block-invalid-ghost",
        }
    }
}

/// A problem found in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Where the problem is.
    pub range: TextRange,
    /// How serious it is.
    pub severity: Severity,
    /// What kind of problem it is.
    pub code: DiagnosticCode,
    /// Human-readable description.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_strings_match_serde() {
        let kinds = [
            BlockErrorKind::Truncated,
            BlockErrorKind::NotTrailing,
            BlockErrorKind::InvalidJson {
                line: 1,
                column: 1,
                message: String::new(),
            },
            BlockErrorKind::MissingVersion,
            BlockErrorKind::UnknownVersion { found: 9 },
            BlockErrorKind::InvalidSchema {
                message: String::new(),
            },
            BlockErrorKind::DuplicateId { id: String::new() },
            BlockErrorKind::OverlappingSpans {
                first: String::new(),
                second: String::new(),
            },
            BlockErrorKind::OverlappingGhosts {
                first: String::new(),
                second: String::new(),
            },
            BlockErrorKind::InvalidSpan {
                id: String::new(),
                reason: String::new(),
            },
            BlockErrorKind::InvalidGhost {
                id: String::new(),
                reason: String::new(),
            },
        ];
        let mut seen = std::collections::HashSet::new();
        for kind in &kinds {
            let code = DiagnosticCode::for_block_error(kind);
            assert!(seen.insert(code), "one code per kind: {kind:?}");
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
        }
    }
}
