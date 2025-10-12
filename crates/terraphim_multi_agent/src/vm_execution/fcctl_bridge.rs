use super::models::*;
use super::session_adapter::DirectSessionAdapter;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub struct FcctlBridge {
    config: HistoryConfig,
    http_client: Client,
    api_base_url: String,
    agent_sessions: Arc<RwLock<std::collections::HashMap<String, VmSession>>>,
    direct_adapter: Option<Arc<DirectSessionAdapter>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct VmSession {
    vm_id: String,
    agent_id: String,
    history: Vec<CommandHistoryEntry>,
    last_snapshot_id: Option<String>,
    created_at: chrono::DateTime<Utc>,
}

impl FcctlBridge {
    pub fn new(config: HistoryConfig, api_base_url: String) -> Self {
        let direct_adapter = if config.integration_mode == "direct" {
            let data_dir = PathBuf::from("/tmp/fcctl-sessions");
            Some(Arc::new(DirectSessionAdapter::new(
                data_dir,
                api_base_url.clone(),
            )))
        } else {
            None
        };

        Self {
            config,
            http_client: Client::new(),
            api_base_url,
            agent_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            direct_adapter,
        }
    }

    pub async fn track_execution(
        &self,
        vm_id: &str,
        agent_id: &str,
        request: &VmExecuteRequest,
        response: &VmExecuteResponse,
    ) -> Result<Option<String>, VmExecutionError> {
        if !self.config.enabled {
            debug!("History tracking disabled, skipping");
            return Ok(None);
        }

        let snapshot_id = if self.should_create_snapshot(response.exit_code) {
            match self.create_snapshot(vm_id, agent_id).await {
                Ok(id) => {
                    info!("Created snapshot {} for VM {} after execution", id, vm_id);
                    Some(id)
                }
                Err(e) => {
                    warn!("Failed to create snapshot for VM {}: {}", vm_id, e);
                    None
                }
            }
        } else {
            None
        };

        let entry = CommandHistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            vm_id: vm_id.to_string(),
            agent_id: agent_id.to_string(),
            command: request.code.clone(),
            language: request.language.clone(),
            snapshot_id: snapshot_id.clone(),
            success: response.exit_code == 0,
            exit_code: response.exit_code,
            stdout: response.stdout.clone(),
            stderr: response.stderr.clone(),
            executed_at: response.completed_at,
            duration_ms: response.duration_ms,
        };

        if self.config.persist_history {
            if let Err(e) = self.persist_history_entry(&entry).await {
                warn!("Failed to persist history entry: {}", e);
            }
        }

        let mut sessions = self.agent_sessions.write().await;
        let session_key = format!("{}:{}", vm_id, agent_id);
        let session = sessions.entry(session_key).or_insert_with(|| VmSession {
            vm_id: vm_id.to_string(),
            agent_id: agent_id.to_string(),
            history: Vec::new(),
            last_snapshot_id: None,
            created_at: Utc::now(),
        });

        session.history.push(entry);
        if snapshot_id.is_some() {
            session.last_snapshot_id = snapshot_id.clone();
        }

        if session.history.len() > self.config.max_history_entries {
            session.history.remove(0);
        }

        Ok(snapshot_id)
    }

    fn should_create_snapshot(&self, exit_code: i32) -> bool {
        if self.config.snapshot_on_execution {
            true
        } else {
            self.config.snapshot_on_failure && exit_code != 0
        }
    }

    async fn create_snapshot(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Result<String, VmExecutionError> {
        if self.config.integration_mode == "http" {
            self.create_snapshot_http(vm_id, agent_id).await
        } else if let Some(ref adapter) = self.direct_adapter {
            let session_key = format!("{}:{}", vm_id, agent_id);
            adapter
                .get_or_create_session(vm_id, agent_id, "ubuntu")
                .await?;
            let snapshot_name = format!("agent-{}-{}", agent_id, Utc::now().timestamp());
            adapter
                .create_snapshot_direct(&session_key, &snapshot_name)
                .await
        } else {
            Err(VmExecutionError::Internal(
                "Direct adapter not initialized".to_string(),
            ))
        }
    }

    async fn create_snapshot_http(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Result<String, VmExecutionError> {
        let url = format!("{}/api/vms/{}/snapshots", self.api_base_url, vm_id);
        let snapshot_name = format!("agent-{}-{}", agent_id, Utc::now().timestamp());

        let response = self
            .http_client
            .post(&url)
            .json(&json!({
                "name": snapshot_name,
                "description": format!("Auto-created snapshot for agent {}", agent_id)
            }))
            .send()
            .await
            .map_err(|e| VmExecutionError::ApiError(format!("Failed to create snapshot: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::ApiError(format!(
                "Failed to create snapshot: HTTP {} - {}",
                status, error_text
            )));
        }

        let snapshot_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VmExecutionError::ApiError(format!("Invalid response: {}", e)))?;

        snapshot_data
            .get("snapshot_id")
            .or_else(|| snapshot_data.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| VmExecutionError::ApiError("No snapshot_id in response".to_string()))
    }

    async fn persist_history_entry(
        &self,
        entry: &CommandHistoryEntry,
    ) -> Result<(), VmExecutionError> {
        debug!(
            "Persisting history entry {} for VM {}",
            entry.id, entry.vm_id
        );
        Ok(())
    }

    pub async fn query_history(
        &self,
        request: HistoryQueryRequest,
    ) -> Result<HistoryQueryResponse, VmExecutionError> {
        if self.config.integration_mode == "http" {
            self.query_history_http(request).await
        } else {
            let sessions = self.agent_sessions.read().await;
            let session_key = format!(
                "{}:{}",
                request.vm_id,
                request.agent_id.as_ref().unwrap_or(&"".to_string())
            );

            if let Some(session) = sessions.get(&session_key) {
                let mut entries = session.history.clone();

                if request.failures_only {
                    entries.retain(|e| !e.success);
                }

                if let Some(limit) = request.limit {
                    entries.truncate(limit);
                }

                Ok(HistoryQueryResponse {
                    vm_id: request.vm_id,
                    entries: entries.clone(),
                    total: entries.len(),
                })
            } else {
                Ok(HistoryQueryResponse {
                    vm_id: request.vm_id,
                    entries: vec![],
                    total: 0,
                })
            }
        }
    }

    async fn query_history_http(
        &self,
        request: HistoryQueryRequest,
    ) -> Result<HistoryQueryResponse, VmExecutionError> {
        let url = format!("{}/api/vms/{}/history", self.api_base_url, request.vm_id);

        let response = self
            .http_client
            .get(&url)
            .query(&[
                ("limit", request.limit.unwrap_or(100).to_string()),
                ("failures_only", request.failures_only.to_string()),
            ])
            .send()
            .await
            .map_err(|e| VmExecutionError::ApiError(format!("Failed to query history: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::HistoryError(format!(
                "HTTP {} - {}",
                status, error_text
            )));
        }

        let history_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| VmExecutionError::ApiError(format!("Invalid response: {}", e)))?;

        let entries = history_data
            .get("history")
            .or_else(|| history_data.get("entries"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        let total = history_data
            .get("total")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(HistoryQueryResponse {
            vm_id: request.vm_id,
            entries,
            total,
        })
    }

    pub async fn rollback_to_snapshot(
        &self,
        request: RollbackRequest,
    ) -> Result<RollbackResponse, VmExecutionError> {
        if self.config.integration_mode == "http" {
            self.rollback_http(request).await
        } else {
            Err(VmExecutionError::Internal(
                "Direct integration mode not yet implemented".to_string(),
            ))
        }
    }

    async fn rollback_http(
        &self,
        request: RollbackRequest,
    ) -> Result<RollbackResponse, VmExecutionError> {
        let pre_rollback_snapshot_id = if request.create_pre_rollback_snapshot {
            match self.create_snapshot(&request.vm_id, "pre-rollback").await {
                Ok(id) => Some(id),
                Err(e) => {
                    warn!("Failed to create pre-rollback snapshot: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let url = format!(
            "{}/api/vms/{}/rollback/{}",
            self.api_base_url, request.vm_id, request.snapshot_id
        );

        let response = self
            .http_client
            .post(&url)
            .send()
            .await
            .map_err(|e| VmExecutionError::ApiError(format!("Failed to rollback: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(VmExecutionError::RollbackFailed(format!(
                "HTTP {} - {}",
                status, error_text
            )));
        }

        Ok(RollbackResponse {
            vm_id: request.vm_id,
            restored_snapshot_id: request.snapshot_id,
            pre_rollback_snapshot_id,
            rolled_back_at: Utc::now(),
            success: true,
            error: None,
        })
    }

    pub async fn auto_rollback_on_failure(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Result<Option<RollbackResponse>, VmExecutionError> {
        if !self.config.auto_rollback_on_failure {
            return Ok(None);
        }

        let sessions = self.agent_sessions.read().await;
        let session_key = format!("{}:{}", vm_id, agent_id);

        if let Some(session) = sessions.get(&session_key) {
            if let Some(last_snapshot_id) = &session.last_snapshot_id {
                info!(
                    "Auto-rollback: restoring VM {} to snapshot {}",
                    vm_id, last_snapshot_id
                );

                let rollback_request = RollbackRequest {
                    vm_id: vm_id.to_string(),
                    snapshot_id: last_snapshot_id.clone(),
                    create_pre_rollback_snapshot: false,
                };

                drop(sessions);

                match self.rollback_to_snapshot(rollback_request).await {
                    Ok(response) => Ok(Some(response)),
                    Err(e) => {
                        error!("Auto-rollback failed for VM {}: {}", vm_id, e);
                        Err(e)
                    }
                }
            } else {
                warn!(
                    "Auto-rollback requested but no snapshot available for VM {}",
                    vm_id
                );
                Ok(None)
            }
        } else {
            warn!("No session found for VM {} and agent {}", vm_id, agent_id);
            Ok(None)
        }
    }

    pub async fn get_last_successful_snapshot(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Option<String> {
        let sessions = self.agent_sessions.read().await;
        let session_key = format!("{}:{}", vm_id, agent_id);

        sessions.get(&session_key).and_then(|session| {
            session
                .history
                .iter()
                .rev()
                .find(|entry| entry.success && entry.snapshot_id.is_some())
                .and_then(|entry| entry.snapshot_id.clone())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_creation() {
        let config = HistoryConfig::default();
        let bridge = FcctlBridge::new(config, "http://localhost:8080".to_string());
        assert!(bridge.config.enabled);
    }

    #[tokio::test]
    async fn test_should_create_snapshot() {
        let config = HistoryConfig {
            enabled: true,
            snapshot_on_execution: false,
            snapshot_on_failure: true,
            ..Default::default()
        };
        let bridge = FcctlBridge::new(config, "http://localhost:8080".to_string());

        assert!(bridge.should_create_snapshot(1));
        assert!(!bridge.should_create_snapshot(0));

        let config_all = HistoryConfig {
            enabled: true,
            snapshot_on_execution: true,
            snapshot_on_failure: false,
            ..Default::default()
        };
        let bridge_all = FcctlBridge::new(config_all, "http://localhost:8080".to_string());

        assert!(bridge_all.should_create_snapshot(0));
        assert!(bridge_all.should_create_snapshot(1));
    }
}
