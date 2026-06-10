//! End-to-end task execution: compile -> policy -> host execution -> logs -> result.

use crate::checkout;
use crate::client::GiteaRunnerClient;
use crate::logs::LogStreamer;
use crate::policy::PolicyPlanner;
use crate::state::RunnerState;
use crate::status::{SingleStatusWriter, StatusState};
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
    /// Clone base URL (Gitea `instance_url`) used to fetch the target repo.
    instance_url: String,
    /// Checkout root: per-repo working trees live at `<root>/<owner>/<repo>`.
    /// Also the fallback working dir for tasks that carry no repository/sha.
    checkout_dir: PathBuf,
    /// Optional legacy commit-status mirror (writer, context) for migration.
    legacy: Option<(Arc<SingleStatusWriter>, String)>,
}

impl<C: GiteaRunnerClient, P: PolicyPlanner> TaskWorker<C, P> {
    /// Create a worker bound to a client, planner, clone base URL, and checkout
    /// root. `instance_url` is the Gitea base the target repository is fetched
    /// from before the build runs; `checkout_dir` is the root under which
    /// per-repo working trees are materialised (and the fallback working dir for
    /// tasks that carry no repository/sha).
    pub fn new(
        client: Arc<C>,
        planner: Arc<P>,
        instance_url: impl Into<String>,
        checkout_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            client,
            planner,
            instance_url: instance_url.into(),
            checkout_dir: checkout_dir.into(),
            legacy: None,
        }
    }

    /// Attach a legacy commit-status mirror (e.g. `adf/build`) posted alongside
    /// the native protocol result during migration.
    pub fn with_legacy_mirror(mut self, writer: Arc<SingleStatusWriter>, context: String) -> Self {
        self.legacy = Some((writer, context));
        self
    }

    /// Post to the legacy mirror if configured and the task carries `owner/repo`+sha.
    async fn mirror(&self, task: &Task, state: StatusState, desc: &str) {
        let Some((writer, context)) = &self.legacy else {
            return;
        };
        let (Some(full), Some(sha)) = (
            workflow_payload::repository(task),
            workflow_payload::head_sha(task),
        ) else {
            return;
        };
        let mut parts = full.splitn(2, '/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            if let Err(e) = writer.post(owner, repo, &sha, state, context, desc).await {
                log::warn!("legacy status mirror failed: {e}");
            }
        }
    }

    /// Post a FAILURE result and a one-line log to Gitea when a task fails before
    /// execution begins (compile, policy, or checkout phase). Best-effort: errors
    /// from the reporting itself are silently discarded so the caller can still
    /// propagate the original error cleanly.
    async fn post_pre_run_failure(&self, state: &RunnerState, task_id: i64, msg: &str) {
        let mut logs = LogStreamer::new(task_id);
        logs.add_line(msg.to_string());
        let _ = logs.flush(&*self.client, state, true).await;
        let now = chrono::Utc::now().to_rfc3339();
        let _ = self
            .client
            .update_task(
                state,
                UpdateTaskRequest {
                    state: TaskState {
                        id: task_id,
                        result: result::FAILURE,
                        started_at: Some(now.clone()),
                        stopped_at: Some(now),
                        steps: Vec::new(),
                    },
                    outputs: BTreeMap::new(),
                },
            )
            .await;
    }

    /// Resolve the working directory the build should run in.
    ///
    /// Returns `Ok(checkout_dir)` for tasks that carry no repository/sha (e.g.
    /// proof/one-step tasks). Returns `Err` if a checkout was required but failed
    /// so the caller can report FAILURE to Gitea rather than risking a build
    /// against stale or wrong code.
    async fn resolve_work_dir(&self, state: &RunnerState, task: &Task) -> Result<PathBuf> {
        let (Some(full), Some(sha)) = (
            workflow_payload::repository(task),
            workflow_payload::head_sha(task),
        ) else {
            // Keys only -- the context Struct carries a token, so never log values.
            let keys: Vec<&str> = task
                .context
                .as_object()
                .map(|o| o.keys().map(String::as_str).collect())
                .unwrap_or_default();
            log::info!(
                "task {} carries no repository/sha; running in checkout_dir without checkout (context keys: {:?})",
                task.id,
                keys
            );
            return Ok(self.checkout_dir.clone());
        };

        let mut parts = full.splitn(2, '/');
        let (Some(owner), Some(repo)) = (parts.next(), parts.next()) else {
            return Err(RunnerError::Compile(format!(
                "task {} repository `{full}` is not `owner/repo`",
                task.id
            )));
        };

        // Authenticate the checkout with the per-job repository token Gitea puts
        // in the task (github.token / secrets.GITHUB_TOKEN). The runner's own
        // registration token (`state.token`) cannot fetch repository content, so
        // it is only a last-resort fallback (e.g. public repos / odd payloads).
        let job_token = workflow_payload::job_token(task).unwrap_or_else(|| state.token.clone());
        match checkout::ensure_checkout(
            &self.instance_url,
            owner,
            repo,
            &sha,
            Some(job_token.as_str()),
            &self.checkout_dir,
        )
        .await
        {
            Ok(dir) => {
                log::info!("checked out {owner}/{repo}@{sha} into {}", dir.display());
                Ok(dir)
            }
            Err(e) => Err(RunnerError::Execution(format!(
                "checkout of {owner}/{repo}@{sha} failed: {e}"
            ))),
        }
    }

    /// Run `task` to completion; returns whether it succeeded.
    pub async fn run(&self, state: &RunnerState, task: Task) -> Result<bool> {
        // P1-4: compile the workflow payload and apply policy. On error, report
        // FAILURE to Gitea so the task does not stay pending indefinitely.
        let workflow = match workflow_payload::compile_task(&task) {
            Ok(w) => w,
            Err(e) => {
                self.post_pre_run_failure(state, task.id, &format!("workflow compile failed: {e}"))
                    .await;
                return Err(e);
            }
        };
        let plan = match self.planner.compile(workflow).await {
            Ok(p) => p,
            Err(e) => {
                self.post_pre_run_failure(state, task.id, &format!("policy rejected: {e}"))
                    .await;
                return Err(e);
            }
        };

        // P1-3: check out the target repo. On checkout failure, report FAILURE to
        // Gitea rather than silently degrading to a potentially stale work dir.
        let work_dir = match self.resolve_work_dir(state, &task).await {
            Ok(d) => d,
            Err(e) => {
                self.post_pre_run_failure(state, task.id, &e.to_string())
                    .await;
                return Err(e);
            }
        };

        // Build the reused host execution stack (no VM, no snapshots) rooted at
        // the resolved per-repo working tree.
        let session_manager = Arc::new(SessionManager::with_provider(
            Arc::new(HostVmProvider),
            SessionManagerConfig::default(),
        ));
        let exec = WorkflowExecutor::with_executor(
            Arc::new(HostCommandExecutor::new(work_dir)),
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
        self.mirror(&task, StatusState::Pending, "build started")
            .await;

        // Execute, then stream logs in per-step batches (multi-batch UpdateLog).
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
                    // Flush this step's batch so the Gitea UI shows progress as
                    // steps complete (exercises the monotonic multi-batch ack).
                    logs.flush(&*self.client, state, false).await?;
                }
                logs.add_line(wf.summary.clone());
                wf.success
            }
            Err(e) => {
                logs.add_line(format!("execution error: {e}"));
                false
            }
        };

        // Close the log stream and report the final result.
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
        self.mirror(
            &task,
            if success {
                StatusState::Success
            } else {
                StatusState::Failure
            },
            if success {
                "native build passed"
            } else {
                "native build failed"
            },
        )
        .await;

        let _ = session_manager.release_session(&session.id).await;
        Ok(success)
    }
}
