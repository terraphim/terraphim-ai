//! End-to-end task execution: compile -> policy -> host execution -> logs -> result.

use crate::checkout;
use crate::client::GiteaRunnerClient;
use crate::logs::LogStreamer;
use crate::policy::PolicyPlanner;
use crate::state::RunnerState;
use crate::status::{SingleStatusWriter, StatusState};
use crate::task_journal::TaskJournal;
use crate::types::{Task, TaskState, UpdateTaskRequest, result};
use crate::{Result, RunnerError, workflow_payload};
use std::collections::BTreeMap;
use std::future::Future;
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
    /// Frequency of nonterminal task updates while a claimed workflow runs.
    heartbeat_interval: Duration,
    /// Consecutive heartbeat failures tolerated before failing closed.
    heartbeat_failure_attempts: u32,
    /// Per-process bound for checkout Git operations.
    checkout_timeout: Duration,
    /// Per-request bound for best-effort commit-status publication.
    status_request_timeout: Duration,
}

impl<C: GiteaRunnerClient + 'static, P: PolicyPlanner> TaskWorker<C, P> {
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
            heartbeat_interval: crate::config::DEFAULT_HEARTBEAT_INTERVAL,
            heartbeat_failure_attempts: crate::config::DEFAULT_HEARTBEAT_FAILURE_ATTEMPTS,
            checkout_timeout: crate::config::DEFAULT_GIT_OPERATION_TIMEOUT,
            status_request_timeout: crate::config::DEFAULT_HTTP_REQUEST_TIMEOUT,
        }
    }

    /// Override the claimed-task heartbeat interval.
    ///
    /// Production construction passes [`crate::config::RunnerConfig::heartbeat_interval`];
    /// tests use a short interval with deterministic workflow synchronization.
    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Override the consecutive heartbeat failure budget.
    pub fn with_heartbeat_failure_attempts(mut self, attempts: u32) -> Self {
        assert!(attempts > 0, "heartbeat failure attempts must be non-zero");
        self.heartbeat_failure_attempts = attempts;
        self
    }

    /// Override the timeout applied to each checkout Git subprocess.
    pub fn with_checkout_timeout(mut self, timeout: Duration) -> Self {
        assert!(!timeout.is_zero(), "checkout timeout must be non-zero");
        self.checkout_timeout = timeout;
        self
    }

    /// Bound best-effort commit-status HTTP requests.
    pub fn with_status_request_timeout(mut self, timeout: Duration) -> Self {
        self.status_request_timeout = timeout;
        self
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
        let writer = SingleStatusWriter::new_with_timeout(
            &self.instance_url,
            token,
            self.status_request_timeout,
        );
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
            self.checkout_timeout,
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

    /// Directory under which per-task journals are kept.
    ///
    /// One runner, one journal root: a second process cannot legally use the
    /// same root because journals are append-only and a racing restart would
    /// truncate committed history. The directory lives under `checkout_dir`
    /// for the same reason -- it is per-runner, already private, and survives
    /// across runs.
    fn journal_root(&self) -> std::path::PathBuf {
        self.checkout_dir.join(".terraphim-journal")
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
        // server already acked. The redactor runs every line at `add_line` time
        // so secret material never reaches the wire (Refs #101); the journal
        // makes the same guarantee across a process restart.
        let mut logs = LogStreamer::new(task.id).with_redactor({
            let state = state.clone();
            let task = task.clone();
            move |line: &str| redact(line, &state, &task)
        });
        if let Ok(journal) = TaskJournal::open(
            &self.journal_root(),
            task.id,
            crate::task_journal::JournalMeta {
                runner_uuid: state.uuid.clone(),
                repository: workflow_payload::repository(&task),
                sha: workflow_payload::head_sha(&task),
                ..Default::default()
            },
        ) {
            logs = logs.with_journal(journal);
        } else {
            // Losing the journal does not lose the running stream -- rows still
            // buffer and ack in memory -- but it drops restart durability, which
            // is exactly what #101 adds. Surface it loudly rather than letting a
            // full or unwritable journal root degrade silently.
            log::error!(
                "task {}: could not open log journal at {}; restart durability is \
                 disabled for this task (delivery continues from the in-memory buffer)",
                task.id,
                self.journal_root().display()
            );
        }
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
        let status_detail = failure_status_description(&detail);
        log::error!("task {} failed after claim: {detail}", task.id);

        if logs.is_sealed() {
            log::warn!(
                "failure occurred after task {} log stream was sealed; preserving the closed stream",
                task.id
            );
        } else {
            logs.add_line(format!("runner error: {detail}"));
            if let Err(e) = logs.flush(&*self.client, state, true).await {
                log::warn!(
                    "failure log delivery for task {} failed: {}",
                    task.id,
                    redact(&e.to_string(), state, task)
                );
            }
        }

        // Only tasks whose payload compiles have a derivable status context.
        if let Ok(workflow) = workflow_payload::compile_task(task) {
            self.mirror(task, StatusState::Failure, &status_detail)
                .await;
            self.post_native_commit_status(task, &workflow, StatusState::Failure, &status_detail)
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
        // FetchTask has already claimed the task. Start lease maintenance before
        // payload compilation and observe it across every subsequent await,
        // including checkout and session/Firecracker setup.
        let mut heartbeat = TaskHeartbeat::spawn(
            self.client.clone(),
            state.clone(),
            task.id,
            self.heartbeat_interval,
            self.heartbeat_failure_attempts,
        );
        // Compile the workflow payload, then apply policy (allowlist + cargo->rch).
        let workflow = match workflow_payload::compile_task(&task) {
            Ok(workflow) => workflow,
            Err(error) => {
                heartbeat.cancel_and_join().await?;
                return Err(error);
            }
        };
        let status_workflow = workflow.clone();
        let plan = race_heartbeat_result(&mut heartbeat, self.planner.compile(workflow)).await?;

        // Check out the target repo at the task's sha so the build runs against
        // real repo content. Tasks that carry no repository/sha (e.g. existing
        // protocol-proof / one-step tasks) skip checkout and run in the bare
        // `checkout_dir`; a checkout that is attempted and fails is fatal.
        let work_dir =
            race_heartbeat_result(&mut heartbeat, self.resolve_work_dir(state, &task)).await?;

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
        let exec = Arc::new(WorkflowExecutor::with_executor(
            executor.clone(),
            session_manager.clone(),
            WorkflowExecutorConfig {
                snapshot_on_success: false,
                auto_rollback: false,
                stop_on_failure: true,
                default_timeout: Duration::from_secs(1800),
                max_execution_time: Duration::from_secs(7200),
            },
        ));
        let session = complete_session_allocation(&mut heartbeat, async {
            session_manager
                .create_session_from_spec(&SessionStartSpec {
                    session_id: SessionId::new(),
                    vm_type: None,
                })
                .await
                .map_err(|e| RunnerError::Execution(e.to_string()))
        })
        .await?;

        // Everything after session creation runs inside this block so the session
        // is released on *every* exit path -- an error that escaped here used to
        // leak the allocated session (a live VM in Firecracker mode).
        let run_result: Result<bool> = async {
            // Allocation cannot be select-cancelled because an already-created
            // Firecracker VM would have no owner capable of releasing it. Observe
            // lease loss only after the returned session enters this release-owned
            // block, then fail promptly before starting any subsequent work.
            if heartbeat.is_finished() {
                return Err(heartbeat
                    .wait()
                    .await
                    .expect_err("heartbeat cannot stop while its owner holds the stop sender"));
            }

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
                    match race_heartbeat(
                        &mut heartbeat,
                        executor.execute(&session, &clone_cmd, Duration::from_secs(120), "/root"),
                    )
                    .await?
                    {
                        Ok(result) if result.success() => log::info!(
                            "Firecracker: repo cloned in {:?} (exit {})",
                            result.duration,
                            result.exit_code
                        ),
                        Ok(result) => log::error!(
                            "Firecracker: git clone failed (exit {}): {}",
                            result.exit_code,
                            result.stderr
                        ),
                        Err(error) => log::error!("Firecracker: git clone error: {error}"),
                    }
                } else {
                    log::info!("Firecracker: task has no repo/sha; running workflow without clone");
                }
            }

            // Report running.
            race_heartbeat_result(&mut heartbeat, async {
                self.client
                    .update_task(state, started_task_state(task.id))
                    .await?;
                Ok(())
            })
            .await?;
            race_heartbeat_result(&mut heartbeat, async {
                self.mirror(&task, StatusState::Pending, "build started")
                    .await;
                self.post_native_commit_status(
                    &task,
                    &status_workflow,
                    StatusState::Pending,
                    "build started",
                )
                .await;
                Ok(())
            })
            .await?;

            // The workflow runs in an owned task so heartbeat lease loss can
            // cancel and join it before the repair path publishes FAILURE.
            // Host commands are kill-on-drop process-group owners; Firecracker
            // commands are bounded by the session release after this block.
            let workflow = plan.workflow.clone();
            let execution_session = session.clone();
            let mut execution = OwnedWorkflowExecution::spawn(async move {
                exec.execute_workflow_in_session(&workflow, &execution_session)
                    .await
            });

            let outcome = tokio::select! {
                biased;
                heartbeat_result = heartbeat.wait() => {
                    let heartbeat_error = heartbeat_result.expect_err(
                        "heartbeat cannot stop while its owner still holds the stop sender"
                    );
                    execution.cancel_and_join().await?;
                    return Err(heartbeat_error);
                }
                execution_result = execution.wait() => match execution_result {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        heartbeat.cancel_and_join().await?;
                        return Err(RunnerError::Execution(format!(
                            "workflow execution task failed to join: {error}"
                        )));
                    }
                }
            };

            let log_delivery: Result<bool> = async {
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
                            // Preserve #3387's durable, monotonic per-step log batches.
                            // Do not select-cancel an in-flight flush on lease loss:
                            // its exclusive ack cursor would become unknowable. Each
                            // RPC has the configured HTTP timeout and no-progress is
                            // bounded; observe heartbeat failure after the batch loop.
                            logs.flush(&*self.client, state, false).await?;
                            propagate_lease_failure_after_allocation(&mut heartbeat).await?;
                        }
                        logs.add_line(wf.summary.clone());
                        wf.success
                    }
                    Err(e) => {
                        logs.add_line(format!("execution error: {e}"));
                        false
                    }
                };

                // Seal monotonically. As above, never cancel UpdateLog mid-flight;
                // observe lease loss immediately after its bounded batch loop.
                logs.flush(&*self.client, state, true).await?;
                if heartbeat.is_finished() {
                    return Err(heartbeat.wait().await.expect_err(
                        "heartbeat cannot stop while its owner holds the stop sender",
                    ));
                }
                Ok(success)
            }
            .await;

            let success = match log_delivery {
                Ok(success) => success,
                Err(error) => {
                    heartbeat.cancel_and_join().await?;
                    return Err(error);
                }
            };

            // Post terminal commit status before UpdateTask: Gitea revokes the
            // per-job token after terminal publication (Refs #2464).
            let status_delivery = async {
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
                self.post_native_commit_status(
                    &task,
                    &status_workflow,
                    terminal_state,
                    terminal_desc,
                )
                .await;
                Ok::<(), RunnerError>(())
            };

            heartbeat = await_status_delivery(heartbeat, status_delivery).await?;

            // Last safe point: join before publishing terminal state, so no
            // heartbeat can race after SUCCESS/FAILURE.
            heartbeat.stop_and_join().await?;
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

