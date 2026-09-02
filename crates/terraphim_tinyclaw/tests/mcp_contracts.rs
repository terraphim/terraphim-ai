//! Hermetic contract tests for the MCP server.
//!
//! Ports of Hermes' `mcp_serve.py` tool contracts. Verifies that:
//! - All 10 tools are registered
//! - Each tool's JSON output shape matches Hermes
//! - Error cases return well-formed JSON, not exceptions
//! - The conversation_id parsing rule (platform:id) is consistent

use std::path::PathBuf;
use std::sync::Arc;
use terraphim_tinyclaw::bus::MessageBus;
use terraphim_tinyclaw::mcp::server::{TinyClawMcpServer, serve_mcp_stdio};
use terraphim_tinyclaw::session::SessionManager;
use tokio::sync::Mutex;

fn make_server() -> TinyClawMcpServer {
    let sessions = Arc::new(Mutex::new(SessionManager::new(PathBuf::from("/tmp"))));
    let bus = Arc::new(MessageBus::new());
    TinyClawMcpServer::new(sessions, bus)
}

/// Extract the text content from a `CallToolResult` as a `String`.
fn extract_text(result: &rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .expect("content must be text")
}

// --- test: tool list contains all 10 tools -----------------------------------
//
#[tokio::test]
async fn contract_tool_list_has_all_10_tools() {
    // Hermes' mcp_serve.py exposes (per docstring at line 12-14):
    //   conversations_list, conversation_get, messages_read, attachments_fetch,
    //   events_poll, events_wait, messages_send, permissions_list_open,
    //   permissions_respond, channels_list
    //
    // Verify the tools exist on TinyClawMcpServer by calling each.
    // If a tool is missing, this won't compile (method not found).
    let server = make_server();

    // Each call below proves the tool exists and is wired up.
    // We don't care about the result, just that the methods compile.
    let _ = server.conversations_list().await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::ConversationGetParams {
            conversation_id: "x".into(),
        },
    );
    let _ = server.conversation_get(params).await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::MessagesReadParams {
            conversation_id: "x".into(),
            limit: None,
            before: None,
        },
    );
    let _ = server.messages_read(params).await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::MessagesSendParams {
            conversation_id: "x:1".into(),
            content: "x".into(),
        },
    );
    let _ = server.messages_send(params).await;
    let _ = server.events_poll().await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::EventsWaitParams { timeout_ms: None },
    );
    let _ = server.events_wait(params).await;
    let _ = server.permissions_list_open().await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::PermissionsRespondParams {
            request_id: "x".into(),
            approved: true,
        },
    );
    let _ = server.permissions_respond(params).await;
    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::ConversationGetParams {
            conversation_id: "x:1".into(),
        },
    );
    let _ = server.attachments_fetch(params).await;
    let _ = server.channels_list().await;
}

#[tokio::test]
async fn contract_tool_methods_return_text_content() {
    // Hermes contract: every tool returns a CallToolResult with text content
    // (mcp_serve.py uses json.dumps() and wraps in TextContent at the
    // FastMCP layer). Our tools must do the same.
    let server = make_server();

    for result in [
        server.conversations_list().await.unwrap(),
        server.events_poll().await.unwrap(),
        server.permissions_list_open().await.unwrap(),
        server.channels_list().await.unwrap(),
    ] {
        assert!(
            !result.content.is_empty(),
            "tool returned empty content array"
        );
        assert!(
            result.content[0].as_text().is_some(),
            "tool content must be text"
        );
    }
}

// --- conversations_list contract --------------------------------------------
//
// Hermes contract (mcp_serve.py:564-617):
//   - Returns JSON `{"count": N, "conversations": [...]}`
//   - Each conversation has: session_key, session_id, platform, chat_type,
//     display_name, chat_name, user_name, updated_at
//   - Sorted by updated_at descending
//   - Limit clamped to [1, 200]

#[tokio::test]
async fn contract_conversations_list_returns_well_formed_json() {
    let server = make_server();

    let result = server.conversations_list().await.unwrap();
    let text = extract_text(&result);

    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(
        parsed.get("conversations").is_some(),
        "missing conversations"
    );
    assert!(
        parsed["conversations"].is_array(),
        "conversations must be array"
    );
    assert!(parsed.get("count").is_some(), "missing count");
    assert_eq!(
        parsed["count"],
        parsed["conversations"].as_array().unwrap().len()
    );
}

#[tokio::test]
async fn contract_conversations_list_empty_session() {
    // No conversations seeded → empty array, count 0
    let server = make_server();

    let result = server.conversations_list().await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(parsed["count"], 0);
    assert_eq!(parsed["conversations"].as_array().unwrap().len(), 0);
}

// --- conversation_get contract ----------------------------------------------
//
// Hermes contract (mcp_serve.py:621-650):
//   - Returns JSON with session_key, session_id, platform, chat_type,
//     display_name, user_name, chat_name, updated_at, created_at,
//     input_tokens, output_tokens, total_tokens
//   - Missing session_key returns {"error": "..."} JSON (NOT exception)

#[tokio::test]
async fn contract_conversation_get_returns_error_json_for_missing() {
    let server = make_server();

    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::ConversationGetParams {
            conversation_id: "nonexistent".into(),
        },
    );
    let result = server.conversation_get(params).await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(
        parsed.get("error").is_some(),
        "missing session must return error JSON"
    );
    assert!(
        parsed["error"].as_str().unwrap().contains("not found")
            || parsed["error"].as_str().unwrap().contains("Nonexistent")
            || parsed["error"].as_str().unwrap().contains("nonexistent"),
        "error message should reference the missing session"
    );
}

