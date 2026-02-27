use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use terraphim_tinyclaw::config::{CronConfig, SpawnerConfig, ToolsConfig};
use terraphim_tinyclaw::session::SessionManager;
use terraphim_tinyclaw::tools::{
    ToolCall, create_registry_from_config_with_runtime_and_orchestration,
};
use tokio::sync::{Mutex, mpsc};

#[tokio::test]
async fn test_cron_tool_schedules_and_dispatches_message() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let sessions = Arc::new(Mutex::new(SessionManager::new(
        temp_dir.path().join("sessions"),
    )));
    let (outbound_tx, mut outbound_rx) = mpsc::channel(8);

    let cron_cfg = CronConfig {
        enabled: true,
        tick_seconds: 1,
        persist_path: Some("cron/jobs.json".into()),
        max_jobs: 32,
    };

    let registry = create_registry_from_config_with_runtime_and_orchestration(
        &ToolsConfig::default(),
        &SpawnerConfig::default(),
        &cron_cfg,
        Some(sessions.clone()),
        Some(outbound_tx.clone()),
        Some(temp_dir.path().to_path_buf()),
    );

    assert!(registry.has("cron"));

    let at = (Utc::now() + ChronoDuration::seconds(1)).to_rfc3339();
    let add_call = ToolCall {
        id: "cron-add-1".to_string(),
        name: "cron".to_string(),
        arguments: serde_json::json!({
            "action": "add",
            "requester_session_key": "cli:source",
            "session_key": "cli:target",
            "message": "scheduled ping",
            "schedule": {
                "kind": "at",
                "at": at
            }
        }),
    };

    let add_raw = registry.execute(&add_call).await.unwrap();
    let add_payload: serde_json::Value = serde_json::from_str(&add_raw).unwrap();
    assert_eq!(add_payload["status"], "scheduled");
    let job_id = add_payload["job"]["id"].as_str().unwrap().to_string();

    let outbound = tokio::time::timeout(std::time::Duration::from_secs(5), outbound_rx.recv())
        .await
        .expect("expected scheduled outbound dispatch")
        .expect("expected outbound message");

    assert_eq!(outbound.channel, "cli");
    assert_eq!(outbound.chat_id, "target");
    assert_eq!(outbound.content, "scheduled ping");

    {
        let mut guard = sessions.lock().await;
        let snapshot = guard
            .get_session_snapshot("cli:target")
            .expect("session should exist");
        assert!(
            snapshot
                .messages
                .iter()
                .any(|message| message.content == "scheduled ping")
        );
    }

    let remove_call = ToolCall {
        id: "cron-remove-1".to_string(),
        name: "cron".to_string(),
        arguments: serde_json::json!({
            "action": "remove",
            "id": job_id,
        }),
    };

    let remove_raw = registry.execute(&remove_call).await.unwrap();
    let remove_payload: serde_json::Value = serde_json::from_str(&remove_raw).unwrap();
    assert_eq!(remove_payload["status"], "ok");
    assert_eq!(remove_payload["removed"], true);
}

#[tokio::test]
async fn test_cron_tool_blocks_cross_channel_target() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let sessions = Arc::new(Mutex::new(SessionManager::new(
        temp_dir.path().join("sessions"),
    )));
    let (outbound_tx, _outbound_rx) = mpsc::channel(4);

    let registry = create_registry_from_config_with_runtime_and_orchestration(
        &ToolsConfig::default(),
        &SpawnerConfig::default(),
        &CronConfig {
            enabled: true,
            ..Default::default()
        },
        Some(sessions),
        Some(outbound_tx),
        Some(temp_dir.path().to_path_buf()),
    );

    let add_call = ToolCall {
        id: "cron-add-blocked".to_string(),
        name: "cron".to_string(),
        arguments: serde_json::json!({
            "action": "add",
            "requester_session_key": "cli:source",
            "session_key": "telegram:target",
            "message": "nope",
            "schedule": {
                "kind": "every",
                "every_seconds": 1
            }
        }),
    };

    let err = registry.execute(&add_call).await.unwrap_err();
    assert!(matches!(
        err,
        terraphim_tinyclaw::tools::ToolError::Blocked { .. }
    ));
}
