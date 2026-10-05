//! Diagnostic helpers for Terraphim LSP.
//!
//! Converts [`KgAnalysis`] results into LSP diagnostics: unknown terms as
//! warnings, plus the core's own diagnostics (a malformed annotation block).

use terraphim_lsp_core::LineIndex;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::convert;
use crate::kg_analysis::KgAnalysis;

/// Build LSP diagnostics from a KG analysis result.
///
/// Unknown terms are reported as warnings at the start of the document.
/// Matched terms do not produce diagnostics. Prefer
/// [`build_diagnostics_with_positions`], which locates each term.
pub fn build_diagnostics(analysis: &KgAnalysis) -> Vec<Diagnostic> {
    analysis
        .unknown_terms
        .iter()
        .map(|term| unknown_term_diagnostic(term, Range::default()))
        .collect()
}

/// Build diagnostics with ranges mapped to LSP positions.
///
/// Each unknown term is reported at its first occurrence in the document;
/// terms that cannot be located are skipped. Core diagnostics (such as a
/// malformed annotation block) are always included.
pub fn build_diagnostics_with_positions(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    let index = LineIndex::new(text);
    let unknown = analysis.unknown_terms.iter().filter_map(|term| {
        let range = find_term_range(text, &index, term)?;
        Some(unknown_term_diagnostic(term, range))
    });
    let core = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| convert::diagnostic(&index, diagnostic));
    unknown.chain(core).collect()
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

/// Locate the first occurrence of a term in the document (ASCII
/// case-insensitively) and return its LSP range.
fn find_term_range(text: &str, index: &LineIndex<'_>, term: &str) -> Option<Range> {
    let byte_start = text.char_indices().map(|(start, _)| start).find(|&start| {
        text.get(start..start + term.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(term))
    })?;
    Some(convert::byte_range(
        index,
        byte_start,
        byte_start + term.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_lsp_core::KgEngine;
    use tower_lsp::lsp_types::Position;

    fn unknown(terms: &[&str]) -> KgAnalysis {
        KgAnalysis {
            unknown_terms: terms.iter().map(|t| t.to_string()).collect(),
            ..KgAnalysis::empty()
        }
    }

    #[test]
    fn test_build_diagnostics_reports_unknown_terms() {
        let diagnostics = build_diagnostics(&unknown(&["xyz"]));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Unknown term: xyz");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_build_diagnostics_with_positions() {
        let diagnostics = build_diagnostics_with_positions(&unknown(&["xyz"]), "rust and xyz");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 0);
        assert_eq!(diagnostics[0].range.start.character, 9);
        assert_eq!(diagnostics[0].range.end.character, 12);
    }

    #[test]
    fn test_positions_count_utf16_units() {
        let diagnostics = build_diagnostics_with_positions(&unknown(&["xyz"]), "😀 é\nab XYZ");
        assert_eq!(diagnostics[0].range.start.line, 1);
        assert_eq!(diagnostics[0].range.start.character, 3);
        assert_eq!(diagnostics[0].range.end.character, 6);
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
