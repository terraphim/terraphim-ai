//! Cron job persistence via `terraphim_persistence`.
//!
//! Wave 3 of the Hermes parity arc. Jobs are stored as a JSON-serialised list
//! under a single key, matching Hermes' `jobs.json` flat-file approach.

use std::collections::HashMap;
use std::sync::Arc;
use terraphim_persistence::DeviceStorage;

use super::CronError;
use super::job::CronJob;

/// Persistent store for cron jobs.
#[derive(Clone)]
pub struct CronStore {
    storage: Arc<DeviceStorage>,
    key: String,
}

impl CronStore {
    /// Create a new store using the given storage backend and key prefix.
    ///
    /// The store reads/writes a single `HashMap<String, CronJob>` under `key`.
    pub fn new(storage: Arc<DeviceStorage>, key: impl Into<String>) -> Self {
        Self {
            storage,
            key: key.into(),
        }
    }

    /// Load all jobs from the store.
    pub async fn load_all(&self) -> Result<Vec<CronJob>, CronError> {
        match self.storage.restore::<HashMap<String, CronJob>>(&self.key).await {
            Ok(Some(map)) => Ok(map.into_values().collect()),
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(CronError::Store(e.to_string())),
        }
    }

    /// Save all jobs to the store (atomic write via DeviceStorage).
    pub async fn save_all(&self, jobs: &[CronJob]) -> Result<(), CronError> {
        let mut map = HashMap::new();
        for job in jobs {
            map.insert(job.id.clone(), job.clone());
        }
        self.storage
            .persist(&self.key, &map)
            .await
            .map_err(|e| CronError::Store(e.to_string()))
    }

    /// Load and return jobs as a map for O(1) lookup.
    pub async fn load_map(&self) -> Result<HashMap<String, CronJob>, CronError> {
        match self.storage.restore::<HashMap<String, CronJob>>(&self.key).await {
            Ok(Some(map)) => Ok(map),
            Ok(None) => Ok(HashMap::new()),
            Err(e) => Err(CronError::Store(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cron::job::{JobState, Schedule};
    use tempfile::TempDir;

    async fn make_store() -> (CronStore, TempDir) {
        let dir = TempDir::new().unwrap();
        std::env::set_var("TERRAPHIM_HOME", dir.path());
        let storage = Arc::new(
            DeviceStorage::new()
                .await
                .expect("DeviceStorage::new should succeed"),
        );
        let store = CronStore::new(storage, "test_cron_jobs");
        (store, dir)
    }

    #[tokio::test]
    async fn test_store_round_trip() {
        let (store, _dir) = make_store().await;

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
        let (store, _dir) = make_store().await;
        let loaded = store.load_all().await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn test_store_overwrite() {
        let (store, _dir) = make_store().await;

        let job1 = CronJob::new("first", Schedule::Delay { secs: 60 });
        store.save_all(&[job1.clone()]).await.unwrap();

        let job2 = CronJob::new("second", Schedule::Interval { secs: 120 });
        store.save_all(&[job2.clone()]).await.unwrap();

        let loaded = store.load_all().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, job2.id);
    }
}
