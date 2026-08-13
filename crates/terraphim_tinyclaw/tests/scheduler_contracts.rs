//! Contract tests for the Hermes-parity schedule surface (#3147).
//!
//! All tests use memory-only `DeviceStorage` so they are hermetic and
//! need no filesystem or running services.

mod common;

use serde_json::json;
use std::sync::{Arc, Barrier};
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
async fn orchestrator_schedule_rejects_omitted_cli_tool_before_persistence() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::new(fragment.clone()));

    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "run daily report",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect_err("orchestrator backend must require explicit scheduler.cli_tool");

    match err {
        ToolError::InvalidArguments { message, .. } => {
            assert!(message.contains("scheduler.cli_tool"), "got: {message}");
            assert!(message.contains("required"), "got: {message}");
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
    assert!(
        !fragment.exists(),
        "cli_tool rejection must happen before persistence"
    );
}

#[tokio::test]
async fn orchestrator_schedule_rejects_unowned_fragment_without_rewriting_bytes() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("operator-owned.toml");
    let original = br#"# operator-managed include
[[agents]]
name = "nightly-maintenance"
layer = "Core"
cli_tool = "echo"
task = "do not touch"
schedule = "0 1 * * *"
"#;
    std::fs::write(&fragment, original).expect("write unowned fragment");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "run daily report",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect_err("unowned fragment must be rejected");
    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(
                message.contains("refusing") || message.contains("unknown or unowned"),
                "got: {message}"
            );
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&fragment).expect("read fragment"),
        original,
        "rejected unowned TOML must remain byte-for-byte unchanged"
    );
}

#[tokio::test]
async fn orchestrator_schedule_rejects_mixed_owned_fragment_without_rewriting_bytes() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("mixed.toml");
    let original = br#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false

[[agents]]
name = "operator-agent"
layer = "Core"
cli_tool = "echo"
task = "operator-owned task"
schedule = "0 1 * * *"
capabilities = ["not-tinyclaw"]
"#;
    std::fs::write(&fragment, original).expect("write mixed fragment");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
    let err = tool
        .execute(json!({
            "op": "delete",
            "id": "unknown",
        }))
        .await
        .expect_err("mixed fragment must be rejected before mutation");
    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(message.contains("not a TinyClaw-owned"), "got: {message}");
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&fragment).expect("read fragment"),
        original,
        "rejected mixed TOML must remain byte-for-byte unchanged"
    );
}

#[tokio::test]
async fn orchestrator_schedule_rejects_unknown_future_fields_without_rewriting_bytes() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("future-owned.toml");
    let original = br#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false

[[agents]]
name = "tinyclaw-existing"
layer = "Core"
cli_tool = "echo"
task = "future compatible task"
schedule = "0 1 * * *"
capabilities = ["tinyclaw-schedule", "tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
future_field = "must not be discarded"
"#;
    std::fs::write(&fragment, original).expect("write future fragment");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "run daily report",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect_err("unknown fields must not be silently discarded");
    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(
                message.contains("unknown") || message.contains("refusing to rewrite"),
                "got: {message}"
            );
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&fragment).expect("read fragment"),
        original,
        "rejected future TOML must remain byte-for-byte unchanged"
    );
}

#[tokio::test]
async fn orchestrator_schedule_owned_fragment_create_list_delete_round_trips() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &fragment,
        r#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false
