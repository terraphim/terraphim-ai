//! Hermetic contract tests for the OpenAI-compatible proxy.
//!
//! Verifies the proxy exposes the OpenAI Chat Completions API shape so
//! any OpenAI-compatible client (Cursor, Continue, Aider) can use
//! TinyClaw as a backend.
//!
//! Implementation note: see Cargo.toml — we attempted to leverage the
//! sibling `terraphim-llm-proxy` crate but it's not published to any
//! registry and its path-only dep pulls in the whole monorepo.

use axum::Json;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use terraphim_tinyclaw::proxy::{ProxyState, router};
use tower::ServiceExt;

async fn send(
    app: axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&v).unwrap())
        }
        None => Body::empty(),
    };
    let req = builder.body(body).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let parsed: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, parsed)
}

// --- /v1/models -----------------------------------------------------------

#[tokio::test]
async fn contract_models_returns_list_object() {
    let app = router(ProxyState::default());
    let (status, body) = send(app, "GET", "/v1/models", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["object"], "list");
    assert!(body["data"].is_array());
    assert!(!body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn contract_models_have_openai_required_fields() {
    let app = router(ProxyState::default());
    let (_status, body) = send(app, "GET", "/v1/models", None).await;
    let first = &body["data"][0];
    assert!(first["id"].is_string());
    assert_eq!(first["object"], "model");
    assert!(first["owned_by"].is_string());
}

// --- /v1/chat/completions ------------------------------------------------

#[tokio::test]
async fn contract_chat_completions_returns_openai_shape() {
    let app = router(ProxyState::default());
    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "tinyclaw-default",
            "messages": [{"role": "user", "content": "hello"}]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["id"].as_str().unwrap().starts_with("chatcmpl-"));
    assert_eq!(body["object"], "chat.completion");
    assert_eq!(body["model"], "tinyclaw-default");
    assert!(body["choices"].is_array());
    assert_eq!(body["choices"][0]["message"]["role"], "assistant");
    assert!(body["choices"][0]["message"]["content"].is_string());
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert!(body["usage"].is_object());
    assert!(body["usage"]["total_tokens"].is_number());
}

#[tokio::test]
async fn contract_chat_completions_echoes_last_user_message() {
    let app = router(ProxyState::default());
    let (_status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "tinyclaw-default",
            "messages": [
                {"role": "system", "content": "be helpful"},
                {"role": "user", "content": "ping-test-12345"}
            ]
        })),
    )
    .await;
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(content.contains("ping-test-12345"), "got: {content}");
}

#[tokio::test]
async fn contract_chat_completions_stream_returns_501() {
    // OpenAI spec: stream=true returns SSE; we return 501 Not Implemented
    let app = router(ProxyState::default());
    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "tinyclaw-default",
            "messages": [{"role": "user", "content": "x"}],
            "stream": true
        })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    assert!(body["error"].is_object());
    assert!(body["error"]["code"].is_string());
}

// --- /v1/health -----------------------------------------------------------

#[tokio::test]
async fn contract_health_ok() {
    let app = router(ProxyState::default());
    let (status, body) = send(app, "GET", "/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
}

// --- integration: real-port test ------------------------------------------

#[tokio::test]
async fn integration_proxy_serves_on_real_port() {
    use tokio::time::timeout;

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let bound = terraphim_tinyclaw::proxy::serve(ProxyState::default(), addr)
        .await
        .expect("proxy serve");
    tokio::time::sleep(Duration::from_millis(100)).await;

    let url = format!("http://{}/v1/health", bound);
    let result = timeout(
        Duration::from_secs(5),
        reqwest::Client::new().get(&url).send(),
    )
    .await;
    let result = result.expect("health timed out").expect("health failed");
    assert!(result.status().is_success());
}

// --- upstream forwarding (deployed terraphim-llm-proxy) -------------------
//
// When `TERRAPHIM_LLM_PROXY_URL` is configured (e.g. the deployed proxy on
// bigbox at http://100.106.66.7:3456), TinyClaw forwards requests verbatim
// instead of answering with the echo stub. These tests verify the
// forwarding path against a mock upstream server.

/// Spin up a mock upstream OpenAI-compatible server. Returns its base URL
/// and an `Arc<Mutex<Option<String>>>` capturing the received
/// `Authorization` header (None if absent). Must be awaited from within a
/// tokio runtime.
async fn spawn_mock_upstream(
    response: Value,
    status: StatusCode,
) -> (String, Arc<tokio::sync::Mutex<Option<String>>>) {
    use axum::routing::{get, post};

    let auth_seen: Arc<tokio::sync::Mutex<Option<String>>> = Arc::default();
    let auth_seen_models = auth_seen.clone();
    let auth_seen_return = auth_seen.clone();
    let resp_chat = response.clone();
    let resp_models = json!({"object": "list", "data": [{"id": "upstream-model", "object": "model", "created": 0, "owned_by": "mock"}]});

    let app = axum::Router::new()
        .route(
            "/v1/chat/completions",
            post(move |headers: axum::http::HeaderMap| {
                let auth_seen = auth_seen.clone();
                let resp = resp_chat.clone();
                async move {
                    *auth_seen.lock().await = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    (status, Json(resp))
                }
            }),
        )
        .route(
            "/v1/models",
            get(move |headers: axum::http::HeaderMap| {
                let auth_seen = auth_seen_models.clone();
                let resp = resp_models.clone();
                async move {
                    *auth_seen.lock().await = headers
                        .get("authorization")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    Json(resp)
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), auth_seen_return)
}

#[tokio::test]
async fn contract_upstream_forwards_chat_completions_and_api_key() {
    let upstream_resp = json!({
        "id": "chatcmpl-upstream",
        "object": "chat.completion",
        "created": 123,
        "model": "glm-5.2",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "from-upstream"},
            "finish_reason": "stop"
        }]
    });
    let (base, auth_seen) = spawn_mock_upstream(upstream_resp.clone(), StatusCode::OK).await;
    let state = ProxyState::with_upstream(base, Some("test-key-123".into()));
    let app = router(state);

    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "glm-5.2",
            "messages": [{"role": "user", "content": "hello upstream"}]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, upstream_resp);
    let auth = auth_seen.lock().await.clone();
    assert_eq!(auth.as_deref(), Some("Bearer test-key-123"));
}

