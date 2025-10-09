use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;
use terraphim_multi_agent::vm_execution::*;
use tokio::time::{timeout, Duration};

#[cfg(test)]
mod direct_session_unit_tests {
    use super::*;

    #[tokio::test]
    async fn test_session_adapter_creation() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let sessions = adapter.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_session_info_not_found() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let info = adapter.get_session_info("non-existent").await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_close_non_existent_session() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let result = adapter.close_session("non-existent").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VmExecutionError::SessionNotFound(_)
        ));
    }
}

#[cfg(test)]
mod direct_session_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_session_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        assert!(session_id.contains("test-vm-1"));
        assert!(session_id.contains("agent-1"));

        let info = adapter.get_session_info(&session_id).await;
        assert!(info.is_some());

        let session_info = info.unwrap();
        assert_eq!(session_info.vm_id, "test-vm-1");
        assert_eq!(session_info.agent_id, "agent-1");
        assert_eq!(session_info.command_count, 0);

        adapter.close_session(&session_id).await.unwrap();

        let info_after_close = adapter.get_session_info(&session_id).await;
        assert!(info_after_close.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_session_reuse() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id_1 = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let session_id_2 = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        assert_eq!(session_id_1, session_id_2);

        adapter.close_session(&session_id_1).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_command_in_session() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let (output, exit_code) = adapter
            .execute_command_direct(&session_id, "echo 'Hello from direct session'")
            .await
            .unwrap();

        assert_eq!(exit_code, 0);
        assert!(output.contains("Hello from direct session"));

        let info = adapter.get_session_info(&session_id).await.unwrap();
        assert_eq!(info.command_count, 1);

        adapter.close_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_multiple_commands_in_session() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let commands = vec!["echo 'Command 1'", "echo 'Command 2'", "echo 'Command 3'"];

        for (i, cmd) in commands.iter().enumerate() {
            let (output, exit_code) = adapter
                .execute_command_direct(&session_id, cmd)
                .await
                .unwrap();

            assert_eq!(exit_code, 0);
            assert!(output.contains(&format!("Command {}", i + 1)));
        }

        let info = adapter.get_session_info(&session_id).await.unwrap();
        assert_eq!(info.command_count, 3);

        adapter.close_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_snapshot_creation() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        adapter
            .execute_command_direct(&session_id, "echo 'Setting up state'")
            .await
            .unwrap();

        let snapshot_id = adapter
            .create_snapshot_direct(&session_id, "test-snapshot")
            .await
            .unwrap();

        assert!(!snapshot_id.is_empty());
        assert!(snapshot_id.contains("test-snapshot"));

        adapter.close_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_snapshot_and_rollback() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let (output1, _) = adapter
            .execute_command_direct(
                &session_id,
                "echo 'State 1' > /tmp/state.txt && cat /tmp/state.txt",
            )
            .await
            .unwrap();
        assert!(output1.contains("State 1"));

        let snapshot_id = adapter
            .create_snapshot_direct(&session_id, "state-1")
            .await
            .unwrap();

        let (output2, _) = adapter
            .execute_command_direct(
                &session_id,
                "echo 'State 2' > /tmp/state.txt && cat /tmp/state.txt",
            )
            .await
            .unwrap();
        assert!(output2.contains("State 2"));

        adapter
            .rollback_direct(&session_id, &snapshot_id)
            .await
            .unwrap();

        let (output3, _) = adapter
            .execute_command_direct(&session_id, "cat /tmp/state.txt")
            .await
            .unwrap();
        assert!(output3.contains("State 1"));

        adapter.close_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_concurrent_sessions() {
        let temp_dir = tempdir().unwrap();
        let adapter = Arc::new(DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        ));

        let mut handles = vec![];

        for i in 1..=3 {
            let adapter_clone = Arc::clone(&adapter);
            let handle = tokio::spawn(async move {
                let vm_id = format!("vm-{}", i);
                let agent_id = format!("agent-{}", i);

                let session_id = adapter_clone
                    .get_or_create_session(&vm_id, &agent_id, "ubuntu")
                    .await
                    .unwrap();

                let (output, exit_code) = adapter_clone
                    .execute_command_direct(&session_id, &format!("echo 'Session {}'", i))
                    .await
                    .unwrap();

                assert_eq!(exit_code, 0);
                assert!(output.contains(&format!("Session {}", i)));

                adapter_clone.close_session(&session_id).await.unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let remaining_sessions = adapter.list_sessions().await;
        assert_eq!(remaining_sessions.len(), 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_error_handling_invalid_command() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let (output, exit_code) = adapter
            .execute_command_direct(&session_id, "nonexistentcommand")
            .await
            .unwrap();

        assert_ne!(exit_code, 0);
        assert!(output.contains("not found") || !output.is_empty());

        adapter.close_session(&session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_connection_info() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("test-vm-1", "agent-1", "ubuntu")
            .await
            .unwrap();

        let info = adapter.get_connection_info(&session_id).await;

        assert!(info.is_ok());
        let info_str = info.unwrap();
        assert!(!info_str.is_empty());

        adapter.close_session(&session_id).await.unwrap();
    }
}

#[cfg(test)]
mod fcctl_bridge_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_with_direct_mode() {
        let config = HistoryConfig {
            enabled: true,
            snapshot_on_execution: true,
            snapshot_on_failure: false,
            auto_rollback_on_failure: false,
            max_history_entries: 100,
            persist_history: false,
            integration_mode: "direct".to_string(),
        };

        let bridge = FcctlBridge::new(config, "http://localhost:8080".to_string());
    }

    #[tokio::test]
    async fn test_bridge_with_http_mode() {
        let config = HistoryConfig {
            enabled: true,
            snapshot_on_execution: true,
            snapshot_on_failure: false,
            auto_rollback_on_failure: false,
            max_history_entries: 100,
            persist_history: false,
            integration_mode: "http".to_string(),
        };

        let bridge = FcctlBridge::new(config, "http://localhost:8080".to_string());
    }

    #[tokio::test]
    #[ignore]
    async fn test_bridge_direct_vs_http_comparison() {
        let direct_config = HistoryConfig {
            enabled: true,
            snapshot_on_execution: false,
            snapshot_on_failure: false,
            auto_rollback_on_failure: false,
            max_history_entries: 100,
            persist_history: false,
            integration_mode: "direct".to_string(),
        };

        let http_config = HistoryConfig {
            enabled: true,
            snapshot_on_execution: false,
            snapshot_on_failure: false,
            auto_rollback_on_failure: false,
            max_history_entries: 100,
            persist_history: false,
            integration_mode: "http".to_string(),
        };

        let direct_bridge = FcctlBridge::new(direct_config, "http://localhost:8080".to_string());

        let http_bridge = FcctlBridge::new(http_config, "http://localhost:8080".to_string());
    }
}
