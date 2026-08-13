//! Fetch/dispatch loop: poll `FetchTask`, run one task at a time, advance the
//! org `tasks_version`.

use crate::Result;
use crate::client::GiteaRunnerClient;
use crate::config::RunnerConfig;
use crate::policy::PolicyPlanner;
use crate::state::RunnerState;
use crate::status::SingleStatusWriter;
use crate::task_worker::TaskWorker;
use crate::workflow_payload;
use sd_notify;
use std::path::PathBuf;
use std::sync::Arc;

/// Drives the runner: registers/declares, then polls for tasks.
pub struct Poller<C: GiteaRunnerClient, P: PolicyPlanner> {
    client: Arc<C>,
    planner: Arc<P>,
    config: RunnerConfig,
    checkout_dir: PathBuf,
    /// Built once from `config.legacy_status_mirror`: (writer, context).
    legacy: Option<(Arc<SingleStatusWriter>, String)>,
    /// Built once from `config.status_token` for native commit-status posts.
    status_fallback: Option<Arc<SingleStatusWriter>>,
}

impl<C: GiteaRunnerClient + 'static, P: PolicyPlanner + 'static> Poller<C, P> {
    /// Create a poller.
    pub fn new(
        client: Arc<C>,
        planner: Arc<P>,
        config: RunnerConfig,
        checkout_dir: impl Into<PathBuf>,
    ) -> Self {
        let legacy = config.legacy_status_mirror.as_ref().map(|m| {
            (
                Arc::new(SingleStatusWriter::new(
                    config.instance_url.clone(),
                    m.token.clone(),
                )),
                m.context.clone(),
            )
        });
        let status_fallback = config.status_token.as_ref().map(|token| {
            Arc::new(SingleStatusWriter::new(
                config.instance_url.clone(),
                token.clone(),
            ))
        });
        Self {
            client,
            planner,
            config,
            checkout_dir: checkout_dir.into(),
            legacy,
            status_fallback,
        }
    }

    /// `FetchTask` under `config.poll_timeout`.
    ///
    /// This is the only part of an iteration it is safe to cancel: nothing has
    /// been claimed yet, so abandoning the call loses nothing. Cancelling is a
    /// backstop for a hang below reqwest's own `http_request_timeout`.
    async fn fetch_bounded(
        &self,
        state: &RunnerState,
        tasks_version: i64,
    ) -> Result<crate::types::FetchTaskResponse> {
        match tokio::time::timeout(
            self.config.poll_timeout,
            self.client.fetch_task(state, tasks_version),
        )
        .await
        {
            Ok(resp) => resp,
            Err(_elapsed) => Err(crate::RunnerError::Protocol(format!(
                "FetchTask timed out after {:?}; verify Gitea is reachable at {}",
                self.config.poll_timeout, self.config.instance_url,
            ))),
        }
    }

    /// Build the worker that owns a claimed task's lifecycle.
    fn build_worker(&self) -> TaskWorker<C, P> {
        let mut worker = TaskWorker::new(
            self.client.clone(),
            self.planner.clone(),
            self.config.instance_url.clone(),
            self.checkout_dir.clone(),
        );
        if let Some((writer, context)) = &self.legacy {
            worker = worker.with_legacy_mirror(writer.clone(), context.clone());
        }
        if let Some(writer) = &self.status_fallback {
            worker = worker.with_status_fallback(writer.clone());
        }
        worker.with_vm_config(
            self.config.vm_mode,
            self.config.fcctl_url.clone(),
            self.config.fcctl_vm_type.clone(),
        )
    }