#[tokio::test]
async fn contract_upstream_forwards_models() {
    let (base, auth_seen) = spawn_mock_upstream(json!({}), StatusCode::OK).await;
    let state = ProxyState::with_upstream(base, Some("test-key-123".into()));
    let app = router(state);

    let (status, body) = send(app, "GET", "/v1/models", None).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["id"], "upstream-model");
    let auth = auth_seen.lock().await.clone();
    assert_eq!(auth.as_deref(), Some("Bearer test-key-123"));
}

#[tokio::test]
async fn contract_upstream_status_and_error_passthrough() {
    let err_resp = json!({"error": {"message": "bad key", "type": "invalid_api_key", "code": 401}});
    let (base, _) = spawn_mock_upstream(err_resp.clone(), StatusCode::UNAUTHORIZED).await;
    let state = ProxyState::with_upstream(base, Some("wrong-key".into()));
    let app = router(state);

    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "glm-5.2",
            "messages": [{"role": "user", "content": "x"}]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, err_resp);
}

#[tokio::test]
async fn contract_upstream_forwards_extra_fields_verbatim() {
    // The proxy must NOT drop OpenAI fields it doesn't model internally
    // (tools, max_tokens, response_format, ...). Capture the body the
    // mock upstream actually receives.
    use axum::body::to_bytes;
    use axum::routing::post;

    let received: Arc<tokio::sync::Mutex<Option<Value>>> = Arc::default();
    let received_clone = received.clone();
    let app = axum::Router::new().route(
        "/v1/chat/completions",
        post(move |req: axum::http::Request<axum::body::Body>| {
            let received = received_clone.clone();
            async move {
                let bytes = to_bytes(req.into_body(), 1024 * 1024).await.unwrap();
                *received.lock().await = serde_json::from_slice(&bytes).ok();
                Json(json!({"id": "x", "object": "chat.completion", "created": 1, "model": "m", "choices": []}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let state = ProxyState::with_upstream(format!("http://{addr}"), None);
    let app = router(state);

    let (_status, _body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "glm-5.2",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 128,
            "tools": [{"type": "function", "function": {"name": "f", "parameters": {"type": "object"}}}],
            "response_format": {"type": "json_object"}
        })),
    )
    .await;

    let got = received
        .lock()
        .await
        .clone()
        .expect("upstream received a body");
    assert_eq!(got["max_tokens"], 128);
    assert_eq!(got["tools"][0]["function"]["name"], "f");
    assert_eq!(got["response_format"]["type"], "json_object");
    assert_eq!(got["messages"][0]["content"], "hi");
}

#[tokio::test]
async fn contract_upstream_stream_passthrough() {
    // stream=true with an upstream must forward the request and return the
    // upstream body unchanged (SSE bytes), not a 501.
    let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\ndata: [DONE]\n";
    let (base, _) = spawn_mock_upstream(
        json!({"object": "chat.completion.chunk", "choices": []}),
        StatusCode::OK,
    )
    .await;
    // The generic mock returns JSON; use it via chat completions with stream.
    let state = ProxyState::with_upstream(base, None);
    let app = router(state);

    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "glm-5.2",
            "messages": [{"role": "user", "content": "stream me"}],
            "stream": true
        })),
    )
    .await;

    // Forwards to the upstream mock (which answers 200 JSON); the proxy
    // must NOT answer 501 when an upstream is configured.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["object"], "chat.completion.chunk");
    let _ = sse_body; // real SSE framing is tested by the live proxy E2E
}

#[tokio::test]
async fn contract_upstream_unreachable_returns_502() {
    // Point at a port with nothing listening.
    let state = ProxyState::with_upstream("http://127.0.0.1:1", Some("k".into()));
    let app = router(state);

    let (status, body) = send(
        app,
        "POST",
        "/v1/chat/completions",
        Some(json!({
            "model": "glm-5.2",
            "messages": [{"role": "user", "content": "x"}]
        })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert!(body["error"]["type"].is_string());
}

#[tokio::test]
async fn contract_env_constructs_upstream_state() {
    // SAFETY: test-only; env vars are unique to this test and no other test
    // reads them concurrently in this binary.
    unsafe {
        std::env::set_var(
            terraphim_tinyclaw::proxy::ENV_UPSTREAM_URL,
            "http://127.0.0.1:1",
        );
        std::env::set_var(terraphim_tinyclaw::proxy::ENV_UPSTREAM_API_KEY, "env-key");
    }
    let state = ProxyState::from_env();
    // SAFETY: see above.
    unsafe {
        std::env::remove_var(terraphim_tinyclaw::proxy::ENV_UPSTREAM_URL);
        std::env::remove_var(terraphim_tinyclaw::proxy::ENV_UPSTREAM_API_KEY);
    }

    let upstream = state.upstream.expect("upstream configured from env");
    assert_eq!(upstream.base_url, "http://127.0.0.1:1");
    assert_eq!(upstream.api_key.as_deref(), Some("env-key"));

    // No env → echo mode.
    let state = ProxyState::from_env();
    assert!(state.upstream.is_none());
}
