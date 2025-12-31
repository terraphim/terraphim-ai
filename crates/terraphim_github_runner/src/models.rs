//! Core data types for GitHub runner operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a workflow execution session
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a VM snapshot
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// GitHub webhook event types we handle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GitHubEventType {
    /// Pull request opened or synchronized
    PullRequest,
    /// Push to a branch
    Push,
    /// Workflow dispatch (manual trigger)
    WorkflowDispatch,
    /// Unknown event type
    Unknown(String),
}

/// GitHub webhook event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubEvent {
    /// Event type
    pub event_type: GitHubEventType,
    /// Action within the event (e.g., "opened", "synchronize")
    pub action: Option<String>,
    /// Repository information
    pub repository: RepositoryInfo,
    /// Pull request details (if applicable)
    pub pull_request: Option<PullRequestInfo>,
    /// Git reference (branch/tag)
    pub git_ref: Option<String>,
    /// Commit SHA
    pub sha: Option<String>,
    /// Raw payload for additional data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Repository information from webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Full name (owner/repo)
    pub full_name: String,
    /// Clone URL
    pub clone_url: Option<String>,
    /// Default branch
    pub default_branch: Option<String>,
}

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    /// PR number
    pub number: u64,
    /// PR title
    pub title: String,
    /// PR URL
    pub html_url: String,
    /// Head branch
    pub head_branch: Option<String>,
    /// Base branch
    pub base_branch: Option<String>,
}

/// Context for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Unique session ID for this execution
    pub session_id: SessionId,
    /// The triggering GitHub event
    pub event: GitHubEvent,
    /// VM ID allocated for execution
    pub vm_id: Option<String>,
    /// Start time of execution
    pub started_at: DateTime<Utc>,
    /// Environment variables to inject
    pub env_vars: HashMap<String, String>,
    /// Working directory in VM
    pub working_dir: String,
    /// Accumulated snapshots during execution
    pub snapshots: Vec<SnapshotId>,
    /// Execution history for learning
    pub execution_history: Vec<ExecutionStep>,
}

impl WorkflowContext {
    /// Create a new workflow context from a GitHub event
    pub fn new(event: GitHubEvent) -> Self {
        Self {
            session_id: SessionId::new(),
            event,
            vm_id: None,
            started_at: Utc::now(),
            env_vars: HashMap::new(),
            working_dir: "/workspace".to_string(),
            snapshots: Vec::new(),
            execution_history: Vec::new(),
        }
    }

    /// Add a snapshot to the context
    pub fn add_snapshot(&mut self, snapshot_id: SnapshotId) {
        self.snapshots.push(snapshot_id);
    }

    /// Get the last snapshot ID
    pub fn last_snapshot(&self) -> Option<&SnapshotId> {
        self.snapshots.last()
    }

    /// Add an execution step to history
    pub fn add_execution_step(&mut self, step: ExecutionStep) {
        self.execution_history.push(step);
    }
}

/// A single execution step in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step identifier
    pub id: Uuid,
    /// Step name/description
    pub name: String,
    /// Command that was executed
    pub command: String,
    /// Execution status
    pub status: ExecutionStatus,
    /// Exit code (if completed)
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Snapshot taken after this step (if successful)
    pub snapshot_id: Option<SnapshotId>,
    /// When this step started
    pub started_at: DateTime<Utc>,
    /// When this step completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl ExecutionStep {
    /// Create a new pending execution step
    pub fn new(name: String, command: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            command,
            status: ExecutionStatus::Pending,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 0,
            snapshot_id: None,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Mark step as running
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Utc::now();
    }

    /// Mark step as completed
    pub fn complete(
        &mut self,
        exit_code: i32,
        stdout: String,
        stderr: String,
        snapshot_id: Option<SnapshotId>,
    ) {
        self.completed_at = Some(Utc::now());
        self.exit_code = Some(exit_code);
        self.stdout = stdout;
        self.stderr = stderr;
        self.snapshot_id = snapshot_id;

        if exit_code == 0 {
            self.status = ExecutionStatus::Success;
        } else {
            self.status = ExecutionStatus::Failed;
        }

        if let Some(completed) = self.completed_at {
            self.duration_ms = (completed - self.started_at).num_milliseconds() as u64;
        }
    }

    /// Check if step succeeded
    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Success
    }
}

/// Status of an execution step
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    RolledBack,
}

/// Result of a complete workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Session ID
    pub session_id: SessionId,
    /// Overall success status
    pub success: bool,
    /// All execution steps
    pub steps: Vec<ExecutionStep>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Final snapshot (if any)
    pub final_snapshot: Option<SnapshotId>,
    /// Summary message for GitHub comment
    pub summary: String,
    /// Lessons learned during execution
    pub lessons: Vec<String>,
    /// Suggestions for optimization
    pub suggestions: Vec<String>,
}

