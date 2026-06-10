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
        Self {
            client,
            planner,
            config,
            checkout_dir: checkout_dir.into(),
            legacy,
        }
    }

    /// Run one fetch/dispatch iteration. Returns the updated `tasks_version`.
    /// Exposed for tests; the daemon calls this in a loop.
    pub async fn poll_once(&self, state: &RunnerState, tasks_version: i64) -> Result<i64> {
        let resp = self.client.fetch_task(state, tasks_version).await?;
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
                // timeout. Best-effort: a release failure must not crash the loop.
                log::info!(
                    "releasing task id={} for repo `{name}` (not in active_repos)",
                    task.id
                );
                if let Err(e) = self
                    .client
                    .update_task(state, skip_task_state(task.id))
                    .await
                {
                    log::warn!("failed to release skipped task id={}: {e}", task.id);
                }
                return Ok(resp.tasks_version);
            }
        }

        let mut worker = TaskWorker::new(
            self.client.clone(),
            self.planner.clone(),
            self.config.instance_url.clone(),
            self.checkout_dir.clone(),
        );
        if let Some((writer, context)) = &self.legacy {
            worker = worker.with_legacy_mirror(writer.clone(), context.clone());
        }
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
    pub async fn run_forever(&self, state: &RunnerState) -> Result<()> {
        eprintln!("DEBUG: run_forever entered");
        loop {
            if let Err(e) = self.poll_once(state, 0).await {
                log::error!("poll error: {e}");
            }
            tokio::time::sleep(self.config.poll_interval).await;
        }
    }
}

/// #2185: minimal `UpdateTask` payload marking a task SKIPPED. Result code 4
/// maps to Gitea `StatusSkipped`, which is terminal (`Status::IsDone`) and not
/// counted as a run (`HasRun` is false) -- it releases a claimed-but-unservable
/// task without recording a misleading failure.
fn skip_task_state(task_id: i64) -> crate::types::UpdateTaskRequest {
    crate::types::UpdateTaskRequest {
        state: crate::types::TaskState {
            id: task_id,
            result: 4,
            started_at: None,
            stopped_at: None,
            steps: Vec::new(),
        },
        outputs: std::collections::BTreeMap::new(),
    }
}
