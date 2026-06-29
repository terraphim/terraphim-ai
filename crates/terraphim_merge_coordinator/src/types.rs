//! Merge-coordinator shared types (#1805): exit codes, evaluation
//! verdicts, error variants, and merge outcomes.

use std::fmt;

/// Exit code semantics per Operational-1 decision.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// All PRs processed successfully.
    Success = 0,
    /// At least one PR evaluation failed but nothing catastrophic occurred.
    EvaluationFailures = 1,
    /// A merge succeeded but a subsequent close call failed (Failure-1 path).
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
    Merged {
        /// Issue numbers that were successfully closed after merging.
        closed_issues: Vec<u64>,
    },
    /// Merge succeeded but at least one close call failed (Failure-1 path).
    PartialFailure {
        /// Whether the merge itself succeeded (always `true` in this variant).
        merged: bool,
        /// Issue numbers whose close call failed after merging.
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

    #[test]
    fn blocker_kind_display_and_serde() {
        assert_eq!(BlockerKind::CiFailed.to_string(), "ci_failed");
        assert_eq!(BlockerKind::CiPending.to_string(), "ci_pending");
        assert_eq!(BlockerKind::CiNoStatus.to_string(), "ci_no_status");
        assert_eq!(BlockerKind::NotMergeable.to_string(), "not_mergeable");

        let json = serde_json::to_string(&BlockerKind::CiFailed).unwrap();
        assert_eq!(json, "\"ci_failed\"");
        let json = serde_json::to_string(&BlockerKind::CiPending).unwrap();
        assert_eq!(json, "\"ci_pending\"");
    }
}

/// Classifies why a PR is blocked from auto-merge.
///
/// Enables operators and ADF monitors to distinguish CI failures
/// (which may self-resolve on retry) from policy/confidence holds
/// (which need human intervention) at a glance in logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    /// CI status returned "failure" — the PR has a failing check.
    CiFailed,
    /// CI status returned "pending" — the check hasn't started or is running.
    CiPending,
    /// No CI status was found for the head commit.
    CiNoStatus,
    /// PR is not mergeable but CI is green — blocked by policy/confidence.
    NotMergeable,
}

impl fmt::Display for BlockerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockerKind::CiFailed => write!(f, "ci_failed"),
            BlockerKind::CiPending => write!(f, "ci_pending"),
            BlockerKind::CiNoStatus => write!(f, "ci_no_status"),
            BlockerKind::NotMergeable => write!(f, "not_mergeable"),
        }
    }
}

/// Error type for the merge-coordinator surface.
#[derive(Debug, thiserror::Error)]
pub enum MergeCoordinatorError {
    /// Another instance of the coordinator is running.
    #[error("PID lock held by another instance (pid={pid}, age_secs={age_secs})")]
    LockHeld {
        /// PID of the process holding the lock.
        pid: i32,
        /// Age of the lock file in seconds.
        age_secs: u64,
    },

    /// Underlying I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Gitea API returned an error or unexpected response.
    #[error("Gitea API failure: {0}")]
    Api(String),
}

impl MergeCoordinatorError {
    /// Convenience constructor for API failures.
    pub fn api(s: impl Into<String>) -> Self {
        Self::Api(s.into())
    }
}
