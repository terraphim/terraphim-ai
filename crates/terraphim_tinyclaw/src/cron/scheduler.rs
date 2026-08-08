//! Cron scheduler — tick loop that fires due jobs.
//!
//! Wave 3 of the Hermes parity arc. The scheduler runs a periodic tick loop
//! (default 60s) that:
//! 1. Loads all jobs from the store
//! 2. Filters to due jobs (next_run_at <= now AND state == Scheduled)
//! 3. Executes each due job
//! 4. Updates state and persists
//!
//! Execution is delegated to a caller-provided `JobExecutor` to keep the
//! scheduler agnostic of the agent runtime.

use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use super::CronError;
use super::job::{CronJob, JobState, RepeatConfig};
use super::store::CronStore;

/// Outcome of executing a single job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    /// Job executed successfully.
    Ok,
    /// Job execution failed.
    Err(String),
}

/// Trait for executing a job's prompt.
///
/// The TinyClaw agent loop implements this. Tests provide a closure.
#[async_trait::async_trait]
pub trait JobExecutor: Send + Sync + 'static {
    /// Execute the job and return the outcome.
    async fn execute(&self, job: &CronJob) -> JobOutcome;
}

/// Cron scheduler.
pub struct CronScheduler {
    store: CronStore,
    executor: Arc<dyn JobExecutor>,
    tick_interval: Duration,
    handle: Mutex<Option<JoinHandle<()>>>,
    shutdown: Mutex<Option<Arc<tokio::sync::Notify>>>,
}

impl CronScheduler {
    /// Create a new scheduler.
    pub fn new(store: CronStore, executor: Arc<dyn JobExecutor>, tick_interval: Duration) -> Self {
        Self {
            store,
            executor,
            tick_interval,
            handle: Mutex::new(None),
            shutdown: Mutex::new(None),
        }
    }

    /// Start the scheduler tick loop in a background task.
    ///
    /// Returns immediately. The task runs until `stop()` is called or the
    /// process exits.
    pub async fn start(self: Arc<Self>) -> Result<(), CronError> {
        let notify = Arc::new(tokio::sync::Notify::new());
        {
            let mut guard = self.shutdown.lock().await;
            if guard.is_some() {
                return Err(CronError::Execution("scheduler already started".into()));
            }
            *guard = Some(notify.clone());
        }

        let me = self.clone();
        let tick = me.tick_interval;
        let handle = tokio::spawn(async move {
            me.run(notify, tick).await;
        });

        {
            let mut guard = self.handle.lock().await;
            *guard = Some(handle);
        }

        info!(tick_secs = ?tick, "cron scheduler started");
        Ok(())
    }

    /// Stop the scheduler tick loop.
    pub async fn stop(&self) {
        if let Some(notify) = self.shutdown.lock().await.take() {
            notify.notify_waiters();
        }
        if let Some(handle) = self.handle.lock().await.take() {
            let _ = handle.await;
        }
        info!("cron scheduler stopped");
    }

    /// Run a single tick: load jobs, fire due ones, persist.
    pub async fn tick(&self) -> Result<usize, CronError> {
        let now = Utc::now();
        let mut jobs = self.store.load_all().await?;

        // Compute due IDs BEFORE recomputing next_run_at (otherwise a Delay
        // job would always have next_run_at > now after recompute).
        let due_ids: Vec<String> = jobs
            .iter()
            .filter(|j| j.is_due(now))
            .map(|j| j.id.clone())
            .collect();

        // Recompute next_run_at only for jobs that are NOT due (so that
        // due jobs keep their already-elapsed next_run_at).
        for job in &mut jobs {
            if !due_ids.contains(&job.id) {
                job.recompute_next_run(now);
            }
        }

        let due_count = due_ids.len();
        debug!(due = due_count, "cron tick: due jobs");

        // Execute each due job
        for id in due_ids {
            let Some(mut job) = jobs.iter().find(|j| j.id == id).cloned() else {
                continue;
            };
            job.state = JobState::Running;
            let outcome = self.executor.execute(&job).await;
            job.last_run_at = Some(now);
            job.state = JobState::Scheduled;

            match outcome {
                JobOutcome::Ok => {
                    job.last_status = Some("ok".into());
                }
                JobOutcome::Err(msg) => {
                    job.last_status = Some(format!("error: {msg}"));
                    warn!(job_id = %job.id, error = %msg, "cron job failed");
                }
            }

            // Handle repeat counting
            if let Some(repeat) = &mut job.repeat {
                repeat.completed += 1;
                if repeat.exhausted() {
                    job.state = JobState::Completed;
                    job.enabled = false;
                    job.next_run_at = None;
                }
            } else {
                // One-shot jobs complete after firing
                if matches!(
                    job.schedule,
                    super::job::Schedule::Delay { .. } | super::job::Schedule::At { .. }
                ) {
                    job.state = JobState::Completed;
                    job.enabled = false;
                    job.next_run_at = None;
                }
            }

            // Recompute next_run_at for surviving jobs
            job.recompute_next_run(now);

            // Update in the jobs list
            if let Some(slot) = jobs.iter_mut().find(|j| j.id == job.id) {
                *slot = job;
            }
        }

        self.store.save_all(&jobs).await?;
        Ok(due_count)
    }

