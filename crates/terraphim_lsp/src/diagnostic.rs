use terraphim_types::{FindingSeverity, ReviewFinding};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

pub fn finding_to_diagnostic(finding: &ReviewFinding) -> Diagnostic {
    let line = if finding.line == 0 {
        0
    } else {
        finding.line - 1
    };

    Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, u32::MAX)),
        severity: Some(severity_to_lsp(finding.severity)),
        source: Some("terraphim-edm".to_string()),
        message: finding.finding.clone(),
        code: None,
        code_description: None,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn severity_to_lsp(severity: FindingSeverity) -> DiagnosticSeverity {
    match severity {
        FindingSeverity::Critical | FindingSeverity::High => DiagnosticSeverity::ERROR,
        FindingSeverity::Medium => DiagnosticSeverity::WARNING,
        FindingSeverity::Low => DiagnosticSeverity::INFORMATION,
        FindingSeverity::Info => DiagnosticSeverity::HINT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::FindingCategory;

    fn test_finding(line: u32, severity: FindingSeverity) -> ReviewFinding {
        ReviewFinding {
            file: "test.rs".to_string(),
            line,
            severity,
            category: FindingCategory::Quality,
            finding: "test finding".to_string(),
            suggestion: None,
            confidence: 0.95,
        }
    }

    #[test]
    fn test_line_zero_maps_to_zero() {
        let diag = finding_to_diagnostic(&test_finding(0, FindingSeverity::High));
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.line, 0);
    }

    #[test]
    fn test_line_one_maps_to_zero() {
        let diag = finding_to_diagnostic(&test_finding(1, FindingSeverity::High));
        assert_eq!(diag.range.start.line, 0);
    }

    #[test]
    fn test_line_five_maps_to_four() {
        let diag = finding_to_diagnostic(&test_finding(5, FindingSeverity::High));
        assert_eq!(diag.range.start.line, 4);
    }

    #[test]
    fn test_critical_maps_to_error() {
        let diag = finding_to_diagnostic(&test_finding(1, FindingSeverity::Critical));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_high_maps_to_error() {
        let diag = finding_to_diagnostic(&test_finding(1, FindingSeverity::High));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_medium_maps_to_warning() {
        let diag = finding_to_diagnostic(&test_finding(1, FindingSeverity::Medium));
        assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_source_is_terraphim_edm() {
        let diag = finding_to_diagnostic(&test_finding(1, FindingSeverity::High));
        assert_eq!(diag.source.as_deref(), Some("terraphim-edm"));
    }
}
