use terraphim_automata::{find_matches, load_thesaurus_from_json};
use terraphim_types::{
    FindingCategory, FindingSeverity, ReviewAgentOutput, ReviewFinding, Thesaurus,
};

use crate::exclusion::is_non_production;

const DEFAULT_EDM_TIER1_JSON: &str = include_str!("../data/edm_tier1.json");
const SUPPRESSION_MARKER: &str = "terraphim: allow(stub)";
const SCANNER_AGENT_NAME: &str = "edm-scanner";

#[derive(Debug, Clone)]
pub struct NegativeContributionScanner {
    thesaurus: Thesaurus,
}

impl NegativeContributionScanner {
    pub fn new() -> Self {
        let thesaurus = load_thesaurus_from_json(DEFAULT_EDM_TIER1_JSON)
            .expect("Failed to load embedded edm_tier1.json");
        Self { thesaurus }
    }

    pub fn from_thesaurus(thesaurus: Thesaurus) -> Self {
        Self { thesaurus }
    }

    pub fn scan_file(&self, path: &str, content: &str) -> Vec<ReviewFinding> {
        if is_non_production(path, content) {
            return Vec::new();
        }

        let line_starts = build_line_starts(content);

        let matches = match find_matches(content, self.thesaurus.clone(), true) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("EDM scan failed for {}: {e}", path);
                return Vec::new();
            }
        };

        let mut findings = Vec::new();

        for mat in &matches {
            let (start, end) = mat.pos.unwrap_or((0, 0));
            let line_number = byte_to_line(&line_starts, start);
            let line_content = get_line_content(content, &line_starts, line_number);

            if line_content.contains(SUPPRESSION_MARKER) {
                continue;
            }

            let (description, suggestion, severity) =
                parse_url_metadata(mat.normalized_term.url.as_deref(), &mat.term);

            findings.push(ReviewFinding {
                file: path.to_string(),
                line: line_number as u32,
                severity,
                category: FindingCategory::Quality,
                finding: format!("{description}: {}", &content[start..end.min(content.len())]),
                suggestion: Some(suggestion),
                confidence: 0.95,
            });
        }

        findings
    }

    pub fn scan_files(&self, files: &[(String, String)]) -> Vec<ReviewFinding> {
        files
            .iter()
            .flat_map(|(path, content)| self.scan_file(path, content))
            .collect()
    }

    pub fn scan_to_output(&self, files: &[(String, String)]) -> ReviewAgentOutput {
        let findings = self.scan_files(files);
        let pass = findings.is_empty();
        let summary = if pass {
            "No Explicit Deferral Markers detected.".to_string()
        } else {
            format!(
                "Found {} Explicit Deferral Marker(s): {}",
                findings.len(),
                findings
                    .iter()
                    .map(|f| format!("{}:{}", f.file, f.line))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };

        ReviewAgentOutput {
            agent: SCANNER_AGENT_NAME.to_string(),
            findings,
            summary,
            pass,
        }
    }

    pub fn thesaurus(&self) -> &Thesaurus {
        &self.thesaurus
    }
}

impl Default for NegativeContributionScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_url_metadata(url: Option<&str>, term: &str) -> (String, String, FindingSeverity) {
    let default_desc = format!("EDM pattern matched: {term}");
    let default_suggestion = "Replace with implementation".to_string();

    let Some(url) = url else {
        return (default_desc, default_suggestion, FindingSeverity::High);
    };

    let parts: Vec<&str> = url.split("||").collect();
    let description = parts.first().map(|s| s.to_string()).unwrap_or(default_desc);
    let suggestion = parts
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or(default_suggestion);
    let severity = match parts.get(2).copied() {
        Some("critical") => FindingSeverity::Critical,
        Some("high") => FindingSeverity::High,
        Some("medium") => FindingSeverity::Medium,
        Some("low") => FindingSeverity::Low,
        Some("info") => FindingSeverity::Info,
        _ => FindingSeverity::High,
    };

    (description, suggestion, severity)
}

fn build_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, byte) in content.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn byte_to_line(line_starts: &[usize], byte_offset: usize) -> usize {
    match line_starts.binary_search(&byte_offset) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    }
}

