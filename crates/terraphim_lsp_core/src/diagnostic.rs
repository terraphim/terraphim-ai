//! Plain-data diagnostics, independent of any LSP crate.

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticCode {
    /// The annotation block's opening fence has no closing fence.
    AnnotationBlockTruncated,
    /// Non-whitespace text follows the annotation block's closing fence.
    AnnotationBlockNotTrailing,
    /// The annotation block's content does not parse as JSON.
    AnnotationBlockInvalidJson,
}

impl DiagnosticCode {
    /// The code as a string, as used in LSP `Diagnostic.code`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnnotationBlockTruncated => "annotation-block-truncated",
            Self::AnnotationBlockNotTrailing => "annotation-block-not-trailing",
            Self::AnnotationBlockInvalidJson => "annotation-block-invalid-json",
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
        for code in [
            DiagnosticCode::AnnotationBlockTruncated,
            DiagnosticCode::AnnotationBlockNotTrailing,
            DiagnosticCode::AnnotationBlockInvalidJson,
        ] {
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
        }
    }
}
