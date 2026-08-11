//! Hermetic contract tests for `SubagentTool` (#3145).
//!
//! Spawns real local processes via a trivial Agent-type Provider
//! (`sh -c '...'`), so no external LLM provider is needed. The spawner
//! runs on a dedicated current-thread runtime (its spawn future is !Send
//! due to a tracing span), which the tool handles internally.

use serde_json::json;
use std::path::PathBuf;
use terraphim_persistence::DeviceStorage;
use terraphim_spawner::AgentSpawner;
use terraphim_tinyclaw::tools::subagent::SubagentTool;
use terraphim_tinyclaw::tools::{Tool, ToolError};
use terraphim_types::capability::{Capability, Provider, ProviderType};

fn make_tool() -> SubagentTool {
    // `sh` provider: the spawner appends the task as the `-c` script body.
    let provider = Provider::new(
        "@test-agent",
        "Test Agent",
        ProviderType::Agent {
            agent_id: "@test".to_string(),
            cli_command: "sh".to_string(),
            working_dir: PathBuf::from("/tmp"),
        },
        vec![Capability::CodeGeneration],
    );
    SubagentTool::with_spawner(AgentSpawner::new(), provider, None, 15)
}

#[tokio::test]
async fn subagent_spawn_returns_id() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "spawn", "task": "echo subagent-done"}))
        .await
        .expect("spawn should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["op"], "spawn");
    let id = v["id"].as_str().unwrap();
    assert_eq!(id.len(), 16, "handle id is 16 hex chars");
}

#[tokio::test]
async fn subagent_status_and_list() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "spawn", "task": "sleep 1; echo ok"}))
        .await
        .unwrap();
    let id = out.parse::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let status = tool
        .execute(json!({"op": "status", "id": id}))
        .await
        .expect("status should succeed");
    let vs: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(vs["id"], id);
    assert!(vs["pid"].as_u64().is_some());

    let list = tool
        .execute(json!({"op": "list"}))
        .await
        .expect("list should succeed");
    let vl: serde_json::Value = serde_json::from_str(&list).unwrap();
    assert_eq!(vl["count"], 1);
    assert_eq!(vl["agents"][0]["id"], id);
}

#[tokio::test]
async fn subagent_collect_output() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "spawn", "task": "sleep 0.3; echo hello-from-subagent"}))
        .await
        .unwrap();
    let id = out.parse::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for the process to finish so output is captured.
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    let collected = tool
        .execute(json!({"op": "collect", "id": id}))
        .await
        .expect("collect should succeed");
    let vc: serde_json::Value = serde_json::from_str(&collected).unwrap();
    assert_eq!(vc["op"], "collect");
    let lines = vc["lines"].as_array().unwrap();
    assert!(
        lines
            .iter()
            .any(|l| l.as_str().unwrap().contains("hello-from-subagent")),
        "expected subagent output in collected lines, got: {lines:?}"
    );
}

#[tokio::test]
async fn subagent_terminate_removes_handle() {
    let tool = make_tool();
    let out = tool
        .execute(json!({"op": "spawn", "task": "sleep 30"}))
        .await
        .unwrap();
    let id = out.parse::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let term = tool
        .execute(json!({"op": "terminate", "id": id}))
        .await
        .expect("terminate should succeed");
    let vt: serde_json::Value = serde_json::from_str(&term).unwrap();
    assert_eq!(vt["op"], "terminate");

    // Handle removed: status now fails with unknown id.
    let err = tool
        .execute(json!({"op": "status", "id": id}))
        .await
        .expect_err("status of terminated agent must fail");
    assert!(matches!(err, ToolError::ExecutionFailed { .. }));
}

#[tokio::test]
async fn subagent_unknown_id_rejected() {
    let tool = make_tool();
    let err = tool
        .execute(json!({"op": "status", "id": "0000000000000000"}))
        .await
        .expect_err("unknown id must fail");
    assert!(matches!(err, ToolError::ExecutionFailed { .. }));
}

#[tokio::test]
async fn subagent_missing_args_and_unknown_op() {
    let tool = make_tool();
    let err = tool
        .execute(json!({"op": "spawn"}))
        .await
        .expect_err("spawn without task must fail");
    assert!(matches!(err, ToolError::InvalidArguments { .. }));

    let err2 = tool
        .execute(json!({"op": "explode"}))
        .await
        .expect_err("unknown op must fail");
    assert!(matches!(err2, ToolError::InvalidArguments { .. }));
}

#[tokio::test]
async fn subagent_registry_persists_across_recreates() {
    // Durable registry via terraphim_persistence (memory-only storage): a
    // spawned agent's metadata survives a tool re-create (restart proxy).
    let storage_ref = terraphim_persistence::DeviceStorage::init_memory_only()
        .await
        .expect("memory-only storage");
    let storage = std::sync::Arc::new(DeviceStorage {
        ops: storage_ref.ops.clone(),
        fastest_op: storage_ref.fastest_op.clone(),
    });

    let provider = Provider::new(
        "@test-agent",
        "Test Agent",
        ProviderType::Agent {
            agent_id: "@test".to_string(),
            cli_command: "sh".to_string(),
            working_dir: PathBuf::from("/tmp"),
        },
        vec![Capability::CodeGeneration],
    );
    let tool = SubagentTool::with_spawner(AgentSpawner::new(), provider.clone(), None, 15)
        .with_persistence(storage.clone(), "test-subagent-registry");

    let out = tool
        .execute(json!({"op": "spawn", "task": "echo persisted"}))
        .await
        .expect("spawn should succeed");
    let id = out.parse::<serde_json::Value>().unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Simulate a restart: new tool instance, same storage key, no live handles.
    let fresh = SubagentTool::with_spawner(AgentSpawner::new(), provider, None, 15)
        .with_persistence(storage, "test-subagent-registry");
    let list = fresh
        .execute(json!({"op": "list"}))
        .await
        .expect("list should succeed");
    let vl: serde_json::Value = serde_json::from_str(&list).unwrap();

    let persisted = vl["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .expect("persisted record present after restart");
    assert_eq!(persisted["live"], false);
    assert_eq!(persisted["task"], "echo persisted");
    assert!(persisted["pid"].as_u64().unwrap() > 0);
}
