//! Hermetic contract tests for `BrowserTool` (#3148).
//!
//! Uses a local axum server as the target so no external network is
//! needed in CI.

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use std::fs;
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use terraphim_tinyclaw::tools::browser::BrowserTool;
use terraphim_tinyclaw::tools::{Tool, ToolError};

fn make_browser() -> BrowserTool {
    make_browser_with_agent_binary(None)
}

fn make_browser_with_agent_binary(agent_binary: Option<String>) -> BrowserTool {
    let cfg = terraphim_tinyclaw::config::BrowserConfig {
        enabled: true,
        timeout_secs: 10,
        max_bytes: 512 * 1024,
        proxy: None,
        agent_binary,
    };
    BrowserTool::from_config(&cfg).expect("browser tool builds")
}

fn write_executable(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}

/// Spin up a local HTTP server; returns its base URL.
async fn spawn_test_server() -> String {
    let app = Router::new()
        .route(
            "/page",
            get(|| async {
                (
                    [("content-type", "text/html")],
                    "<html><head><title>Test Page</title></head><body><h1>Hello World</h1><p>some body text</p></body></html>",
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
async fn browser_click_type_screenshot_unavailable() {
    let tool = make_browser();
    for op in ["click", "type", "screenshot"] {
        let err = tool
            .execute(json!({"op": op, "url": "http://example.com"}))
            .await
            .expect_err(&format!("{op} must be unavailable"));
        assert!(
            matches!(err, ToolError::BackendUnavailable { .. }),
            "{op} should be BackendUnavailable, got: {err:?}"
        );
    }
}

#[tokio::test]
async fn browser_native_ops_probe_agent_capability_and_fail_closed_when_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let shim = temp.path().join("terraphim-agent");
    write_executable(
        &shim,
        r#"#!/bin/sh
if [ "$*" = "--robot --format json robot capabilities" ]; then
  printf '%s\n' '{"features":{"web_operations":false},"commands":["robot"]}'
  exit 0
fi
echo "unexpected args: $*" >&2
exit 2
"#,
    );

    let tool = make_browser_with_agent_binary(Some(shim.display().to_string()));
    let err = tool
        .execute(json!({"op": "screenshot", "url": "http://example.test"}))
        .await
        .expect_err("disabled web_operations must fail closed");

    match err {
        ToolError::BackendUnavailable { message, .. } => {
            assert!(
                message.contains("web_operations=false"),
                "message should contain capability evidence, got: {message}"
            );
        }
        other => panic!("expected BackendUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn browser_native_ops_reject_placeholder_agent_web_protocol() {
    let temp = tempfile::tempdir().unwrap();
    let shim = temp.path().join("terraphim-agent");
    write_executable(
        &shim,
        r#"#!/bin/sh
case "$*" in
  "--robot --format json robot capabilities")
    printf '%s\n' '{"features":{"web_operations":true},"commands":["robot","web"]}'
    exit 0
    ;;
  "--help")
    printf '%s\n' 'Commands:'
    printf '%s\n' '  web  Web operations'
    exit 0
    ;;
  "web screenshot http://example.test")
    printf '%s\n' 'Web screenshot functionality is not yet implemented.'
    exit 0
    ;;
esac
echo "unexpected args: $*" >&2
exit 2
"#,
    );

    let tool = make_browser_with_agent_binary(Some(shim.display().to_string()));
    let err = tool
        .execute(json!({"op": "screenshot", "url": "http://example.test"}))
        .await
        .expect_err("placeholder output must not be accepted as screenshot success");

    match err {
        ToolError::BackendUnavailable { message, .. } => {
            assert!(
                message.contains("placeholder-only") || message.contains("no verified"),
                "message should contain protocol evidence, got: {message}"
            );
        }
        other => panic!("expected BackendUnavailable, got {other:?}"),
    }
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