impl WorkflowResult {
    /// Create a successful result
    pub fn success(context: &WorkflowContext) -> Self {
        let total_duration = context
            .execution_history
            .iter()
            .map(|s| s.duration_ms)
            .sum();

        Self {
            session_id: context.session_id.clone(),
            success: true,
            steps: context.execution_history.clone(),
            total_duration_ms: total_duration,
            final_snapshot: context.last_snapshot().cloned(),
            summary: format!("Workflow completed successfully in {}ms", total_duration),
            lessons: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(context: &WorkflowContext, error: &str) -> Self {
        let total_duration = context
            .execution_history
            .iter()
            .map(|s| s.duration_ms)
            .sum();

        Self {
            session_id: context.session_id.clone(),
            success: false,
            steps: context.execution_history.clone(),
            total_duration_ms: total_duration,
            final_snapshot: context.last_snapshot().cloned(),
            summary: format!("Workflow failed: {}", error),
            lessons: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add a lesson learned
    pub fn add_lesson(&mut self, lesson: String) {
        self.lessons.push(lesson);
    }

    /// Add an optimization suggestion
    pub fn add_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    /// Format result for GitHub PR comment
    pub fn format_for_github(&self) -> String {
        let mut comment = String::new();

        // Status header
        let status_emoji = if self.success { "✅" } else { "❌" };
        comment.push_str(&format!("## {} Workflow Result\n\n", status_emoji));
        comment.push_str(&format!("{}\n\n", self.summary));

        // Steps table
        comment.push_str("### Execution Steps\n\n");
        comment.push_str("| Step | Status | Duration |\n");
        comment.push_str("|------|--------|----------|\n");

        for step in &self.steps {
            let status_icon = match step.status {
                ExecutionStatus::Success => "✅",
                ExecutionStatus::Failed => "❌",
                ExecutionStatus::Running => "🔄",
                ExecutionStatus::Skipped => "⏭️",
                ExecutionStatus::RolledBack => "↩️",
                ExecutionStatus::Pending => "⏳",
            };
            comment.push_str(&format!(
                "| {} | {} | {}ms |\n",
                step.name, status_icon, step.duration_ms
            ));
        }

        // Suggestions
        if !self.suggestions.is_empty() {
            comment.push_str("\n### Optimization Suggestions\n\n");
            for suggestion in &self.suggestions {
                comment.push_str(&format!("- {}\n", suggestion));
            }
        }

        comment.push_str(&format!(
            "\n---\n*Total duration: {}ms | Session: {}*\n",
            self.total_duration_ms, self.session_id
        ));

        comment
    }
}

/// Configuration for the GitHub runner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Whether the runner is enabled
    pub enabled: bool,
    /// VM type to use
    pub vm_type: String,
    /// Execution timeout in milliseconds
    pub execution_timeout_ms: u64,
    /// Create snapshot after each successful command
    pub snapshot_on_success: bool,
    /// Auto-rollback on failure
    pub auto_rollback: bool,
    /// Number of failures before creating a lesson
    pub lesson_threshold: u32,
    /// LLM model to use for workflow understanding
    pub llm_model: Option<String>,
    /// Maximum concurrent workflows
    pub max_concurrent_workflows: u32,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vm_type: "focal-optimized".to_string(),
            execution_timeout_ms: 30000,
            snapshot_on_success: true, // Per-command snapshots as decided
            auto_rollback: true,
            lesson_threshold: 3, // 3 failures before lesson as decided
            llm_model: None,
            max_concurrent_workflows: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_execution_step_lifecycle() {
        let mut step = ExecutionStep::new("Build".to_string(), "cargo build".to_string());
        assert_eq!(step.status, ExecutionStatus::Pending);

        step.start();
        assert_eq!(step.status, ExecutionStatus::Running);

        step.complete(0, "Built successfully".to_string(), String::new(), None);
        assert_eq!(step.status, ExecutionStatus::Success);
        assert!(step.is_success());
    }

    #[test]
    fn test_workflow_context() {
        let event = GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: RepositoryInfo {
                full_name: "owner/repo".to_string(),
                clone_url: None,
                default_branch: Some("main".to_string()),
            },
            pull_request: Some(PullRequestInfo {
                number: 123,
                title: "Test PR".to_string(),
                html_url: "https://github.com/owner/repo/pull/123".to_string(),
                head_branch: Some("feature".to_string()),
                base_branch: Some("main".to_string()),
            }),
            git_ref: None,
            sha: Some("abc123".to_string()),
            extra: HashMap::new(),
        };

        let mut ctx = WorkflowContext::new(event);
        assert!(ctx.vm_id.is_none());
        assert!(ctx.snapshots.is_empty());

        ctx.add_snapshot(SnapshotId::new("snap-1".to_string()));
        assert_eq!(ctx.last_snapshot().unwrap().0, "snap-1");
    }

    #[test]
    fn test_workflow_result_formatting() {
        let event = GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: RepositoryInfo {
                full_name: "owner/repo".to_string(),
                clone_url: None,
                default_branch: None,
            },
            pull_request: None,
            git_ref: None,
            sha: None,
            extra: HashMap::new(),
        };

        let ctx = WorkflowContext::new(event);
        let result = WorkflowResult::success(&ctx);

        let github_comment = result.format_for_github();
        assert!(github_comment.contains("✅"));
        assert!(github_comment.contains("Workflow completed successfully"));
    }

    #[test]
    fn test_runner_config_defaults() {
        let config = RunnerConfig::default();
        assert!(config.enabled);
        assert!(config.snapshot_on_success);
        assert_eq!(config.lesson_threshold, 3);
    }
}
