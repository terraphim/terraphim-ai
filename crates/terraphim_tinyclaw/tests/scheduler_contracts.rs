//! Contract tests for the Hermes-parity schedule surface (#3147).
//!
//! All tests use memory-only `DeviceStorage` so they are hermetic and
//! need no filesystem or running services.

mod common;

use serde_json::json;
use std::sync::Arc;
use terraphim_persistence::DeviceStorage;
use terraphim_tinyclaw::cron::CronStore;
use terraphim_tinyclaw::tools::scheduler::ScheduleTool;
use terraphim_tinyclaw::tools::{Tool, ToolError};

/// Build a schedule tool over a fresh memory-only store. Each caller
/// passes a unique key: `init_memory_only` returns a process-wide static
/// storage, so parallel tests must not share a store key.
async fn make_tool(key: &str) -> ScheduleTool {
    let storage = memory_storage().await;
    let store = CronStore::new(storage, key);
    ScheduleTool::new(store)
}

/// Arc-wrapped memory-only storage (init_memory_only returns a static ref).
async fn memory_storage() -> Arc<DeviceStorage> {
    let storage_ref = DeviceStorage::init_memory_only()
        .await
        .expect("memory-only storage");
    Arc::new(DeviceStorage {
        ops: storage_ref.ops.clone(),
        fastest_op: storage_ref.fastest_op.clone(),
    })
}

#[tokio::test]
async fn schedule_create_returns_id_and_persists() {
    common::scrub_env();
    let tool = make_tool("test_schedules_a").await;

    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "run daily report",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect("create should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output");
    assert_eq!(v["op"], "create");
    assert_eq!(v["status"], "created");
    let id = v["id"].as_str().expect("id present");

    // Round-trip through list.
    let out = tool
        .execute(json!({"op": "list"}))
        .await
        .expect("list should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output");
    assert_eq!(v["count"], 1, "one job listed");
    assert!(v["jobs"][0]["id"] == json!(id));
    assert_eq!(v["jobs"][0]["prompt"], "run daily report");
}

#[tokio::test]
async fn schedule_rejects_invalid_cron() {
    common::scrub_env();
    let tool = make_tool("test_schedules_b").await;

    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "broken",
            "schedule": "not a cron",
        }))
        .await
        .expect_err("invalid cron must be rejected");
    match err {
        ToolError::InvalidArguments { message, .. } => {
            assert!(message.contains("invalid schedule"), "got: {message}");
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[tokio::test]
async fn schedule_delete_removes_job() {
    common::scrub_env();
    let tool = make_tool("test_schedules_c").await;

    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "cleanup",
            "schedule": "every 1h",
        }))
        .await
        .expect("create");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let id = v["id"].as_str().unwrap().to_string();

    let out = tool
        .execute(json!({"op": "delete", "id": id}))
        .await
        .expect("delete");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], "deleted");

    let out = tool.execute(json!({"op": "list"})).await.expect("list");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 0);
}

#[tokio::test]
async fn schedule_delete_unknown_id_fails() {
    common::scrub_env();
    let tool = make_tool("test_schedules_d").await;

    let err = tool
        .execute(json!({"op": "delete", "id": "nope"}))
        .await
        .expect_err("unknown id must fail");
    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(message.contains("unknown job id"), "got: {message}");
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn schedule_persists_across_store_recreation() {
    common::scrub_env();
    let storage = memory_storage().await;

    // First tool instance creates a job.
    let tool = ScheduleTool::new(CronStore::new(storage.clone(), "test_schedules_persist"));
    tool.execute(json!({
        "op": "create",
        "prompt": "survives restart",
        "schedule": "0 6 * * 1",
    }))
    .await
    .expect("create");

    // Simulated restart: fresh tool on the same storage key.
    let tool2 = ScheduleTool::new(CronStore::new(storage.clone(), "test_schedules_persist"));
    let out = tool2
        .execute(json!({"op": "list"}))
        .await
        .expect("list after restart");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["count"], 1, "job survives store recreation");
    assert_eq!(v["jobs"][0]["prompt"], "survives restart");
}

#[tokio::test]
async fn schedule_create_with_skills_and_deliver() {
    common::scrub_env();
    let tool = make_tool("test_schedules_e").await;

    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "briefing",
            "schedule": "every 2h",
            "skills": ["daily-report"],
            "deliver": "telegram:123",
        }))
        .await
        .expect("create with extras");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["status"], "created");
    assert!(v["id"].as_str().unwrap().len() > 8);
}
