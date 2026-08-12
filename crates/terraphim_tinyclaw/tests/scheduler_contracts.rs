//! Contract tests for the Hermes-parity schedule surface (#3147).
//!
//! All tests use memory-only `DeviceStorage` so they are hermetic and
//! need no filesystem or running services.

mod common;

use serde_json::json;
use std::sync::Arc;
use terraphim_persistence::DeviceStorage;
use terraphim_tinyclaw::cron::CronStore;
use terraphim_tinyclaw::tools::scheduler::{OrchestratorScheduleStore, ScheduleTool};
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

#[tokio::test]
async fn schedule_persists_as_orchestrator_agent_across_restart() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let base_config = temp.path().join("orchestrator.toml");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &base_config,
        format!(
            r#"
working_dir = "{}"
include = ["tinyclaw-schedules.toml"]

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "{}"
create_prs = false
"#,
            temp.path().display(),
            temp.path().display()
        ),
    )
    .expect("write base config");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(fragment.clone()));
    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "run daily report",
            "schedule": "0 9 * * *",
            "skills": ["daily-report"],
            "model": "sonnet",
        }))
        .await
        .expect("create should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output");
    let id = v["id"].as_str().expect("id present").to_string();

    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads generated schedule fragment");
    let agent = config
        .agents
        .iter()
        .find(|agent| agent.name == format!("tinyclaw-{id}"))
        .expect("scheduled agent in orchestrator config");
    assert_eq!(agent.schedule.as_deref(), Some("0 9 * * *"));
    assert_eq!(agent.task, "run daily report");
    assert_eq!(agent.skill_chain, vec!["daily-report".to_string()]);

    // Simulated process restart: new store instance reads the durable file.
    let restarted = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(fragment));
    let listed = restarted
        .execute(json!({"op": "list"}))
        .await
        .expect("list after restart");
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("json output");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["jobs"][0]["id"], id);

    restarted
        .execute(json!({"op": "delete", "id": id}))
        .await
        .expect("delete");
    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads after delete");
    assert!(
        config.agents.is_empty(),
        "delete removes orchestrator agent"
    );
}

#[tokio::test]
async fn orchestrator_schedule_persists_project_and_validates_in_multi_project_config() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let base_config = temp.path().join("orchestrator.toml");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &base_config,
        format!(
            r#"
working_dir = "{}"
include = ["tinyclaw-schedules.toml"]

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "{}"
create_prs = false

[[projects]]
id = "tinyclaw"
working_dir = "{}"
"#,
            temp.path().display(),
            temp.path().display(),
            temp.path().display()
        ),
    )
    .expect("write base config");

    let store = OrchestratorScheduleStore::with_project(fragment.clone(), "tinyclaw");
    let tool = ScheduleTool::new_orchestrator(store);
    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "multi-project report",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect("create should succeed");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output");
    let id = v["id"].as_str().expect("id present").to_string();

    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads generated schedule fragment");
    let agent = config
        .agents
        .iter()
        .find(|agent| agent.name == format!("tinyclaw-{id}"))
        .expect("scheduled agent in orchestrator config");
    assert_eq!(agent.project.as_deref(), Some("tinyclaw"));
    config
        .validate()
        .expect("merged multi-project config remains valid");
}

#[tokio::test]
async fn orchestrator_schedule_unknown_project_fails_validation_clearly() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let base_config = temp.path().join("orchestrator.toml");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &base_config,
        format!(
            r#"
working_dir = "{}"
include = ["tinyclaw-schedules.toml"]

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "{}"
create_prs = false

[[projects]]
id = "known"
working_dir = "{}"
"#,
            temp.path().display(),
            temp.path().display(),
            temp.path().display()
        ),
    )
    .expect("write base config");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_project(
        fragment, "missing",
    ));
    tool.execute(json!({
        "op": "create",
        "prompt": "bad project report",
        "schedule": "0 9 * * *",
    }))
    .await
    .expect("create writes fragment; orchestrator validates project refs");

    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads generated schedule fragment");
    let err = config.validate().expect_err("unknown project must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("tinyclaw-"),
        "agent name should be clear: {msg}"
    );
    assert!(
        msg.contains("missing"),
        "unknown project should be clear: {msg}"
    );
}

#[tokio::test]
async fn orchestrator_schedule_missing_project_fails_multi_project_validation_clearly() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let base_config = temp.path().join("orchestrator.toml");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &base_config,
        format!(
            r#"
working_dir = "{}"
include = ["tinyclaw-schedules.toml"]

[nightwatch]
eval_interval_secs = 300
minor_threshold = 0.10
moderate_threshold = 0.20
severe_threshold = 0.40
critical_threshold = 0.70

[compound_review]
schedule = "0 2 * * *"
max_duration_secs = 1800
repo_path = "{}"
create_prs = false

[[projects]]
id = "known"
working_dir = "{}"
"#,
            temp.path().display(),
            temp.path().display(),
            temp.path().display()
        ),
    )
    .expect("write base config");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(fragment));
    tool.execute(json!({
        "op": "create",
        "prompt": "missing project report",
        "schedule": "0 9 * * *",
    }))
    .await
    .expect("legacy no-project write still succeeds");

    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads generated schedule fragment");
    let err = config.validate().expect_err("missing project must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("tinyclaw-"),
        "agent name should be clear: {msg}"
    );
    assert!(
        msg.contains("project"),
        "project mode should be clear: {msg}"
    );
}

#[tokio::test]
async fn orchestrator_schedule_rejects_deliver_before_persistence() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(fragment.clone()));

    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "deliver somewhere",
            "schedule": "0 9 * * *",
            "deliver": "telegram:123",
        }))
        .await
        .expect_err("orchestrator backend must not silently drop deliver");

    match err {
        ToolError::InvalidArguments { message, .. } => {
            assert!(message.contains("deliver"), "got: {message}");
            assert!(message.contains("not supported"), "got: {message}");
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
    assert!(
        !fragment.exists(),
        "deliver rejection must happen before persistence"
    );
}

#[tokio::test]
async fn orchestrator_schedule_rejects_non_cron_expression() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(
        temp.path().join("tinyclaw-schedules.toml"),
    ));

    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "not orchestrator cron",
            "schedule": "every 2h",
        }))
        .await
        .expect_err("orchestrator backend accepts only cron");
    match err {
        ToolError::InvalidArguments { message, .. } => {
            assert!(
                message.contains("invalid orchestrator cron schedule"),
                "got: {message}"
            );
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}