fn get_line_content<'a>(content: &'a str, line_starts: &[usize], line_number: usize) -> &'a str {
    let start_idx = line_number.saturating_sub(1);
    if start_idx >= line_starts.len() {
        return "";
    }
    let line_start = line_starts[start_idx];
    if line_start >= content.len() {
        return "";
    }
    let end = content[line_start..]
        .find('\n')
        .map(|i| line_start + i)
        .unwrap_or(content.len());
    &content[line_start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanner() -> NegativeContributionScanner {
        NegativeContributionScanner::new()
    }

    #[test]
    fn test_detect_todo() {
        let findings = scanner().scan_file("src/main.rs", "fn foo() {\n    todo!()\n}\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
        assert_eq!(findings[0].severity, FindingSeverity::High);
        assert_eq!(findings[0].category, FindingCategory::Quality);
        assert!(findings[0].finding.contains("todo!()"));
    }

    #[test]
    fn test_detect_unimplemented() {
        let findings = scanner().scan_file("src/main.rs", "fn foo() {\n    unimplemented!()\n}\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
    }

    #[test]
    fn test_detect_panic_not_implemented() {
        let findings = scanner().scan_file(
            "src/main.rs",
            "fn foo() {\n    panic!(\"not implemented\")\n}\n",
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
    }

    #[test]
    fn test_detect_panic_todo() {
        let findings = scanner().scan_file("src/main.rs", "fn foo() {\n    panic!(\"TODO\")\n}\n");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_detect_todo_with_message() {
        let findings = scanner().scan_file(
            "src/main.rs",
            "fn foo() {\n    todo!(\"implement later\")\n}\n",
        );
        assert_eq!(findings.len(), 1);
        assert!(findings[0].finding.contains("todo!"));
    }

    #[test]
    fn test_detect_unimplemented_with_message() {
        let findings = scanner().scan_file(
            "src/main.rs",
            "fn foo() {\n    unimplemented!(\"need auth\")\n}\n",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_suppression() {
        let findings = scanner().scan_file(
            "src/main.rs",
            "fn foo() {\n    todo!() // terraphim: allow(stub)\n}\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn test_suppression_does_not_affect_other_lines() {
        let findings = scanner().scan_file(
            "src/main.rs",
            "fn foo() {\n    todo!()\n}\n// terraphim: allow(stub)\n",
        );
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_excluded_tests_dir() {
        let findings =
            scanner().scan_file("tests/integration.rs", "fn test_foo() {\n    todo!()\n}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_excluded_examples_dir() {
        let findings = scanner().scan_file("examples/demo.rs", "fn main() {\n    todo!()\n}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_excluded_build_rs() {
        let findings = scanner().scan_file("build.rs", "fn main() {\n    todo!()\n}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_excluded_inline_test() {
        let findings = scanner().scan_file(
            "src/lib.rs",
            "fn foo() { todo!() }\n#[test]\nfn test_foo() {}\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn test_excluded_cfg_test() {
        let findings = scanner().scan_file(
            "src/lib.rs",
            "fn foo() { todo!() }\n#[cfg(test)]\nmod tests {}\n",
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn test_fixme_not_flagged() {
        let findings = scanner().scan_file("src/main.rs", "fn foo() {\n    // FIXME: broken\n}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_hack_not_flagged() {
        let findings =
            scanner().scan_file("src/main.rs", "fn foo() {\n    // HACK: temporary\n}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_multiple_patterns() {
        let content = "fn foo() {\n    todo!()\n}\nfn bar() {\n    unimplemented!()\n}\n";
        let findings = scanner().scan_file("src/main.rs", content);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_clean_file() {
        let content =
            "fn foo() -> i32 {\n    42\n}\nfn bar() -> String {\n    \"hello\".to_string()\n}\n";
        let findings = scanner().scan_file("src/main.rs", content);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_files_multiple() {
        let files = vec![
            ("src/a.rs".to_string(), "todo!()".to_string()),
            ("src/b.rs".to_string(), "fn main() {}".to_string()),
            ("src/c.rs".to_string(), "unimplemented!()".to_string()),
        ];
        let findings = scanner().scan_files(&files);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn test_scan_to_output_pass() {
        let output = scanner().scan_to_output(&[("src/lib.rs".into(), "fn main() {}".into())]);
        assert!(output.pass);
        assert!(output.summary.contains("No Explicit Deferral Markers"));
        assert!(output.findings.is_empty());
    }

    #[test]
    fn test_scan_to_output_fail() {
        let output = scanner().scan_to_output(&[("src/lib.rs".into(), "todo!()".into())]);
        assert!(!output.pass);
        assert_eq!(output.findings.len(), 1);
        assert_eq!(output.agent, "edm-scanner");
    }

    #[test]
    fn test_from_thesaurus_custom() {
        use terraphim_types::{NormalizedTerm, NormalizedTermValue};
        let mut t = terraphim_types::Thesaurus::new("custom".to_string());
        t.insert(
            NormalizedTermValue::from("FIXME_MAGIC"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("FIXME_MAGIC")),
        );
        let s = NegativeContributionScanner::from_thesaurus(t);
        let findings = s.scan_file("src/main.rs", "fn foo() { FIXME_MAGIC }");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_scanner_is_clone() {
        let s = scanner();
        let s2 = s.clone();
        let findings = s2.scan_file("src/main.rs", "todo!()");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_scanner_default() {
        let s = NegativeContributionScanner::default();
        let findings = s.scan_file("src/main.rs", "todo!()");
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_confidence_value() {
        let findings = scanner().scan_file("src/main.rs", "todo!()");
        assert!((findings[0].confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_suggestion_from_thesaurus() {
        let findings = scanner().scan_file("src/main.rs", "todo!()");
        assert_eq!(
            findings[0].suggestion.as_deref(),
            Some("Replace with implementation")
        );
    }

    #[test]
    fn test_parse_url_metadata() {
        let (desc, sugg, sev) =
            parse_url_metadata(Some("description||suggestion||high"), "todo!()");
        assert_eq!(desc, "description");
        assert_eq!(sugg, "suggestion");
        assert_eq!(sev, FindingSeverity::High);
    }

    #[test]
    fn test_parse_url_metadata_none() {
        let (desc, _, sev) = parse_url_metadata(None, "todo!()");
        assert!(desc.contains("todo!()"));
        assert_eq!(sev, FindingSeverity::High);
    }
}