/// Preserve a redacted post-seal failure explanation in commit status, because
/// #3387 correctly forbids appending another row after `UpdateLog(no_more=true)`.
fn failure_status_description(detail: &str) -> String {
    format!("runner failure: {detail}")
}

/// Attempts allowed for delivering a terminal `UpdateTask` (1 try + 2 retries).
const TERMINAL_UPDATE_ATTEMPTS: u32 = 3;

/// Base backoff between terminal `UpdateTask` attempts; scaled by attempt number.
const TERMINAL_UPDATE_BACKOFF: Duration = Duration::from_millis(200);

/// Owned heartbeat task. Its lifecycle owner joins it before terminal state.
struct TaskHeartbeat {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<Result<()>>,
    joined: bool,
}

impl TaskHeartbeat {
    fn spawn<C: GiteaRunnerClient + 'static>(
        client: Arc<C>,
        state: RunnerState,
        task_id: i64,
        interval: Duration,
        failure_attempts: u32,
    ) -> Self {
        let (stop, mut stop_rx) = tokio::sync::oneshot::channel();
        let join = tokio::spawn(async move {
            let mut consecutive_failures = 0;
            loop {
                tokio::select! {
                    biased;
                    _ = &mut stop_rx => return Ok(()),
                    _ = tokio::time::sleep(interval) => {}
                }

                match client
                    .update_task(&state, heartbeat_task_state(task_id))
                    .await
                {
                    Ok(_) => consecutive_failures = 0,
                    Err(error) => {
                        consecutive_failures += 1;
                        log::warn!(
                            "claimed-task heartbeat attempt {consecutive_failures}/\
                             {failure_attempts} for task {task_id} failed: {error}"
                        );
                        if consecutive_failures >= failure_attempts {
                            return Err(RunnerError::Protocol(format!(
                                "claimed-task heartbeat for task {task_id} failed \
                                 {failure_attempts} consecutive times; failing closed: \
                                 {error}"
                            )));
                        }
                    }
                }
            }
        });
        Self {
            stop: Some(stop),
            join,
            joined: false,
        }
    }

    async fn stop_and_join(&mut self) -> Result<()> {
        if self.joined {
            return Ok(());
        }
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        let joined = (&mut self.join).await.map_err(|error| {
            RunnerError::Protocol(format!(
                "claimed-task heartbeat task failed to join: {error}"
            ))
        })?;
        self.joined = true;
        joined
    }

    async fn cancel_and_join(&mut self) -> Result<()> {
        self.stop.take();
        if self.joined {
            return Ok(());
        }
        if !self.join.is_finished() {
            self.join.abort();
        }
        let joined = match (&mut self.join).await {
            Ok(result) => result,
            Err(error) if error.is_cancelled() => Ok(()),
            Err(error) => Err(RunnerError::Protocol(format!(
                "cancelled claimed-task heartbeat failed to join: {error}"
            ))),
        };
        self.joined = true;
        joined
    }

    fn is_finished(&self) -> bool {
        self.join.is_finished()
    }

    async fn wait(&mut self) -> Result<()> {
        let joined = (&mut self.join).await;
        self.joined = true;
        joined.map_err(|error| {
            RunnerError::Protocol(format!(
                "claimed-task heartbeat task failed to join: {error}"
            ))
        })?
    }
}

