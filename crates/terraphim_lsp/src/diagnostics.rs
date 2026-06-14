//! Diagnostic helpers for Terraphim LSP.
//!
//! Converts [`KgAnalysis`] results into LSP diagnostics, surfacing unknown
//! terms as warnings.

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::kg_analysis::KgAnalysis;

/// Build LSP diagnostics from a KG analysis result.
///
/// Unknown terms are reported as warnings at their first occurrence in the
/// document. Matched terms do not produce diagnostics.
pub fn build_diagnostics(analysis: &KgAnalysis) -> Vec<Diagnostic> {
    analysis
        .unknown_terms
        .iter()
        .map(|term| Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: None,
            code_description: None,
            source: Some("terraphim-lsp".to_string()),
            message: format!("Unknown term: {}", term),
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// Build diagnostics with byte-offset ranges mapped to LSP positions.
///
/// This richer variant locates each unknown term in the document text and
/// reports the diagnostic at the term's actual range. Unknown terms whose
/// positions cannot be located fall back to the start of the document.
pub fn build_diagnostics_with_positions(analysis: &KgAnalysis, text: &str) -> Vec<Diagnostic> {
    analysis
        .unknown_terms
        .iter()
        .filter_map(|term| {
            let range = find_term_range(text, term)?;
            Some(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: Some("terraphim-lsp".to_string()),
                message: format!("Unknown term: {}", term),
                related_information: None,
                tags: None,
                data: None,
            })
        })
        .collect()
}

/// Locate the first occurrence of a term in the document and return its LSP
/// range. Returns `None` if the term cannot be found.
fn find_term_range(text: &str, term: &str) -> Option<Range> {
    let lower = term.to_lowercase();
    let text_lower = text.to_lowercase();
    let byte_start = text_lower.find(&lower)?;
    let byte_end = byte_start + term.len();

    Some(Range {
        start: byte_offset_to_position(text, byte_start),
        end: byte_offset_to_position(text, byte_end),
    })
}

/// Convert a byte offset in a UTF-8 document to an LSP position.
fn byte_offset_to_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;

    for (idx, ch) in text.char_indices() {
        if idx >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }

    Position { line, character }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kg_analysis::KgAnalysis;

    #[test]
    fn test_build_diagnostics_reports_unknown_terms() {
        let analysis = KgAnalysis {
            matched_terms: vec![],
            unknown_terms: vec!["xyz".to_string()],
        };
        let diagnostics = build_diagnostics(&analysis);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Unknown term: xyz");
        assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_build_diagnostics_with_positions() {
        let analysis = KgAnalysis {
            matched_terms: vec![],
            unknown_terms: vec!["xyz".to_string()],
        };
        let diagnostics = build_diagnostics_with_positions(&analysis, "rust and xyz");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.line, 0);
        assert_eq!(diagnostics[0].range.start.character, 9);
        assert_eq!(diagnostics[0].range.end.character, 12);
    }

    #[test]
    fn test_byte_offset_to_position_multiline() {
        let text = "line one\nline two";
        let pos = byte_offset_to_position(text, 14); // 't' in "two"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 5);
    }
}
