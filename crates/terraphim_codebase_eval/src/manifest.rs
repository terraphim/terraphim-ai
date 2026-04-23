//! Manifest type definitions for the codebase evaluation flow.
//!
//! These types model the "Data Model" section of the
//! `terraphim-codebase-eval-check` specification.

use serde::{Deserialize, Serialize};

/// Whether a haystack represents the baseline or the candidate (post-change) state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineOrCandidate {
    /// The original state before changes.
    Baseline,
    /// The modified state after changes.
    Candidate,
}

/// Describes a haystack to be indexed for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaystackDescriptor {
    /// Unique identifier for this haystack.
    pub id: String,
    /// Filesystem path to the repository root.
    pub path: String,
    /// Git commit SHA for reproducibility.
    pub commit_sha: String,
    /// Whether this is the baseline or candidate.
    pub state: BaselineOrCandidate,
    /// Optional metadata (branch, timestamp, agent info).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Per-metric scoring weights for role-based evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for search score deltas.
    #[serde(default = "default_one")]
    pub search_score: f64,
    /// Weight for graph density changes.
    #[serde(default = "default_one")]
    pub graph_density: f64,
    /// Weight for entity count changes.
    #[serde(default = "default_one")]
    pub entity_count: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            search_score: default_one(),
            graph_density: default_one(),
            entity_count: default_one(),
        }
    }
}

fn default_one() -> f64 {
    1.0
}

/// Definition of a role for evaluation queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    /// Unique role identifier (e.g. "code-reviewer").
    pub role_id: String,
    /// Human-readable description of the role.
    pub description: String,
    /// Named term sets (Aho-Corasick dictionaries) for this role.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub term_sets: Vec<String>,
    /// Scoring weights for metrics under this role.
    #[serde(default)]
    pub scoring_weights: ScoringWeights,
}

/// Expected signal direction for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedSignal {
    /// The score should increase after changes.
    Increase,
    /// The score should decrease after changes.
    Decrease,
}

/// A single query to execute against a haystack for a given role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySpec {
    /// The search query text.
    pub query_text: String,
    /// The role to use when executing this query.
    pub role_id: String,
    /// Whether the signal should increase or decrease.
    pub expected_signal: ExpectedSignal,
    /// Minimum confidence threshold for the result to count.
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
}

fn default_confidence_threshold() -> f64 {
    0.5
}

/// Pass/fail result for a single metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassFail {
    Pass,
    Fail,
}

/// A single metric record capturing before/after values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRecord {
    /// Unique identifier for this metric.
    pub metric_id: String,
    /// The tool that produced this metric (e.g. "clippy", "cargo-test", "tokei").
    pub tool: String,
    /// Value before the change (baseline).
    #[serde(default)]
    pub value_before: Option<f64>,
    /// Value after the change (candidate).
    #[serde(default)]
    pub value_after: Option<f64>,
    /// Computed delta (value_after - value_before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    /// Whether the metric passed or failed its threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_fail: Option<PassFail>,
}

/// Global thresholds for verdict determination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    /// Score increase percentage to classify as "Improved".
    #[serde(default = "default_improved_threshold")]
    pub improved_pct: f64,
    /// Score decrease percentage to classify as "Degraded".
    #[serde(default = "default_degraded_pct")]
    pub degraded_pct: f64,
    /// Maximum number of new test failures before automatic "Degraded" verdict.
    #[serde(default = "default_critical_test_failures")]
    pub critical_test_failures: u32,
}

fn default_improved_threshold() -> f64 {
    10.0
}

fn default_degraded_pct() -> f64 {
    5.0
}

fn default_critical_test_failures() -> u32 {
    0
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            improved_pct: default_improved_threshold(),
            degraded_pct: default_degraded_pct(),
            critical_test_failures: default_critical_test_failures(),
        }
    }
}

/// The top-level evaluation manifest.
///
/// Contains all haystacks, role definitions, queries, and thresholds
/// needed to execute a before/after codebase evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationManifest {
    /// Haystacks to index (baseline and candidate).
    pub haystacks: Vec<HaystackDescriptor>,
    /// Role definitions for evaluation.
    pub roles: Vec<RoleDefinition>,
    /// Queries to execute for scoring.
    pub queries: Vec<QuerySpec>,
    /// Global verdict thresholds.
    #[serde(default)]
    pub thresholds: Thresholds,
}