impl Drop for TaskHeartbeat {
    fn drop(&mut self) {
        if !self.joined {
            self.join.abort();
        }
    }
}

async fn race_heartbeat<F, T>(heartbeat: &mut TaskHeartbeat, operation: F) -> Result<T>
where
    F: Future<Output = T>,
{
    tokio::pin!(operation);
    tokio::select! {
        biased;
        heartbeat_result = heartbeat.wait() => {
            Err(heartbeat_result.expect_err(
                "heartbeat cannot stop while its owner still holds the stop sender"
            ))
        }
        value = &mut operation => Ok(value),
    }
}

/// Race a fallible post-claim operation against lease loss. An operation error
/// cancels and joins the heartbeat before repair terminalization begins.
async fn race_heartbeat_result<F, T>(heartbeat: &mut TaskHeartbeat, operation: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(operation);
    tokio::select! {
        biased;
        heartbeat_result = heartbeat.wait() => {
            Err(heartbeat_result.expect_err(
                "heartbeat cannot stop while its owner still holds the stop sender"
            ))
        }
        operation_result = &mut operation => match operation_result {
            Ok(value) => Ok(value),
            Err(error) => {
                heartbeat.cancel_and_join().await?;
                Err(error)
            }
        }
    }
}

async fn complete_session_allocation<F, T>(
    heartbeat: &mut TaskHeartbeat,
    allocation: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match allocation.await {
        Ok(value) => Ok(value),
        Err(error) => {
            heartbeat.cancel_and_join().await?;
            Err(error)
        }
    }
}