    async fn run(self: Arc<Self>, notify: Arc<tokio::sync::Notify>, tick: Duration) {
        let mut interval = tokio::time::interval(tick);
        // Don't fire immediately — wait for the first tick
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = notify.notified() => {
                    debug!("cron scheduler received shutdown");
                    return;
                }
                _ = interval.tick() => {
                    if let Err(e) = self.tick().await {
                        error!(error = %e, "cron tick failed");
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for CronScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CronScheduler")
            .field("tick_interval", &self.tick_interval)
            .finish()
    }
}

/// Helper: create a `RepeatConfig` from an optional max count.
pub fn repeat(times: Option<u32>) -> RepeatConfig {
    RepeatConfig {
        times,
        completed: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::job::Schedule;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use terraphim_persistence::DeviceStorage;

    async fn make_scheduler() -> (Arc<CronScheduler>, Arc<AtomicUsize>) {
        // Use memory-only DeviceStorage for hermetic tests
        let _ = DeviceStorage::init_memory_only().await;
        let storage = DeviceStorage::arc_memory_only().await.unwrap();
        // Unique key per test to avoid interference
        let key = format!("test_scheduler_jobs_{}", uuid::Uuid::new_v4().simple());
        let store = CronStore::new(storage, key);
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        struct TestExecutor(Arc<AtomicUsize>);
        #[async_trait::async_trait]
        impl JobExecutor for TestExecutor {
            async fn execute(&self, _job: &CronJob) -> JobOutcome {
                self.0.fetch_add(1, Ordering::SeqCst);
                JobOutcome::Ok
            }
        }

        let scheduler = Arc::new(CronScheduler::new(
            store,
            Arc::new(TestExecutor(counter_clone)),
            Duration::from_secs(60),
        ));
        (scheduler, counter)
    }

    #[tokio::test]
    async fn test_tick_fires_due_job() {
        let (scheduler, counter) = make_scheduler().await;

        // Create a job that's already due (next_run_at in the past)
        let mut job = CronJob::new("test", Schedule::Delay { secs: 60 });
        job.next_run_at = Some(Utc::now() - Duration::from_secs(1));
        scheduler.store.save_all(&[job]).await.unwrap();

        let fired = scheduler.tick().await.unwrap();
        assert_eq!(fired, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Job should be marked completed (one-shot)
        let jobs = scheduler.store.load_all().await.unwrap();
        assert_eq!(jobs[0].state, JobState::Completed);
    }

    #[tokio::test]
    async fn test_tick_skips_paused_job() {
        let (scheduler, counter) = make_scheduler().await;

        let mut job = CronJob::new("test", Schedule::Delay { secs: 60 });
        job.next_run_at = Some(Utc::now() - Duration::from_secs(1));
        job.state = JobState::Paused;
        scheduler.store.save_all(&[job]).await.unwrap();

        let fired = scheduler.tick().await.unwrap();
        assert_eq!(fired, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_tick_skips_future_job() {
        let (scheduler, counter) = make_scheduler().await;

        let mut job = CronJob::new("test", Schedule::Delay { secs: 3600 });
        job.next_run_at = Some(Utc::now() + Duration::from_secs(3600));
        scheduler.store.save_all(&[job]).await.unwrap();

        let fired = scheduler.tick().await.unwrap();
        assert_eq!(fired, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_repeat_exhaustion() {
        let (scheduler, counter) = make_scheduler().await;

        let mut job = CronJob::new("test", Schedule::Interval { secs: 60 });
        job.next_run_at = Some(Utc::now() - Duration::from_secs(1));
        job.repeat = Some(super::repeat(Some(2)));
        scheduler.store.save_all(&[job]).await.unwrap();

        // First tick: fires, completed=1
        scheduler.tick().await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        let jobs = scheduler.store.load_all().await.unwrap();
        assert_eq!(jobs[0].state, JobState::Scheduled);
        assert_eq!(jobs[0].repeat.as_ref().unwrap().completed, 1);

        // Second tick: fires, completed=2 → exhausted
        // Need to push next_run_at into the past again since interval=60s
        let mut jobs = scheduler.store.load_all().await.unwrap();
        jobs[0].next_run_at = Some(Utc::now() - Duration::from_secs(1));
        scheduler.store.save_all(&jobs).await.unwrap();

        scheduler.tick().await.unwrap();
        let jobs = scheduler.store.load_all().await.unwrap();
        assert_eq!(jobs[0].state, JobState::Completed);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
