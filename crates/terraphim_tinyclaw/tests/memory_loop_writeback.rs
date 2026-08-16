//! Live-loop integration tests for MemoryBackend consumption (#3227, T4).
//!
//! These tests exercise the `tests/memory_contracts.rs` round-trip
//! semantics through the *live* `ToolCallingLoop` (real bus, real
//! backend, real subprocess shim for the `terraphim-agent` memory
//! bridge — no mocks):
//!
//! 1. A turn driven through the loop persists the session through the
//!    `MemoryBackend` trait (reloaded from disk by a fresh backend).
//! 2. A compressed session leaves a memory item with the provenance tag
//!    `session-compression:<session_key>` in the memory bridge.
//! 3. Legacy session files (pre-trait on-disk layout) load unchanged
//!    through the new backend path.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use terraphim_tinyclaw::agent::agent_loop::{HybridLlmRouter, ToolCallingLoop};
use terraphim_tinyclaw::agent::proxy_client::ProxyClientConfig;
use terraphim_tinyclaw::bus::{InboundMessage, MessageBus, OutboundMessage};
use terraphim_tinyclaw::config::{AgentConfig, DirectLlmConfig, MemoryConfig};
use terraphim_tinyclaw::memory::{MemoryBackend, SharedBackend, jsonl::JsonlBackend};
use terraphim_tinyclaw::session::{ChatMessage, MessageRole, Session, SessionManager};
use terraphim_tinyclaw::tools::ToolRegistry;
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Router whose proxy and direct LLM are both unreachable (port 1 refuses
/// immediately), so `text_only` returns the deterministic fallback string
/// and `compress` falls back to the extractive summary.
fn unreachable_router() -> HybridLlmRouter {
    let proxy_config = ProxyClientConfig {
        base_url: "http://127.0.0.1:1".to_string(),
        timeout_ms: 1000,
        retry_after_secs: 1,
        ..Default::default()
    };
    let direct_config = DirectLlmConfig {
        base_url: Some("http://127.0.0.1:1".to_string()),
        ..Default::default()
    };
    HybridLlmRouter::new(proxy_config, direct_config)
}

/// Build a loop wired onto a `JsonlBackend` shared with a `SessionManager`
/// rooted at `sessions_dir`.
fn build_agent(sessions_dir: &Path, memory_config: Option<&MemoryConfig>) -> ToolCallingLoop {
    let sessions = Arc::new(Mutex::new(SessionManager::new(sessions_dir.to_path_buf())));
    let backend: SharedBackend = Arc::new(JsonlBackend::from_shared(sessions));
    let agent_config = AgentConfig {
        max_iterations: 10,
        ..Default::default()
    };
    ToolCallingLoop::with_backend(
        &agent_config,
        unreachable_router(),
        Arc::new(ToolRegistry::new()),
        backend,
        "Test system prompt".to_string(),
        memory_config,
    )
}

/// Write a shell shim standing in for the `terraphim-agent` binary.
/// Returns the path to the shim binary.
fn write_shim(dir: &Path, script: &str) -> PathBuf {
    let shim_path = dir.join("terraphim-agent-shim");
    std::fs::write(&shim_path, script).expect("write shim");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&shim_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms).unwrap();
    }
    shim_path
}

/// Spawn the live loop on a fresh bus.
fn spawn_loop(agent: ToolCallingLoop) -> (Arc<MessageBus>, tokio::task::JoinHandle<()>) {
    let bus = Arc::new(MessageBus::new());
    let bus_clone = bus.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = agent.run(bus_clone).await {
            log::error!("agent loop error in test: {e}");
        }
    });
    (bus, handle)
}

/// Drive one turn through the live loop and await the outbound reply.
async fn run_turn(bus: &MessageBus, content: &str) -> OutboundMessage {
    bus.inbound_sender()
        .send(InboundMessage::new("cli", "user1", "chat1", content))
        .await
        .expect("inbound send");
    let mut rx = bus.outbound_rx.lock().await;
    timeout(Duration::from_secs(15), rx.recv())
        .await
        .expect("outbound response within 15s")
        .expect("outbound channel open")
}

