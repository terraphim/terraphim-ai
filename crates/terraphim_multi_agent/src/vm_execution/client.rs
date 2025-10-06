use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::fcctl_bridge::FcctlBridge;
use super::hooks::*;
use super::models::*;
use crate::MultiAgentError;

/// HTTP client for communicating with fcctl-web VM execution API
#[derive(Clone)]
pub struct VmExecutionClient {
    /// HTTP client
    client: Client,
    /// Base URL for the fcctl-web API
    base_url: String,
    /// Default timeout for requests
    timeout: Duration,
    /// Authentication token (if required)
    auth_token: Option<String>,
    /// History bridge (if history tracking is enabled)
    history_bridge: Option<Arc<FcctlBridge>>,
    /// History configuration
    history_config: HistoryConfig,
    /// Hook manager for pre/post processing
    hook_manager: Arc<HookManager>,
}

impl VmExecutionClient {
    /// Create a new VM execution client
    pub fn new(config: &VmExecutionConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.execution_timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        let history_bridge = if config.history.enabled {
            Some(Arc::new(FcctlBridge::new(
                config.history.clone(),
                config.api_base_url.clone(),
            )))
        } else {
            None
        };

        let mut hook_manager = HookManager::new();
        hook_manager.add_hook(Arc::new(DangerousPatternHook::new()));
        hook_manager.add_hook(Arc::new(SyntaxValidationHook::new()));
        hook_manager.add_hook(Arc::new(ExecutionLoggerHook));
        hook_manager.add_hook(Arc::new(OutputSanitizerHook));

        Self {
            client,
            base_url: config.api_base_url.clone(),
            timeout: Duration::from_millis(config.execution_timeout_ms),
            auth_token: None,
            history_bridge,
            history_config: config.history.clone(),
            hook_manager: Arc::new(hook_manager),
        }
    }

    pub fn with_hook_manager(mut self, hook_manager: Arc<HookManager>) -> Self {
        self.hook_manager = hook_manager;
        self
    }

