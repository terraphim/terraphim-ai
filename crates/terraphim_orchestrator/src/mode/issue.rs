//! Issue-driven mode controller.
//!
//! Polls Gitea/Linear for issues and dispatches agents to work on them.

use crate::{ConcurrencyController, DispatchTask, Dispatcher, WorkflowConfig};
use std::time::Duration;
use terraphim_tracker::{Issue, IssueTracker, PagerankClient};
use tracing::{error, info, warn};

/// Issue-driven mode controller.
pub struct IssueMode {
    /// Configuration.
    config: WorkflowConfig,
    /// Issue tracker client.
    tracker: Box<dyn IssueTracker>,
    /// PageRank client (optional).
    pagerank: Option<PagerankClient>,
    /// Dispatcher queue.
    dispatcher: Dispatcher,
    /// Concurrency controller.
    concurrency: ConcurrencyController,
    /// Set of running issue IDs to prevent duplicates.
    running: std::collections::HashSet<String>,
    /// Project id that owns the tracker driving this mode.
    project: String,
}

impl IssueMode {
    /// Create a new issue mode controller.
    pub fn new(
        config: WorkflowConfig,
        tracker: Box<dyn IssueTracker>,
        concurrency: ConcurrencyController,
    ) -> Self {
        Self::with_project(
            config,
            tracker,
            concurrency,
            crate::dispatcher::LEGACY_PROJECT_ID.to_string(),
        )
    }

    /// Create a new issue mode controller bound to a specific project id.
    pub fn with_project(
        config: WorkflowConfig,
        tracker: Box<dyn IssueTracker>,
        concurrency: ConcurrencyController,
        project: String,
    ) -> Self {
        let pagerank = if config.tracker.use_robot_api {
            Some(PagerankClient::new(
                &config.tracker.endpoint,
                &config.tracker.api_key,
            ))
        } else {
            None
        };

        Self {
            config,
            tracker,
            pagerank,
            dispatcher: Dispatcher::new(),
            concurrency,
            running: std::collections::HashSet::new(),
            project,
        }
    }

    /// Run the issue mode poll loop.
    pub async fn run(mut self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        let poll_interval = Duration::from_secs(self.config.poll_interval_secs);

        info!(
            poll_interval_secs = self.config.poll_interval_secs,
            "starting issue-driven mode"
        );

        loop {
            tokio::select! {
                _ = tokio::time::sleep(poll_interval) => {
                    if let Err(e) = self.poll_and_dispatch().await {
                        error!("poll cycle failed: {}", e);
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("shutting down issue-driven mode");
                        break;
                    }
                }
            }
        }
    }

    /// Poll for issues and dispatch agents.
    async fn poll_and_dispatch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Fetch candidate issues
        let mut issues = self.tracker.fetch_candidate_issues().await?;

        // Fetch PageRank scores if enabled
        if let Some(ref pagerank) = self.pagerank {
            match pagerank
                .fetch_ready(&self.config.tracker.owner, &self.config.tracker.repo)
                .await
            {
                Ok(ready) => {
                    PagerankClient::merge_scores(&mut issues, &ready.ready_issues);
                }
                Err(e) => {
                    warn!("failed to fetch PageRank scores: {}", e);
                }
            }
        }

        // Sort by priority and PageRank
        #[allow(clippy::unnecessary_sort_by)]
        issues.sort_by(|a, b| {
            let a_score = compute_sort_score(a);
            let b_score = compute_sort_score(b);
            a_score.cmp(&b_score)
        });