#[tokio::test]
async fn live_loop_round_trip_persists_session_through_backend() {
    common::scrub_env();
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");

    let agent = build_agent(&sessions_dir, None);
    let (bus, handle) = spawn_loop(agent);

    let outbound = run_turn(&bus, "hello from the live loop").await;
    assert!(!outbound.content.is_empty());
    handle.abort();

    // memory_contracts semantics through the live loop: reload via a
    // FRESH backend on the same dir (proves persistence, not cache).
    let backend = JsonlBackend::new(sessions_dir.clone());
    let reloaded = backend.get_or_create("cli:chat1").await;
    assert_eq!(reloaded.message_count(), 2);
    assert_eq!(reloaded.messages[0].role, MessageRole::User);
    assert_eq!(reloaded.messages[0].content, "hello from the live loop");
    assert_eq!(reloaded.messages[1].role, MessageRole::Assistant);
    assert_eq!(reloaded.messages[1].content, outbound.content);

    // The backend also lists the persisted session id.
    let listed = backend.list().await.unwrap();
    assert!(listed.contains(&"cli:chat1".to_string()));
}

#[tokio::test]
async fn compressed_session_leaves_memory_item_with_provenance_tag() {
    common::scrub_env();
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let capture_log = tmp.path().join("capture.log");

    // Shim records argv and stdin for every `terraphim-agent` invocation.
    let shim = write_shim(
        tmp.path(),
        &format!(
            r#"#!/bin/sh
echo "ARGS: $*" >> "{}"
echo "STDIN: $(cat)" >> "{}"
"#,
            capture_log.display(),
            capture_log.display()
        ),
    );

    let mem_cfg = MemoryConfig {
        enabled: true,
        binary: shim.to_string_lossy().to_string(),
        ..Default::default()
    };

    let agent = build_agent(&sessions_dir, Some(&mem_cfg));
    let (bus, handle) = spawn_loop(agent);

    // keep_last_messages defaults to 4, so the 9th user message pushes the
    // session past 4*2=8 messages and triggers compression.
    for i in 1..=9 {
        let outbound = run_turn(&bus, &format!("message {i}")).await;
        assert!(!outbound.content.is_empty());
    }
    handle.abort();

    // The compression summary was captured to the memory bridge with the
    // provenance tag (capture is awaited before the outbound reply, so no
    // race here). Compression fires twice across 9 turns: turn 5 reaches
    // 9 messages (u+a pairs), gets trimmed to 4+1=5; turn 8 reaches 10
    // and trims again.
    let log = std::fs::read_to_string(&capture_log).expect("capture log written by shim");
    let captures = log.matches("session-compression:cli:chat1").count();
    assert_eq!(
        captures, 2,
        "expected two provenance-tagged memory captures, got {captures}:\n{log}"
    );
    assert!(
        log.contains("memory capture --provenance-tag session-compression:cli:chat1"),
        "expected provenance-tagged memory capture, got:\n{log}"
    );
    assert!(
        log.contains("Summary of"),
        "expected the extractive fallback summary in the captured item, got:\n{log}"
    );

    // The session itself carries the summary and only the trimmed tail:
    // after the second compression (turn 8) the session holds 4+1=5
    // messages; turn 9 appends a user+assistant pair -> 7. Without
    // trimming the session would hold 18 messages.
    let backend = JsonlBackend::new(sessions_dir.clone());
    let reloaded = backend.get_or_create("cli:chat1").await;
    let summary = reloaded.summary.as_deref().expect("summary set");
    assert!(
        summary.contains("Summary of"),
        "expected extractive summary on session, got: {summary}"
    );
    assert_eq!(
        reloaded.message_count(),
        7,
        "trimmed tail + final turn (without compression this would be 18)"
    );
}

#[tokio::test]
async fn legacy_session_files_load_unchanged_through_backend() {
    common::scrub_env();
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");

    // Write a session in the legacy on-disk layout (SessionManager::save
    // appends one JSON line per save to <dir>/cli_chat1.jsonl).
    {
        let mut legacy = Session::new("cli:chat1");
        legacy.add_message(ChatMessage::user("legacy question", "user1"));
        legacy.add_message(ChatMessage::assistant("legacy answer"));
        let manager = SessionManager::new(sessions_dir.clone());
        manager.save(&legacy).unwrap();
    }

    let agent = build_agent(&sessions_dir, None);
    let (bus, handle) = spawn_loop(agent);

    let outbound = run_turn(&bus, "new question").await;
    handle.abort();

    // The legacy messages survived the new backend path unchanged and the
    // new turn was appended after them.
    let backend = JsonlBackend::new(sessions_dir.clone());
    let reloaded = backend.get_or_create("cli:chat1").await;
    assert_eq!(reloaded.message_count(), 4);
    assert_eq!(reloaded.messages[0].content, "legacy question");
    assert_eq!(reloaded.messages[1].content, "legacy answer");
    assert_eq!(reloaded.messages[2].content, "new question");
    assert_eq!(reloaded.messages[3].content, outbound.content);
}
