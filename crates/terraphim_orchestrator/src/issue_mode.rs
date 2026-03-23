//! Issue mode controller for issue-driven task scheduling.
//!
//! Polls the issue tracker for ready issues and submits them to the dispatch queue.
//! Supports mapping issues to agents based on labels and title patterns.

use std::collections::HashSet;

use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

use terraphim_tracker::{
    GiteaTracker, IssueTracker, ListIssuesParams, TrackedIssue, TrackerConfig,
};

use crate::config::AgentDefinition;
use crate::dispatcher::{DispatchQueue, DispatchTask};
use crate::error::OrchestratorError;

/// Controller for issue-driven task scheduling.
pub struct IssueMode {
    /// The issue tracker client.
    tracker: GiteaTracker,
    /// Dispatch queue for submitting tasks.
    dispatch_queue: DispatchQueue,
    /// Agent definitions for issue-to-agent mapping.
    agents: Vec<AgentDefinition>,
    /// Poll interval in seconds.
    poll_interval_secs: u64,
    /// Channel for shutdown signals.
    shutdown_rx: mpsc::Receiver<()>,
    /// Set of currently running issue IDs (to avoid duplicates).
    running_issues: HashSet<u64>,
    /// Label-to-agent mapping rules.
    label_mappings: Vec<(String, String)>,
    /// Title pattern-to-agent mapping rules.
    pattern_mappings: Vec<(String, String)>,
}

impl IssueMode {
    /// Create a new issue mode controller.
    pub fn new(
        tracker_config: TrackerConfig,
        dispatch_queue: DispatchQueue,
        agents: Vec<AgentDefinition>,
        poll_interval_secs: u64,
    ) -> Result<(Self, mpsc::Sender<()>), OrchestratorError> {
        let tracker = GiteaTracker::new(tracker_config).map_err(|e| {
            OrchestratorError::Configuration(format!("Failed to create tracker: {}", e))
        })?;

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        // Default label mappings
        let label_mappings = vec![
            ("ADF".to_string(), "implementation-swarm".to_string()),
            ("security".to_string(), "security-sentinel".to_string()),
            ("bug".to_string(), "bug-hunter".to_string()),
            ("documentation".to_string(), "docs-writer".to_string()),
        ];

        // Default pattern mappings (regex patterns to agent names)
        let pattern_mappings = vec![
            (r"\[ADF\]".to_string(), "implementation-swarm".to_string()),
            (r"(?i)security".to_string(), "security-sentinel".to_string()),
            (
                r"(?i)documentation|docs".to_string(),
                "docs-writer".to_string(),
            ),
        ];

        Ok((
            Self {
                tracker,
                dispatch_queue,
                agents,
                poll_interval_secs,
                shutdown_rx,
                running_issues: HashSet::new(),
                label_mappings,
                pattern_mappings,
            },
            shutdown_tx,
        ))
    }

    /// Set custom label-to-agent mappings.
    pub fn with_label_mappings(mut self, mappings: Vec<(String, String)>) -> Self {
        self.label_mappings = mappings;
        self
    }

    /// Set custom title pattern-to-agent mappings.
    pub fn with_pattern_mappings(mut self, mappings: Vec<(String, String)>) -> Self {
        self.pattern_mappings = mappings;
        self
    }

