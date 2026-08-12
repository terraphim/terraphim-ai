//! End-to-end dispatch tests for TinyClaw shared agent-loop entry paths.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use terraphim_tinyclaw::acp::AcpState;
use terraphim_tinyclaw::acp::router::{JsonRpcRequest, dispatch as acp_dispatch};
use terraphim_tinyclaw::agent::agent_loop::{HybridLlmRouter, ToolCallingLoop};
use terraphim_tinyclaw::agent::proxy_client::ProxyClientConfig;
use terraphim_tinyclaw::bus::{InboundMessage, MessageBus};
use terraphim_tinyclaw::config::{AgentConfig, DirectLlmConfig, MemoryConfig};
use terraphim_tinyclaw::dashboard::{DashboardState, router as dashboard_router};
use terraphim_tinyclaw::proxy::{ProxyState, router as proxy_router};
use terraphim_tinyclaw::session::SessionManager;
use terraphim_tinyclaw::tools::{Tool, ToolRegistry};
use terraphim_tinyclaw::tui::TuiSurface;
use tower::ServiceExt;

struct StaticTool {
    name: &'static str,
    response: &'static str,
}

#[async_trait::async_trait]
impl Tool for StaticTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "static test tool"
    }

    fn parameters_schema(&self) -> Value {
        json!({"type": "object", "properties": {}})
    }

    async fn execute(&self, _args: Value) -> Result<String, terraphim_tinyclaw::tools::ToolError> {
        Ok(self.response.to_string())
    }
}

