//! Hermetic contract tests for the ACP adapter.
//!
//! Ports of Hermes' `tests/acp/test_session.py` and `test_server.py`
//! contracts:
//! - `initialize` returns protocol version, agent info, capabilities
//! - `new_session` creates a session
//! - `load_session` returns existing or errors
//! - `list_sessions` enumerates
//! - `send_message` appends to a session
//! - `cancel` is a no-op success for known sessions, error for unknown
//! - `load_session` for unknown session returns `-32004`

mod common;

use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use terraphim_tinyclaw::acp::AcpState;
use terraphim_tinyclaw::acp::router::{JsonRpcRequest, dispatch};
use terraphim_tinyclaw::session::MessageRole;

/// Per-test counter so each test gets a unique sessions_dir and isolates
/// from other tests sharing the filesystem.
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn make_state() -> AcpState {
    common::scrub_env();
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("acp_test_{n}_{}", uuid::Uuid::new_v4().simple()));
    std::fs::create_dir_all(&dir).unwrap();
    AcpState::new(dir)
}

fn rpc(id: u32, method: &str, params: Value) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
        id: Some(json!(id)),
    }
}

async fn call(state: &AcpState, method: &str, params: Value) -> Value {
    let req = rpc(1, method, params);
    let resp = dispatch(state, req).await;
    serde_json::to_value(&resp).unwrap()
}

// --- initialize -------------------------------------------------------------

#[tokio::test]
async fn contract_initialize_returns_protocol_version() {
    let state = make_state();
    let resp = call(&state, "initialize", json!({})).await;
    assert!(resp["result"].is_object(), "expected result, got: {resp}");
    assert!(resp["result"]["protocolVersion"].is_string());
    assert_eq!(resp["result"]["protocolVersion"], "0.1");
}

#[tokio::test]
async fn contract_initialize_returns_agent_info() {
    let state = make_state();
    let resp = call(&state, "initialize", json!({})).await;
    let info = &resp["result"]["agentInfo"];
    assert_eq!(info["name"], "tinyclaw");
    assert!(info["version"].is_string());
}

#[tokio::test]
async fn contract_initialize_returns_capabilities() {
    let state = make_state();
    let resp = call(&state, "initialize", json!({})).await;
    let caps = &resp["result"]["capabilities"];
    assert_eq!(caps["loadSession"], true);
    assert_eq!(caps["streaming"], false);
}

// --- new_session ------------------------------------------------------------

#[tokio::test]
async fn contract_new_session_creates_session() {
    let state = make_state();
    let resp = call(&state, "new_session", json!("chat-1")).await;
    assert!(resp["result"]["session_id"].is_string());
    assert_eq!(resp["result"]["session_id"], "chat-1");
}

#[tokio::test]
async fn contract_new_session_idempotent() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-2")).await;
    let resp = call(&state, "new_session", json!("chat-2")).await;
    assert_eq!(resp["result"]["session_id"], "chat-2");
}

// --- load_session -----------------------------------------------------------

#[tokio::test]
async fn contract_load_session_returns_existing() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-3")).await;
    let resp = call(&state, "load_session", json!("chat-3")).await;
    assert_eq!(resp["result"]["session_id"], "chat-3");
}

#[tokio::test]
async fn contract_load_session_not_found_returns_error() {
    let state = make_state();
    let resp = call(&state, "load_session", json!("ghost")).await;
    assert!(resp["error"].is_object(), "expected error: {resp}");
    assert_eq!(resp["error"]["code"], -32004);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

// --- list_sessions ---------------------------------------------------------

#[tokio::test]
async fn contract_list_sessions_empty_initially() {
    let state = make_state();
    let resp = call(&state, "list_sessions", json!({})).await;
    assert!(resp["result"]["sessions"].is_array());
    assert_eq!(resp["result"]["sessions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn contract_list_sessions_returns_created() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("s1")).await;
    let _ = call(&state, "new_session", json!("s2")).await;
    let resp = call(&state, "list_sessions", json!({})).await;
    let sessions = resp["result"]["sessions"].as_array().unwrap();
    assert!(sessions.contains(&json!("s1")));
    assert!(sessions.contains(&json!("s2")));
}

// --- send_message ----------------------------------------------------------

#[tokio::test]
async fn contract_send_message_dispatches_one_user_turn_without_acp_persisting_it() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-4")).await;
    let resp = call(
        &state,
        "send_message",
        json!({
            "session_id": "chat-4",
            "role": "user",
            "content": "hello"
        }),
    )
    .await;
    assert_eq!(resp["result"]["session_id"], "chat-4");
    assert_eq!(resp["result"]["message_index"], 0);

    let received = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        state.bus.inbound_rx.lock().await.recv(),
    )
    .await
    .expect("user message should dispatch")
    .expect("one inbound user turn");
    assert_eq!(received.channel, "acp");
    assert_eq!(received.chat_id, "chat-4");
    assert_eq!(received.content, "hello");

    let no_duplicate = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        state.bus.inbound_rx.lock().await.recv(),
    )
    .await;
    assert!(no_duplicate.is_err(), "user turn dispatched more than once");

    let mut manager = state.sessions.lock().await;
    let session = manager.get_or_create("chat-4");
    assert_eq!(
        session.message_count(),
        0,
        "ACP must not pre-save dispatched user turns"
    );
}