    /// Run the issue mode polling loop.
    pub async fn run(mut self) {
        info!(
            "Starting issue mode controller with {}s poll interval",
            self.poll_interval_secs
        );

        let mut ticker = interval(Duration::from_secs(self.poll_interval_secs));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.poll_and_dispatch().await {
                        error!("Error polling issues: {}", e);
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!("Issue mode controller shutting down");
                    break;
                }
            }
        }
    }

    /// Poll for issues and dispatch tasks.
    async fn poll_and_dispatch(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Polling for ready issues");

        // Fetch open issues sorted by PageRank (via gitea-robot)
        let params = ListIssuesParams::new().with_state(terraphim_tracker::IssueState::Open);

        let issues = self.tracker.list_issues(params).await.map_err(|e| {
            Box::new(OrchestratorError::TrackerError(e.to_string()))
                as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Sort by PageRank score (highest first)
        let mut sorted_issues: Vec<_> = issues.into_iter().collect();
        sorted_issues.sort_by(|a, b| {
            b.page_rank_score
                .unwrap_or(0.0)
                .partial_cmp(&a.page_rank_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        info!("Found {} open issues", sorted_issues.len());

        for issue in sorted_issues {
            // Skip if already running
            if self.running_issues.contains(&issue.id) {
                debug!("Issue #{} already running, skipping", issue.id);
                continue;
            }

            // Check if issue is blocked (has blocking dependencies)
            if self.is_issue_blocked(&issue).await {
                debug!("Issue #{} is blocked, skipping", issue.id);
                continue;
            }

            // Map issue to agent
            let agent_name = self.map_issue_to_agent(&issue);

            if let Some(agent) = agent_name {
                // Create dispatch task
                let priority = self.calculate_priority(&issue);
                let task = DispatchTask::IssueTask(agent.clone(), issue.id, priority);

                // Submit to dispatch queue
                match self.dispatch_queue.submit(task) {
                    Ok(()) => {
                        info!(
                            "Submitted issue #{} to agent '{}' with priority {}",
                            issue.id, agent, priority
                        );
                        self.running_issues.insert(issue.id);
                    }
                    Err(e) => {
                        warn!("Failed to submit issue #{}: {}", issue.id, e);
                    }
                }
            } else {
                debug!("No agent mapping found for issue #{}", issue.id);
            }
        }

        // Clean up completed issues from running set
        self.cleanup_completed_issues().await;

        Ok(())
    }

    /// Check if an issue is blocked (has unresolved dependencies).
    async fn is_issue_blocked(&self, _issue: &TrackedIssue) -> bool {
        // TODO: Check for blocked dependencies via gitea-robot graph API
        // For now, assume no issues are blocked
        false
    }

    /// Map an issue to an agent based on labels and title patterns.
    fn map_issue_to_agent(&self, issue: &TrackedIssue) -> Option<String> {
        // First try label mappings
        for (label, agent) in &self.label_mappings {
            if issue.labels.iter().any(|l| l.eq_ignore_ascii_case(label)) {
                // Verify agent exists
                if self.agents.iter().any(|a| &a.name == agent) {
                    return Some(agent.clone());
                }
            }
        }

        // Then try title pattern mappings
        for (pattern, agent) in &self.pattern_mappings {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if regex.is_match(&issue.title) {
                    // Verify agent exists
                    if self.agents.iter().any(|a| &a.name == agent) {
                        return Some(agent.clone());
                    }
                }
            }
        }

        // Default: find first Growth-layer agent
        self.agents
            .iter()
            .find(|a| matches!(a.layer, crate::config::AgentLayer::Growth))
            .map(|a| a.name.clone())
    }

    /// Calculate priority for an issue (0-255, higher = more urgent).
    fn calculate_priority(&self, issue: &TrackedIssue) -> u8 {
        let mut priority = 50u8; // Base priority

        // Increase priority based on PageRank score
        if let Some(score) = issue.page_rank_score {
            priority += (score * 50.0) as u8; // Up to +50 for high PageRank
        }

        // Increase priority for security labels
        if issue
            .labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case("security"))
        {
            priority = priority.saturating_add(50);
        }

        // Increase priority for bug labels
        if issue.labels.iter().any(|l| l.eq_ignore_ascii_case("bug")) {
            priority = priority.saturating_add(30);
        }

        // Cap at 255
        priority.min(255)
    }

    /// Clean up completed issues from the running set.
    async fn cleanup_completed_issues(&mut self) {
        let mut to_remove = Vec::new();

        for issue_id in &self.running_issues {
            match self.tracker.get_issue(*issue_id).await {
                Ok(issue) => {
                    if issue.is_closed() {
                        to_remove.push(*issue_id);
                        info!("Issue #{} completed and closed", issue_id);
                    }
                }
                Err(e) => {
                    warn!("Failed to check status of issue #{}: {}", issue_id, e);
                }
            }
        }

        for issue_id in to_remove {
            self.running_issues.remove(&issue_id);
        }
    }

    /// Get the number of currently running issues.
    pub fn running_count(&self) -> usize {
        self.running_issues.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AgentDefinition, AgentLayer};

    fn create_test_agents() -> Vec<AgentDefinition> {
        vec![
            AgentDefinition {
                name: "implementation-swarm".to_string(),
                layer: AgentLayer::Growth,
                cli_tool: "opencode".to_string(),
                task: "Implement features".to_string(),
                model: None,
                schedule: None,
                capabilities: vec!["implementation".to_string()],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            },
            AgentDefinition {
                name: "security-sentinel".to_string(),
                layer: AgentLayer::Safety,
                cli_tool: "opencode".to_string(),
                task: "Security audit".to_string(),
                model: None,
                schedule: None,
                capabilities: vec!["security".to_string()],
                max_memory_bytes: None,
                provider: None,
                fallback_provider: None,
                fallback_model: None,
                provider_tier: None,
                persona_name: None,
                persona_symbol: None,
                persona_vibe: None,
                meta_cortex_connections: vec![],
                skill_chain: vec![],
            },
        ]
    }

    #[test]
    fn test_map_issue_to_agent_by_label() {
        let agents = create_test_agents();
        let queue = DispatchQueue::new(10);
        let tracker_config =
            TrackerConfig::new("https://git.example.com", "token", "owner", "repo");

        let (issue_mode, _) = IssueMode::new(tracker_config, queue, agents, 60).unwrap();

        // Create issue with ADF label
        let mut issue = TrackedIssue::new(1, "[ADF] Test issue");
        issue.labels = vec!["ADF".to_string()];

        let agent = issue_mode.map_issue_to_agent(&issue);
        assert_eq!(agent, Some("implementation-swarm".to_string()));

        // Create issue with security label
        let mut issue2 = TrackedIssue::new(2, "Security vulnerability");
        issue2.labels = vec!["security".to_string()];

        let agent2 = issue_mode.map_issue_to_agent(&issue2);
        assert_eq!(agent2, Some("security-sentinel".to_string()));
    }

    #[test]
    fn test_map_issue_to_agent_by_pattern() {
        let agents = create_test_agents();
        let queue = DispatchQueue::new(10);
        let tracker_config =
            TrackerConfig::new("https://git.example.com", "token", "owner", "repo");

        let (issue_mode, _) = IssueMode::new(tracker_config, queue, agents, 60).unwrap();

        // Create issue with [ADF] pattern in title
        let issue = TrackedIssue::new(1, "[ADF] Implement new feature");

        let agent = issue_mode.map_issue_to_agent(&issue);
        assert_eq!(agent, Some("implementation-swarm".to_string()));

        // Create issue with security pattern in title
        let issue2 = TrackedIssue::new(2, "SECURITY: Fix authentication bug");

        let agent2 = issue_mode.map_issue_to_agent(&issue2);
        assert_eq!(agent2, Some("security-sentinel".to_string()));
    }

    #[test]
    fn test_map_issue_default_to_growth_agent() {
        let agents = create_test_agents();
        let queue = DispatchQueue::new(10);
        let tracker_config =
            TrackerConfig::new("https://git.example.com", "token", "owner", "repo");

        let (issue_mode, _) = IssueMode::new(tracker_config, queue, agents, 60).unwrap();

        // Create issue with no matching labels or patterns
        let issue = TrackedIssue::new(1, "Some random issue");

        let agent = issue_mode.map_issue_to_agent(&issue);
        // Should default to first Growth-layer agent
        assert_eq!(agent, Some("implementation-swarm".to_string()));
    }

    #[test]
    fn test_calculate_priority_with_pagerank() {
        let agents = vec![];
        let queue = DispatchQueue::new(10);
        let tracker_config =
            TrackerConfig::new("https://git.example.com", "token", "owner", "repo");

        let (issue_mode, _) = IssueMode::new(tracker_config, queue, agents, 60).unwrap();

        // Issue with high PageRank
        let mut high_rank = TrackedIssue::new(1, "Important issue");
        high_rank.page_rank_score = Some(0.95);

        let priority = issue_mode.calculate_priority(&high_rank);
        assert!(priority > 50); // Should be higher than base

        // Issue with security label
        let mut security = TrackedIssue::new(2, "Security issue");
        security.labels = vec!["security".to_string()];

        let priority_sec = issue_mode.calculate_priority(&security);
        assert!(priority_sec >= 100); // Base 50 + security 50

        // Issue with bug label
        let mut bug = TrackedIssue::new(3, "Bug issue");
        bug.labels = vec!["bug".to_string()];

        let priority_bug = issue_mode.calculate_priority(&bug);
        assert!(priority_bug >= 80); // Base 50 + bug 30
    }

    #[test]
    fn test_priority_capped_at_255() {
        let agents = vec![];
        let queue = DispatchQueue::new(10);
        let tracker_config =
            TrackerConfig::new("https://git.example.com", "token", "owner", "repo");

        let (issue_mode, _) = IssueMode::new(tracker_config, queue, agents, 60).unwrap();

        // Issue with maximum PageRank and security label
        let mut max_priority = TrackedIssue::new(1, "Critical security");
        max_priority.page_rank_score = Some(1.0);
        max_priority.labels = vec!["security".to_string()];

        let priority = issue_mode.calculate_priority(&max_priority);
        assert!(priority <= 255);
    }
}
