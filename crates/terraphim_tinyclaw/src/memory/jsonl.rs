//! JSONL file backend for [`MemoryBackend`].
//!
//! Wraps the existing [`crate::session::SessionManager`]. Default backend
//! for desktop / single-machine deployments. Per-session state lives in
//! a `<sessions_dir>/<session_key>.jsonl` file.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::memory::{MemoryBackend, MemoryError};
use crate::session::{Session, SessionManager};

/// JSONL file backend.
///
/// Holds a shared `SessionManager` (which itself owns an in-memory cache
/// of loaded sessions + a disk-backed jsonl file per session). The
/// manager is behind an `Arc<tokio::sync::Mutex>` so the same manager can
/// be shared with other consumers (e.g. the session tools) while the
/// agent loop drives persistence through the [`MemoryBackend`] trait.
pub struct JsonlBackend {
    manager: Arc<Mutex<SessionManager>>,
}

impl JsonlBackend {
    /// Create a new JsonlBackend rooted at `sessions_dir`.
    ///
    /// Creates the directory if it doesn't exist.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            manager: Arc::new(Mutex::new(SessionManager::new(sessions_dir))),
        }
    }

    /// Wrap an already-shared `SessionManager`.
    ///
    /// Use this when other components (e.g. `SessionListTool`) hold the
    /// same `Arc<Mutex<SessionManager>>`: the backend and those components
    /// then serialise on the same mutex and observe the same cache, which
    /// preserves the pre-trait locking behaviour of the agent loop.
    pub fn from_shared(manager: Arc<Mutex<SessionManager>>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl MemoryBackend for JsonlBackend {
    async fn get_or_create(&self, session_id: &str) -> Session {
        let mut manager = self.manager.lock().await;
        let session = manager.get_or_create(session_id);
        session.clone()
    }

    async fn persist(&self, session: &Session) -> Result<(), MemoryError> {
        let mut manager = self.manager.lock().await;
        manager
            .save(session)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        // Invalidate the in-memory cache so the next `get_or_create`
        // reloads from disk (jsonl is append-only; without this, the
        // session in the cache would never reflect disk updates).
        manager.invalidate_cache(&session.key);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, MemoryError> {
        let manager = self.manager.lock().await;
        let sessions = manager
            .list_sessions()
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ChatMessage;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn make_backend() -> (TempDir, JsonlBackend) {
        let tmp = TempDir::new().unwrap();
        let backend = JsonlBackend::new(tmp.path().to_path_buf());
        (tmp, backend)
    }

    #[tokio::test]
    async fn test_jsonl_get_or_create_creates_new_session() {
        let (_tmp, backend) = make_backend();
        let session = backend.get_or_create("chat-1").await;
        assert_eq!(session.key, "chat-1");
        assert!(session.is_empty());
        assert_eq!(session.message_count(), 0);
    }

    #[tokio::test]
    async fn test_jsonl_persist_round_trip() {
        let tmp = TempDir::new().unwrap();
        let id = format!("chat-{}", Uuid::new_v4().simple());
        let backend = JsonlBackend::new(tmp.path().to_path_buf());
        let mut session = backend.get_or_create(&id).await;
        session.add_message(ChatMessage::user("hello", "user-1"));
        session.add_message(ChatMessage::assistant("world"));
        backend.persist(&session).await.unwrap();

        // Reload from disk via a fresh backend with the SAME dir (proves
        // persistence, not cache)
        let backend2 = JsonlBackend::new(tmp.path().to_path_buf());
        let reloaded = backend2.get_or_create(&id).await;
        assert_eq!(reloaded.message_count(), 2);
        assert_eq!(reloaded.messages[0].content, "hello");
        assert_eq!(reloaded.messages[1].content, "world");
    }

    #[tokio::test]
    async fn test_jsonl_list_returns_persisted_ids() {
        let (_tmp, backend) = make_backend();
        for i in 0..3 {
            let mut s = backend.get_or_create(&format!("s-{i}")).await;
            s.add_message(ChatMessage::user("hi", "u"));
            backend.persist(&s).await.unwrap();
        }
        let listed = backend.list().await.unwrap();
        let expected: Vec<String> = (0..3).map(|i| format!("s-{i}")).collect();
        assert!(expected.iter().all(|id| listed.contains(id)));
    }

    #[tokio::test]
    async fn test_jsonl_from_shared_observes_same_manager() {
        // A backend built from a shared manager must see writes made
        // through the same manager (and vice versa) because both
        // serialise on the same mutex and cache.
        let tmp = TempDir::new().unwrap();
        let shared = Arc::new(Mutex::new(SessionManager::new(tmp.path().to_path_buf())));
        let backend = JsonlBackend::from_shared(shared.clone());

        let mut session = backend.get_or_create("shared-1").await;
        session.add_message(ChatMessage::user("hello", "user-1"));
        backend.persist(&session).await.unwrap();

        // The shared manager reloads from disk after the backend's
        // persist invalidated the cache entry.
        let mut manager = shared.lock().await;
        let reloaded = manager.get_or_create("shared-1");
        assert_eq!(reloaded.message_count(), 1);
        assert_eq!(reloaded.messages[0].content, "hello");
    }
}
