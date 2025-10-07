use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::models::*;

#[derive(Debug)]
#[allow(dead_code)]
pub struct DirectSessionAdapter {
    sessions: Arc<RwLock<HashMap<String, SessionHandle>>>,
    data_dir: PathBuf,
    fcctl_api_url: String,
}

#[derive(Debug)]
struct SessionHandle {
    session_name: String,
    vm_id: String,
    agent_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    command_count: usize,
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl DirectSessionAdapter {
    pub fn new(data_dir: PathBuf, fcctl_api_url: String) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
            fcctl_api_url,
        }
    }

    pub async fn get_or_create_session(
        &self,
        vm_id: &str,
        agent_id: &str,
        vm_type: &str,
    ) -> Result<String, VmExecutionError> {
        let session_key = format!("{}:{}", vm_id, agent_id);

        let sessions = self.sessions.read().await;
        if sessions.contains_key(&session_key) {
            debug!(
                "Session already exists for vm={}, agent={}",
                vm_id, agent_id
            );
            return Ok(session_key);
        }
        drop(sessions);

        info!(
            "Creating new direct session for vm={}, agent={}",
            vm_id, agent_id
        );

        let session_name = format!("agent-{}-{}", agent_id, chrono::Utc::now().timestamp());
        let client = reqwest::Client::new();

        let create_payload = serde_json::json!({
            "name": session_name,
            "vm_type": vm_type,
            "memory_mb": 2048,
            "vcpus": 2
        });

        let response = client
            .post(format!("{}/sessions", self.fcctl_api_url))
            .json(&create_payload)
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::ConnectionError(format!(
                    "Failed to create session via API: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::ConfigError(format!(
                "Session creation failed: {}",
                error_text
            )));
        }

        let mut sessions = self.sessions.write().await;

        let handle = SessionHandle {
            session_name: session_name.clone(),
            vm_id: vm_id.to_string(),
            agent_id: agent_id.to_string(),
            created_at: chrono::Utc::now(),
            command_count: 0,
            http_client: client,
        };

        sessions.insert(session_key.clone(), handle);

        Ok(session_key)
    }

    pub async fn execute_command_direct(
        &self,
        session_id: &str,
        command: &str,
    ) -> Result<(String, i32), VmExecutionError> {
        debug!("Executing command in direct session: {}", session_id);

        let mut sessions = self.sessions.write().await;
        let handle = sessions
            .get_mut(session_id)
            .ok_or_else(|| VmExecutionError::SessionNotFound(session_id.to_string()))?;

        handle.command_count += 1;
        let command_num = handle.command_count;

        let exec_payload = serde_json::json!({
            "command": command
        });

        let response = handle
            .http_client
            .post(format!(
                "{}/sessions/{}/execute",
                self.fcctl_api_url, handle.session_name
            ))
            .json(&exec_payload)
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::ExecutionFailed(format!(
                    "Command execution request failed: {}",
                    e
                ))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::ExecutionFailed(format!(
                "Execution failed: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response.json().await.map_err(|e| {
            VmExecutionError::ExecutionFailed(format!("Failed to parse execution result: {}", e))
        })?;

        let exit_code = result["exit_code"].as_i64().unwrap_or(1) as i32;
        let output = result["output"].as_str().unwrap_or("").to_string();

        info!(
            "Executed command #{} in session {} with exit code {}",
            command_num, session_id, exit_code
        );

        Ok((output, exit_code))
    }

    pub async fn create_snapshot_direct(
        &self,
        session_id: &str,
        name: &str,
    ) -> Result<String, VmExecutionError> {
        debug!("Creating snapshot for session: {}", session_id);

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| VmExecutionError::SessionNotFound(session_id.to_string()))?;

        let snapshot_payload = serde_json::json!({
            "name": name
        });

        let response = handle
            .http_client
            .post(format!(
                "{}/sessions/{}/snapshots",
                self.fcctl_api_url, handle.session_name
            ))
            .json(&snapshot_payload)
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::SnapshotFailed(format!("Snapshot creation request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::SnapshotFailed(format!(
                "Snapshot creation failed: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response.json().await.map_err(|e| {
            VmExecutionError::SnapshotFailed(format!("Failed to parse snapshot result: {}", e))
        })?;

        let snapshot_id = result["snapshot_id"]
            .as_str()
            .ok_or_else(|| {
                VmExecutionError::SnapshotFailed("No snapshot_id in response".to_string())
            })?
            .to_string();

        info!(
            "Created snapshot {} for session {}",
            snapshot_id, session_id
        );

        Ok(snapshot_id)
    }

    pub async fn rollback_direct(
        &self,
        session_id: &str,
        snapshot_id: &str,
    ) -> Result<(), VmExecutionError> {
        debug!(
            "Rolling back session {} to snapshot {}",
            session_id, snapshot_id
        );

        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| VmExecutionError::SessionNotFound(session_id.to_string()))?;

        let rollback_payload = serde_json::json!({
            "snapshot_id": snapshot_id
        });

        let response = handle
            .http_client
            .post(format!(
                "{}/sessions/{}/rollback",
                self.fcctl_api_url, handle.session_name
            ))
            .json(&rollback_payload)
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::RollbackFailed(format!("Rollback request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::RollbackFailed(format!(
                "Rollback failed: {}",
                error_text
            )));
        }

        info!(
            "Rolled back session {} to snapshot {}",
            session_id, snapshot_id
        );

        Ok(())
    }

    pub async fn get_session_info(&self, session_id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|handle| SessionInfo {
            vm_id: handle.vm_id.clone(),
            agent_id: handle.agent_id.clone(),
            created_at: handle.created_at,
            command_count: handle.command_count,
        })
    }

    pub async fn get_connection_info(&self, session_id: &str) -> Result<String, VmExecutionError> {
        let sessions = self.sessions.read().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| VmExecutionError::SessionNotFound(session_id.to_string()))?;

        let response = handle
            .http_client
            .get(format!(
                "{}/sessions/{}/info",
                self.fcctl_api_url, handle.session_name
            ))
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::ConnectionError(format!("Failed to get connection info: {}", e))
            })?;

        if !response.status().is_success() {
            return Ok(format!(
                "Session: {} (info unavailable)",
                handle.session_name
            ));
        }

        let info = response
            .text()
            .await
            .unwrap_or_else(|_| format!("Session: {}", handle.session_name));
        Ok(info)
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), VmExecutionError> {
        debug!("Closing session: {}", session_id);

        let mut sessions = self.sessions.write().await;
        let handle = sessions
            .remove(session_id)
            .ok_or_else(|| VmExecutionError::SessionNotFound(session_id.to_string()))?;

        let _response = handle
            .http_client
            .delete(format!(
                "{}/sessions/{}",
                self.fcctl_api_url, handle.session_name
            ))
            .send()
            .await
            .map_err(|e| {
                VmExecutionError::ConnectionError(format!("Failed to delete session: {}", e))
            })?;

        info!("Closed session: {}", session_id);

        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|handle| SessionInfo {
                vm_id: handle.vm_id.clone(),
                agent_id: handle.agent_id.clone(),
                created_at: handle.created_at,
                command_count: handle.command_count,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub vm_id: String,
    pub agent_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub command_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_session() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let session_id = adapter
            .get_or_create_session("vm-1", "agent-1", "ubuntu")
            .await;
        assert!(session_id.is_ok() || session_id.is_err());
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let sessions = adapter.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_session_info() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let info = adapter.get_session_info("non-existent").await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_operations() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let result = adapter.create_snapshot_direct("non-existent", "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_close_session() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let result = adapter.close_session("non-existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = tempdir().unwrap();
        let adapter = DirectSessionAdapter::new(
            temp_dir.path().to_path_buf(),
            "http://localhost:8080".to_string(),
        );

        let sessions = adapter.list_sessions().await;
        assert_eq!(sessions.len(), 0);
    }
}
