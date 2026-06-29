//! Verdict Engine for Terraphim codebase evaluation.
//!
//! Compares baseline and candidate metrics captured by `scripts/evaluate-agent.sh`
//! and classifies the outcome as [`Verdict::Improved`], [`Verdict::Degraded`], or
//! [`Verdict::Neutral`].

use serde::{Deserialize, Serialize};

/// Metrics captured for one codebase snapshot (baseline or candidate).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    /// Number of `cargo test` failures (0 = all passing).
    pub test_failures: u32,
    /// Number of `cargo clippy` warnings emitted.
    pub clippy_warnings: u32,
    /// Number of `cargo clippy` errors emitted.
    pub clippy_errors: u32,
    /// Total tests executed.
    pub test_count: u32,
}

/// Per-metric delta between baseline and candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Delta {
    pub test_failures: i64,
    pub clippy_warnings: i64,
    pub clippy_errors: i64,
    pub test_count: i64,
}

impl Delta {
    pub fn compute(baseline: &Metrics, candidate: &Metrics) -> Self {
        Self {
            test_failures: candidate.test_failures as i64 - baseline.test_failures as i64,
            clippy_warnings: candidate.clippy_warnings as i64 - baseline.clippy_warnings as i64,
            clippy_errors: candidate.clippy_errors as i64 - baseline.clippy_errors as i64,
            test_count: candidate.test_count as i64 - baseline.test_count as i64,
        }
    }
}

/// Classification of the AI-agent's net effect on the codebase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Verdict {
    /// The candidate is strictly better: no new failures, fewer warnings/errors.
    Improved,
    /// The candidate introduced regressions (new test failures or errors).
    Degraded,
    /// The change is a wash — neither clearly better nor worse.
    Neutral,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Improved => write!(f, "Improved"),
            Verdict::Degraded => write!(f, "Degraded"),
            Verdict::Neutral => write!(f, "Neutral"),
        }
    }
}

/// Full evaluation report: inputs, deltas, and final verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub baseline: Metrics,
    pub candidate: Metrics,
    pub delta: Delta,
    pub verdict: Verdict,
    pub rationale: String,
}

/// Evaluate baseline vs candidate metrics and produce a report.
///
/// Rules (applied in order):
/// 1. New test failures or new clippy **errors** → **Degraded**.
/// 2. Fewer failures, fewer warnings, and no new errors → **Improved**.
/// 3. Otherwise → **Neutral**.
pub fn evaluate(baseline: &Metrics, candidate: &Metrics) -> EvalReport {
    let delta = Delta::compute(baseline, candidate);

    let (verdict, rationale) = classify(&delta);

    EvalReport {
        baseline: baseline.clone(),
        candidate: candidate.clone(),
        delta,
        verdict,
        rationale,
    }
}

fn classify(delta: &Delta) -> (Verdict, String) {
    // Rule 1: hard regressions.
    if delta.test_failures > 0 {
        return (
            Verdict::Degraded,
            format!(
                "Candidate introduced {} new test failure(s).",
                delta.test_failures
            ),
        );
    }
    if delta.clippy_errors > 0 {
        return (
            Verdict::Degraded,
            format!(
                "Candidate introduced {} new clippy error(s).",
                delta.clippy_errors
            ),
        );
    }

    // Rule 2: clear improvement.
    let fewer_failures = delta.test_failures <= 0;
    let fewer_warnings = delta.clippy_warnings <= 0;
    let no_new_errors = delta.clippy_errors <= 0;
    let more_tests = delta.test_count >= 0;

    if fewer_failures && fewer_warnings && no_new_errors && more_tests {
        // At least one metric must actually improve to call it Improved.
        if delta.test_failures < 0 || delta.clippy_warnings < 0 || delta.test_count > 0 {
            return (
                Verdict::Improved,
                format!(
                    "Candidate reduced failures by {}, warnings by {}, added {} test(s).",
                    -delta.test_failures, -delta.clippy_warnings, delta.test_count
                ),
            );
        }
    }

    // Rule 3: neutral.
    (
        Verdict::Neutral,
        format!(
            "Mixed signals: failures {:+}, warnings {:+}, errors {:+}.",
            delta.test_failures, delta.clippy_warnings, delta.clippy_errors
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> Metrics {
        Metrics {
            test_failures: 0,
            clippy_warnings: 5,
            clippy_errors: 0,
            test_count: 100,
        }
    }

    #[test]
    fn new_test_failure_is_degraded() {
        let candidate = Metrics {
            test_failures: 2,
            ..baseline()
        };
        let report = evaluate(&baseline(), &candidate);
        assert_eq!(report.verdict, Verdict::Degraded);
        assert!(report.rationale.contains("test failure"));
    }

    #[test]
    fn new_clippy_error_is_degraded() {
        let candidate = Metrics {
            clippy_errors: 1,
            ..baseline()
        };
        let report = evaluate(&baseline(), &candidate);
        assert_eq!(report.verdict, Verdict::Degraded);
        assert!(report.rationale.contains("clippy error"));
    }

    #[test]
    fn fewer_warnings_no_regressions_is_improved() {
        let candidate = Metrics {
            clippy_warnings: 2,
            test_count: 105,
            ..baseline()
        };
        let report = evaluate(&baseline(), &candidate);
        assert_eq!(report.verdict, Verdict::Improved);
    }

    #[test]
    fn identical_metrics_is_neutral() {
        let report = evaluate(&baseline(), &baseline());
        assert_eq!(report.verdict, Verdict::Neutral);
    }

    #[test]
    fn more_warnings_but_no_failures_is_neutral() {
        let candidate = Metrics {
            clippy_warnings: 8,
            ..baseline()
        };
        let report = evaluate(&baseline(), &candidate);
        assert_eq!(report.verdict, Verdict::Neutral);
    }

    #[test]
    fn delta_compute() {
        let b = Metrics {
            test_failures: 0,
            clippy_warnings: 5,
            clippy_errors: 0,
            test_count: 100,
        };
        let c = Metrics {
            test_failures: 1,
            clippy_warnings: 3,
            clippy_errors: 1,
            test_count: 102,
        };
        let d = Delta::compute(&b, &c);
        assert_eq!(d.test_failures, 1);
        assert_eq!(d.clippy_warnings, -2);
        assert_eq!(d.clippy_errors, 1);
        assert_eq!(d.test_count, 2);
    }

    #[test]
    fn verdict_display() {
        assert_eq!(Verdict::Improved.to_string(), "Improved");
        assert_eq!(Verdict::Degraded.to_string(), "Degraded");
        assert_eq!(Verdict::Neutral.to_string(), "Neutral");
    }

    #[test]
    fn eval_report_serialises_to_json() {
        let report = evaluate(
            &baseline(),
            &Metrics {
                clippy_warnings: 2,
                test_count: 103,
                ..baseline()
            },
        );
        let json = serde_json::to_string(&report).expect("serialise");
        assert!(json.contains("\"Improved\""));
    }
}
