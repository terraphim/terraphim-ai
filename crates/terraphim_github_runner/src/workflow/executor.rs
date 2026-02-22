//! Workflow execution with step-by-step snapshots
//!
//! Executes parsed workflows in a VM, creating snapshots after each successful step.

use crate::error::Result;
use crate::models::{ExecutionStatus, ExecutionStep, SnapshotId, WorkflowContext, WorkflowResult};
use crate::session::{Session, SessionManager, SessionState};
use crate::workflow::parser::{ParsedWorkflow, WorkflowStep};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Trait for executing commands in a VM
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a shell command in the VM
    async fn execute(
        &self,
        session: &Session,
        command: &str,
        timeout: Duration,
        working_dir: &str,
    ) -> Result<CommandResult>;

    /// Create a snapshot of the current VM state
    async fn create_snapshot(&self, session: &Session, name: &str) -> Result<SnapshotId>;

    /// Rollback to a previous snapshot
    async fn rollback(&self, session: &Session, snapshot_id: &SnapshotId) -> Result<()>;
}

/// Result of executing a single command
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
}

impl CommandResult {
    /// Check if command succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Mock command executor for testing
pub struct MockCommandExecutor {
    /// Simulated execution delay
    pub execution_delay: Duration,
    /// Commands that should fail (for testing)
    pub failing_commands: Vec<String>,
    /// Snapshot counter
    snapshot_counter: std::sync::atomic::AtomicU64,
}

impl Default for MockCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCommandExecutor {
    /// Create a new mock executor
    pub fn new() -> Self {
        Self {
            execution_delay: Duration::from_millis(10),
            failing_commands: Vec::new(),
            snapshot_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Create an executor where specific commands fail
    pub fn with_failures(commands: Vec<String>) -> Self {
        Self {
            execution_delay: Duration::from_millis(10),
            failing_commands: commands,
            snapshot_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl CommandExecutor for MockCommandExecutor {
    async fn execute(
        &self,
        _session: &Session,
        command: &str,
        _timeout: Duration,
        _working_dir: &str,
    ) -> Result<CommandResult> {
        // Simulate execution delay
        tokio::time::sleep(self.execution_delay).await;

        // Check if this command should fail
        let should_fail = self.failing_commands.iter().any(|c| command.contains(c));

        if should_fail {
            Ok(CommandResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Simulated failure for command: {}", command),
                duration: self.execution_delay,
            })
        } else {
            Ok(CommandResult {
                exit_code: 0,
                stdout: format!("Successfully executed: {}", command),
                stderr: String::new(),
                duration: self.execution_delay,
            })
        }
    }

    async fn create_snapshot(&self, _session: &Session, name: &str) -> Result<SnapshotId> {
        let count = self
            .snapshot_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(SnapshotId::new(format!("mock-snap-{}-{}", name, count)))
    }

    async fn rollback(&self, _session: &Session, _snapshot_id: &SnapshotId) -> Result<()> {
        Ok(())
    }
}

/// Configuration for workflow execution
#[derive(Debug, Clone)]
pub struct WorkflowExecutorConfig {
    /// Create snapshot after each successful step
    pub snapshot_on_success: bool,
    /// Automatically rollback on failure
    pub auto_rollback: bool,
    /// Stop execution on first failure
    pub stop_on_failure: bool,
    /// Default timeout for steps without explicit timeout
    pub default_timeout: Duration,
    /// Maximum total execution time
    pub max_execution_time: Duration,
}

impl Default for WorkflowExecutorConfig {
    fn default() -> Self {
        Self {
            snapshot_on_success: true,
            auto_rollback: true,
            stop_on_failure: true,
            default_timeout: Duration::from_secs(300),
            max_execution_time: Duration::from_secs(3600),
        }
    }
}

/// Executes parsed workflows with snapshot management
pub struct WorkflowExecutor {
    /// Command executor for running commands in VM
    command_executor: Arc<dyn CommandExecutor>,
    /// Session manager for VM lifecycle
    session_manager: Arc<SessionManager>,
    /// Execution configuration
    config: WorkflowExecutorConfig,
}

impl WorkflowExecutor {
    /// Create a new workflow executor with mock executor (for testing)
    pub fn new(session_manager: Arc<SessionManager>, config: WorkflowExecutorConfig) -> Self {
        Self {
            command_executor: Arc::new(MockCommandExecutor::new()),
            session_manager,
            config,
        }
    }

    /// Create a workflow executor with a custom command executor
    pub fn with_executor(
        command_executor: Arc<dyn CommandExecutor>,
        session_manager: Arc<SessionManager>,
        config: WorkflowExecutorConfig,
    ) -> Self {
        Self {
            command_executor,
            session_manager,
            config,
        }
    }

    /// Execute a complete workflow
    pub async fn execute_workflow(
        &self,
        workflow: &ParsedWorkflow,
        context: &WorkflowContext,
    ) -> Result<WorkflowResult> {
        let started_at = Utc::now();
        let mut executed_steps = Vec::new();
        let mut snapshots = Vec::new();
        let mut last_snapshot: Option<SnapshotId> = None;

        // Create or get session
        let session = self.session_manager.create_session(context).await?;

        log::info!(
            "Starting workflow '{}' for session {}",
            workflow.name,
            session.id
        );

        // Update session state to executing
        self.session_manager
            .update_session_state(&session.id, SessionState::Executing)?;

        // Run setup commands first
        for setup_cmd in &workflow.setup_commands {
            log::debug!("Running setup command: {}", setup_cmd);
            let result = self
                .command_executor
                .execute(
                    &session,
                    setup_cmd,
                    self.config.default_timeout,
                    "/workspace",
                )
                .await;

            if let Err(e) = result {
                log::error!("Setup command failed: {}", e);
                return self.build_failed_result(
                    &session.id,
                    executed_steps,
                    snapshots,
                    started_at,
                    format!("Setup failed: {}", e),
                );
            }

            let result = result.unwrap();
            if !result.success() {
                log::error!("Setup command failed with exit code {}", result.exit_code);
                return self.build_failed_result(
                    &session.id,
                    executed_steps,
                    snapshots,
                    started_at,
                    format!("Setup command failed: {}", result.stderr),
                );
            }
        }

        // Execute main workflow steps
        for (index, step) in workflow.steps.iter().enumerate() {
            log::info!(
                "Executing step {}/{}: {}",
                index + 1,
                workflow.steps.len(),
                step.name
            );

            let step_result =
                self.execute_step(&session, step, index, &mut last_snapshot, &mut snapshots);
            let step_result = step_result.await;

            match step_result {
                Ok(executed_step) => {
                    let step_succeeded = executed_step.status == ExecutionStatus::Success;
                    executed_steps.push(executed_step);

                    if !step_succeeded && self.config.stop_on_failure && !step.continue_on_error {
                        // Rollback if configured
                        if self.config.auto_rollback {
                            if let Some(ref snapshot_id) = last_snapshot {
                                log::info!("Rolling back to snapshot {}", snapshot_id);
                                let _ = self.command_executor.rollback(&session, snapshot_id).await;
                            }
                        }

                        return self.build_failed_result(
                            &session.id,
                            executed_steps,
                            snapshots,
                            started_at,
                            format!("Step '{}' failed", step.name),
                        );
                    }
                }
                Err(e) => {
                    log::error!("Step execution error: {}", e);
                    executed_steps.push(ExecutionStep {
                        id: Uuid::new_v4(),
                        name: step.name.clone(),
                        command: step.command.clone(),
                        status: ExecutionStatus::Failed,
                        exit_code: None,
                        stdout: String::new(),
                        stderr: e.to_string(),
                        duration_ms: 0,
                        snapshot_id: None,
                        started_at: Utc::now(),
                        completed_at: Some(Utc::now()),
                    });

                    if self.config.stop_on_failure && !step.continue_on_error {
                        // Rollback if configured
                        if self.config.auto_rollback {
                            if let Some(ref snapshot_id) = last_snapshot {
                                log::info!("Rolling back to snapshot {}", snapshot_id);
                                let _ = self.command_executor.rollback(&session, snapshot_id).await;
                            }
                        }

                        return self.build_failed_result(
                            &session.id,
                            executed_steps,
                            snapshots,
                            started_at,
                            format!("Step '{}' error: {}", step.name, e),
                        );
                    }
                }
            }
        }

        // Run cleanup commands (ignore failures)
        for cleanup_cmd in &workflow.cleanup_commands {
            log::debug!("Running cleanup command: {}", cleanup_cmd);
            let _ = self
                .command_executor
                .execute(
                    &session,
                    cleanup_cmd,
                    self.config.default_timeout,
                    "/workspace",
                )
                .await;
        }

        // Update session state to completed
        self.session_manager
            .update_session_state(&session.id, SessionState::Completed)?;

        let completed_at = Utc::now();
        let total_duration = (completed_at - started_at).num_milliseconds() as u64;

        log::info!(
            "Workflow '{}' completed successfully in {}ms",
            workflow.name,
            total_duration
        );

        Ok(WorkflowResult {
            session_id: session.id.clone(),
            success: true,
            steps: executed_steps,
            total_duration_ms: total_duration,
            final_snapshot: snapshots.last().cloned(),
            summary: format!(
                "Workflow '{}' completed successfully in {}ms",
                workflow.name, total_duration
            ),
            lessons: Vec::new(),
            suggestions: Vec::new(),
        })
    }

    /// Execute a single workflow step
    async fn execute_step(
        &self,
        session: &Session,
        step: &WorkflowStep,
        index: usize,
        last_snapshot: &mut Option<SnapshotId>,
        snapshots: &mut Vec<SnapshotId>,
    ) -> Result<ExecutionStep> {
        let timeout = Duration::from_secs(step.timeout_seconds);
        let start_time = std::time::Instant::now();

        // Execute the command
        let result = self
            .command_executor
            .execute(session, &step.command, timeout, &step.working_dir)
            .await?;

        let duration = start_time.elapsed();
        let success = result.success();

        // Create snapshot on success if configured
        let snapshot_id = if success && self.config.snapshot_on_success {
            let snapshot_name = format!("step-{}-{}", index, sanitize_name(&step.name));
            match self
                .command_executor
                .create_snapshot(session, &snapshot_name)
                .await
            {
                Ok(id) => {
                    log::debug!("Created snapshot {} after step '{}'", id, step.name);
                    // Record snapshot in session manager
                    self.session_manager.add_snapshot(&session.id, id.clone())?;
                    snapshots.push(id.clone());
                    *last_snapshot = Some(id.clone());
                    Some(id)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create snapshot after step '{}': {}",
                        step.name,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        let completed_at = Utc::now();
        Ok(ExecutionStep {
            id: Uuid::new_v4(),
            name: step.name.clone(),
            command: step.command.clone(),
            status: if success {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Failed
            },
            exit_code: Some(result.exit_code),
            stdout: result.stdout,
            stderr: result.stderr,
            duration_ms: duration.as_millis() as u64,
            snapshot_id,
            started_at: completed_at - chrono::Duration::milliseconds(duration.as_millis() as i64),
            completed_at: Some(completed_at),
        })
    }

    /// Build a failed workflow result
    fn build_failed_result(
        &self,
        session_id: &crate::models::SessionId,
        steps: Vec<ExecutionStep>,
        snapshots: Vec<SnapshotId>,
        started_at: chrono::DateTime<Utc>,
        error_message: String,
    ) -> Result<WorkflowResult> {
        // Update session state to failed
        let _ = self
            .session_manager
            .update_session_state(session_id, SessionState::Failed);

        let completed_at = Utc::now();
        let total_duration = (completed_at - started_at).num_milliseconds() as u64;

        Ok(WorkflowResult {
            session_id: session_id.clone(),
            success: false,
            steps,
            total_duration_ms: total_duration,
            final_snapshot: snapshots.last().cloned(),
            summary: format!("Workflow failed: {}", error_message),
            lessons: Vec::new(),
            suggestions: Vec::new(),
        })
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    /// Get the current configuration
    pub fn config(&self) -> &WorkflowExecutorConfig {
        &self.config
    }
}

/// Sanitize a name for use in snapshot identifiers
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GitHubEvent, GitHubEventType, RepositoryInfo};
    use crate::session::SessionManagerConfig;
    use std::collections::HashMap;

    fn create_test_event() -> GitHubEvent {
        GitHubEvent {
            event_type: GitHubEventType::PullRequest,
            action: Some("opened".to_string()),
            repository: RepositoryInfo {
                full_name: "test/repo".to_string(),
                clone_url: None,
                default_branch: Some("main".to_string()),
            },
            pull_request: None,
            git_ref: None,
            sha: Some("abc123".to_string()),
            extra: HashMap::new(),
        }
    }

    fn create_simple_workflow() -> ParsedWorkflow {
        ParsedWorkflow {
            name: "Test Workflow".to_string(),
            trigger: "push".to_string(),
            environment: HashMap::new(),
            setup_commands: vec!["echo setup".to_string()],
            steps: vec![
                WorkflowStep {
                    name: "Build".to_string(),
                    command: "cargo build".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                },
                WorkflowStep {
                    name: "Test".to_string(),
                    command: "cargo test".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                },
            ],
            cleanup_commands: vec!["echo cleanup".to_string()],
            cache_paths: vec![],
        }
    }

    #[tokio::test]
    async fn test_execute_workflow_success() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let executor = WorkflowExecutor::new(session_manager, WorkflowExecutorConfig::default());

        let workflow = create_simple_workflow();
        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.steps.len(), 2);
        // Should have a final snapshot from the last successful step
        assert!(result.final_snapshot.is_some());
        // Verify each step has a snapshot
        assert!(result.steps.iter().all(|s| s.snapshot_id.is_some()));
    }

    #[tokio::test]
    async fn test_execute_workflow_with_failure() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let mock_executor = Arc::new(MockCommandExecutor::with_failures(vec![
            "cargo test".to_string()
        ]));

        let executor = WorkflowExecutor::with_executor(
            mock_executor,
            session_manager,
            WorkflowExecutorConfig::default(),
        );

        let workflow = create_simple_workflow();
        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.summary.contains("failed"));
        // First step succeeded, second failed
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].status, ExecutionStatus::Success);
        assert_eq!(result.steps[1].status, ExecutionStatus::Failed);
        // First step should have snapshot, second shouldn't
        assert!(result.steps[0].snapshot_id.is_some());
        assert!(result.steps[1].snapshot_id.is_none());
    }

