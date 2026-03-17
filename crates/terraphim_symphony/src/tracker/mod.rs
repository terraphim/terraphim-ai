//! Issue tracker abstraction and normalised issue model.
//!
//! Provides the [`IssueTracker`] trait and the [`Issue`] model that all tracker
//! implementations normalise to.

pub mod gitea;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A normalised issue from any tracker.
///
/// All tracker implementations must normalise their data into this model.
/// See Symphony spec Section 4.1.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Stable tracker-internal ID.
    pub id: String,
    /// Human-readable ticket key (e.g. `ABC-123`, `owner/repo#42`).
    pub identifier: String,
    /// Issue title.
    pub title: String,
    /// Issue body/description, if any.
    pub description: Option<String>,
    /// Priority (lower = higher priority). `None` means unset.
    pub priority: Option<i32>,
    /// Current tracker state name (e.g. "Todo", "In Progress").
    pub state: String,
    /// Tracker-provided branch name metadata, if any.
    pub branch_name: Option<String>,
    /// URL to the issue in the tracker UI.
    pub url: Option<String>,
    /// Labels, normalised to lowercase.
    pub labels: Vec<String>,
    /// Issues that block this one.
    pub blocked_by: Vec<BlockerRef>,
    /// PageRank score from dependency graph analysis. Higher = more downstream impact.
    pub pagerank_score: Option<f64>,
    /// Creation timestamp.
    pub created_at: Option<DateTime<Utc>>,
    /// Last update timestamp.
    pub updated_at: Option<DateTime<Utc>>,
}

/// A reference to a blocking issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockerRef {
    /// Tracker-internal ID, if known.
    pub id: Option<String>,
    /// Human-readable identifier, if known.
    pub identifier: Option<String>,
    /// Current state of the blocker, if known.
    pub state: Option<String>,
}

/// The issue tracker contract.
///
/// Implementations must support three operations used by the orchestrator:
/// candidate fetch (polling), state refresh (reconciliation), and
/// state-based fetch (startup cleanup).
#[async_trait]
pub trait IssueTracker: Send + Sync {
    /// Fetch issues in configured active states for the configured project.
    ///
    /// Used during poll ticks to find dispatch candidates.
    async fn fetch_candidate_issues(&self) -> crate::Result<Vec<Issue>>;

    /// Fetch current states for specific issue IDs.
    ///
    /// Used during reconciliation to check whether running issues are still active.
    /// Returns minimal issue records (at least `id` and `state`).
    async fn fetch_issue_states_by_ids(&self, ids: &[String]) -> crate::Result<Vec<Issue>>;

    /// Fetch issues currently in the given states.
    ///
    /// Used during startup terminal cleanup to find issues whose workspaces
    /// should be removed.
    async fn fetch_issues_by_states(&self, states: &[String]) -> crate::Result<Vec<Issue>>;
}

impl Issue {
    /// Check whether this issue has the minimum required fields for dispatch.
    pub fn is_dispatchable(&self) -> bool {
        !self.id.is_empty()
            && !self.identifier.is_empty()
            && !self.title.is_empty()
            && !self.state.is_empty()
    }

    /// Check whether all blockers are in terminal states.
    pub fn all_blockers_terminal(&self, terminal_states: &[String]) -> bool {
        self.blocked_by.iter().all(|b| {
            b.state
                .as_ref()
                .is_some_and(|s| terminal_states.iter().any(|t| t.eq_ignore_ascii_case(s)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_issue() -> Issue {
        Issue {
            id: "abc123".into(),
            identifier: "MT-42".into(),
            title: "Fix the widget".into(),
            description: Some("It is broken.".into()),
            priority: Some(1),
            state: "Todo".into(),
            branch_name: None,
            url: Some("https://example.com/MT-42".into()),
            labels: vec!["bug".into(), "p1".into()],
            blocked_by: vec![],
            pagerank_score: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }

    #[test]
    fn dispatchable_with_required_fields() {
        let issue = sample_issue();
        assert!(issue.is_dispatchable());
    }

    #[test]
    fn not_dispatchable_without_id() {
        let mut issue = sample_issue();
        issue.id = String::new();
        assert!(!issue.is_dispatchable());
    }

    #[test]
    fn not_dispatchable_without_state() {
        let mut issue = sample_issue();
        issue.state = String::new();
        assert!(!issue.is_dispatchable());
    }

    #[test]
    fn no_blockers_means_all_terminal() {
        let issue = sample_issue();
        assert!(issue.all_blockers_terminal(&["Done".into(), "Closed".into()]));
    }

    #[test]
    fn terminal_blockers_pass() {
        let mut issue = sample_issue();
        issue.blocked_by = vec![BlockerRef {
            id: Some("def456".into()),
            identifier: Some("MT-10".into()),
            state: Some("Done".into()),
        }];
        assert!(issue.all_blockers_terminal(&["Done".into(), "Closed".into()]));
    }

    #[test]
    fn non_terminal_blockers_fail() {
        let mut issue = sample_issue();
        issue.blocked_by = vec![BlockerRef {
            id: Some("def456".into()),
            identifier: Some("MT-10".into()),
            state: Some("In Progress".into()),
        }];
        assert!(!issue.all_blockers_terminal(&["Done".into(), "Closed".into()]));
    }

    #[test]
    fn blocker_state_comparison_is_case_insensitive() {
        let mut issue = sample_issue();
        issue.blocked_by = vec![BlockerRef {
            id: None,
            identifier: None,
            state: Some("done".into()),
        }];
        assert!(issue.all_blockers_terminal(&["Done".into()]));
    }
}
