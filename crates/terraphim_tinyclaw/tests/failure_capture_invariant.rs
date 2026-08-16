//! Invariant failure-capture integration tests (#3225).
//!
//! Learning-from-failure must be a loop invariant: when an exec-class
//! tool call fails inside a turn, the agent loop captures the failed
//! command via `terraphim-agent learn capture` with no model
//! involvement. These tests drive real turns through the live
//! `ToolCallingLoop` (real bus, real `ShellTool`, real `terraphim-agent`
//! binary, real HTTP stub standing in for the LLM proxy — no mocks):
//!
//! 1. A failing shell command in a turn produces a learning file in the
//!    workspace learnings store.
//! 2. A failing command matching the test-runner ignore globs
//!    (`cargo test*`) produces NO learning.
//! 3. A broken `terraphim-agent` binary degrades to a `warn` log — the
//!    turn still completes (fail-open).
//!
//! The real `terraphim-agent` binary is required on PATH. If it is
//! absent the tests FAIL LOUDLY rather than skipping silently.

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use terraphim_tinyclaw::agent::agent_loop::{HybridLlmRouter, ToolCallingLoop};
use terraphim_tinyclaw::agent::proxy_client::ProxyClientConfig;
use terraphim_tinyclaw::bus::{InboundMessage, MessageBus, OutboundMessage};
use terraphim_tinyclaw::config::{AgentConfig, DirectLlmConfig, MemoryConfig};
use terraphim_tinyclaw::memory::jsonl::JsonlBackend;
use terraphim_tinyclaw::session::SessionManager;
use terraphim_tinyclaw::tools::ToolRegistry;
use terraphim_tinyclaw::tools::shell::ShellTool;
use tokio::sync::Mutex;
use tokio::time::timeout;

/// Locate the real `terraphim-agent` binary on PATH. Fails loudly when
/// absent: silent skips would let the invariant regress unnoticed.
fn agent_binary() -> PathBuf {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("terraphim-agent");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!(
        "terraphim-agent binary not found on PATH. \
         These tests exercise the real learn-capture subprocess; \
         install terraphim-agent instead of skipping."
    );
}

/// Scripted Anthropic-shaped responses for the stub LLM endpoint.
struct StubLlm {
    responses: Vec<Value>,
    hits: AtomicUsize,
}

async fn messages_handler(
    State(state): State<Arc<StubLlm>>,
    Json(_body): Json<Value>,
) -> Json<Value> {
    let idx = state.hits.fetch_add(1, Ordering::SeqCst);
    let response = state
        .responses
        .get(idx)
        .or_else(|| state.responses.last())
        .cloned()
        .expect("stub LLM needs at least one scripted response");
    Json(response)
}

/// Anthropic-format response carrying a single tool call.
fn tool_use_response(command: &str) -> Value {
    json!({
        "model": "stub-model",
        "content": [{
            "type": "tool_use",
            "id": "call_1",
            "name": "shell",
            "input": { "command": command }
        }],
        "stop_reason": "tool_use",
        "usage": { "input_tokens": 10, "output_tokens": 5 }
    })
}

/// Anthropic-format plain-text response (terminates the tool loop).
fn text_response(text: &str) -> Value {
    json!({
        "model": "stub-model",
        "content": [{ "type": "text", "text": text }],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 10, "output_tokens": 5 }
    })
}

/// Spawn a real HTTP server on a loopback port answering
/// `POST /v1/messages` with the scripted responses. Returns its base URL.
async fn spawn_stub_llm(responses: Vec<Value>) -> String {
    let state = Arc::new(StubLlm {
        responses,
        hits: AtomicUsize::new(0),
    });
    let app = Router::new()
        .route("/v1/messages", post(messages_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stub LLM");
    let addr = listener.local_addr().expect("stub LLM addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve stub LLM");
    });
    format!("http://{}", addr)
}

/// Build a live loop pointed at the stub LLM, with the memory bridge
/// enabled against the given binary and the workspace rooted at
/// `workspace`.
fn build_agent(
    workspace: &std::path::Path,
    proxy_base_url: &str,
    agent_binary: &str,
) -> ToolCallingLoop {
    let sessions = Arc::new(Mutex::new(SessionManager::new(workspace.join("sessions"))));
    let backend = Arc::new(JsonlBackend::from_shared(sessions));

    let agent_config = AgentConfig {
        max_iterations: 5,
        workspace: workspace.to_path_buf(),
        ..Default::default()
    };
    let proxy_config = ProxyClientConfig {
        base_url: proxy_base_url.to_string(),
        timeout_ms: 10_000,
        retry_after_secs: 1,
        ..Default::default()
    };
    let router = HybridLlmRouter::new(proxy_config, DirectLlmConfig::default());

    let mut tools = ToolRegistry::new();
    tools.register(Box::new(ShellTool::new()));

    let memory_config = MemoryConfig {
        enabled: true,
        binary: agent_binary.to_string(),
        timeout_secs: 30,
        ..Default::default()
    };

    ToolCallingLoop::with_backend(
        &agent_config,
        router,
        Arc::new(tools),
        backend,
        "Test system prompt".to_string(),
        Some(&memory_config),
    )
}

