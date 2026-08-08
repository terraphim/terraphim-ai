//! JSONL file backend for [`MemoryBackend`].
//!
//! Wraps the existing [`crate::session::SessionManager`]. Default backend
//! for desktop / single-machine deployments. Per-session state lives in
//! a `<sessions_dir>/<session_key>.jsonl` file.

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::memory::{MemoryBackend, MemoryError};
use crate::session::{Session, SessionManager};

/// JSONL file backend.
///
/// Holds a `SessionManager` (which itself owns an in-memory cache of
/// loaded sessions + a disk-backed jsonl file per session).
pub struct JsonlBackend {
    manager: Mutex<SessionManager>,
}

impl JsonlBackend {
    /// Create a new JsonlBackend rooted at `sessions_dir`.
    ///
    /// Creates the directory if it doesn't exist.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            manager: Mutex::new(SessionManager::new(sessions_dir)),
        }
    }
}

#[async_trait]
impl MemoryBackend for JsonlBackend {
    async fn get_or_create(&self, session_id: &str) -> Session {
        let mut manager = self.manager.lock().unwrap();
        let session = manager.get_or_create(session_id);
        session.clone()
    }

    async fn persist(&self, session: &Session) -> Result<(), MemoryError> {
        let mut manager = self.manager.lock().unwrap();
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
        let manager = self.manager.lock().unwrap();
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
}
