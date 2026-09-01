//! Hermetic integration tests for `SandboxTool` (#3146).
//!
//! These tests use the real TinyClaw `SandboxTool::from_config` path and
//! the production `terraphim_rlm` local backend. They do not replace RLM
//! with a test executor.

mod common;

use serde_json::{Value, json};
use terraphim_tinyclaw::config::SandboxConfig;
use terraphim_tinyclaw::tools::sandbox::SandboxTool;
use terraphim_tinyclaw::tools::{Tool, ToolError};

async fn make_tool(timeout_secs: u64, max_output_bytes: usize) -> SandboxTool {
    common::scrub_env();
    let cfg = SandboxConfig {
        enabled: true,
        backend: "local".to_string(),
        timeout_secs,
        max_output_bytes,
    };
    SandboxTool::from_config(&cfg)
        .await
        .expect("real local RLM sandbox builds")
}

#[tokio::test]
async fn sandbox_execute_code_returns_result_from_real_local_backend() {
    common::scrub_env();
    let tool = make_tool(5, 1024).await;
    let out = tool
        .execute(json!({"op": "execute_code", "code": "print('hi-from-real-python')"}))
        .await
        .expect("execute_code should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "execute_code");
    assert_eq!(v["success"], true);
    assert!(
        v["stdout"]
            .as_str()
            .unwrap()
            .contains("hi-from-real-python")
    );
    assert!(v["session_id"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn sandbox_requested_docker_backend_reports_honored_or_local_fallback() {
    common::scrub_env();
    let cfg = SandboxConfig {
        enabled: true,
        backend: "docker".to_string(),
        timeout_secs: 5,
        max_output_bytes: 1024,
    };
    let tool = SandboxTool::from_config(&cfg)
        .await
        .expect("docker preference should initialize or fall back");
    let created = tool
        .execute(json!({"op": "session_create"}))
        .await
        .expect("session_create");
    let sid = serde_json::from_str::<Value>(&created).unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let status = tool
        .execute(json!({"op": "session_status", "session_id": sid}))
        .await
        .expect("session_status");
    let status: Value = serde_json::from_str(&status).unwrap();
    let backend = status["backend"].as_str().unwrap();
    assert!(
        backend == "docker" || backend == "local",
        "backend must honor docker or explicitly fall back to local, got {backend}"
    );
}

#[tokio::test]
async fn sandbox_timeout_is_reported_by_real_backend() {
    common::scrub_env();
    let tool = make_tool(1, 1024).await;
    let out = tool
        .execute(json!({"op": "execute_bash", "command": "sleep 2; echo late"}))
        .await
        .expect("timeouts are execution results");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], false);
    assert!(
        v["stderr"].as_str().unwrap().contains("timed out"),
        "expected timeout stderr, got {v}"
    );
}

#[tokio::test]
async fn sandbox_output_is_bounded_on_char_boundary() {
    common::scrub_env();
    let tool = make_tool(5, 16).await;
    let out = tool
        .execute(json!({"op": "execute_bash", "command": "printf 'abcdefghijklmnopqrstuvwxyz'"}))
        .await
        .expect("execute_bash should succeed");
    let v: Value = serde_json::from_str(&out).unwrap();
    let stdout = v["stdout"].as_str().unwrap();
    assert!(
        stdout.contains("truncated"),
        "stdout was not bounded: {stdout}"
    );
    assert!(stdout.len() < 80, "bounded stdout stayed small: {stdout}");
}

#[tokio::test]
async fn sandbox_sessions_are_isolated() {
    common::scrub_env();
    let tool = make_tool(5, 2048).await;
    let a = tool.execute(json!({"op": "session_create"})).await.unwrap();
    let b = tool.execute(json!({"op": "session_create"})).await.unwrap();
    let sid_a = serde_json::from_str::<Value>(&a).unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let sid_b = serde_json::from_str::<Value>(&b).unwrap()["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(sid_a, sid_b);

    let out_a = tool
        .execute(json!({"op": "execute_bash", "session_id": sid_a, "command": "echo session-a"}))
        .await
        .unwrap();
    let out_b = tool
        .execute(json!({"op": "execute_bash", "session_id": sid_b, "command": "echo session-b"}))
        .await
        .unwrap();
    assert!(out_a.contains("session-a"));
    assert!(!out_a.contains("session-b"));
    assert!(out_b.contains("session-b"));
    assert!(!out_b.contains("session-a"));
}

#[tokio::test]
async fn sandbox_recursive_query_uses_real_rlm_and_fails_fast_without_llm_bridge() {
    common::scrub_env();
    let tool = make_tool(5, 1024).await;
    let out = tool
        .execute(json!({"op": "recursive_query", "prompt": "answer FINAL('ok')"}))
        .await
        .expect("recursive_query should return a bounded fail-fast JSON result");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "recursive_query");
    assert_eq!(v["success"], false);
    assert_eq!(v["response"], "");
    assert!(v["iterations"].as_u64().unwrap() <= 1);
}

#[tokio::test]
async fn sandbox_unknown_op_and_missing_args() {
    common::scrub_env();
    let tool = make_tool(5, 1024).await;
    let err = tool
        .execute(json!({"op": "nope"}))
        .await
        .expect_err("unknown op must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));

    let err2 = tool
        .execute(json!({"op": "execute_code"}))
        .await
        .expect_err("missing code must fail");
    assert!(matches!(err2, ToolError::InvalidArguments { .. }));
}

#[tokio::test]
async fn sandbox_invalid_session_id_rejected() {
    common::scrub_env();
    let tool = make_tool(5, 1024).await;
    let err = tool
        .execute(json!({"op": "session_status", "session_id": "not-a-ulid"}))
        .await
        .expect_err("bad session id must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));
}
