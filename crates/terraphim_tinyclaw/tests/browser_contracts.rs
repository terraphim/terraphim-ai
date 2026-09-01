//! Hermetic contract tests for `BrowserTool` (#3148).
//!
//! Uses a local axum server as the target so no external network is
//! needed in CI.

mod common;

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
    common::scrub_env();
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
async fn browser_click_type_screenshot_report_backend_unavailable() {
    let base = spawn_test_server().await;
    let tool = make_browser();

    tool.execute(json!({"op": "navigate", "url": format!("{base}/page")}))
        .await
        .expect("navigate should establish a browser session");

    let clicked = tool
        .execute(json!({"op": "click", "selector": "#login"}))
        .await
        .expect_err("click must not be faked by cached HTML");
    assert!(matches!(clicked, ToolError::BackendUnavailable { .. }));

    let typed = tool
        .execute(json!({"op": "type", "selector": "#username", "text": "alex"}))
        .await
        .expect_err("type must not be faked by cached HTML");
    assert!(matches!(typed, ToolError::BackendUnavailable { .. }));

    let shot = tool
        .execute(json!({"op": "screenshot"}))
        .await
        .expect_err("screenshot must not emit a placeholder artifact");
    assert!(matches!(shot, ToolError::BackendUnavailable { .. }));
}

#[cfg(unix)]
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

#[cfg(unix)]
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

// --- body-cap regressions (P2 from review #3221) --------------------------

fn make_browser_with_max_bytes(max_bytes: usize) -> BrowserTool {
    common::scrub_env();
    let cfg = terraphim_tinyclaw::config::BrowserConfig {
        enabled: true,
        timeout_secs: 10,
        max_bytes,
        proxy: None,
        agent_binary: None,
    };
    BrowserTool::from_config(&cfg).expect("browser tool builds")
}

use axum::http::StatusCode;
use std::convert::Infallible;

/// Build a server that emits a chunked body (no Content-Length) larger
/// than `body_len` bytes. Returns the base URL.
async fn spawn_chunked_large_body(body_len: usize) -> String {
    use axum::body::Body;
    use futures_util::stream;

    let chunk = vec![b'a'; 16 * 1024];
    let chunks_needed = body_len.div_ceil(chunk.len()).max(1);
    let app = Router::new().route(
        "/big",
        get(move || async move {
            let stream =
                stream::iter(std::iter::repeat_n(chunk, chunks_needed).map(Ok::<_, Infallible>));
            // Intentionally omit Content-Length so the receiver cannot
            // trust headers and must stream-cap.
            let mut resp = axum::response::Response::new(Body::from_stream(stream));
            *resp.status_mut() = StatusCode::OK;
            resp
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Server that advertises a small Content-Length but streams a larger
/// body. The receiver must stream-cap the body instead of trusting the
/// header.
async fn spawn_lying_content_length(body_len: usize, advertised_len: usize) -> String {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::io::BufReader;
    // TcpStream is implicit through the listener.accept() socket.

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(p) => p,
                Err(_) => return,
            };
            tokio::spawn(async move {
                let mut buf_reader = BufReader::new(&mut socket);
                // Drain request headers.
                let mut header = Vec::new();
                loop {
                    let mut byte = [0u8; 1];
                    if buf_reader.read_exact(&mut byte).await.is_err() {
                        return;
                    }
                    header.push(byte[0]);
                    if header.len() >= 4 && &header[header.len() - 4..] == b"\r\n\r\n" {
                        break;
                    }
                }
                let response_header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {advertised_len}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                );
                if socket.write_all(response_header.as_bytes()).await.is_err() {
                    return;
                }
                let chunk = vec![b'b'; 16 * 1024];
                let chunks_needed = body_len.div_ceil(chunk.len()).max(1);
                for _ in 0..chunks_needed {
                    let header = format!("{:x}\r\n", chunk.len());
                    if socket.write_all(header.as_bytes()).await.is_err()
                        || socket.write_all(&chunk).await.is_err()
                        || socket.write_all(b"\r\n").await.is_err()
                    {
                        return;
                    }
                }
                let _ = socket.write_all(b"0\r\n\r\n").await;
                let _ = socket.shutdown().await;
            });
        }
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn browser_navigate_rejects_unbounded_chunked_body() {
    // max_bytes = 4 KiB, but server emits 2 MiB of chunked body with
    // NO Content-Length. The browser tool must surface a "response too
    // large" error instead of buffering the entire body.
    let tool = make_browser_with_max_bytes(4 * 1024);
    let base = spawn_chunked_large_body(2 * 1024 * 1024).await;

    let err = tool
        .execute(json!({"op": "navigate", "url": format!("{base}/big")}))
        .await
        .expect_err("oversize chunked body must be rejected");

    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(
                message.contains("response too large"),
                "message should report too-large body, got: {message}"
            );
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn browser_api_rejects_lying_content_length() {
    // max_bytes = 4 KiB. Server advertises Content-Length: 1024 (well
    // under the cap) but streams ~2 MiB. The browser tool must NOT
    // trust the header and must cap by reading.
    let tool = make_browser_with_max_bytes(4 * 1024);
    let base = spawn_lying_content_length(2 * 1024 * 1024, 1024).await;

    let out = tool
        .execute(json!({"op": "api", "method": "GET", "url": format!("{base}/lie")}))
        .await
        .expect("api should return an error response, not panic");

    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "api");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("response too large"),
        "api must surface too-large error when body exceeds max_bytes, got: {err}"
    );
}

#[tokio::test]
async fn browser_navigate_accepts_body_under_max_bytes() {
    // Sanity check: a small chunked body still works after the
    // streaming refactor. This is the GREEN half of the regression.
    let tool = make_browser_with_max_bytes(64 * 1024);
    let base = spawn_chunked_large_body(1024).await;

    let out = tool
        .execute(json!({"op": "navigate", "url": format!("{base}/big")}))
        .await
        .expect("under-cap chunked body should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "navigate");
    assert_eq!(v["status"], 200);
}

#[tokio::test]
async fn browser_navigate_accepts_body_exactly_at_max_bytes() {
    let tool = make_browser_with_max_bytes(16 * 1024);
    let base = spawn_chunked_large_body(16 * 1024).await;

    let out = tool
        .execute(json!({"op": "navigate", "url": format!("{base}/big")}))
        .await
        .expect("body exactly at max_bytes must succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], 200);
    assert_eq!(v["bytes"], 16 * 1024);
}

#[tokio::test]
async fn browser_navigate_rejects_body_one_byte_over_max_bytes() {
    let tool = make_browser_with_max_bytes(16 * 1024);
    let base = spawn_chunked_large_body((16 * 1024) + 1).await;

    let err = tool
        .execute(json!({"op": "navigate", "url": format!("{base}/big")}))
        .await
        .expect_err("body one byte over max_bytes must fail");

    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(
                message.contains("response too large"),
                "message should report too-large body, got: {message}"
            );
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
}
