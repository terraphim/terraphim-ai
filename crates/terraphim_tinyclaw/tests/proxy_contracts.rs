//! Hermetic contract tests for the OpenAI-compatible proxy.
//!
//! Verifies the proxy exposes the OpenAI Chat Completions API shape so
//! any OpenAI-compatible client (Cursor, Continue, Aider) can use
//! TinyClaw as a backend.
//!
//! Implementation note: see Cargo.toml — we attempted to leverage the
//! sibling `terraphim-llm-proxy` crate but it's not published to any
//! registry and its path-only dep pulls in the whole monorepo.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use std::net::SocketAddr;
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