/// Drive one turn through the live loop and await the outbound reply.
async fn run_turn(agent: ToolCallingLoop, content: &str) -> OutboundMessage {
    let bus = Arc::new(MessageBus::new());
    let bus_clone = bus.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = agent.run(bus_clone).await {
            log::error!("agent loop error in test: {e}");
        }
    });

    bus.inbound_sender()
        .send(InboundMessage::new("cli", "user1", "chat1", content))
        .await
        .expect("inbound send");
    let outbound = {
        let mut rx = bus.outbound_rx.lock().await;
        timeout(Duration::from_secs(60), rx.recv())
            .await
            .expect("outbound response within 60s")
            .expect("outbound channel open")
    };
    handle.abort();
    outbound
}

/// Learning files captured under the workspace learnings store.
fn learning_files(workspace: &std::path::Path) -> Vec<PathBuf> {
    let dir = workspace.join(".terraphim").join("learnings");
    if !dir.is_dir() {
        return Vec::new();
    }
    std::fs::read_dir(dir)
        .expect("read learnings dir")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("learning-"))
                .unwrap_or(false)
        })
        .collect()
}

#[tokio::test]
async fn failed_shell_command_is_captured_as_learning() {
    common::scrub_env();
    let binary = agent_binary();
    let tmp = tempfile::tempdir().unwrap();
    // terraphim-agent stores project-locally only when `.terraphim`
    // already exists in the working directory.
    std::fs::create_dir_all(tmp.path().join(".terraphim")).unwrap();

    let failing_command = "ls /definitely-missing-tinyclaw-3225";
    let stub = spawn_stub_llm(vec![
        tool_use_response(failing_command),
        text_response("the command failed"),
    ])
    .await;

    let agent = build_agent(tmp.path(), &stub, &binary.to_string_lossy());
    let outbound = run_turn(agent, "please list that directory").await;
    assert!(!outbound.content.is_empty());

    let learnings = learning_files(tmp.path());
    assert!(
        !learnings.is_empty(),
        "a failed shell command in a turn must produce a learning file under {}",
        tmp.path().join(".terraphim/learnings").display()
    );
    let body = std::fs::read_to_string(&learnings[0]).expect("read learning file");
    assert!(
        body.contains(failing_command),
        "learning file should record the failed command.\nBody: {body}"
    );
}

#[tokio::test]
async fn test_runner_command_is_not_captured() {
    common::scrub_env();
    let binary = agent_binary();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".terraphim")).unwrap();

    // A real failing command that matches the `cargo test*` ignore glob:
    // cargo exits non-zero because the manifest path does not exist.
    let ignored_command = "cargo test --manifest-path /definitely-missing-tinyclaw-3225/Cargo.toml";
    let stub = spawn_stub_llm(vec![
        tool_use_response(ignored_command),
        text_response("the tests did not run"),
    ])
    .await;

    let agent = build_agent(tmp.path(), &stub, &binary.to_string_lossy());
    let outbound = run_turn(agent, "run the test suite").await;
    assert!(!outbound.content.is_empty());

    let learnings = learning_files(tmp.path());
    assert!(
        learnings.is_empty(),
        "test-runner commands must not be captured as learnings, found: {learnings:?}"
    );
}

#[tokio::test]
async fn capture_failure_is_fail_open() {
    common::scrub_env();
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".terraphim")).unwrap();

    let failing_command = "ls /definitely-missing-tinyclaw-3225";
    let stub = spawn_stub_llm(vec![
        tool_use_response(failing_command),
        text_response("turn completed despite capture failure"),
    ])
    .await;

    // Point the memory bridge at a nonexistent binary: the capture
    // subprocess cannot spawn. The turn must still complete — a capture
    // failure is a warn log, never an error to the turn.
    let agent = build_agent(tmp.path(), &stub, "/nonexistent/terraphim-agent-3225");
    let outbound = run_turn(agent, "please list that directory").await;
    assert_eq!(outbound.content, "turn completed despite capture failure");
}
