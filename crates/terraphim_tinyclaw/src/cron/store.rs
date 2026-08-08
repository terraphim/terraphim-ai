//! Cron job persistence via `terraphim_persistence::DeviceStorage`.
//!
//! Wave 3 of the Hermes parity arc. Each job is stored as a JSON document
//! under a key derived from the job ID. A separate index document tracks
//! the set of job IDs.
//!
//! Uses `DeviceStorage::fastest_op` (opendal `Operator`) for raw read/write
//! to keep the implementation independent of the `Persistable` trait
//! (which has private fields).

use std::sync::Arc;
use terraphim_persistence::DeviceStorage;

use super::CronError;
use super::job::CronJob;

/// Persistent store for cron jobs.
///
/// For hermetic tests, call `DeviceStorage::init_memory_only()` before
/// constructing the store.
#[derive(Clone)]
pub struct CronStore {
    storage: Arc<DeviceStorage>,
    /// Key for the job-index document.
    index_key: String,
}

impl CronStore {
    /// Create a new store.
    pub fn new(storage: Arc<DeviceStorage>, index_key: impl Into<String>) -> Self {
        Self {
            storage,
            index_key: index_key.into(),
        }
    }

    /// Load all job IDs from the index. Returns an empty vec if the index
    /// does not exist yet.
    async fn load_index(&self) -> Result<Vec<String>, CronError> {
        match self.storage.fastest_op.read(&self.index_key).await {
            Ok(bytes) => {
                let index: Vec<String> = serde_json::from_slice(bytes.to_bytes().as_ref())
                    .map_err(|e| CronError::Store(format!("parse index: {e}")))?;
                Ok(index)
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Save the job ID index.
    async fn save_index(&self, ids: &[String]) -> Result<(), CronError> {
        let json = serde_json::to_vec(ids)
            .map_err(|e| CronError::Store(format!("serialise index: {e}")))?;
        self.storage
            .fastest_op
            .write(&self.index_key, json)
            .await
            .map_err(|e| CronError::Store(format!("write index: {e}")))?;
        Ok(())
    }

    /// Load a single job by ID.
    async fn load_job(&self, id: &str) -> Result<Option<CronJob>, CronError> {
        let key = format!("cron_job:{id}");
        match self.storage.fastest_op.read(&key).await {
            Ok(bytes) => {
                let job: CronJob = serde_json::from_slice(bytes.to_bytes().as_ref())
                    .map_err(|e| CronError::Store(format!("parse job {id}: {e}")))?;
                Ok(Some(job))
            }
            Err(e) => {
                let kind = e.kind();
                if format!("{kind:?}").contains("NotFound") {
                    Ok(None)
                } else {
                    Err(CronError::Store(format!("read job {id}: {e}")))
                }
            }
        }
    }

    /// Save a single job.
    async fn save_job(&self, job: &CronJob) -> Result<(), CronError> {
        let key = format!("cron_job:{}", job.id);
        let json =
            serde_json::to_vec(job).map_err(|e| CronError::Store(format!("serialise job: {e}")))?;
        self.storage
            .fastest_op
            .write(&key, json)
            .await
            .map_err(|e| CronError::Store(format!("write job: {e}")))?;
        Ok(())
    }

    /// Load all jobs.
    pub async fn load_all(&self) -> Result<Vec<CronJob>, CronError> {
        let ids = self.load_index().await?;
        let mut jobs = Vec::new();
        for id in ids {
            if let Some(job) = self.load_job(&id).await? {
                jobs.push(job);
            }
        }
        Ok(jobs)
    }

    /// Get a single job by ID.
    ///
    /// Hermes contract: `get_job(job_id) -> Optional[Dict]` returns the
    /// job or None if not found. This ports `cron/jobs.py:get_job`.
    pub async fn get_job(&self, job_id: &str) -> Result<Option<CronJob>, CronError> {
        self.load_job(job_id).await
    }

    /// Remove a job by ID.
    ///
    /// Hermes contract: `remove_job(job_id) -> bool` returns True if the
    /// job existed and was removed, False if not found. This ports
    /// `cron/jobs.py:remove_job`.
    pub async fn remove_job(&self, job_id: &str) -> Result<bool, CronError> {
        let mut jobs = self.load_all().await?;
        let before = jobs.len();
        jobs.retain(|j| j.id != job_id);
        if jobs.len() < before {
            self.save_all(&jobs).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Save all jobs (replaces index + persists each job).
    pub async fn save_all(&self, jobs: &[CronJob]) -> Result<(), CronError> {
        for job in jobs {
            self.save_job(job).await?;
        }
        let ids: Vec<String> = jobs.iter().map(|j| j.id.clone()).collect();
        self.save_index(&ids).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::job::{JobState, Schedule};

    async fn make_store() -> CronStore {
        // Ensure memory-only backend is initialised
        let _ = DeviceStorage::init_memory_only().await;
        let storage = DeviceStorage::arc_memory_only()
            .await
            .expect("arc memory-only DeviceStorage");
        let key = format!("test_cron_index_{}", uuid::Uuid::new_v4().simple());
        CronStore::new(storage, key)
    }

    #[tokio::test]
    async fn test_store_round_trip() {
        let store = make_store().await;

        let mut job = CronJob::new("hello world", Schedule::Delay { secs: 60 });
        job.state = JobState::Paused;

        store.save_all(&[job.clone()]).await.unwrap();

        let loaded = store.load_all().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, job.id);
        assert_eq!(loaded[0].state, JobState::Paused);
        assert_eq!(loaded[0].prompt, "hello world");
    }

    #[tokio::test]
    async fn test_store_empty() {
        let store = make_store().await;
        let loaded = store.load_all().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_store_overwrite() {
        let store = make_store().await;

        let job1 = CronJob::new("first", Schedule::Delay { secs: 60 });
        store.save_all(std::slice::from_ref(&job1)).await.unwrap();

        let job2 = CronJob::new("second", Schedule::Interval { secs: 120 });
        store.save_all(std::slice::from_ref(&job2)).await.unwrap();

        let loaded = store.load_all().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, job2.id);
    }
}
