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
    FcctlWebProvider, HostCommandExecutor, HostVmProvider, SessionId, SessionManager,
    SessionManagerConfig, SessionStartSpec, VmCommandExecutor, WorkflowExecutor,
    WorkflowExecutorConfig,
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
    /// VM execution mode (Host = fail-open default, Firecracker = hermetic VMs).
    vm_mode: crate::config::VmMode,
    /// fcctl-web base URL when vm_mode is Firecracker.
    fcctl_url: String,
    /// VM type to allocate from fcctl-web (e.g. "rust-ci").
    fcctl_vm_type: String,
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
            vm_mode: crate::config::VmMode::Host,
            fcctl_url: "http://127.0.0.1:8080".to_string(),
            fcctl_vm_type: "rust-ci".to_string(),
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

    /// Configure VM execution mode (Host = fail-open default, Firecracker = hermetic VMs).
    pub fn with_vm_config(
        mut self,
        vm_mode: crate::config::VmMode,
        fcctl_url: impl Into<String>,
        fcctl_vm_type: impl Into<String>,
    ) -> Self {
        self.vm_mode = vm_mode;
        self.fcctl_url = fcctl_url.into();
        self.fcctl_vm_type = fcctl_vm_type.into();
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
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next())
            && let Err(e) = writer.post(owner, repo, &sha, state, context, desc).await
        {
            log::warn!("legacy status mirror failed: {e}");
        }
    }

    /// Resolve the working directory the build should run in.
    ///
    /// If the task carries `owner/repo` + sha, the target repo is checked out at
    /// that commit under `checkout_dir` and the resolved tree is returned.
    ///
    /// **Fail closed (Refs #3222).** A task that names a repository and sha but
    /// whose checkout fails returns an error rather than the bare `checkout_dir`.
    /// The previous fallback let a build run — and report SUCCESS — against a
    /// working tree that was not the commit under test, which is the worst
    /// possible outcome for a merge gate. Only tasks that carry *no*
    /// repository/sha at all (existing proof/one-step tasks, which have nothing
    /// to fetch) legitimately run in the bare `checkout_dir`.
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
            // The task *claims* a repository but the value is unusable. Running
            // anyway would evaluate the wrong tree, so fail closed.
            return Err(RunnerError::Checkout(format!(
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
            Err(e) => Err(RunnerError::Checkout(format!("{owner}/{repo}@{sha}: {e}"))),
        }
    }

    /// Run `task` to completion; returns whether it succeeded.
    ///
    /// # Terminal-lifecycle ownership (Refs #3222)
    ///
    /// `FetchTask` has already *claimed* the task by the time this is called, so
    /// returning an error without reporting one leaves the Gitea job pending
    /// until the server-side zombie timeout — the 12m52s stall observed in run
    /// 23137. This method is therefore the sole finalizer for the task: whatever
    /// stage fails (payload compile, policy, checkout, session creation,
    /// execution, log or status delivery), a terminal `UpdateTask` is attempted
    /// exactly once with bounded retries before the original error is returned.
    /// The poller logs the result and continues; it must not terminalize again.
    pub async fn run(&self, state: &RunnerState, task: Task) -> Result<bool> {
        let mut terminalized = false;
        // Owned here so a repair path can append to the *same* log stream: a
        // fresh streamer would restart at row index 0 and overwrite rows the
        // server already acked.
        let mut logs = LogStreamer::new(task.id);
        match self
            .run_claimed(state, &task, &mut logs, &mut terminalized)
            .await
        {
            Ok(success) => Ok(success),
            Err(e) if terminalized => {
                // The terminal result was already delivered; nothing to repair.
                Err(e)
            }
            Err(e) => match self
                .report_claimed_failure(state, &task, &mut logs, &e)
                .await
            {
                Ok(()) => Err(e),
                Err(report_err) => Err(RunnerError::Protocol(format!(
                    "task {} failed ({}) and the terminal result could not be delivered: {}",
                    task.id,
                    redact(&e.to_string(), state, &task),
                    redact(&report_err.to_string(), state, &task),
                ))),
            },
        }
    }

    /// Repair path for a claimed task that failed before it could terminalize
    /// itself: emit a bounded, redacted failure line, post the failure commit
    /// status while the per-job token is still valid (#2464), then deliver the
    /// terminal `UpdateTask`. Log and status delivery are best-effort — losing a
    /// log line is survivable, losing the terminal result strands the job.
    async fn report_claimed_failure(
        &self,
        state: &RunnerState,
        task: &Task,
        logs: &mut LogStreamer,
        err: &RunnerError,
    ) -> Result<()> {
        let detail = redact(&err.to_string(), state, task);
        log::error!("task {} failed after claim: {detail}", task.id);

        logs.add_line(format!("runner error: {detail}"));
        if let Err(e) = logs.flush(&*self.client, state, true).await {
            log::warn!(
                "failure log delivery for task {} failed: {}",
                task.id,
                redact(&e.to_string(), state, task)
            );
        }

        // Only tasks whose payload compiles have a derivable status context.
        if let Ok(workflow) = workflow_payload::compile_task(task) {
            self.mirror(task, StatusState::Failure, TERMINAL_FAILURE_DESC)
                .await;
            self.post_native_commit_status(
                task,
                &workflow,
                StatusState::Failure,
                TERMINAL_FAILURE_DESC,
            )
            .await;
        }

        self.send_terminal_update(state, task.id, result::FAILURE)
            .await
    }

    /// Finalize a claimed task the runner will not execute (Refs #3222).
    ///
    /// `FetchTask` claims a task before the poller's coexistence guard can reject
    /// it, so the claim still has to be concluded. Routing that through the same
    /// terminal-update path as every other claimed task keeps `TaskWorker` the
    /// single lifecycle owner: result 4 (`SKIPPED`, terminal and not counted as a
    /// run) with `stoppedAt`, delivered under the same bounded retry budget. The
    /// caller only logs the outcome.
    pub async fn finalize_skipped(&self, state: &RunnerState, task_id: i64) -> Result<()> {
        self.send_terminal_update(state, task_id, result::SKIPPED)
            .await
    }

    /// Deliver a terminal `UpdateTask` with a bounded retry budget. Gitea holds
    /// the job open until it sees this, so a transient 5xx/network blip must not
    /// abandon the task; a persistent outage returns the last error to the caller.
    async fn send_terminal_update(
        &self,
        state: &RunnerState,
        task_id: i64,
        result_code: i32,
    ) -> Result<()> {
        let mut last_err = None;
        for attempt in 1..=TERMINAL_UPDATE_ATTEMPTS {
            match self
                .client
                .update_task(state, terminal_task_state(task_id, result_code))
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    log::warn!(
                        "terminal UpdateTask attempt {attempt}/{TERMINAL_UPDATE_ATTEMPTS} \
                         for task {task_id} failed: {e}"
                    );
                    last_err = Some(e);
                    if attempt < TERMINAL_UPDATE_ATTEMPTS {
                        tokio::time::sleep(TERMINAL_UPDATE_BACKOFF * attempt).await;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            RunnerError::Protocol(format!("terminal UpdateTask for task {task_id} never ran"))
        }))
    }

    /// Execute an already-claimed task. Sets `terminalized` once the terminal
    /// `UpdateTask` has been accepted by the server; every error path out of
    /// here is repaired by [`TaskWorker::run`].
    async fn run_claimed(
        &self,
        state: &RunnerState,
        task: &Task,
        logs: &mut LogStreamer,
        terminalized: &mut bool,
    ) -> Result<bool> {
        let task = task.clone();
        // Compile the workflow payload, then apply policy (allowlist + cargo->rch).
        let workflow = workflow_payload::compile_task(&task)?;
        let status_workflow = workflow.clone();
        let plan = self.planner.compile(workflow).await?;

        // Check out the target repo at the task's sha so the build runs against
        // real repo content. Tasks that carry no repository/sha (e.g. existing
        // protocol-proof / one-step tasks) skip checkout and run in the bare
        // `checkout_dir`; a checkout that is attempted and fails is fatal.
        let work_dir = self.resolve_work_dir(state, &task).await?;

        // Build the execution stack. In Host mode (default, fail-open), commands
        // run directly on the host. In Firecracker mode, commands run inside
        // ephemeral Firecracker microVMs via fcctl-web.
        let http_client = Arc::new(reqwest::Client::new());
        let (provider, executor): (
            Arc<dyn terraphim_github_runner::VmProvider>,
            Arc<dyn terraphim_github_runner::CommandExecutor>,
        ) = match self.vm_mode {
            crate::config::VmMode::Firecracker => {
                log::info!(
                    "vm_mode=Firecracker: using fcctl-web at {} (vm_type={})",
                    self.fcctl_url,
                    self.fcctl_vm_type
                );
                let auth_token = std::env::var("FIRECRACKER_AUTH_TOKEN").ok();
                (
                    Arc::new(FcctlWebProvider::new(self.fcctl_url.clone(), auth_token)),
                    Arc::new(VmCommandExecutor::new(self.fcctl_url.clone(), http_client)),
                )
            }
            crate::config::VmMode::Host => (
                Arc::new(HostVmProvider),
                Arc::new(HostCommandExecutor::new(work_dir)),
            ),
        };
        let session_manager = Arc::new(SessionManager::with_provider(
            provider,
            SessionManagerConfig {
                default_vm_type: self.fcctl_vm_type.clone(),
                ..Default::default()
            },
        ));
        let exec = WorkflowExecutor::with_executor(
            executor.clone(),
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

        // Everything after session creation runs inside this block so the session
        // is released on *every* exit path -- an error that escaped here used to
        // leak the allocated session (a live VM in Firecracker mode).
        let run_result: Result<bool> = async {
            // In Firecracker mode, clone the repo inside the VM before running
            // the workflow.  The host checkout is skipped (sources live in the VM).
            if self.vm_mode == crate::config::VmMode::Firecracker {
                if let (Some(full), Some(sha)) = (
                    workflow_payload::repository(&task),
                    workflow_payload::head_sha(&task),
                ) {
                    let job_token =
                        workflow_payload::job_token(&task).unwrap_or_else(|| state.token.clone());
                    let base = self.instance_url.trim_end_matches('/');
                    let host = base
                        .strip_prefix("https://")
                        .or_else(|| base.strip_prefix("http://"))
                        .unwrap_or(base);
                    let clone_url = format!("https://{}@{}/{full}.git", job_token, host);
                    let clone_cmd = format!(
                        "rm -rf /workspace && git init /workspace && cd /workspace && \
                         git remote add origin {clone_url} && \
                         git fetch --depth 1 origin {sha} && \
                         git checkout FETCH_HEAD"
                    );
                    log::info!(
                        "Firecracker: cloning {full}@{sha:.8} into VM {} at /workspace",
                        session.vm_id
                    );
                    match executor
                        .execute(&session, &clone_cmd, Duration::from_secs(120), "/root")
                        .await
                    {
                        Ok(r) if r.success() => {
                            log::info!(
                                "Firecracker: repo cloned in {:?} (exit {})",
                                r.duration,
                                r.exit_code
                            );
                        }
                        Ok(r) => {
                            log::error!(
                                "Firecracker: git clone failed (exit {}): {}",
                                r.exit_code,
                                r.stderr
                            );
                        }
                        Err(e) => log::error!("Firecracker: git clone error: {e}"),
                    }
                } else {
                    log::info!("Firecracker: task has no repo/sha; running workflow without clone");
                }
            }

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
                TERMINAL_FAILURE_DESC
            };
            self.mirror(&task, terminal_state, terminal_desc).await;
            self.post_native_commit_status(&task, &status_workflow, terminal_state, terminal_desc)
                .await;
            self.send_terminal_update(
                state,
                task.id,
                if success {
                    result::SUCCESS
                } else {
                    result::FAILURE
                },
            )
            .await?;
            // From here on the task is finished on the server: the repair path in
            // `run` must not send a second terminal result.
            *terminalized = true;

            Ok(success)
        }
        .await;

        let _ = session_manager.release_session(&session.id).await;
        run_result
    }
}