    #[tokio::test]
    async fn test_execute_workflow_continue_on_error() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let mock_executor = Arc::new(MockCommandExecutor::with_failures(vec![
            "cargo test".to_string()
        ]));

        let executor = WorkflowExecutor::with_executor(
            mock_executor,
            session_manager,
            WorkflowExecutorConfig {
                stop_on_failure: false,
                ..Default::default()
            },
        );

        let workflow = ParsedWorkflow {
            name: "Test Workflow".to_string(),
            trigger: "push".to_string(),
            environment: HashMap::new(),
            setup_commands: vec![],
            steps: vec![
                WorkflowStep {
                    name: "Build".to_string(),
                    command: "cargo build".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: true,
                    timeout_seconds: 300,
                },
                WorkflowStep {
                    name: "Test".to_string(),
                    command: "cargo test".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: true, // Continue even if this fails
                    timeout_seconds: 300,
                },
                WorkflowStep {
                    name: "Deploy".to_string(),
                    command: "deploy.sh".to_string(),
                    working_dir: "/workspace".to_string(),
                    continue_on_error: false,
                    timeout_seconds: 300,
                },
            ],
            cleanup_commands: vec![],
            cache_paths: vec![],
        };

        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        // All steps executed even though one failed
        assert_eq!(result.steps.len(), 3);
        assert_eq!(result.steps[0].status, ExecutionStatus::Success);
        assert_eq!(result.steps[1].status, ExecutionStatus::Failed);
        assert_eq!(result.steps[2].status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_setup_command_failure() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let mock_executor = Arc::new(MockCommandExecutor::with_failures(vec![
            "setup-fail".to_string()
        ]));

        let executor = WorkflowExecutor::with_executor(
            mock_executor,
            session_manager,
            WorkflowExecutorConfig::default(),
        );

        let workflow = ParsedWorkflow {
            name: "Test Workflow".to_string(),
            trigger: "push".to_string(),
            environment: HashMap::new(),
            setup_commands: vec!["setup-fail".to_string()],
            steps: vec![WorkflowStep {
                name: "Build".to_string(),
                command: "cargo build".to_string(),
                working_dir: "/workspace".to_string(),
                continue_on_error: false,
                timeout_seconds: 300,
            }],
            cleanup_commands: vec![],
            cache_paths: vec![],
        };

        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.summary.contains("Setup"));
        // No main steps executed
        assert_eq!(result.steps.len(), 0);
    }