async fn propagate_lease_failure_after_allocation(heartbeat: &mut TaskHeartbeat) -> Result<()> {
    if heartbeat.is_finished() {
        return Err(heartbeat
            .wait()
            .await
            .expect_err("heartbeat cannot stop while its owner holds the stop sender"));
    }
    Ok(())
}

async fn await_status_delivery<F>(
    mut heartbeat: TaskHeartbeat,
    status_delivery: F,
) -> Result<TaskHeartbeat>
where
    F: Future<Output = Result<()>>,
{
    tokio::pin!(status_delivery);
    tokio::select! {
        biased;
        heartbeat_result = heartbeat.wait() => {
            return Err(heartbeat_result.expect_err(
                "heartbeat cannot stop while its owner still holds the stop sender"
            ));
        }
        status_result = &mut status_delivery => {
            if let Err(error) = status_result {
                heartbeat.cancel_and_join().await?;
                return Err(error);
            }
        }
    }
    Ok(heartbeat)
}

type WorkflowTaskResult = terraphim_github_runner::Result<terraphim_github_runner::WorkflowResult>;

/// Owned workflow cancellation boundary. Lease loss aborts and joins it so
/// process-group guards run before terminal failure publication.
struct OwnedWorkflowExecution {
    join: tokio::task::JoinHandle<WorkflowTaskResult>,
}

