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
    /// Dedicated API token for native commit-status posts (RUNNER_STATUS_TOKEN /
    /// GITEA_TOKEN). Per-job `github.token` often lacks statuses scope on private repos.
    status_fallback: Option<Arc<SingleStatusWriter>>,
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
            status_fallback: None,
        }
    }

    /// Attach a legacy commit-status mirror (e.g. `adf/build`) posted alongside
    /// the native protocol result during migration.
    pub fn with_legacy_mirror(mut self, writer: Arc<SingleStatusWriter>, context: String) -> Self {
        self.legacy = Some((writer, context));
        self
    }

    /// Attach a fallback writer for native commit-status posts when the per-job
    /// token is missing or returns HTTP 401.
    pub fn with_status_fallback(mut self, writer: Arc<SingleStatusWriter>) -> Self {
        self.status_fallback = Some(writer);
        self
    }

    /// Post branch-protection commit status using the per-job token (Refs #2464).
    ///
    /// Context format matches Gitea Actions: `{workflow} / {job} ({event})`.
    async fn post_native_commit_status(
        &self,
        task: &Task,
        workflow: &terraphim_github_runner::ParsedWorkflow,
        state: StatusState,
        desc: &str,
    ) {
        let (Some(full), Some(sha)) = (
            workflow_payload::repository(task),
            workflow_payload::head_sha(task),
        ) else {
            return;
        };
        let mut parts = full.splitn(2, '/');
        let (Some(owner), Some(repo)) = (parts.next(), parts.next()) else {
            return;
        };
        let context = workflow_payload::commit_status_context(task, workflow);

        // Prefer the dedicated status token when configured: per-job github.token
        // can authenticate checkout but still return HTTP 401 on /statuses for private repos.
        if let Some(fallback) = &self.status_fallback {
            match fallback
                .post(owner, repo, &sha, state, &context, desc)
                .await
            {
                Ok(()) => return,
                Err(e) => log::warn!(
                    "native commit status post via runner status token failed for {owner}/{repo}@{sha}: {e}"
                ),
            }
        }

        let Some(token) = workflow_payload::job_token(task) else {
            if self.status_fallback.is_none() {
                log::warn!(
                    "native commit status skipped: no per-job token on task {}",
                    task.id
                );
            }
            return;
        };
        let writer = SingleStatusWriter::new(&self.instance_url, token);
        if let Err(e) = writer.post(owner, repo, &sha, state, &context, desc).await {
            log::warn!("native commit status post failed for {owner}/{repo}@{sha}: {e}");
        }
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

    /// Resolve the working directory the build should run in.
    ///
    /// If the task carries `owner/repo` + sha, the target repo is checked out at
    /// that commit under `checkout_dir` and the resolved tree is returned. If the
    /// task carries no repository/sha, or the checkout fails, the bare
    /// `checkout_dir` is returned and a message is logged so existing
    /// proof/one-step tasks (which have no repo to fetch) still run.
    async fn resolve_work_dir(&self, state: &RunnerState, task: &Task) -> PathBuf {
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
            return self.checkout_dir.clone();
        };

        let mut parts = full.splitn(2, '/');
        let (Some(owner), Some(repo)) = (parts.next(), parts.next()) else {
            log::warn!(
                "task {} repository `{full}` is not `owner/repo`; skipping checkout",
                task.id
            );
            return self.checkout_dir.clone();
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
                dir
            }
            Err(e) => {
                // Surface the failure but degrade gracefully: the build then runs
                // in the bare checkout_dir and will fail loudly if it needs repo
                // content, rather than the runner crashing here.
                log::warn!("checkout of {owner}/{repo}@{sha} failed: {e}; using checkout_dir");
                self.checkout_dir.clone()
            }
        }
    }

    /// Run `task` to completion; returns whether it succeeded.
    pub async fn run(&self, state: &RunnerState, task: Task) -> Result<bool> {
        // Compile the workflow payload, then apply policy (allowlist + cargo->rch).
        let workflow = workflow_payload::compile_task(&task)?;
        let status_workflow = workflow.clone();
        let plan = self.planner.compile(workflow).await?;

        // Check out the target repo at the task's sha so the build runs against
        // real repo content. Tasks that carry no repository/sha (e.g. existing
        // protocol-proof / one-step tasks) skip checkout gracefully and run in
        // the bare `checkout_dir`, preserving prior behaviour.
        let work_dir = self.resolve_work_dir(state, &task).await;

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
        self.post_native_commit_status(
            &task,
            &status_workflow,
            StatusState::Pending,
            "build started",
        )
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

        // Close the log stream, then post terminal commit status *before* marking the
        // task complete. Gitea revokes the per-job `github.token` once UpdateTask
        // reports SUCCESS/FAILURE; posting status afterward yields HTTP 401 (Refs #2464).
        logs.flush(&*self.client, state, true).await?;
        let terminal_state = if success {
            StatusState::Success
        } else {
            StatusState::Failure
        };
        let terminal_desc = if success {
            "native build passed"
        } else {
            "native build failed"
        };
        self.mirror(&task, terminal_state, terminal_desc).await;
        self.post_native_commit_status(&task, &status_workflow, terminal_state, terminal_desc)
            .await;
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

#[cfg(test)]
mod tests {
    /// Regression guard for #2464: terminal commit status must use the per-job token
    /// while it is still valid (before UpdateTask reports SUCCESS/FAILURE).
    #[test]
    fn terminal_commit_status_precedes_task_completion() {
        let src = include_str!("task_worker.rs");
        let marker = "// Close the log stream, then post terminal commit status";
        let block = src.split(marker).nth(1).expect("terminal close block");
        let status_pos = block
            .find("post_native_commit_status")
            .expect("terminal status post");
        let update_pos = block.find("update_task").expect("terminal update_task");
        assert!(
            status_pos < update_pos,
            "post_native_commit_status must run before terminal update_task (Refs #2464)"
        );
    }
}