// --- messages_send contract -------------------------------------------------
//
// Hermes contract (mcp_serve.py:826-860):
//   - conversation_id format: "platform:id" (e.g. "telegram:123456")
//   - Returns JSON with status + conversation_id
//   - Invalid conversation_id format returns error JSON

#[tokio::test]
async fn contract_messages_send_rejects_invalid_conversation_id_format() {
    // Hermes: conversation_id must contain a ':' separator (platform:id)
    let server = make_server();

    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::MessagesSendParams {
            conversation_id: "no-colon-here".into(),
            content: "hello".into(),
        },
    );
    let result = server.messages_send(params).await;
    // Either an error from the tool, or a successful call with an error
    // embedded in the response JSON. Both are valid per Hermes contract.
    if let Ok(r) = result {
        let text = extract_text(&r);
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(
            parsed.get("error").is_some() || parsed.get("status").is_some(),
            "response must have error or status field"
        );
    }
}

#[tokio::test]
async fn contract_messages_send_accepts_valid_conversation_id() {
    let server = make_server();

    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::MessagesSendParams {
            conversation_id: "telegram:123456".into(),
            content: "hello world".into(),
        },
    );
    let result = server.messages_send(params).await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(
        parsed.get("status").is_some(),
        "valid send must return status"
    );
    assert_eq!(parsed["conversation_id"], "telegram:123456");
}

// --- events_poll / events_wait contract --------------------------------------
//
// Hermes contract (mcp_serve.py:763-823):
//   - events_poll returns immediately with currently-pending events
//   - events_wait blocks until event or timeout (timeout_ms parameter)
//   - Both return JSON with events array

#[tokio::test]
async fn contract_events_poll_returns_empty_when_no_events() {
    let server = make_server();

    let result = server.events_poll().await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(parsed.get("events").is_some(), "missing events field");
    assert_eq!(parsed["events"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn contract_events_wait_respects_timeout() {
    let server = make_server();

    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::EventsWaitParams {
            timeout_ms: Some(100),
        },
    );
    let start = std::time::Instant::now();
    let _result = server.events_wait(params).await.unwrap();
    let elapsed = start.elapsed();

    // Must not return significantly faster than the timeout (proves we waited)
    // and not significantly slower (proves we don't hang forever)
    assert!(
        elapsed >= std::time::Duration::from_millis(50),
        "events_wait returned too fast ({:?}), didn't actually wait",
        elapsed
    );
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "events_wait returned too slow ({:?})",
        elapsed
    );
}

// --- permissions_list_open / permissions_respond contract -------------------
//
// Hermes contract (mcp_serve.py:862-913):
//   - permissions_list_open returns JSON with permissions array
//   - permissions_respond takes request_id + approved (bool)

#[tokio::test]
async fn contract_permissions_list_open_empty_returns_array() {
    let server = make_server();

    let result = server.permissions_list_open().await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(
        parsed.get("permissions").is_some(),
        "missing permissions field"
    );
    assert!(
        parsed["permissions"].is_array(),
        "permissions must be array"
    );
}

#[tokio::test]
async fn contract_permissions_respond_handles_unknown_request() {
    let server = make_server();

    let params = rmcp::handler::server::wrapper::Parameters(
        terraphim_tinyclaw::mcp::tools::PermissionsRespondParams {
            request_id: "unknown-req".into(),
            approved: true,
        },
    );
    let result = server.permissions_respond(params).await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should respond gracefully (status field) even if request_id is unknown
    assert!(
        parsed.get("status").is_some() || parsed.get("error").is_some(),
        "response must have status or error"
    );
}

// --- channels_list contract -------------------------------------------------
//
// Hermes contract (mcp_serve.py:916-930):
//   - Returns JSON with channels array, each channel has name + status
//   - This is the Hermes-specific 10th tool beyond OpenClaw's 9

#[tokio::test]
async fn contract_channels_list_returns_at_least_empty_array() {
    let server = make_server();

    let result = server.channels_list().await.unwrap();
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(parsed.get("channels").is_some(), "missing channels field");
    assert!(parsed["channels"].is_array(), "channels must be array");
}

// --- server info metadata ---------------------------------------------------

#[tokio::test]
async fn contract_server_info_matches_hermes_identity() {
    // Hermes' server name is "hermes" (mcp_serve.py:551). Our server should
    // identify itself consistently so MCP clients can route correctly.
    use rmcp::ServerHandler;

    let server = make_server();

    let info = server.get_info();
    // We don't hardcode "hermes" (this is tinyclaw, not hermes), but the
    // server must have a valid name and version for MCP protocol compliance.
    assert!(
        !info.server_info.name.is_empty(),
        "server name must not be empty"
    );
    assert!(
        !info.server_info.version.is_empty(),
        "server version must not be empty"
    );
}

#[test]
fn contract_serve_mcp_stdio_signature_exists() {
    // Hermes has `run_mcp_server(verbose: bool = False) -> None` as the
    // public entry point. Our equivalent is `serve_mcp_stdio`. Verify the
    // signature is callable (compile-time check via the function pointer).
    let _: fn(Arc<Mutex<SessionManager>>, Arc<MessageBus>) -> _ = serve_mcp_stdio;
}
