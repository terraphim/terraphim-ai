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

        // Coexistence guard: skip repos not in the active allowlist.
        if let Some(full) = workflow_payload::repository(&task) {
            let name = full.rsplit('/').next().unwrap_or(&full);
            if !self.config.accepts_repo(name) {
                log::info!("skipping task for repo `{name}` (not in active_repos)");
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
    pub async fn run_forever(&self, state: &RunnerState) -> Result<()> {
        let mut tasks_version = 0i64;
        loop {
            match self.poll_once(state, tasks_version).await {
                Ok(v) => tasks_version = v,
                Err(e) => log::error!("poll error: {e}"),
            }
            tokio::time::sleep(self.config.poll_interval).await;
        }
    }
}
