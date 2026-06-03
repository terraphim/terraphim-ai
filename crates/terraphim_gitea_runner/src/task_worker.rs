//! End-to-end task execution: compile -> policy -> host execution -> logs -> result.

use crate::client::GiteaRunnerClient;
use crate::logs::LogStreamer;
use crate::policy::PolicyPlanner;
use crate::state::RunnerState;
use crate::types::{Task, TaskState, UpdateTaskRequest, result};
use crate::{Result, RunnerError, workflow_payload};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_github_runner::{
    HostCommandExecutor, HostVmProvider, SessionId, SessionManager, SessionManagerConfig,
    SessionStartSpec, WorkflowExecutor, WorkflowExecutorConfig,
};

/// Executes a single fetched task through the reused host stack under policy.
pub struct TaskWorker<C: GiteaRunnerClient, P: PolicyPlanner> {
    client: Arc<C>,
    planner: Arc<P>,
    /// Checkout root the host executor runs commands in.
    checkout_dir: PathBuf,
}

impl<C: GiteaRunnerClient, P: PolicyPlanner> TaskWorker<C, P> {
    /// Create a worker bound to a client, planner, and checkout directory.
    pub fn new(client: Arc<C>, planner: Arc<P>, checkout_dir: impl Into<PathBuf>) -> Self {
        Self {
            client,
            planner,
            checkout_dir: checkout_dir.into(),
        }
    }

    /// Run `task` to completion; returns whether it succeeded.
    pub async fn run(&self, state: &RunnerState, task: Task) -> Result<bool> {
        // Compile the workflow payload, then apply policy (allowlist + cargo->rch).
        let workflow = workflow_payload::compile_task(&task)?;
        let plan = self.planner.compile(workflow).await?;

        // Build the reused host execution stack (no VM, no snapshots).
        let session_manager = Arc::new(SessionManager::with_provider(
            Arc::new(HostVmProvider),
            SessionManagerConfig::default(),
        ));
        let exec = WorkflowExecutor::with_executor(
            Arc::new(HostCommandExecutor::new(self.checkout_dir.clone())),
            session_manager.clone(),
            WorkflowExecutorConfig {
                snapshot_on_success: false,
                auto_rollback: false,
                stop_on_failure: true,
                default_timeout: Duration::from_secs(1800),
                max_execution_time: Duration::from_secs(7200),
            },
        );
        let session = session_manager
            .create_session_from_spec(&SessionStartSpec {
                session_id: SessionId::new(),
                vm_type: None,
            })
            .await
            .map_err(|e| RunnerError::Execution(e.to_string()))?;

        // Report running.
        self.client
            .update_task(
                state,
                UpdateTaskRequest {
                    state: TaskState {
                        id: task.id,
                        // In-progress heartbeat: non-terminal (UNSPECIFIED) so the
                        // server records startedAt without completing the task.
                        result: result::UNSPECIFIED,
                        started_at: Some(chrono::Utc::now().to_rfc3339()),
                        stopped_at: None,
                        steps: Vec::new(),
                    },
                    outputs: BTreeMap::new(),
                },
            )
            .await?;

        // Execute and stream logs.
        let mut logs = LogStreamer::new(task.id);
        let outcome = exec
            .execute_workflow_in_session(&plan.workflow, &session)
            .await;

        let success = match &outcome {
            Ok(wf) => {
                for step in &wf.steps {
                    logs.add_line(format!(
                        "[{:?}] {} (exit {:?})",
                        step.status, step.name, step.exit_code
                    ));
                    for line in step.stdout.lines() {
                        logs.add_line(line.to_string());
                    }
                    for line in step.stderr.lines() {
                        logs.add_line(line.to_string());
                    }
                }
                logs.add_line(wf.summary.clone());
                wf.success
            }
            Err(e) => {
                logs.add_line(format!("execution error: {e}"));
                false
            }
        };

        // Flush logs (closing the stream) and report the final result.
        logs.flush(&*self.client, state, true).await?;
        self.client
            .update_task(
                state,
                UpdateTaskRequest {
                    state: TaskState {
                        id: task.id,
                        result: if success {
                            result::SUCCESS
                        } else {
                            result::FAILURE
                        },
                        started_at: None,
                        stopped_at: Some(chrono::Utc::now().to_rfc3339()),
                        steps: Vec::new(),
                    },
                    outputs: BTreeMap::new(),
                },
            )
            .await?;

        let _ = session_manager.release_session(&session.id).await;
        Ok(success)
    }
}