        // Filter blocked issues
        let active_states: Vec<String> = self
            .config
            .tracker
            .states
            .active
            .iter()
            .map(|s| s.to_lowercase())
            .collect();
        let terminal_states: Vec<String> = self
            .config
            .tracker
            .states
            .terminal
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        for issue in issues {
            // Skip if already running
            if self.running.contains(&issue.id) {
                continue;
            }

            // Skip if blocked
            if !issue.all_blockers_terminal(&terminal_states) {
                continue;
            }

            // Skip if not dispatchable
            if !issue.is_dispatchable() {
                continue;
            }

            // Check if issue state is active
            if !active_states
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&issue.state))
            {
                continue;
            }

            // Try to acquire concurrency slot for this project
            match self.concurrency.acquire_issue_driven(&self.project).await {
                Some(permit) => {
                    // Create dispatch task
                    let task = DispatchTask::IssueDriven {
                        identifier: issue.identifier.clone(),
                        title: issue.title.clone(),
                        priority: issue.priority,
                        pagerank_score: issue.pagerank_score,
                        project: self.project.clone(),
                    };

                    self.dispatcher.enqueue(task);
                    self.running.insert(issue.id.clone());

                    info!(
                        issue_id = %issue.id,
                        identifier = %issue.identifier,
                        priority = ?issue.priority,
                        pagerank = ?issue.pagerank_score,
                        "dispatched issue"
                    );

                    // Drop permit when agent completes (simplified)
                    drop(permit);
                }
                None => {
                    // No slots available, stop dispatching
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get dispatcher statistics.
    pub fn dispatcher_stats(&self) -> &crate::dispatcher::DispatcherStats {
        self.dispatcher.stats()
    }
}

/// Compute sort score (lower = higher priority).
fn compute_sort_score(issue: &Issue) -> i64 {
    // Base priority (lower = more urgent)
    let base = issue.priority.map(|p| p as i64 * 100).unwrap_or(500);

    // PageRank bonus (higher = more important = lower score)
    let pagerank_bonus = issue
        .pagerank_score
        .map(|pr| -(pr * 100.0) as i64)
        .unwrap_or(0);

    base + pagerank_bonus
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockTracker {
        issues: Vec<Issue>,
    }

    #[async_trait]
    impl IssueTracker for MockTracker {
        async fn fetch_candidate_issues(&self) -> terraphim_tracker::Result<Vec<Issue>> {
            Ok(self.issues.clone())
        }

        async fn fetch_issue_states_by_ids(
            &self,
            _ids: &[String],
        ) -> terraphim_tracker::Result<Vec<Issue>> {
            Ok(vec![])
        }

        async fn fetch_issues_by_states(
            &self,
            _states: &[String],
        ) -> terraphim_tracker::Result<Vec<Issue>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_mock_tracker_fetch_candidate_issues() {
        let tracker = MockTracker {
            issues: vec![Issue {
                id: "1".into(),
                identifier: "TEST-1".into(),
                title: "Test Issue".into(),
                description: None,
                priority: Some(1),
                state: "open".into(),
                branch_name: None,
                url: None,
                labels: vec![],
                blocked_by: vec![],
                pagerank_score: None,
                created_at: None,
                updated_at: None,
            }],
        };

        let issues = tracker.fetch_candidate_issues().await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].identifier, "TEST-1");
    }

    #[test]
    fn test_compute_sort_score() {
        let issue1 = Issue {
            id: "1".into(),
            identifier: "TEST-1".into(),
            title: "High priority".into(),
            description: None,
            priority: Some(1),
            state: "open".into(),
            branch_name: None,
            url: None,
            labels: vec![],
            blocked_by: vec![],
            pagerank_score: None,
            created_at: None,
            updated_at: None,
        };

        let issue2 = Issue {
            id: "2".into(),
            identifier: "TEST-2".into(),
            title: "Low priority".into(),
            description: None,
            priority: Some(4),
            state: "open".into(),
            branch_name: None,
            url: None,
            labels: vec![],
            blocked_by: vec![],
            pagerank_score: Some(2.5),
            created_at: None,
            updated_at: None,
        };

        // Issue1: priority 1 = score 100
        // Issue2: priority 4 = 400 - 250 (pagerank) = 150
        // So issue1 should have lower score (higher priority)
        assert!(compute_sort_score(&issue1) < compute_sort_score(&issue2));
    }
}
