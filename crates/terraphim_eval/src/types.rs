//! Data model for the codebase evaluation metrics runner.
//!
//! Mirrors the "Metric Record" entity from
//! `docs/specifications/terraphim-codebase-eval-check.md` (§ Data Model):
//! a single normalised observation produced by one tool invocation.

use std::path::PathBuf;

/// A single normalised metric observation.
///
/// Produced by [`crate::run_clippy`] / [`crate::run_test`] and intended for
/// consumption by the (yet-unimplemented) Verdict Engine and Artifacts Store.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetricRecord {
    /// Stable identifier for the metric kind, e.g. `"clippy"`, `"test"`.
    pub metric_id: String,
    /// Name of the underlying tool, e.g. `"cargo-clippy"`, `"cargo-test"`.
    pub tool: String,
    /// Coarse pass/fail classification.
    pub pass_fail: PassFail,
    /// Normalised counts extracted from the tool's output.
    pub counts: MetricCounts,
    /// Raw process exit code, when a subprocess was invoked.
    pub raw_exit_code: Option<i32>,
    /// When the observation was taken.
    pub timestamp: jiff::Timestamp,
    /// Absolute path the tool was invoked in, for provenance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_dir: Option<PathBuf>,
}

impl MetricRecord {
    /// Construct a record with the given identity, counts, exit code, and dir.
    #[must_use]
    pub fn new(
        metric_id: impl Into<String>,
        tool: impl Into<String>,
        counts: MetricCounts,
        raw_exit_code: Option<i32>,
        manifest_dir: Option<PathBuf>,
    ) -> Self {
        let pass_fail = if counts.errors > 0 || counts.failed > 0 {
            PassFail::Fail
        } else {
            PassFail::Pass
        };
        Self {
            metric_id: metric_id.into(),
            tool: tool.into(),
            pass_fail,
            counts,
            raw_exit_code,
            timestamp: jiff::Timestamp::now(),
            manifest_dir,
        }
    }
}

/// Coarse pass/fail verdict for a single metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PassFail {
    /// The tool reported no errors/failures.
    Pass,
    /// The tool reported at least one error or test failure.
    Fail,
}

/// Normalised counts extracted from a tool's machine-readable output.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct MetricCounts {
    /// Linter warnings (clippy `warning`-level diagnostics).
    pub warnings: u64,
    /// Linter/compiler errors (clippy `error`-level diagnostics).
    pub errors: u64,
    /// Tests that passed.
    pub passed: u64,
    /// Tests that failed.
    pub failed: u64,
    /// Tests marked ignored / not run.
    pub ignored: u64,
}
