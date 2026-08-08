//! SQLite backend for [`MemoryBackend`].
//!
//! Wraps [`terraphim_persistence::DeviceStorage`] (sqlite backend, opendal-
//! mediated). Each session is persisted as a single JSON document under
//! a key derived from the session ID.

use std::sync::Arc;

use async_trait::async_trait;
use terraphim_persistence::DeviceStorage;

use crate::memory::sqlite_key::{derive_index_key, derive_session_key};
use crate::memory::{MemoryBackend, MemoryError};
use crate::session::Session;

/// SQLite-backed memory store.
///
/// Each session is stored as a JSON document. A separate index document
/// records all known session IDs for cheap batch loading. Same pattern
/// as `cron::store`, adapted for memory.
#[derive(Debug, Clone)]
pub struct SqliteBackend {
    storage: Arc<DeviceStorage>,
    key_prefix: String,
}

impl SqliteBackend {
    /// Create a new SqliteBackend with the given device storage + key prefix.
    ///
    /// The `key_prefix` allows multiple backends in the same process
    /// without collision (one per tenant / role / config).
    pub fn new(storage: Arc<DeviceStorage>, key_prefix: impl Into<String>) -> Self {
        Self {
            storage,
            key_prefix: key_prefix.into(),
        }
    }
}

#[async_trait]
impl MemoryBackend for SqliteBackend {
    async fn get_or_create(&self, session_id: &str) -> Session {
        let session_key = derive_session_key(&self.key_prefix, session_id);
        // Read from storage; if not present, create new
        match self.storage.fastest_op.read(&session_key).await {
            Ok(bytes) => {
                let session: Session = serde_json::from_slice(&bytes.to_bytes())
                    .unwrap_or_else(|_| Session::new(session_id));
                session
            }
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Session::new(session_id),
            Err(e) => {
                // Fall back to empty session on other errors; persist will retry
                eprintln!(
                    "SqliteBackend::get_or_create({}) read error: {e}",
                    session_id
                );
                Session::new(session_id)
            }
        }
    }

    async fn persist(&self, session: &Session) -> Result<(), MemoryError> {
        let session_key = derive_session_key(&self.key_prefix, &session.key);
        let json = serde_json::to_vec(session)?;

        // Atomic-ish: write session first, then update index. If the session
        // write fails, we leave the index stale (still has the old session
        // ID). On next `get_or_create` we'll re-load the old session; user
        // can decide if it was lost. This matches the cron::store ordering.
        self.storage
            .fastest_op
            .write(&session_key, json)
            .await
            .map_err(|e| MemoryError::Storage(format!("write session: {e}")))?;

        // Update the index: read existing IDs, add ours if missing, write back.
        let index_key = derive_index_key(&self.key_prefix);
        let mut ids: Vec<String> = match self.storage.fastest_op.read(&index_key).await {
            Ok(bytes) => serde_json::from_slice(&bytes.to_bytes()).unwrap_or_default(),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(MemoryError::Storage(format!("read index: {e}"))),
        };
        if !ids.contains(&session.key) {
            ids.push(session.key.clone());
            let json_idx = serde_json::to_vec(&ids)?;
            self.storage
                .fastest_op
                .write(&index_key, json_idx)
                .await
                .map_err(|e| MemoryError::Storage(format!("write index: {e}")))?;
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<String>, MemoryError> {
        let index_key = derive_index_key(&self.key_prefix);
        match self.storage.fastest_op.read(&index_key).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes.to_bytes()).unwrap_or_default()),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(MemoryError::Storage(format!("read index: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ChatMessage;
    use uuid::Uuid;

    async fn make_backend_pair() -> (SqliteBackend, SqliteBackend) {
        let _ = DeviceStorage::init_memory_only().await;
        let storage = DeviceStorage::arc_memory_only().await.unwrap();
        // SAME prefix for both backends (proves they share storage state)
        let prefix = format!("test_mem_{}", Uuid::new_v4().simple());
        let a = SqliteBackend::new(storage.clone(), prefix.clone());
        let b = SqliteBackend::new(storage, prefix);
        (a, b)
    }

    #[tokio::test]
    async fn test_sqlite_get_or_create_creates_new_session() {
        let (backend, _) = make_backend_pair().await;
        let session = backend.get_or_create("chat-1").await;
        assert_eq!(session.key, "chat-1");
        assert!(session.is_empty());
    }

    #[tokio::test]
    async fn test_sqlite_persist_round_trip() {
        let (backend, backend2) = make_backend_pair().await;
        let id = format!("chat-{}", Uuid::new_v4().simple());
        let mut session = backend.get_or_create(&id).await;
        session.add_message(ChatMessage::user("hello", "user-1"));
        session.add_message(ChatMessage::assistant("world"));
        backend.persist(&session).await.unwrap();

        // Reload via a fresh backend sharing the same key_prefix
        let reloaded = backend2.get_or_create(&id).await;
        assert_eq!(reloaded.message_count(), 2);
        assert_eq!(reloaded.messages[0].content, "hello");
    }

    #[tokio::test]
    async fn test_sqlite_list_returns_persisted_ids() {
        let (backend, _) = make_backend_pair().await;
        for i in 0..3 {
            let mut s = backend.get_or_create(&format!("s-{i}")).await;
            s.add_message(ChatMessage::user("hi", "u"));
            backend.persist(&s).await.unwrap();
        }
        let listed = backend.list().await.unwrap();
        assert!(listed.contains(&"s-0".to_string()));
        assert!(listed.contains(&"s-1".to_string()));
        assert!(listed.contains(&"s-2".to_string()));
    }
}
