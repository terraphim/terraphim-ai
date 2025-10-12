use chrono::Utc;
use terraphim_multi_agent::vm_execution::*;

#[tokio::test]
async fn test_history_config_defaults() {
    let config = HistoryConfig::default();

    assert!(config.enabled);
    assert!(!config.snapshot_on_execution);
    assert!(config.snapshot_on_failure);
    assert!(!config.auto_rollback_on_failure);
    assert_eq!(config.max_history_entries, 100);
    assert!(config.persist_history);
    assert_eq!(config.integration_mode, "http");
}

#[tokio::test]
async fn test_vm_execution_config_with_history() {
    let config = VmExecutionConfig {
        enabled: true,
        history: HistoryConfig {
            enabled: true,
            auto_rollback_on_failure: true,
            ..Default::default()
        },
        ..Default::default()
    };

    assert!(config.history.enabled);
    assert!(config.history.auto_rollback_on_failure);
}

#[tokio::test]
async fn test_fcctl_bridge_creation() {
    let history_config = HistoryConfig::default();
    let _bridge = FcctlBridge::new(history_config, "http://localhost:8080".to_string());
}

#[tokio::test]
async fn test_history_query_request() {
    let request = HistoryQueryRequest {
        vm_id: "test-vm-123".to_string(),
        agent_id: Some("agent-1".to_string()),
        limit: Some(50),
        failures_only: true,
        start_date: None,
        end_date: None,
    };

    assert_eq!(request.vm_id, "test-vm-123");
    assert_eq!(request.agent_id, Some("agent-1".to_string()));
    assert_eq!(request.limit, Some(50));
    assert!(request.failures_only);
}

#[tokio::test]
async fn test_rollback_request() {
    let request = RollbackRequest {
        vm_id: "test-vm-123".to_string(),
        snapshot_id: "snapshot-abc".to_string(),
        create_pre_rollback_snapshot: true,
    };

    assert_eq!(request.vm_id, "test-vm-123");
    assert_eq!(request.snapshot_id, "snapshot-abc");
    assert!(request.create_pre_rollback_snapshot);
}

#[tokio::test]
async fn test_command_history_entry_creation() {
    let entry = CommandHistoryEntry {
        id: "entry-1".to_string(),
        vm_id: "vm-123".to_string(),
        agent_id: "agent-1".to_string(),
        command: "print('hello')".to_string(),
        language: "python".to_string(),
        snapshot_id: Some("snapshot-1".to_string()),
        success: true,
        exit_code: 0,
        stdout: "hello\n".to_string(),
        stderr: "".to_string(),
        executed_at: Utc::now(),
        duration_ms: 150,
    };

    assert!(entry.success);
    assert_eq!(entry.exit_code, 0);
    assert_eq!(entry.language, "python");
}

#[tokio::test]
async fn test_vm_execution_error_variants() {
    let errors = [
        VmExecutionError::HistoryError("test error".to_string()),
        VmExecutionError::SnapshotNotFound("snap-123".to_string()),
        VmExecutionError::RollbackFailed("rollback error".to_string()),
    ];

    assert_eq!(errors[0].to_string(), "History error: test error");
    assert_eq!(errors[1].to_string(), "Snapshot not found: snap-123");
    assert_eq!(errors[2].to_string(), "Rollback failed: rollback error");
}

#[tokio::test]
async fn test_vm_execution_client_with_history() {
    let mut config = VmExecutionConfig {
        enabled: true,
        api_base_url: "http://localhost:8080".to_string(),
        ..Default::default()
    };
    config.history.enabled = true;
    config.history.snapshot_on_failure = true;

    let _client = VmExecutionClient::new(&config);
}

#[tokio::test]
async fn test_vm_execution_client_without_history() {
    let mut config = VmExecutionConfig {
        enabled: true,
        api_base_url: "http://localhost:8080".to_string(),
        ..Default::default()
    };
    config.history.enabled = false;

    let client = VmExecutionClient::new(&config);

    let result = client.get_last_successful_snapshot("vm-1", "agent-1").await;
    assert!(result.is_none());
}

#[test]
fn test_history_config_serialization() {
    let config = HistoryConfig {
        enabled: true,
        snapshot_on_execution: false,
        snapshot_on_failure: true,
        auto_rollback_on_failure: true,
        max_history_entries: 50,
        persist_history: true,
        integration_mode: "http".to_string(),
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: HistoryConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.enabled, config.enabled);
    assert_eq!(deserialized.max_history_entries, config.max_history_entries);
    assert_eq!(deserialized.integration_mode, config.integration_mode);
}

#[test]
fn test_vm_execution_config_json_with_history() {
    let json = r#"{
        "enabled": true,
        "api_base_url": "http://localhost:8080",
        "vm_pool_size": 3,
        "default_vm_type": "terraphim-minimal",
        "execution_timeout_ms": 30000,
        "allowed_languages": ["python", "javascript"],
        "auto_provision": true,
        "code_validation": true,
        "max_code_length": 10000,
        "history": {
            "enabled": true,
            "snapshot_on_execution": false,
            "snapshot_on_failure": true,
            "auto_rollback_on_failure": true,
            "max_history_entries": 100,
            "persist_history": true,
            "integration_mode": "http"
        }
    }"#;

    let config: VmExecutionConfig = serde_json::from_str(json).unwrap();

    assert!(config.enabled);
    assert!(config.history.enabled);
    assert!(config.history.auto_rollback_on_failure);
    assert_eq!(config.history.max_history_entries, 100);
}