async fn spawn_proxy_sequence() -> (String, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_route = calls.clone();
    let app = Router::new().route(
        "/v1/messages",
        post(move |Json(body): Json<Value>| {
            let calls = calls_for_route.clone();
            async move {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                if call == 0 {
                    let tools = body["tools"].as_array().cloned().unwrap_or_default();
                    let pick = ["memory_capture", "subagent", "sandbox", "schedule", "browser"]
                        .iter()
                        .find(|name| tools.iter().any(|t| t["name"] == **name))
                        .copied()
                        .unwrap_or("memory_capture");
                    let input = match pick {
                        "memory_capture" => json!({"content": "remember sushi", "provenance_tag": "uat"}),
                        "subagent" => json!({"op": "list"}),
                        "sandbox" => json!({"op": "execute_bash", "command": "echo sandbox-e2e"}),
                        "schedule" => json!({"op": "list"}),
                        "browser" => json!({"op": "extract", "url": "http://127.0.0.1:1/"}),
                        _ => json!({}),
                    };
                    Json(json!({
                        "model": "test",
                        "stop_reason": "tool_use",
                        "usage": {"input_tokens": 1, "output_tokens": 1},
                        "content": [{"type": "tool_use", "id": "tool-1", "name": pick, "input": input}]
                    }))
                } else {
                    Json(json!({
                        "model": "test",
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 1, "output_tokens": 1},
                        "content": [{"type": "text", "text": "agent-loop-final-response"}]
                    }))
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), calls)
}

async fn spawn_memory_asserting_proxy() -> (String, Arc<tokio::sync::Mutex<Option<String>>>) {
    let system_seen: Arc<tokio::sync::Mutex<Option<String>>> = Arc::default();
    let system_for_route = system_seen.clone();
    let app = Router::new().route(
        "/v1/messages",
        post(move |Json(body): Json<Value>| {
            let system_for_route = system_for_route.clone();
            async move {
                *system_for_route.lock().await = body["system"].as_str().map(|s| s.to_string());
                let system = body["system"].as_str().unwrap_or_default();
                let text = if system.contains("prefers sushi") {
                    "I remembered your sushi preference."
                } else {
                    "No memory context was applied."
                };
                Json(json!({
                    "model": "test",
                    "stop_reason": "end_turn",
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                    "content": [{"type": "text", "text": text}]
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), system_seen)
}

fn write_memory_shim(dir: &std::path::Path) -> std::path::PathBuf {
    let shim = dir.join("terraphim-agent");
    let script = r#"#!/bin/sh
case "$1 $2" in
  "memory apply")
    echo "Fresh-session memory: user prefers sushi for team lunches."
    ;;
  "memory capture")
    cat > /dev/null
    echo "Memory captured: fresh-session"
    ;;
  "memory export")
    echo '{"memory_items":[{"id":"fresh","content":"user prefers sushi for team lunches"}]}'
    ;;
  *)
    echo "unknown: $*" >&2
    exit 1
    ;;
esac
"#;
    std::fs::write(&shim, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    shim
}

async fn make_loop(
    bus: Arc<MessageBus>,
    tools: ToolRegistry,
    sessions_dir: std::path::PathBuf,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    let (proxy_url, _calls) = spawn_proxy_sequence().await;
    let router = HybridLlmRouter::new(
        ProxyClientConfig {
            base_url: proxy_url,
            api_key: "test-key".to_string(),
            timeout_ms: 5_000,
            model: Some("tinyclaw-test".to_string()),
            retry_after_secs: 1,
        },
        DirectLlmConfig::default(),
    );
    let agent = AgentConfig {
        workspace: sessions_dir.clone(),
        max_iterations: 4,
        ..AgentConfig::default()
    };
    let sessions = Arc::new(tokio::sync::Mutex::new(SessionManager::new(sessions_dir)));
    let loop_ = ToolCallingLoop::new(
        &agent,
        router,
        Arc::new(tools),
        sessions,
        "system prompt".to_string(),
        None,
    );
    tokio::spawn(async move { loop_.run(bus).await })
}

async fn make_memory_loop(
    bus: Arc<MessageBus>,
    sessions_dir: std::path::PathBuf,
    memory_config: MemoryConfig,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    Arc<tokio::sync::Mutex<Option<String>>>,
) {
    let (proxy_url, system_seen) = spawn_memory_asserting_proxy().await;
    let router = HybridLlmRouter::new(
        ProxyClientConfig {
            base_url: proxy_url,
            api_key: "test-key".to_string(),
            timeout_ms: 5_000,
            model: Some("tinyclaw-test".to_string()),
            retry_after_secs: 1,
        },
        DirectLlmConfig::default(),
    );
    let agent = AgentConfig {
        workspace: sessions_dir.clone(),
        max_iterations: 4,
        ..AgentConfig::default()
    };
    let loop_ = ToolCallingLoop::new(
        &agent,
        router,
        Arc::new(ToolRegistry::new()),
        Arc::new(tokio::sync::Mutex::new(SessionManager::new(sessions_dir))),
        "system prompt".to_string(),
        Some(&memory_config),
    );
    (
        tokio::spawn(async move { loop_.run(bus).await }),
        system_seen,
    )
}

async fn read_bus_message(bus: &Arc<MessageBus>) -> InboundMessage {
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        bus.inbound_rx.lock().await.recv(),
    )
    .await
    .expect("message timed out")
    .expect("message present")
}

#[tokio::test]
async fn tui_dashboard_proxy_and_acp_dispatch_into_same_agent_loop_entry_bus() {
    common::scrub_env();
    let bus = Arc::new(MessageBus::new());

    TuiSurface::new()
        .submit(bus.clone(), "from tui")
        .await
        .unwrap();
    assert_eq!(read_bus_message(&bus).await.channel, "tui");

    let state =
        DashboardState::new_in_memory(tempfile::tempdir().unwrap().path().to_path_buf()).await;
    let dashboard_bus = state.bus.clone();
    let app = dashboard_router(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/agent/messages")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"content":"from dashboard"}"#))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    assert_eq!(read_bus_message(&dashboard_bus).await.channel, "dashboard");

    let app = proxy_router(ProxyState::default().with_agent_bus(bus.clone()));
    let req = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"tinyclaw-default","messages":[{"role":"user","content":"from proxy"}]}"#,
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(read_bus_message(&bus).await.channel, "proxy");

    let acp_dir = tempfile::tempdir().unwrap();
    let state = AcpState::with_bus(acp_dir.path().to_path_buf(), bus.clone());
    let _ = acp_dispatch(
        &state,
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "new_session".to_string(),
            params: json!("acp-chat"),
            id: Some(json!(1)),
        },
    )
    .await;
    let _ = acp_dispatch(
        &state,
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "send_message".to_string(),
            params: json!({"session_id":"acp-chat","role":"user","content":"from acp"}),
            id: Some(json!(2)),
        },
    )
    .await;
    assert_eq!(read_bus_message(&bus).await.channel, "acp");
}

#[tokio::test]
async fn fresh_session_memory_capture_retrieve_apply_response_flow_uat() {
    common::scrub_env();
    let tmp = tempfile::tempdir().unwrap();
    let shim = write_memory_shim(tmp.path());
    let bus = Arc::new(MessageBus::new());
    let memory = MemoryConfig {
        enabled: true,
        binary: shim.to_string_lossy().to_string(),
        role: None,
        timeout_secs: 5,
        max_context_chars: 4000,
    };
    let (_handle, system_seen) =
        make_memory_loop(bus.clone(), tmp.path().join("fresh-session"), memory).await;

    bus.inbound_sender()
        .send(InboundMessage::new(
            "cli",
            "user",
            "fresh-session",
            "What should we order for lunch?",
        ))
        .await
        .unwrap();

    let outbound = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        bus.outbound_rx.lock().await.recv(),
    )
    .await
    .expect("outbound response timed out")
    .expect("outbound response");

    assert!(outbound.content.contains("sushi preference"));
    let system = system_seen.lock().await.clone().unwrap_or_default();
    assert!(system.contains("Fresh-session memory"));
    assert!(system.contains("prefers sushi"));
}