impl OwnedWorkflowExecution {
    fn spawn(future: impl Future<Output = WorkflowTaskResult> + Send + 'static) -> Self {
        Self {
            join: tokio::spawn(future),
        }
    }

    async fn wait(&mut self) -> std::result::Result<WorkflowTaskResult, tokio::task::JoinError> {
        (&mut self.join).await
    }

    async fn cancel_and_join(mut self) -> Result<()> {
        self.join.abort();
        match (&mut self.join).await {
            Ok(_) => Ok(()),
            Err(error) if error.is_cancelled() => Ok(()),
            Err(error) => Err(RunnerError::Execution(format!(
                "cancelled workflow task failed to join: {error}"
            ))),
        }
    }
}

impl Drop for OwnedWorkflowExecution {
    fn drop(&mut self) {
        self.join.abort();
    }
}

/// First nonterminal update: records when execution began.
fn started_task_state(task_id: i64) -> UpdateTaskRequest {
    UpdateTaskRequest {
        state: TaskState {
            id: task_id,
            result: result::UNSPECIFIED,
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            stopped_at: None,
            steps: Vec::new(),
        },
        outputs: BTreeMap::new(),
    }
}

/// Periodic lease refresh: minimal and explicitly nonterminal.
fn heartbeat_task_state(task_id: i64) -> UpdateTaskRequest {
    UpdateTaskRequest {
        state: TaskState {
            id: task_id,
            result: result::UNSPECIFIED,
            started_at: None,
            stopped_at: None,
            steps: Vec::new(),
        },
        outputs: BTreeMap::new(),
    }
}

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
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Notify;

    struct BlockingHeartbeatClient {
        heartbeat_started: Arc<Notify>,
        heartbeat_cancelled: Arc<AtomicBool>,
    }

    struct FailingHeartbeatClient;

    struct CancellationProof(Arc<AtomicBool>);

    impl Drop for CancellationProof {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl GiteaRunnerClient for BlockingHeartbeatClient {
        async fn register(
            &self,
            _: crate::types::RegisterRequest,
        ) -> Result<crate::types::RunnerInfo> {
            unreachable!()
        }

        async fn declare(
            &self,
            _: &RunnerState,
            _: crate::types::DeclareRequest,
        ) -> Result<crate::types::DeclareResponse> {
            unreachable!()
        }

        async fn fetch_task(
            &self,
            _: &RunnerState,
            _: i64,
        ) -> Result<crate::types::FetchTaskResponse> {
            unreachable!()
        }

        async fn update_task(
            &self,
            _: &RunnerState,
            _: UpdateTaskRequest,
        ) -> Result<crate::types::UpdateTaskResponse> {
            let _proof = CancellationProof(self.heartbeat_cancelled.clone());
            self.heartbeat_started.notify_one();
            std::future::pending().await
        }

        async fn update_log(
            &self,
            _: &RunnerState,
            _: crate::types::UpdateLogRequest,
        ) -> Result<crate::types::UpdateLogResponse> {
            unreachable!()
        }
    }

    #[async_trait]
    impl GiteaRunnerClient for FailingHeartbeatClient {
        async fn register(
            &self,
            _: crate::types::RegisterRequest,
        ) -> Result<crate::types::RunnerInfo> {
            unreachable!()
        }

        async fn declare(
            &self,
            _: &RunnerState,
            _: crate::types::DeclareRequest,
        ) -> Result<crate::types::DeclareResponse> {
            unreachable!()
        }

        async fn fetch_task(
            &self,
            _: &RunnerState,
            _: i64,
        ) -> Result<crate::types::FetchTaskResponse> {
            unreachable!()
        }

        async fn update_task(
            &self,
            _: &RunnerState,
            _: UpdateTaskRequest,
        ) -> Result<crate::types::UpdateTaskResponse> {
            Err(RunnerError::Protocol("deterministic lease failure".into()))
        }

        async fn update_log(
            &self,
            _: &RunnerState,
            _: crate::types::UpdateLogRequest,
        ) -> Result<crate::types::UpdateLogResponse> {
            unreachable!()
        }
    }

    fn test_state() -> RunnerState {
        RunnerState {
            uuid: "test-uuid".into(),
            token: "test-token".into(),
            name: "test-runner".into(),
            version: "test-version".into(),
            labels: vec![],
            ephemeral: false,
        }
    }

    #[tokio::test]
    async fn status_delivery_error_joins_an_in_flight_heartbeat_before_propagating() {
        let heartbeat_started = Arc::new(Notify::new());
        let heartbeat_cancelled = Arc::new(AtomicBool::new(false));
        let client = Arc::new(BlockingHeartbeatClient {
            heartbeat_started: heartbeat_started.clone(),
            heartbeat_cancelled: heartbeat_cancelled.clone(),
        });
        let heartbeat =
            TaskHeartbeat::spawn(client, test_state(), 3390, Duration::from_millis(1), 3);
        let status_delivery = async move {
            heartbeat_started.notified().await;
            Err(RunnerError::Protocol(
                "deterministic status-delivery failure".into(),
            ))
        };

        let error = match await_status_delivery(heartbeat, status_delivery).await {
            Err(error) => error,
            Ok(_) => panic!("status-delivery failure must propagate"),
        };

        assert!(error.to_string().contains("status-delivery failure"));
        assert!(
            heartbeat_cancelled.load(Ordering::SeqCst),
            "status error returned before the in-flight heartbeat was cancelled and joined"
        );
    }

    #[tokio::test]
    async fn session_allocation_completes_before_lease_failure_is_observed() {
        let mut heartbeat = TaskHeartbeat::spawn(
            Arc::new(FailingHeartbeatClient),
            test_state(),
            3390,
            Duration::from_millis(1),
            1,
        );
        let allocation_completed = Arc::new(AtomicBool::new(false));
        let completed = allocation_completed.clone();

        let session = complete_session_allocation(&mut heartbeat, async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            completed.store(true, Ordering::SeqCst);
            Ok("owned-session")
        })
        .await;

        assert!(
            allocation_completed.load(Ordering::SeqCst),
            "lease loss must not cancel an allocation that may already own a VM"
        );
        assert_eq!(
            session.expect("the acquired session must reach the release-owned scope"),
            "owned-session"
        );
        let lease_error = propagate_lease_failure_after_allocation(&mut heartbeat)
            .await
            .expect_err("the release-owned scope must promptly propagate pending lease loss");
        assert!(
            lease_error
                .to_string()
                .contains("deterministic lease failure"),
            "{lease_error}"
        );
    }
}