    /// Run one fetch/dispatch iteration. Returns the updated `tasks_version`.
    /// Exposed for tests; the daemon calls this in a loop.
    ///
    /// Only the `FetchTask` call is time-bounded (by `config.poll_timeout`).
    /// Everything after it operates on a task Gitea has already *claimed*, and a
    /// claimed task must not be cancelled before its owner terminalizes it --
    /// see [`Poller::run_forever`].
    pub async fn poll_once(&self, state: &RunnerState, tasks_version: i64) -> Result<i64> {
        let resp = self.fetch_bounded(state, tasks_version).await?;
        let Some(task) = resp.task else {
            return Ok(resp.tasks_version);
        };
        // Log the task id so distinct runs for the same SHA are
        // distinguishable (the "double-fetch" observation was two distinct
        // runs, not one task fetched twice -- Gitea's claim is guarded).
        log::info!("fetched task id={}", task.id);

        // Coexistence guard: skip repos not in the active allowlist.
        if let Some(full) = workflow_payload::repository(&task) {
            let name = full.rsplit('/').next().unwrap_or(&full);
            if !self.config.accepts_repo(name) {
                // #2185: FetchTask already CLAIMED this task (StatusRunning,
                // assigned to this runner). Report it skipped (terminal) so
                // Gitea marks it done instead of orphaning it until the zombie
                // timeout. #3222: the claim is concluded through `TaskWorker`, the
                // single lifecycle owner, so it carries `stoppedAt` and the same
                // bounded retry as any other terminal result; the poller only
                // logs (a release failure must not crash the loop).
                log::info!(
                    "releasing task id={} for repo `{name}` (not in active_repos)",
                    task.id
                );
                if let Err(e) = self.build_worker().finalize_skipped(state, task.id).await {
                    log::warn!("failed to release skipped task id={}: {e}", task.id);
                }
                return Ok(resp.tasks_version);
            }
        }

        let worker = self.build_worker();
        // Ownership contract (Refs #3222): `TaskWorker` is the sole finalizer for
        // a task handed to it -- it has already delivered (or exhausted its
        // bounded retries on) the terminal `UpdateTask` by the time it returns.
        // The poller must only log and keep polling; terminalizing here as well
        // would race a second conclusion onto a task the server already closed.
        match worker.run(state, task).await {
            Ok(ok) => log::info!("task complete: success={ok}"),
            Err(e) => log::error!("task failed: {e}"),
        }
        Ok(resp.tasks_version)
    }

    /// Poll forever at the configured interval.
    ///
    /// #2185: always poll with `tasks_version = 0` so Gitea runs `PickTask`
    /// every iteration. Gitea gates `PickTask` on `tasks_version != latestVersion`
    /// and bumps the version at run *creation* -- before the job becomes
    /// `Waiting`. If we cached the returned version, a job that becomes Waiting
    /// after our last poll would never be offered (no further version change)
    /// until an unrelated bump or a runner restart -- the stuck-run race. Sending
    /// 0 forces a pick each poll; the extra `PickTask` query is negligible.
    ///
    /// `config.poll_timeout` bounds the `FetchTask` call *only* (see
    /// [`Poller::fetch_bounded`]). It deliberately does not wrap the whole
    /// iteration: `poll_once` awaits `TaskWorker::run` for a task Gitea has
    /// already claimed, and builds legitimately run far longer than a poll
    /// timeout (1800s per step / 7200s total, against a 60s default). Cancelling
    /// that future dropped the worker before it could post its terminal
    /// `UpdateTask`, stranding the claimed job until the server-side zombie
    /// timeout -- exactly the failure #3222 exists to remove. Responsiveness is
    /// unaffected: with no task to run, an iteration is just the bounded fetch.
    /// `config.http_request_timeout` (set on the reqwest client) is the primary
    /// guard; `poll_timeout` is belt-and-suspenders for kernel-level hangs.
    ///
    /// After every successful poll a `WATCHDOG=1` notification is sent to systemd
    /// if `$NOTIFY_SOCKET` is set. Set `WatchdogSec=` in the `.service` unit to
    /// auto-restart when no heartbeat arrives within the window.
    pub async fn run_forever(&self, state: &RunnerState) -> Result<()> {
        let mut consecutive_errors = 0u32;
        loop {
            match self.poll_once(state, 0).await {
                Ok(_tasks_version) => {
                    consecutive_errors = 0;
                    // Heartbeat: no-op when $NOTIFY_SOCKET is unset.
                    let _ = sd_notify::notify(false, &[sd_notify::NotifyState::Watchdog]);
                }
                Err(e) => {
                    consecutive_errors += 1;
                    log::error!("poll error (streak={consecutive_errors}): {e}");
                }
            }
            tokio::time::sleep(self.config.poll_interval).await;
        }
    }
}