/// Description attached to every terminal failure status/mirror post.
const TERMINAL_FAILURE_DESC: &str = "native build failed";

/// Attempts allowed for delivering a terminal `UpdateTask` (1 try + 2 retries).
const TERMINAL_UPDATE_ATTEMPTS: u32 = 3;

/// Base backoff between terminal `UpdateTask` attempts; scaled by attempt number.
const TERMINAL_UPDATE_BACKOFF: Duration = Duration::from_millis(200);

/// Minimal terminal `UpdateTask` payload: a result code plus `stopped_at`, which
/// is what Gitea needs to move the job out of running.
fn terminal_task_state(task_id: i64, result_code: i32) -> UpdateTaskRequest {
    UpdateTaskRequest {
        state: TaskState {
            id: task_id,
            result: result_code,
            started_at: None,
            stopped_at: Some(chrono::Utc::now().to_rfc3339()),
            steps: Vec::new(),
        },
        outputs: BTreeMap::new(),
    }
}

/// Strip credentials from text that is about to be logged or returned.
///
/// Error strings can carry the runner registration token or the per-job
/// repository token (checkout errors embed the authenticated clone URL), and
/// task secrets can appear in command output. Every known secret value is
/// replaced by a fixed marker; nothing else about the message is altered.
fn redact(text: &str, state: &RunnerState, task: &Task) -> String {
    let mut out = text.to_string();
    let mut secrets: Vec<String> = vec![state.token.clone()];
    if let Some(job_token) = workflow_payload::job_token(task) {
        secrets.push(job_token);
    }
    secrets.extend(task.secrets.values().cloned());
    for secret in secrets {
        if secret.len() >= 8 {
            out = out.replace(&secret, "***");
        }
    }
    out
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
        let update_pos = block
            .find("send_terminal_update")
            .expect("terminal update delivery");
        assert!(
            status_pos < update_pos,
            "post_native_commit_status must run before the terminal update (Refs #2464)"
        );
    }
}