    #[tokio::test]
    async fn test_snapshot_creation_on_success() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let executor = WorkflowExecutor::new(
            session_manager.clone(),
            WorkflowExecutorConfig {
                snapshot_on_success: true,
                ..Default::default()
            },
        );

        let workflow = create_simple_workflow();
        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        assert!(result.success);
        // Should have a final snapshot
        assert!(result.final_snapshot.is_some());

        // Verify each step has a snapshot_id
        for step in &result.steps {
            assert!(step.snapshot_id.is_some());
        }
    }

    #[tokio::test]
    async fn test_no_snapshot_when_disabled() {
        let session_manager = Arc::new(SessionManager::new(SessionManagerConfig::default()));
        let executor = WorkflowExecutor::new(
            session_manager,
            WorkflowExecutorConfig {
                snapshot_on_success: false,
                ..Default::default()
            },
        );

        let workflow = create_simple_workflow();
        let context = WorkflowContext::new(create_test_event());

        let result = executor
            .execute_workflow(&workflow, &context)
            .await
            .unwrap();

        assert!(result.success);
        // No final snapshot
        assert!(result.final_snapshot.is_none());
        // No snapshots on individual steps
        assert!(result.steps.iter().all(|s| s.snapshot_id.is_none()));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Build Step"), "build-step");
        assert_eq!(sanitize_name("Test 123!"), "test-123-");
        assert_eq!(sanitize_name("cargo-build"), "cargo-build");
        assert_eq!(
            sanitize_name("Step_With_Underscores"),
            "step-with-underscores"
        );
    }

    #[test]
    fn test_command_result_success() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
        };
        assert!(result.success());

        let failed_result = CommandResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            duration: Duration::from_millis(100),
        };
        assert!(!failed_result.success());
    }
}
