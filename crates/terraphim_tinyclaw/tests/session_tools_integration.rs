use std::sync::Arc;

use terraphim_tinyclaw::config::ToolsConfig;
use terraphim_tinyclaw::session::{ChatMessage, SessionManager};
use terraphim_tinyclaw::tools::{ToolCall, create_registry_from_config_with_runtime};
use tokio::sync::{Mutex, mpsc};

#[tokio::test]
async fn test_session_tools_registry_and_roundtrip() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let sessions = Arc::new(Mutex::new(SessionManager::new(
        temp_dir.path().to_path_buf(),
    )));
    let (outbound_tx, mut outbound_rx) = mpsc::channel(8);

    {
        let mut guard = sessions.lock().await;
        let session = guard.get_or_create("cli:target");
        session.add_message(ChatMessage::user("seed", "user"));
        let snapshot = session.clone();
        guard.save(&snapshot).unwrap();
    }

    let registry = create_registry_from_config_with_runtime(
        &ToolsConfig::default(),
        Some(sessions.clone()),
        Some(outbound_tx),
        Some(temp_dir.path().to_path_buf()),
    );

    assert!(registry.has("sessions_list"));
    assert!(registry.has("sessions_history"));
    assert!(registry.has("sessions_send"));

    let list_call = ToolCall {
        id: "list-1".to_string(),
        name: "sessions_list".to_string(),
        arguments: serde_json::json!({
            "requester_session_key": "cli:source",
            "limit": 10
        }),
    };
    let list_raw = registry.execute(&list_call).await.unwrap();
    let list_payload: serde_json::Value = serde_json::from_str(&list_raw).unwrap();
    assert_eq!(list_payload["status"], "ok");
    assert!(list_payload["count"].as_u64().unwrap() >= 1);

    let history_call = ToolCall {
        id: "history-1".to_string(),
        name: "sessions_history".to_string(),
        arguments: serde_json::json!({
            "requester_session_key": "cli:source",
            "session_key": "cli:target",
            "limit": 10
        }),
    };
    let history_raw = registry.execute(&history_call).await.unwrap();
    let history_payload: serde_json::Value = serde_json::from_str(&history_raw).unwrap();
    assert_eq!(history_payload["status"], "ok");
    assert_eq!(history_payload["session_key"], "cli:target");
    assert!(history_payload["returned_messages"].as_u64().unwrap() >= 1);

    let send_call = ToolCall {
        id: "send-1".to_string(),
        name: "sessions_send".to_string(),
        arguments: serde_json::json!({
            "requester_session_key": "cli:source",
            "session_key": "cli:target",
            "message": "integration ping"
        }),
    };
    let send_raw = registry.execute(&send_call).await.unwrap();
    let send_payload: serde_json::Value = serde_json::from_str(&send_raw).unwrap();
    assert_eq!(send_payload["status"], "queued");

    let outbound = outbound_rx.recv().await.unwrap();
    assert_eq!(outbound.channel, "cli");
    assert_eq!(outbound.chat_id, "target");
    assert_eq!(outbound.content, "integration ping");
}

#[tokio::test]
async fn test_agent_spawn_baseline_via_registry() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let registry = create_registry_from_config_with_runtime(
        &ToolsConfig::default(),
        None,
        None,
        Some(temp_dir.path().to_path_buf()),
    );

    assert!(registry.has("agent_spawn"));

    let call = ToolCall {
        id: "spawn-1".to_string(),
        name: "agent_spawn".to_string(),
        arguments: serde_json::json!({
            "agent_type": "echo",
            "task": "spawn integration",
            "wait_seconds": 1
        }),
    };

    let raw = registry.execute(&call).await.unwrap();
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(payload["status"].is_string());
    assert!(payload["process_id"].as_u64().is_some());
}