#[tokio::test]
async fn single_conversation_tool_dispatch_e2e_reaches_agent_loop_final_response() {
    common::scrub_env();
    let bus = Arc::new(MessageBus::new());
    let mut tools = ToolRegistry::new();
    tools.register(Box::new(StaticTool {
        name: "memory_capture",
        response: "memory captured",
    }));
    tools.register(Box::new(StaticTool {
        name: "subagent",
        response: "subagent listed",
    }));
    tools.register(Box::new(StaticTool {
        name: "sandbox",
        response: "sandbox output",
    }));
    tools.register(Box::new(StaticTool {
        name: "schedule",
        response: "schedule listed",
    }));
    tools.register(Box::new(StaticTool {
        name: "browser",
        response: "browser extracted",
    }));
    let sessions_tmp = tempfile::tempdir().unwrap();
    let _handle = make_loop(bus.clone(), tools, sessions_tmp.path().to_path_buf()).await;

    bus.inbound_sender()
        .send(InboundMessage::new(
            "cli",
            "user",
            "single-conversation",
            "remember this then use tools",
        ))
        .await
        .unwrap();

    let outbound = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        bus.outbound_rx.lock().await.recv(),
    )
    .await
    .expect("outbound response timed out")
    .expect("outbound response");
    assert_eq!(outbound.channel, "cli");
    assert_eq!(outbound.content, "agent-loop-final-response");
}

#[tokio::test]
async fn messaging_channel_dispatch_e2e_round_trips_through_agent_loop() {
    common::scrub_env();
    let bus = Arc::new(MessageBus::new());
    let mut tools = ToolRegistry::new();
    tools.register(Box::new(StaticTool {
        name: "memory_capture",
        response: "captured",
    }));
    let sessions_tmp = tempfile::tempdir().unwrap();
    let _handle = make_loop(bus.clone(), tools, sessions_tmp.path().to_path_buf()).await;

    bus.inbound_sender()
        .send(InboundMessage::new("slack", "U1", "C1", "channel dispatch"))
        .await
        .unwrap();
    let outbound = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        bus.outbound_rx.lock().await.recv(),
    )
    .await
    .expect("outbound response timed out")
    .expect("outbound response");
    assert_eq!(outbound.channel, "slack");
    assert_eq!(outbound.chat_id, "C1");
    assert_eq!(outbound.content, "agent-loop-final-response");
}
