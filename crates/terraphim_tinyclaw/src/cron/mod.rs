//! Cron job scheduler with persistence.
//!
//! Wave 3 of the Hermes parity arc (epic #3160). Matches Hermes' `cron/` subsystem
//! surface (`cron/jobs.py`, `cron/scheduler.py`).

pub mod job;
pub mod scheduler;
pub mod store;

pub use job::{CronJob, JobState, RepeatConfig, Schedule};
pub use scheduler::CronScheduler;
pub use store::CronStore;

/// Errors the cron subsystem can produce.
#[derive(Debug, thiserror::Error)]
pub enum CronError {
    /// Persistence error.
    #[error("cron store error: {0}")]
    Store(String),

    /// Schedule parse error.
    #[error("invalid schedule: {0}")]
    InvalidSchedule(String),

    /// Job not found.
    #[error("job not found: {0}")]
    JobNotFound(String),

    /// Job execution error.
    #[error("job execution failed: {0}")]
    Execution(String),
}