    /// Set authentication token
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Execute code in a VM
    pub async fn execute_code(
        &self,
        request: VmExecuteRequest,
    ) -> Result<VmExecuteResponse, VmExecutionError> {
        let start_time = std::time::Instant::now();

        let pre_context = PreToolContext {
            code: request.code.clone(),
            language: request.language.clone(),
            agent_id: request.agent_id.clone(),
            vm_id: request
                .vm_id
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            metadata: HashMap::new(),
        };

        let pre_decision = self.hook_manager.run_pre_tool(&pre_context).await?;

        let final_code = match pre_decision {
            HookDecision::Block { reason } => {
                return Err(VmExecutionError::ValidationFailed(reason));
            }
            HookDecision::Modify { transformed_code } => {
                info!("Code transformed by hook");
                transformed_code
            }
            HookDecision::AskUser { prompt } => {
                warn!("User confirmation required: {}", prompt);
                request.code.clone()
            }
            HookDecision::Allow => request.code.clone(),
        };

        let final_request = VmExecuteRequest {
            code: final_code,
            ..request
        };

        let url = format!("{}/api/llm/execute", self.base_url);

        debug!(
            "Executing code in VM: language={}, vm_id={:?}",
            final_request.language, final_request.vm_id
        );

        let mut req_builder = self.client.post(&url).json(&final_request);

        if let Some(ref token) = self.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = timeout(self.timeout, req_builder.send())
            .await
            .map_err(|_| VmExecutionError::Timeout(self.timeout.as_millis() as u64))?
            .map_err(|e| VmExecutionError::ApiError(e.to_string()))?;

        if response.status().is_success() {
            let execution_result: VmExecuteResponse = response.json().await.map_err(|e| {
                VmExecutionError::ApiError(format!("Failed to parse response: {}", e))
            })?;

            info!(
                "Code execution completed: execution_id={}, exit_code={}",
                execution_result.execution_id, execution_result.exit_code
            );

            let duration_ms = start_time.elapsed().as_millis() as u64;

            let post_context = PostToolContext {
                original_code: final_request.code.clone(),
                output: format!("{}{}", execution_result.stdout, execution_result.stderr),
                exit_code: execution_result.exit_code,
                duration_ms,
                agent_id: final_request.agent_id.clone(),
                vm_id: execution_result.vm_id.clone(),
            };

            let post_decision = self.hook_manager.run_post_tool(&post_context).await?;

            match post_decision {
                HookDecision::Block { reason } => {
                    warn!("Execution output blocked by hook: {}", reason);
                    return Err(VmExecutionError::ValidationFailed(reason));
                }
                _ => {}
            }

            if let Some(ref bridge) = self.history_bridge {
                if let Err(e) = bridge
                    .track_execution(
                        &execution_result.vm_id,
                        &final_request.agent_id,
                        &final_request,
                        &execution_result,
                    )
                    .await
                {
                    warn!("Failed to track execution in history: {}", e);
                }

                if execution_result.exit_code != 0 && self.history_config.auto_rollback_on_failure {
                    info!(
                        "Execution failed, attempting auto-rollback for VM {}",
                        execution_result.vm_id
                    );
                    if let Err(e) = bridge
                        .auto_rollback_on_failure(&execution_result.vm_id, &final_request.agent_id)
                        .await
                    {
                        error!("Auto-rollback failed: {}", e);
                    }
                }
            }

            Ok(execution_result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("VM execution API error: {}", error_text);
            Err(VmExecutionError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )))
        }
    }

    /// Parse LLM response and potentially execute extracted code
    pub async fn parse_and_execute(
        &self,
        request: ParseExecuteRequest,
    ) -> Result<ParseExecuteResponse, VmExecutionError> {
        let url = format!("{}/api/llm/parse-execute", self.base_url);

        debug!(
            "Parsing LLM response for code execution: auto_execute={}",
            request.auto_execute
        );

        let mut req_builder = self.client.post(&url).json(&request);

        if let Some(ref token) = self.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = timeout(self.timeout, req_builder.send())
            .await
            .map_err(|_| VmExecutionError::Timeout(self.timeout.as_millis() as u64))?
            .map_err(|e| VmExecutionError::ApiError(e.to_string()))?;

        if response.status().is_success() {
            let parse_result: ParseExecuteResponse = response.json().await.map_err(|e| {
                VmExecutionError::ApiError(format!("Failed to parse response: {}", e))
            })?;

            info!(
                "Parse-execute completed: found {} code blocks, {} executions",
                parse_result.code_blocks.len(),
                parse_result.execution_results.len()
            );

            Ok(parse_result)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Parse-execute API error: {}", error_text);
            Err(VmExecutionError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )))
        }
    }

    /// Get available VMs for an agent
    pub async fn get_vm_pool(&self, agent_id: &str) -> Result<VmPoolResponse, VmExecutionError> {
        let url = format!("{}/api/llm/vm-pool/{}", self.base_url, agent_id);

        debug!("Getting VM pool for agent: {}", agent_id);

        let mut req_builder = self.client.get(&url);

        if let Some(ref token) = self.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = timeout(self.timeout, req_builder.send())
            .await
            .map_err(|_| VmExecutionError::Timeout(self.timeout.as_millis() as u64))?
            .map_err(|e| VmExecutionError::ApiError(e.to_string()))?;

        if response.status().is_success() {
            let pool_info: VmPoolResponse = response.json().await.map_err(|e| {
                VmExecutionError::ApiError(format!("Failed to parse response: {}", e))
            })?;

            debug!(
                "Got VM pool: {} available, {} in use",
                pool_info.available_vms.len(),
                pool_info.in_use_vms.len()
            );

            Ok(pool_info)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("VM pool API error: {}", error_text);
            Err(VmExecutionError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )))
        }
    }

    /// Provision a new VM for an agent
    pub async fn provision_vm(
        &self,
        agent_id: &str,
        vm_type: Option<&str>,
    ) -> Result<VmInstance, VmExecutionError> {
        let url = format!("{}/api/vms", self.base_url);

        let vm_type = vm_type.unwrap_or("focal-optimized");
        debug!("Provisioning VM for agent {}: type={}", agent_id, vm_type);

        let request_body = json!({
            "vm_type": vm_type,
            "vm_name": format!("agent-{}-vm", agent_id)
        });

        let mut req_builder = self.client.post(&url).json(&request_body);

        if let Some(ref token) = self.auth_token {
            req_builder = req_builder.bearer_auth(token);
        }

        let response = timeout(self.timeout, req_builder.send())
            .await
            .map_err(|_| VmExecutionError::Timeout(self.timeout.as_millis() as u64))?
            .map_err(|e| VmExecutionError::ApiError(e.to_string()))?;

        if response.status().is_success() {
            let vm_response: serde_json::Value = response.json().await.map_err(|e| {
                VmExecutionError::ApiError(format!("Failed to parse response: {}", e))
            })?;

            let vm_instance = VmInstance {
                id: vm_response["id"].as_str().unwrap_or_default().to_string(),
                name: vm_response["name"].as_str().unwrap_or_default().to_string(),
                vm_type: vm_response["vm_type"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                status: vm_response["status"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                ip_address: None, // Will be populated when VM is ready
                created_at: chrono::Utc::now(),
                last_activity: None,
            };

            info!(
                "VM provisioned successfully: id={}, name={}",
                vm_instance.id, vm_instance.name
            );
            Ok(vm_instance)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("VM provisioning error: {}", error_text);
            Err(VmExecutionError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )))
        }
    }

    /// Wait for VM to be ready
    pub async fn wait_for_vm_ready(
        &self,
        vm_id: &str,
        max_wait_seconds: u64,
    ) -> Result<VmInstance, VmExecutionError> {
        let url = format!("{}/api/vms/{}", self.base_url, vm_id);
        let start_time = std::time::Instant::now();
        let max_duration = Duration::from_secs(max_wait_seconds);

        debug!(
            "Waiting for VM {} to be ready (max wait: {}s)",
            vm_id, max_wait_seconds
        );

        loop {
            if start_time.elapsed() > max_duration {
                return Err(VmExecutionError::Timeout(max_wait_seconds * 1000));
            }

            let mut req_builder = self.client.get(&url);

            if let Some(ref token) = self.auth_token {
                req_builder = req_builder.bearer_auth(token);
            }

            match req_builder.send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(vm_data) = response.json::<serde_json::Value>().await {
                        let status = vm_data["status"].as_str().unwrap_or("unknown");

                        if status == "running" || status == "ready" {
                            let vm_instance = VmInstance {
                                id: vm_data["id"].as_str().unwrap_or_default().to_string(),
                                name: vm_data["name"].as_str().unwrap_or_default().to_string(),
                                vm_type: vm_data["vm_type"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                status: status.to_string(),
                                ip_address: vm_data["ip_address"].as_str().map(|s| s.to_string()),
                                created_at: chrono::Utc::now(),
                                last_activity: Some(chrono::Utc::now()),
                            };

                            info!("VM {} is ready", vm_id);
                            return Ok(vm_instance);
                        } else {
                            debug!("VM {} status: {} (waiting...)", vm_id, status);
                        }
                    }
                }
                Ok(response) => {
                    warn!("VM status check failed: HTTP {}", response.status());
                }
                Err(e) => {
                    warn!("VM status check error: {}", e);
                }
            }

            // Wait 2 seconds before next check
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    /// Health check for the VM execution service
    pub async fn health_check(&self) -> Result<bool, VmExecutionError> {
        let url = format!("{}/health", self.base_url);

        let response = timeout(Duration::from_secs(5), self.client.get(&url).send())
            .await
            .map_err(|_| VmExecutionError::Timeout(5000))?
            .map_err(|e| VmExecutionError::ApiError(e.to_string()))?;

        Ok(response.status().is_success())
    }

    /// Query command history for a VM
    pub async fn query_history(
        &self,
        request: HistoryQueryRequest,
    ) -> Result<HistoryQueryResponse, VmExecutionError> {
        if let Some(ref bridge) = self.history_bridge {
            bridge.query_history(request).await
        } else {
            Err(VmExecutionError::HistoryError(
                "History tracking is not enabled".to_string(),
            ))
        }
    }

    /// Rollback VM to a previous snapshot
    pub async fn rollback_to_snapshot(
        &self,
        request: RollbackRequest,
    ) -> Result<RollbackResponse, VmExecutionError> {
        if let Some(ref bridge) = self.history_bridge {
            bridge.rollback_to_snapshot(request).await
        } else {
            Err(VmExecutionError::HistoryError(
                "History tracking is not enabled".to_string(),
            ))
        }
    }

    /// Get the last successful snapshot for a VM
    pub async fn get_last_successful_snapshot(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Option<String> {
        if let Some(ref bridge) = self.history_bridge {
            bridge.get_last_successful_snapshot(vm_id, agent_id).await
        } else {
            None
        }
    }

    /// Query command history failures only
    pub async fn query_failures(
        &self,
        vm_id: &str,
        agent_id: Option<String>,
        limit: Option<usize>,
    ) -> Result<HistoryQueryResponse, VmExecutionError> {
        let request = HistoryQueryRequest {
            vm_id: vm_id.to_string(),
            agent_id,
            limit,
            failures_only: true,
            start_date: None,
            end_date: None,
        };
        self.query_history(request).await
    }

    /// Quick rollback to last successful state
    pub async fn rollback_to_last_success(
        &self,
        vm_id: &str,
        agent_id: &str,
    ) -> Result<RollbackResponse, VmExecutionError> {
        let snapshot_id = self
            .get_last_successful_snapshot(vm_id, agent_id)
            .await
            .ok_or_else(|| {
                VmExecutionError::SnapshotNotFound("No successful snapshot found".to_string())
            })?;

        let request = RollbackRequest {
            vm_id: vm_id.to_string(),
            snapshot_id,
            create_pre_rollback_snapshot: true,
        };

        self.rollback_to_snapshot(request).await
    }
}

/// Convenience methods for common operations
impl VmExecutionClient {
    /// Execute Python code with automatic VM provisioning
    pub async fn execute_python(
        &self,
        agent_id: &str,
        code: &str,
    ) -> Result<VmExecuteResponse, VmExecutionError> {
        let request = VmExecuteRequest {
            agent_id: agent_id.to_string(),
            language: "python".to_string(),
            code: code.to_string(),
            vm_id: None, // Auto-provision
            requirements: vec![],
            timeout_seconds: Some(30),
            working_dir: None,
            metadata: None,
        };

        self.execute_code(request).await
    }

    /// Execute JavaScript code with automatic VM provisioning
    pub async fn execute_javascript(
        &self,
        agent_id: &str,
        code: &str,
    ) -> Result<VmExecuteResponse, VmExecutionError> {
        let request = VmExecuteRequest {
            agent_id: agent_id.to_string(),
            language: "javascript".to_string(),
            code: code.to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(30),
            working_dir: None,
            metadata: None,
        };

        self.execute_code(request).await
    }

    /// Execute bash command with automatic VM provisioning
    pub async fn execute_bash(
        &self,
        agent_id: &str,
        command: &str,
    ) -> Result<VmExecuteResponse, VmExecutionError> {
        let request = VmExecuteRequest {
            agent_id: agent_id.to_string(),
            language: "bash".to_string(),
            code: command.to_string(),
            vm_id: None,
            requirements: vec![],
            timeout_seconds: Some(30),
            working_dir: None,
            metadata: None,
        };

        self.execute_code(request).await
    }
}

/// Convert VmExecutionError to MultiAgentError
impl From<VmExecutionError> for MultiAgentError {
    fn from(error: VmExecutionError) -> Self {
        MultiAgentError::External(format!("VM execution error: {}", error))
    }
}

impl std::fmt::Debug for VmExecutionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VmExecutionClient")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("has_auth_token", &self.auth_token.is_some())
            .field("has_history_bridge", &self.history_bridge.is_some())
            .field("history_config", &self.history_config)
            .field("hooks_count", &"<hook_manager>")
            .finish()
    }
}