"#,
    )
    .expect("write owned fragment");

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
    let out = tool
        .execute(json!({
            "op": "create",
            "prompt": "owned round trip",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect("create in owned fragment");
    let v: serde_json::Value = serde_json::from_str(&out).expect("json output");
    let id = v["id"].as_str().expect("id present").to_string();

    let listed = tool
        .execute(json!({"op": "list"}))
        .await
        .expect("list owned fragment");
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("json output");
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["jobs"][0]["id"], id);

    tool.execute(json!({"op": "delete", "id": id}))
        .await
        .expect("delete owned schedule");
    let listed = tool
        .execute(json!({"op": "list"}))
        .await
        .expect("list after delete");
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("json output");
    assert_eq!(listed["count"], 0);

    let content = std::fs::read_to_string(&fragment).expect("read fragment");
    assert!(content.contains("name = \"tinyclaw-schedule-fragment-marker\""));
    assert!(content.contains("tinyclaw-schedule-fragment-schema:1"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn orchestrator_schedule_concurrent_independent_creates_both_survive() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    std::fs::write(
        &fragment,
        r#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false
"#,
    )
    .expect("write owned fragment");

    let barrier = Arc::new(Barrier::new(2));
    let create_a = {
        let fragment = fragment.clone();
        let barrier = Arc::clone(&barrier);
        tokio::task::spawn_blocking(move || {
            let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
                fragment, "echo",
            ));
            barrier.wait();
            tokio_test::block_on(tool.execute(json!({
                "op": "create",
                "prompt": "concurrent report a",
                "schedule": "0 9 * * *",
            })))
        })
    };
    let create_b = {
        let fragment = fragment.clone();
        let barrier = Arc::clone(&barrier);
        tokio::task::spawn_blocking(move || {
            let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
                fragment, "echo",
            ));
            barrier.wait();
            tokio_test::block_on(tool.execute(json!({
                "op": "create",
                "prompt": "concurrent report b",
                "schedule": "30 9 * * *",
            })))
        })
    };

    create_a
        .await
        .expect("create a thread joins")
        .expect("create a succeeds");
    create_b
        .await
        .expect("create b thread joins")
        .expect("create b succeeds");

    let reader =
        ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(fragment, "echo"));
    let listed = reader
        .execute(json!({"op": "list"}))
        .await
        .expect("list after concurrent creates");
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("json output");
    let prompts = listed["jobs"]
        .as_array()
        .expect("jobs array")
        .iter()
        .map(|job| job["prompt"].as_str().expect("prompt").to_string())
        .collect::<Vec<_>>();

    assert_eq!(listed["count"], 2, "jobs listed: {listed}");
    assert!(
        prompts.contains(&"concurrent report a".to_string()),
        "jobs listed: {listed}"
    );
    assert!(
        prompts.contains(&"concurrent report b".to_string()),
        "jobs listed: {listed}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn orchestrator_schedule_concurrent_create_delete_preserves_both_mutations() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("tinyclaw-schedules.toml");
    let seed_tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
    let seeded = seed_tool
        .execute(json!({
            "op": "create",
            "prompt": "seed report",
            "schedule": "0 8 * * *",
        }))
        .await
        .expect("seed create succeeds");
    let seeded: serde_json::Value = serde_json::from_str(&seeded).expect("json output");
    let seeded_id = seeded["id"].as_str().expect("seed id").to_string();

    let barrier = Arc::new(Barrier::new(2));
    let delete_seed = {
        let fragment = fragment.clone();
        let barrier = Arc::clone(&barrier);
        let seeded_id = seeded_id.clone();
        tokio::task::spawn_blocking(move || {
            let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
                fragment, "echo",
            ));
            barrier.wait();
            tokio_test::block_on(tool.execute(json!({
                "op": "delete",
                "id": seeded_id,
            })))
        })
    };
    let create_new = {
        let fragment = fragment.clone();
        let barrier = Arc::clone(&barrier);
        tokio::task::spawn_blocking(move || {
            let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
                fragment, "echo",
            ));
            barrier.wait();
            tokio_test::block_on(tool.execute(json!({
                "op": "create",
                "prompt": "replacement report",
                "schedule": "30 8 * * *",
            })))
        })
    };

    delete_seed
        .await
        .expect("delete thread joins")
        .expect("delete succeeds");
    create_new
        .await
        .expect("create thread joins")
        .expect("create succeeds");

    let reader =
        ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(fragment, "echo"));
    let listed = reader
        .execute(json!({"op": "list"}))
        .await
        .expect("list after concurrent create/delete");
    let listed: serde_json::Value = serde_json::from_str(&listed).expect("json output");
    let prompts = listed["jobs"]
        .as_array()
        .expect("jobs array")
        .iter()
        .map(|job| job["prompt"].as_str().expect("prompt").to_string())
        .collect::<Vec<_>>();

    assert_eq!(listed["count"], 1, "jobs listed: {listed}");
    assert_eq!(prompts, vec!["replacement report".to_string()]);
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

    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));
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
    let restarted =
        ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(fragment, "echo"));
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
        config
            .agents
            .iter()
            .all(|agent| agent.name != format!("tinyclaw-{id}")),
        "delete removes scheduled orchestrator agent"
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

    let store = OrchestratorScheduleStore::with_cli_tool_and_project(
        fragment.clone(),
        "echo",
        Some("tinyclaw".to_string()),
    );
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
async fn orchestrator_schedule_upgrades_legacy_owned_fragment_marker_to_project() {
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
    std::fs::write(
        &fragment,
        r#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false

[[agents]]
name = "tinyclaw-existing"
layer = "Core"
cli_tool = "echo"
task = "legacy owned task"
schedule = "0 1 * * *"
capabilities = ["tinyclaw-schedule", "tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = true
"#,
    )
    .expect("write legacy owned fragment");

    let store = OrchestratorScheduleStore::with_cli_tool_and_project(
        fragment.clone(),
        "echo",
        Some("tinyclaw".to_string()),
    );
    let tool = ScheduleTool::new_orchestrator(store);
    tool.execute(json!({
        "op": "create",
        "prompt": "new multi-project report",
        "schedule": "0 9 * * *",
    }))
    .await
    .expect("legacy owned marker is upgraded before mutation");

    let content = std::fs::read_to_string(&fragment).expect("read fragment");
    assert!(
        content.contains("project = \"tinyclaw\""),
        "marker and agents should carry project after upgrade: {content}"
    );
    let config = terraphim_orchestrator::OrchestratorConfig::from_file(&base_config)
        .expect("orchestrator reloads generated schedule fragment");
    config
        .validate()
        .expect("upgraded fragment validates in multi-project mode");
}

#[tokio::test]
async fn orchestrator_schedule_adds_missing_marker_for_owned_project_fragment() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("missing-marker.toml");
    std::fs::write(
        &fragment,
        r#"[[agents]]
name = "tinyclaw-existing"
layer = "Core"
cli_tool = "echo"
task = "owned task without marker"
schedule = "0 1 * * *"
capabilities = ["tinyclaw-schedule", "tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = true
"#,
    )
    .expect("write owned fragment without marker");

    let store = OrchestratorScheduleStore::with_cli_tool_and_project(
        fragment.clone(),
        "echo",
        Some("tinyclaw".to_string()),
    );
    let tool = ScheduleTool::new_orchestrator(store);
    tool.execute(json!({
        "op": "create",
        "prompt": "new report",
        "schedule": "0 9 * * *",
    }))
    .await
    .expect("missing marker is added for otherwise owned fragment");

    let content = std::fs::read_to_string(&fragment).expect("read fragment");
    assert!(content.contains("name = \"tinyclaw-schedule-fragment-marker\""));
    assert!(content.contains("project = \"tinyclaw\""));
}

#[tokio::test]
async fn orchestrator_schedule_rejects_marker_for_different_project_without_rewriting_bytes() {
    common::scrub_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let fragment = temp.path().join("wrong-project.toml");
    let original = br#"[[agents]]
name = "tinyclaw-schedule-fragment-marker"
layer = "Core"
cli_tool = "tinyclaw-scheduler-marker"
task = "TinyClaw scheduler fragment ownership marker"
schedule = "0 0 1 1 *"
project = "other"
capabilities = ["tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = false

[[agents]]
name = "tinyclaw-existing"
layer = "Core"
cli_tool = "echo"
task = "other project task"
schedule = "0 1 * * *"
project = "other"
capabilities = ["tinyclaw-schedule", "tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler", "tinyclaw-schedule-fragment-schema:1"]
enabled = true
"#;
    std::fs::write(&fragment, original).expect("write other-project fragment");

    let store = OrchestratorScheduleStore::with_cli_tool_and_project(
        fragment.clone(),
        "echo",
        Some("tinyclaw".to_string()),
    );
    let tool = ScheduleTool::new_orchestrator(store);
    let err = tool
        .execute(json!({
            "op": "create",
            "prompt": "must not mutate",
            "schedule": "0 9 * * *",
        }))
        .await
        .expect_err("different project marker must be rejected");
    match err {
        ToolError::ExecutionFailed { message, .. } => {
            assert!(message.contains("project"), "got: {message}");
            assert!(message.contains("other"), "got: {message}");
        }
        other => panic!("expected ExecutionFailed, got {other:?}"),
    }
    assert_eq!(
        std::fs::read(&fragment).expect("read fragment"),
        original,
        "rejected project mismatch must remain byte-for-byte unchanged"
    );
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

    let tool =
        ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool_and_project(
            fragment,
            "echo",
            Some("missing".to_string()),
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

    let tool =
        ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(fragment, "echo"));
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
    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        fragment.clone(),
        "echo",
    ));

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
    let tool = ScheduleTool::new_orchestrator(OrchestratorScheduleStore::with_cli_tool(
        temp.path().join("tinyclaw-schedules.toml"),
        "echo",
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
