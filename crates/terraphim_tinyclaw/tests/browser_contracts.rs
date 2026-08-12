//! Hermetic contract tests for `BrowserTool` (#3148).
//!
//! Uses a local axum server as the target so no external network is
//! needed in CI.

mod common;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::net::SocketAddr;
use terraphim_tinyclaw::tools::browser::BrowserTool;
use terraphim_tinyclaw::tools::{Tool, ToolError};

fn make_browser() -> BrowserTool {
    common::scrub_env();
    let cfg = terraphim_tinyclaw::config::BrowserConfig {
        enabled: true,
        timeout_secs: 10,
        max_bytes: 512 * 1024,
        proxy: None,
    };
    BrowserTool::from_config(&cfg).expect("browser tool builds")
}

/// Spin up a local HTTP server; returns its base URL.
async fn spawn_test_server() -> String {
    let app = Router::new()
        .route(
            "/page",
            get(|| async {
                (
                    [("content-type", "text/html")],
                    "<html><head><title>Test Page</title></head><body><h1>Hello World</h1><p>some body text</p><button id=\"login\">Login</button><input id=\"username\" name=\"username\" /></body></html>",
                )
            }),
        )
        .route(
            "/api/echo",
            post(|body: Json<Value>| async move {
                Json(json!({ "received": body.0 }))
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn browser_navigate_returns_title_and_preview() {
    let base = spawn_test_server().await;
    let tool = make_browser();
    let out = tool
        .execute(json!({"op": "navigate", "url": format!("{base}/page")}))
        .await
        .expect("navigate should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "navigate");
    assert_eq!(v["status"], 200);
    assert_eq!(v["title"], "Test Page");
    assert!(v["preview"].as_str().unwrap().contains("Hello World"));
}

#[tokio::test]
async fn browser_extract_returns_text() {
    let base = spawn_test_server().await;
    let tool = make_browser();
    let out = tool
        .execute(json!({"op": "extract", "url": format!("{base}/page")}))
        .await
        .expect("extract should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], 200);
    let text = v["text"].as_str().unwrap();
    assert!(
        text.contains("Hello World"),
        "text should contain page body, got: {text}"
    );
    assert!(text.contains("some body text"));
}

#[tokio::test]
async fn browser_api_post_round_trip() {
    let base = spawn_test_server().await;
    let tool = make_browser();
    let out = tool
        .execute(json!({
            "op": "api",
            "method": "POST",
            "url": format!("{base}/api/echo"),
            "headers": {"content-type": "application/json"},
            "body": "{\"key\":\"value\"}"
        }))
        .await
        .expect("api should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], 200);
    let body: Value = serde_json::from_str(v["body"].as_str().unwrap()).unwrap();
    assert_eq!(body["received"]["key"], "value");
}

#[tokio::test]
async fn browser_click_type_screenshot_drive_local_page() {
    let base = spawn_test_server().await;
    let tool = make_browser();

    tool.execute(json!({"op": "navigate", "url": format!("{base}/page")}))
        .await
        .expect("navigate should establish a browser session");

    let clicked = tool
        .execute(json!({"op": "click", "selector": "#login"}))
        .await
        .expect("click should succeed against local page");
    let click_json: Value = serde_json::from_str(&clicked).unwrap();
    assert_eq!(click_json["op"], "click");
    assert_eq!(click_json["selector"], "#login");
    assert_eq!(click_json["text"], "Login");

    let typed = tool
        .execute(json!({"op": "type", "selector": "#username", "text": "alex"}))
        .await
        .expect("type should update local form state");
    let type_json: Value = serde_json::from_str(&typed).unwrap();
    assert_eq!(type_json["op"], "type");
    assert_eq!(type_json["selector"], "#username");
    assert_eq!(type_json["value"], "alex");

    let shot = tool
        .execute(json!({"op": "screenshot"}))
        .await
        .expect("screenshot should write a file");
    let shot_json: Value = serde_json::from_str(&shot).unwrap();
    let path = shot_json["path"].as_str().expect("path should be returned");
    assert!(
        std::path::Path::new(path).exists(),
        "screenshot path exists"
    );
    assert_eq!(shot_json["content_type"], "image/png");
}

#[tokio::test]
async fn browser_unknown_op_and_missing_url() {
    let tool = make_browser();
    let err = tool
        .execute(json!({"op": "fly"}))
        .await
        .expect_err("unknown op must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));

    let err2 = tool
        .execute(json!({"op": "navigate"}))
        .await
        .expect_err("missing url must fail");
    assert!(matches!(err2, ToolError::InvalidArguments { .. }));
}

#[tokio::test]
async fn browser_unreachable_host_errors_gracefully() {
    let tool = make_browser();
    // 127.0.0.1 on an unused port — connection refused, must be an
    // ExecutionFailed, not a panic.
    let err = tool
        .execute(json!({"op": "navigate", "url": "http://127.0.0.1:1/"}))
        .await
        .expect_err("unreachable host must fail cleanly");
    assert!(matches!(err, ToolError::ExecutionFailed { .. }));
}