#[tokio::test]
async fn contract_send_message_persists_assistant_without_dispatching() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-5")).await;
    let resp = call(
        &state,
        "send_message",
        json!({"session_id": "chat-5", "role": "assistant", "content": "second"}),
    )
    .await;
    assert_eq!(resp["result"]["message_index"], 0);

    let no_dispatch = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        state.bus.inbound_rx.lock().await.recv(),
    )
    .await;
    assert!(no_dispatch.is_err(), "assistant entries must not dispatch");

    let mut manager = state.sessions.lock().await;
    let session = manager.get_or_create("chat-5");
    assert_eq!(session.message_count(), 1);
    assert_eq!(session.messages[0].role, MessageRole::Assistant);
    assert_eq!(session.messages[0].content, "second");
}

#[tokio::test]
async fn contract_send_message_persists_tool_without_dispatching() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-tool")).await;
    let resp = call(
        &state,
        "send_message",
        json!({"session_id": "chat-tool", "role": "tool", "content": "tool output"}),
    )
    .await;
    assert_eq!(resp["result"]["message_index"], 0);

    let no_dispatch = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        state.bus.inbound_rx.lock().await.recv(),
    )
    .await;
    assert!(no_dispatch.is_err(), "tool entries must not dispatch");

    let mut manager = state.sessions.lock().await;
    let session = manager.get_or_create("chat-tool");
    assert_eq!(session.message_count(), 1);
    assert_eq!(session.messages[0].role, MessageRole::Tool);
    assert_eq!(session.messages[0].content, "tool output");
}

#[tokio::test]
async fn contract_send_message_to_unknown_session_errors() {
    let state = make_state();
    let resp = call(
        &state,
        "send_message",
        json!({
            "session_id": "ghost",
            "role": "user",
            "content": "hi"
        }),
    )
    .await;
    assert_eq!(resp["error"]["code"], -32004);
}

#[tokio::test]
async fn contract_send_message_rejects_invalid_role() {
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-6")).await;
    let resp = call(
        &state,
        "send_message",
        json!({"session_id": "chat-6", "role": "system", "content": "x"}),
    )
    .await;
    assert_eq!(resp["error"]["code"], -32602);
}

// --- cancel ----------------------------------------------------------------

#[tokio::test]
async fn contract_cancel_known_session_succeeds() {
    // TinyClaw's cancel returns success with the ack payload in `result`
    // (code: 0 means "ok"). The error envelope is reserved for actual
    // failures.
    let state = make_state();
    let _ = call(&state, "new_session", json!("chat-7")).await;
    let resp = call(&state, "cancel", json!({"session_id": "chat-7"})).await;
    assert!(resp["result"].is_object(), "expected result, got: {resp}");
    assert_eq!(resp["result"]["code"], 0);
}

#[tokio::test]
async fn contract_cancel_unknown_session_errors() {
    let state = make_state();
    let resp = call(&state, "cancel", json!({"session_id": "ghost"})).await;
    assert_eq!(resp["error"]["code"], -32004);
}

// --- unknown method --------------------------------------------------------

#[tokio::test]
async fn contract_unknown_method_returns_method_not_found() {
    let state = make_state();
    let resp = call(&state, "no_such_method", json!({})).await;
    assert_eq!(resp["error"]["code"], -32601);
}
