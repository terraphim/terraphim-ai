//! Core types for merge-coordinator (#1805).
//!
//! Mirrors the spec at .docs/spec-merge-coordinator.md. Lib code in
//! follow-up commits will use these types in evaluate_all and
//! merge_and_close.

use std::fmt;

/// Exit code semantics per Operational-1 decision.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    EvaluationFailures = 1,
    Critical = 2,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExitCode::Success => write!(f, "success"),
            ExitCode::EvaluationFailures => write!(f, "evaluation_failures"),
            ExitCode::Critical => write!(f, "critical"),
        }
    }
}

/// Verdict produced when evaluating an open PR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalVerdict {
    /// Ready to merge; subagents reached PASS consensus.
    Merge,
    /// Not ready; reason captured for the log line.
    Hold(String),
    /// Subagent verdicts disagree (Edge-2 decision).
    Conflicting,
}

impl fmt::Display for EvalVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalVerdict::Merge => write!(f, "merge"),
            EvalVerdict::Hold(reason) => write!(f, "hold({reason})"),
            EvalVerdict::Conflicting => write!(f, "conflicting"),
        }
    }
}

/// Outcome of attempting a merge + auto-close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Merge succeeded and all referenced Fixes #N issues closed.
    Merged { closed_issues: Vec<u64> },
    /// Merge succeeded but at least one close call failed (Failure-1 path).
    PartialFailure {
        merged: bool,
        close_errors: Vec<u64>,
    },
    /// Skipped without attempting merge (e.g. Hold verdict).
    Skipped(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_repr_matches_spec() {
        assert_eq!(ExitCode::Success as i32, 0);
        assert_eq!(ExitCode::EvaluationFailures as i32, 1);
        assert_eq!(ExitCode::Critical as i32, 2);
    }

    #[test]
    fn exit_code_display() {
        assert_eq!(ExitCode::Success.to_string(), "success");
        assert_eq!(
            ExitCode::EvaluationFailures.to_string(),
            "evaluation_failures"
        );
        assert_eq!(ExitCode::Critical.to_string(), "critical");
    }

    #[test]
    fn eval_verdict_display() {
        assert_eq!(EvalVerdict::Merge.to_string(), "merge");
        assert_eq!(
            EvalVerdict::Hold("waiting for review".into()).to_string(),
            "hold(waiting for review)"
        );
        assert_eq!(EvalVerdict::Conflicting.to_string(), "conflicting");
    }

    #[test]
    fn merge_outcome_merged_carries_closed_issues() {
        let m = MergeOutcome::Merged {
            closed_issues: vec![1804, 1817],
        };
        match m {
            MergeOutcome::Merged { closed_issues } => assert_eq!(closed_issues, vec![1804, 1817]),
            _ => panic!("expected Merged"),
        }
    }
}

/// Error type for the merge-coordinator surface.
#[derive(Debug, thiserror::Error)]
pub enum MergeCoordinatorError {
    #[error("PID lock held by another instance (pid={pid}, age_secs={age_secs})")]
    LockHeld { pid: i32, age_secs: u64 },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Gitea API failure: {0}")]
    Api(String),
}

impl MergeCoordinatorError {
    /// Convenience constructor for API failures.
    pub fn api(s: impl Into<String>) -> Self {
        Self::Api(s.into())
    }
}
