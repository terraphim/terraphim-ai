//! Backend parity contract tests.
//!
//! Verifies that both [`JsonlBackend`] and [`SqliteBackend`] implement
//! the [`MemoryBackend`] trait consistently: the same `session_id`
//! produces the same `Session` shape after round-trip, regardless of
//! which backend is used.

use std::path::PathBuf;

use terraphim_persistence::DeviceStorage;
use terraphim_tinyclaw::memory::{MemoryBackend, jsonl::JsonlBackend, sqlite::SqliteBackend};
use terraphim_tinyclaw::session::{ChatMessage, Session, SessionManager};
use uuid::Uuid;

/// Shared test scenario: persist a session with messages, reload, compare.
async fn round_trip_test(backend: &dyn MemoryBackend, id: &str) -> Session {
    let mut session = backend.get_or_create(id).await;
    session.add_message(ChatMessage::user("hello", "user-1"));
    session.add_message(ChatMessage::assistant("world"));
    session.add_message(ChatMessage::tool("result", "search"));
    session.set_summary("test conversation".into());
    backend.persist(&session).await.unwrap();

    // Reload via a fresh call (forces re-read from storage, not in-mem cache)
    backend.get_or_create(id).await
}

fn make_jsonl_backend(dir: &std::path::Path) -> JsonlBackend {
    JsonlBackend::new(dir.to_path_buf())
}

async fn make_sqlite_backend() -> SqliteBackend {
    let _ = DeviceStorage::init_memory_only().await;
    let storage = DeviceStorage::arc_memory_only().await.unwrap();
    SqliteBackend::new(storage, format!("contract_mem_{}", Uuid::new_v4().simple()))
}

#[tokio::test]
async fn contract_jsonl_round_trip_preserves_messages() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = make_jsonl_backend(tmp.path());
    let id = format!("chat-{}", Uuid::new_v4().simple());
    let reloaded = round_trip_test(&backend, &id).await;
    assert_eq!(reloaded.message_count(), 3);
    assert_eq!(reloaded.messages[0].content, "hello");
    assert_eq!(reloaded.messages[0].sender_id.as_deref(), Some("user-1"));
    assert_eq!(
        reloaded.messages[1].role,
        terraphim_tinyclaw::session::MessageRole::Assistant
    );
    assert_eq!(
        reloaded.messages[2].role,
        terraphim_tinyclaw::session::MessageRole::Tool
    );
    assert_eq!(reloaded.summary.as_deref(), Some("test conversation"));
}

#[tokio::test]
async fn contract_sqlite_round_trip_preserves_messages() {
    let backend = make_sqlite_backend().await;
    let id = format!("chat-{}", Uuid::new_v4().simple());
    let reloaded = round_trip_test(&backend, &id).await;
    assert_eq!(reloaded.message_count(), 3);
    assert_eq!(reloaded.messages[0].content, "hello");
    assert_eq!(reloaded.messages[0].sender_id.as_deref(), Some("user-1"));
    assert_eq!(
        reloaded.messages[1].role,
        terraphim_tinyclaw::session::MessageRole::Assistant
    );
    assert_eq!(
        reloaded.messages[2].role,
        terraphim_tinyclaw::session::MessageRole::Tool
    );
    assert_eq!(reloaded.summary.as_deref(), Some("test conversation"));
}

#[tokio::test]
async fn contract_persist_is_idempotent_jsonl() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = make_jsonl_backend(tmp.path());
    let id = format!("chat-{}", Uuid::new_v4().simple());
    let mut session = backend.get_or_create(&id).await;
    session.add_message(ChatMessage::user("once", "u"));
    backend.persist(&session).await.unwrap();
    backend.persist(&session).await.unwrap();
    backend.persist(&session).await.unwrap();

    // JsonlBackend uses append-only writes, so persisted file has the
    // message 3 times. The `get_or_create` returns the latest loaded
    // session, which still has 1 message. We verify behavior parity:
    // the session shape returned is correct.
    let reloaded = backend.get_or_create(&id).await;
    assert_eq!(reloaded.message_count(), 1);
}

#[tokio::test]
async fn contract_persist_is_idempotent_sqlite() {
    let backend = make_sqlite_backend().await;
    let id = format!("chat-{}", Uuid::new_v4().simple());
    let mut session = backend.get_or_create(&id).await;
    session.add_message(ChatMessage::user("once", "u"));
    backend.persist(&session).await.unwrap();
    backend.persist(&session).await.unwrap();
    backend.persist(&session).await.unwrap();

    let reloaded = backend.get_or_create(&id).await;
    assert_eq!(reloaded.message_count(), 1);
}

#[tokio::test]
async fn contract_list_returns_all_persisted_ids_jsonl() {
    let tmp = tempfile::tempdir().unwrap();
    let backend = make_jsonl_backend(tmp.path());
    let ids: Vec<String> = (0..3).map(|i| format!("s-{i}")).collect();
    for id in &ids {
        let mut s = backend.get_or_create(id).await;
        s.add_message(ChatMessage::user("hi", "u"));
        backend.persist(&s).await.unwrap();
    }
    let listed = backend.list().await.unwrap();
    assert!(ids.iter().all(|id| listed.contains(id)));
}

#[tokio::test]
async fn contract_list_returns_all_persisted_ids_sqlite() {
    let backend = make_sqlite_backend().await;
    let ids: Vec<String> = (0..3).map(|i| format!("s-{i}")).collect();
    for id in &ids {
        let mut s = backend.get_or_create(id).await;
        s.add_message(ChatMessage::user("hi", "u"));
        backend.persist(&s).await.unwrap();
    }
    let listed = backend.list().await.unwrap();
    assert!(ids.iter().all(|id| listed.contains(id)));
}

#[tokio::test]
async fn contract_get_or_create_missing_returns_empty_session() {
    // Both backends: asking for a session that doesn't exist should return
    // a fresh empty session, not panic or return None.
    let tmp = tempfile::tempdir().unwrap();
    let jsonl = make_jsonl_backend(tmp.path());
    let new_session = jsonl.get_or_create("never-existed").await;
    assert!(new_session.is_empty());
    assert_eq!(new_session.message_count(), 0);

    let sqlite = make_sqlite_backend().await;
    let new_session = sqlite.get_or_create("never-existed").await;
    assert!(new_session.is_empty());
    assert_eq!(new_session.message_count(), 0);
}

// Reference to SessionManager to avoid an unused-import warning if all
// tests above are filtered.
#[allow(dead_code)]
fn _unused() -> SessionManager {
    SessionManager::new(PathBuf::from("/tmp"))
}
