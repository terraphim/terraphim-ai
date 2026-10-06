//! Diagnostic helpers for Terraphim LSP.
//!
//! Converts [`KgAnalysis`] results into LSP diagnostics: the core's own
//! diagnostics (a malformed annotation block), ghost hints and, when the
//! `unknownTerms` setting is on, one warning per unknown-word occurrence.

use terraphim_lsp_core::LineIndex;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::convert;
use crate::kg_analysis::KgAnalysis;

/// Build LSP diagnostics for the unknown terms of an analysis without the
/// document text.
///
/// Every diagnostic is placed at the start of the document, because ranges
/// cannot be converted to LSP positions without the text. Prefer
/// [`unknown_term_diagnostics`], which reports each occurrence at its own
/// range.
pub fn build_diagnostics(analysis: &KgAnalysis) -> Vec<Diagnostic> {
    analysis
        .unknown_terms
        .iter()
        .map(|term| unknown_term_diagnostic(&term.text, Range::default()))
        .collect()
}

/// Unknown terms and core diagnostics, with ranges mapped to LSP positions.
///
/// The concatenation of [`unknown_term_diagnostics`] and
/// [`core_diagnostics`]; ghost hints are not included (see
/// [`ghost_diagnostics`]). The server itself only adds unknown terms when
/// the `unknownTerms` setting is on.
pub fn build_diagnostics_with_positions(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = unknown_term_diagnostics(analysis, text);
    diagnostics.extend(core_diagnostics(analysis, text));
    diagnostics
}

/// One warning per unknown-word occurrence, each at its own range, with
/// UTF-16 columns from the core's [`LineIndex`].
pub fn unknown_term_diagnostics(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);
    analysis
        .unknown_terms
        .iter()
        .map(|term| {
            let range = convert::byte_range(&index, term.bytes.start, term.bytes.end);
            unknown_term_diagnostic(&term.text, range)
        })
        .collect()
}

/// The core's diagnostics (such as a malformed annotation block) as LSP
/// diagnostics. Always published.
pub fn core_diagnostics(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);
    analysis
        .diagnostics
        .iter()
        .map(|diagnostic| convert::diagnostic(&index, diagnostic))
        .collect()
}

/// The analysis' ghost hints as LSP diagnostics: severity `Hint`, tagged
/// `Unnecessary` so clients fade the ghosted text.
pub fn ghost_diagnostics(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);
    analysis
        .ghosts
        .iter()
        .map(|diagnostic| convert::diagnostic(&index, diagnostic))
        .collect()
}

fn unknown_term_diagnostic(term: &str, range: Range) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: None,
        code_description: None,
        source: Some(convert::SOURCE.to_string()),
        message: format!("Unknown term: {}", term),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_lsp_core::KgEngine;
    use tower_lsp::lsp_types::Position;

    use crate::kg_analysis::UnknownTerm;

    /// An analysis whose unknown terms are every occurrence of `terms` in
    /// `text`, as the analysis itself records them.
    fn unknown(text: &str, terms: &[&str]) -> KgAnalysis {
        let lower = text.to_lowercase();
        let mut unknown_terms: Vec<UnknownTerm> = terms
            .iter()
            .flat_map(|term| {
                lower.match_indices(*term).map(|(start, _)| UnknownTerm {
                    text: text[start..start + term.len()].to_string(),
                    bytes: start..start + term.len(),
                })
            })
            .collect();
        unknown_terms.sort_by_key(|term| term.bytes.start);
        KgAnalysis {
            unknown_terms,
            ..KgAnalysis::empty()
        }
    }

    fn starts(diagnostics: &[Diagnostic]) -> Vec<(u32, u32, u32)> {
        diagnostics
            .iter()
            .map(|d| {
                (
                    d.range.start.line,
                    d.range.start.character,
                    d.range.end.character,
                )
            })
            .collect()
    }

    #[test]
    fn test_build_diagnostics_reports_unknown_terms() {
        let diagnostics = build_diagnostics(&unknown("xyz", &["xyz"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Unknown term: xyz");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_build_diagnostics_with_positions() {
        let diagnostics =
            build_diagnostics_with_positions(&unknown("rust and xyz", &["xyz"]), "rust and xyz");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 0);
        assert_eq!(diagnostics[0].range.start.character, 9);
        assert_eq!(diagnostics[0].range.end.character, 12);
    }

    #[test]
    fn test_positions_count_utf16_units() {
        let text = "😀 é\nab XYZ";
        let diagnostics = build_diagnostics_with_positions(&unknown(text, &["xyz"]), text);
        assert_eq!(diagnostics[0].range.start.line, 1);
        assert_eq!(diagnostics[0].range.start.character, 3);
        assert_eq!(diagnostics[0].range.end.character, 6);
        assert_eq!(diagnostics[0].message, "Unknown term: XYZ");
    }

    #[test]
    fn test_repeated_terms_are_reported_at_each_occurrence() {
        let text = "the cat\r\nsat on the 😀 mat, the end";
        let diagnostics = unknown_term_diagnostics(&unknown(text, &["the"]), text);
        assert_eq!(starts(&diagnostics), [(0, 0, 3), (1, 7, 10), (1, 19, 22)]);
    }

    #[test]
    fn test_malformed_block_becomes_one_lsp_diagnostic() {
        let text = "body\n\n```terraphim-alternatives\n{\n";
        let analysis = KgAnalysis {
            diagnostics: KgEngine::empty().analyse(text).diagnostics,
            ..KgAnalysis::empty()
        };
        let diagnostics = build_diagnostics_with_positions(&analysis, text);
        assert_eq!(diagnostics.len(), 1);
        let block = &diagnostics[0];
        assert_eq!(
            block.range.start,
            Position {
                line: 2,
                character: 0
            }
        );
        assert_eq!(
            block.code,
            Some(tower_lsp::lsp_types::NumberOrString::String(
                "annotation-block-truncated".to_string()
            ))
        );
        assert_eq!(block.source.as_deref(), Some("terraphim-lsp"));
    }
}
