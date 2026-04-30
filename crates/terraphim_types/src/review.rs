//! Review finding types for multi-agent code review.
//!
//! These types define the protocol for structured code review findings
//! exchanged between review agents. Previously lived in `terraphim_symphony::runner::protocol`.

use serde::{Deserialize, Serialize};

/// Severity level for a review finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Category of a review finding (maps to review groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Security,
    Architecture,
    Performance,
    Quality,
    Domain,
    DesignQuality,
}

/// A single structured finding from a review agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    /// Path to the file containing the finding, relative to the repository root.
    pub file: String,
    /// Line number within the file (0 means file-level finding).
    #[serde(default)]
    pub line: u32,
    /// How severe this finding is.
    pub severity: FindingSeverity,
    /// Which review category this finding belongs to.
    pub category: FindingCategory,
    /// Human-readable description of the issue.
    pub finding: String,
    /// Optional actionable suggestion for resolving the finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Agent confidence in this finding, in [0.0, 1.0]. Defaults to 0.5.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

/// Output schema for a single review agent's results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAgentOutput {
    /// Identifier of the agent that produced this output (e.g. `"security-audit"`).
    pub agent: String,
    /// Individual findings raised by this agent.
    pub findings: Vec<ReviewFinding>,
    /// Short prose summary of the review outcome.
    pub summary: String,
    /// `true` if the agent considers the changeset acceptable despite any findings.
    pub pass: bool,
}

/// Deduplicate findings by keeping the most severe finding per (file, line, category) key.
///
/// Results are sorted by severity (descending), then file, then line.
pub fn deduplicate_findings(findings: Vec<ReviewFinding>) -> Vec<ReviewFinding> {
    use std::collections::HashMap;
    let mut best: HashMap<(String, u32, FindingCategory), ReviewFinding> = HashMap::new();
    for finding in findings {
        let key = (finding.file.clone(), finding.line, finding.category);
        best.entry(key)
            .and_modify(|existing| {
                if finding.severity > existing.severity {
                    *existing = finding.clone();
                }
            })
            .or_insert(finding);
    }
    let mut result: Vec<ReviewFinding> = best.into_values().collect();
    #[allow(clippy::unnecessary_sort_by)]
    result.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    result
}
