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

/// Extra presentation hints on a [`Diagnostic`]. Mirrors the LSP
/// `DiagnosticTag`s this crate produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticTag {
    /// The text is unused or unnecessary: ghosted text, or a trim candidate.
    /// Clients fade it (Zed's `unnecessary_code_fade`) rather than underline
    /// it.
    Unnecessary,
}

/// Stable machine-readable identity of a [`Diagnostic`].
///
/// The annotation-block codes correspond one to one with
/// `terraphim_alternatives::BlockErrorKind`, the editor's parser errors;
/// see [`DiagnosticCode::for_block_error`]. [`DiagnosticCode::Ghosted`]
/// marks text the annotation block ghosts, the `Lab*` codes one Lab mark
/// kind each (see [`crate::code_for_mark`]) and
/// [`DiagnosticCode::TrimCandidate`] a span a trim level would cut.
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
    /// Body text the annotation block ghosts: kept in the file, dimmed in
    /// the editor and dropped on export. Tagged
    /// [`DiagnosticTag::Unnecessary`].
    Ghosted,
    /// Lab: a misspelling from the typo list (has a fix).
    LabTypo,
    /// Lab: a punctuation or spacing slip (has a fix).
    LabPunctuation,
    /// Lab: one of the weakest sentences.
    LabWeakSentence,
    /// Lab: a sentence that runs long.
    LabLongSentence,
    /// Lab: a sentence with a heavy clause structure.
    LabConvolutedSentence,
    /// Lab: a word or phrase outside the role's register.
    LabOffTone,
    /// Lab: a hedge.
    LabHedge,
    /// Lab: filler.
    LabFiller,
    /// A span the previewed trim level would cut. Tagged
    /// [`DiagnosticTag::Unnecessary`].
    TrimCandidate,
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
            Self::Ghosted => "ghosted",
            Self::LabTypo => "lab-typo",
            Self::LabPunctuation => "lab-punctuation",
            Self::LabWeakSentence => "lab-weak-sentence",
            Self::LabLongSentence => "lab-long-sentence",
            Self::LabConvolutedSentence => "lab-convoluted-sentence",
            Self::LabOffTone => "lab-off-tone",
            Self::LabHedge => "lab-hedge",
            Self::LabFiller => "lab-filler",
            Self::TrimCandidate => "trim-candidate",
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
    /// Presentation hints, such as [`DiagnosticTag::Unnecessary`] for
    /// ghosted text. Empty for most diagnostics, and omitted from the JSON
    /// when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<DiagnosticTag>,
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
        let block_codes = kinds.iter().map(DiagnosticCode::for_block_error);
        let other_codes = [
            DiagnosticCode::Ghosted,
            DiagnosticCode::LabTypo,
            DiagnosticCode::LabPunctuation,
            DiagnosticCode::LabWeakSentence,
            DiagnosticCode::LabLongSentence,
            DiagnosticCode::LabConvolutedSentence,
            DiagnosticCode::LabOffTone,
            DiagnosticCode::LabHedge,
            DiagnosticCode::LabFiller,
            DiagnosticCode::TrimCandidate,
        ];
        for code in block_codes.chain(other_codes) {
            assert!(seen.insert(code), "one code per kind: {code:?}");
            let json = serde_json::to_string(&code).unwrap();
            assert_eq!(json, format!("\"{}\"", code.as_str()));
        }
    }
}