impl EvaluationManifest {
    /// Validate the manifest for consistency.
    ///
    /// Checks that all query `role_id` references point to defined roles.
    ///
    /// # Errors
    ///
    /// Returns `ManifestError::Validation` if any query references an
    /// unknown role ID.
    pub fn validate(&self) -> Result<(), crate::ManifestError> {
        let role_ids: std::collections::HashSet<&str> =
            self.roles.iter().map(|r| r.role_id.as_str()).collect();

        for query in &self.queries {
            if !role_ids.contains(query.role_id.as_str()) {
                return Err(crate::ManifestError::Validation(format!(
                    "query '{}' references unknown role_id '{}'",
                    query.query_text, query.role_id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_toml() -> &'static str {
        r#"
[[haystacks]]
id = "baseline-1"
path = "/tmp/repo"
commit_sha = "abc123"
state = "baseline"

[[haystacks]]
id = "candidate-1"
path = "/tmp/repo"
commit_sha = "def456"
state = "candidate"

[[roles]]
role_id = "code-reviewer"
description = "Reviews code for bugs and maintainability"

[[queries]]
query_text = "highlight potential bugs"
role_id = "code-reviewer"
expected_signal = "increase"
confidence_threshold = 0.6
"#
    }

    #[test]
    fn toml_round_trip() {
        let manifest: EvaluationManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.haystacks.len(), 2);
        assert_eq!(manifest.roles.len(), 1);
        assert_eq!(manifest.queries.len(), 1);

        assert_eq!(manifest.haystacks[0].id, "baseline-1");
        assert_eq!(manifest.haystacks[0].state, BaselineOrCandidate::Baseline);
        assert_eq!(manifest.haystacks[1].state, BaselineOrCandidate::Candidate);

        assert_eq!(manifest.queries[0].role_id, "code-reviewer");
        assert_eq!(
            manifest.queries[0].expected_signal,
            ExpectedSignal::Increase
        );

        manifest.validate().unwrap();
    }

    #[test]
    fn toml_serialise_round_trip() {
        let original: EvaluationManifest = toml::from_str(minimal_toml()).unwrap();
        let serialised = toml::to_string(&original).unwrap();
        let deserialised: EvaluationManifest = toml::from_str(&serialised).unwrap();

        assert_eq!(original.haystacks.len(), deserialised.haystacks.len());
        assert_eq!(original.roles.len(), deserialised.roles.len());
        assert_eq!(original.queries.len(), deserialised.queries.len());
    }

    #[test]
    fn unknown_role_id_rejected() {
        let toml = r#"
[[haystacks]]
id = "b"
path = "/tmp"
commit_sha = "x"
state = "baseline"

[[roles]]
role_id = "other-role"
description = "exists"

[[queries]]
query_text = "test query"
role_id = "nonexistent-role"
expected_signal = "increase"
"#
        .trim();

        let manifest: EvaluationManifest = toml::from_str(toml).unwrap();
        let result = manifest.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("nonexistent-role"));
    }

    #[test]
    fn validation_passes_with_matching_roles() {
        let toml = r#"
[[haystacks]]
id = "b"
path = "/tmp"
commit_sha = "x"
state = "baseline"

[[roles]]
role_id = "reviewer"
description = "test"

[[queries]]
query_text = "find bugs"
role_id = "reviewer"
expected_signal = "increase"
"#
        .trim();

        let manifest: EvaluationManifest = toml::from_str(toml).unwrap();
        manifest.validate().unwrap();
    }

    #[test]
    fn metric_record_defaults() {
        let record = MetricRecord {
            metric_id: "m1".to_string(),
            tool: "clippy".to_string(),
            value_before: Some(5.0),
            value_after: Some(3.0),
            delta: Some(-2.0),
            pass_fail: Some(PassFail::Pass),
        };

        let toml = toml::to_string(&record).unwrap();
        let deserialised: MetricRecord = toml::from_str(&toml).unwrap();
        assert_eq!(deserialised.metric_id, "m1");
        assert_eq!(deserialised.value_before, Some(5.0));
        assert_eq!(deserialised.pass_fail, Some(PassFail::Pass));
    }

    #[test]
    fn thresholds_defaults() {
        let manifest: EvaluationManifest = toml::from_str(minimal_toml()).unwrap();
        assert_eq!(manifest.thresholds.improved_pct, 10.0);
        assert_eq!(manifest.thresholds.degraded_pct, 5.0);
        assert_eq!(manifest.thresholds.critical_test_failures, 0);
    }

    #[test]
    fn scoring_weights_defaults() {
        let toml = r#"
[[haystacks]]
id = "b"
path = "/tmp"
commit_sha = "x"
state = "baseline"

[[roles]]
role_id = "r"
description = "test"

[[queries]]
query_text = "find issues"
role_id = "r"
expected_signal = "increase"
"#
        .trim();

        let manifest: EvaluationManifest = toml::from_str(toml).unwrap();
        let weights = &manifest.roles[0].scoring_weights;
        assert_eq!(weights.search_score, 1.0);
        assert_eq!(weights.graph_density, 1.0);
        assert_eq!(weights.entity_count, 1.0);
    }

    #[test]
    fn load_from_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("manifest.toml");
        std::fs::write(&path, minimal_toml()).unwrap();

        let manifest = crate::load_manifest(&path).unwrap();
        assert_eq!(manifest.queries.len(), 1);
    }

    #[test]
    fn load_missing_file() {
        let result = crate::load_manifest(std::path::Path::new("/nonexistent/manifest.toml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::ManifestError::InvalidPath(_)
        ));
    }

    #[test]
    fn load_unsupported_format() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("manifest.json");
        std::fs::write(&path, "{}").unwrap();

        let result = crate::load_manifest(&path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::ManifestError::UnsupportedFormat(_)
        ));
    }

    #[test]
    fn load_fixture_toml() {
        let manifest =
            crate::load_manifest(std::path::Path::new("fixtures/manifest-minimal.toml")).unwrap();

        assert_eq!(manifest.haystacks.len(), 2);
        assert_eq!(manifest.roles.len(), 1);
        assert_eq!(manifest.queries.len(), 2);

        assert_eq!(manifest.roles[0].scoring_weights.search_score, 1.0);
        assert_eq!(manifest.roles[0].scoring_weights.graph_density, 0.8);

        manifest.validate().unwrap();
    }
}
