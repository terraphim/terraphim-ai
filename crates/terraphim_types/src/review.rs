//! Review finding types for multi-agent code review.
//!
//! These types define the protocol for structured code review findings
//! exchanged between review agents. Previously lived in `terraphim_symphony::runner::protocol`.

use serde::{Deserialize, Serialize};

/// Severity level for a review finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    /// Informational note, no action required.
    Info,
    /// Minor issue with negligible impact.
    Low,
    /// Moderate issue that should be addressed before merging.
    Medium,
    /// Serious issue that must be resolved.
    High,
    /// Blocker: must be fixed immediately.
    Critical,
}

/// Category of a review finding (maps to review groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    /// Security vulnerability or exposure.
    Security,
    /// Architectural or structural concern.
    Architecture,
    /// Performance regression or inefficiency.
    Performance,
    /// Code quality issue (readability, duplication, etc.).
    Quality,
    /// Domain logic or business rule violation.
    Domain,
    /// Design quality issue (abstraction, coupling, cohesion).
    DesignQuality,
}

/// A single structured finding from a review agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFinding {
    /// Path to the file containing the finding.
    pub file: String,
    /// Line number within the file (0 if not applicable).
    #[serde(default)]
    pub line: u32,
    /// Severity of the finding.
    pub severity: FindingSeverity,
    /// Category the finding belongs to.
    pub category: FindingCategory,
    /// Human-readable description of the finding.
    pub finding: String,
    /// Optional actionable suggestion for resolving the finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Reviewer confidence score in `[0.0, 1.0]` (default 0.5).
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

/// Output schema for a single review agent's results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAgentOutput {
    /// Identifier of the review agent that produced this output.
    pub agent: String,
    /// List of findings from this agent.
    pub findings: Vec<ReviewFinding>,
    /// Human-readable summary of the review.
    pub summary: String,
    /// `true` if the review passed with no blocking findings.
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
