//! Firecracker VM-based command execution
//!
//! This module provides a `VmCommandExecutor` that bridges the workflow executor
//! to real Firecracker VMs via the VmExecutionClient HTTP API.

use crate::error::{GitHubRunnerError, Result};
use crate::models::SnapshotId;
use crate::session::Session;
use crate::workflow::executor::{CommandExecutor, CommandResult};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Command executor that uses Firecracker VMs via HTTP API
pub struct VmCommandExecutor {
    /// Base URL for the fcctl-web API
    api_base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Snapshot counter
    snapshot_counter: AtomicU64,
    /// JWT auth token for API authentication
    auth_token: Option<String>,
}

impl VmCommandExecutor {
    /// Create a new VM command executor
    ///
    /// # Arguments
    /// * `api_base_url` - Base URL for the fcctl-web API (e.g., "http://localhost:8080")
    pub fn new(api_base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        // Try to get auth token from environment
        let auth_token = std::env::var("FIRECRACKER_AUTH_TOKEN").ok();

        Self {
            api_base_url: api_base_url.into(),
            client,
            snapshot_counter: AtomicU64::new(0),
            auth_token,
        }
    }

    /// Create a new VM command executor with authentication
    pub fn with_auth(api_base_url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            api_base_url: api_base_url.into(),
            client,
            snapshot_counter: AtomicU64::new(0),
            auth_token: Some(auth_token.into()),
        }
    }

    /// Build the execute endpoint URL
    fn execute_url(&self) -> String {
        format!("{}/api/llm/execute", self.api_base_url)
    }

    /// Build the snapshot endpoint URL
    fn snapshot_url(&self, vm_id: &str) -> String {
        format!("{}/api/vms/{}/snapshots", self.api_base_url, vm_id)
    }

    /// Build the rollback endpoint URL
    fn rollback_url(&self, vm_id: &str, snapshot_id: &str) -> String {
        format!(
            "{}/api/vms/{}/rollback/{}",
            self.api_base_url, vm_id, snapshot_id
        )
    }
}

#[async_trait]
impl CommandExecutor for VmCommandExecutor {
    async fn execute(
        &self,
        session: &Session,
        command: &str,
        timeout: Duration,
        working_dir: &str,
    ) -> Result<CommandResult> {
        info!(
            "Executing command in Firecracker VM {}: {}",
            session.vm_id, command
        );

        let start = std::time::Instant::now();

        // Build request payload
        let payload = serde_json::json!({
            "agent_id": format!("workflow-executor-{}", session.id),
            "language": "bash",
            "code": command,
            "vm_id": session.vm_id,
            "timeout_seconds": timeout.as_secs(),
            "working_dir": working_dir,
        });

        // Send request to fcctl-web API with optional auth
        let mut request = self.client.post(self.execute_url()).json(&payload);

        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| GitHubRunnerError::ExecutionFailed {
                command: command.to_string(),
                reason: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| GitHubRunnerError::ExecutionFailed {
                    command: command.to_string(),
                    reason: format!("Failed to parse response: {}", e),
                })?;

        let duration = start.elapsed();

        if status.is_success() {
            let exit_code = body["exit_code"].as_i64().unwrap_or(0) as i32;
            let stdout = body["stdout"].as_str().unwrap_or("").to_string();
            let stderr = body["stderr"].as_str().unwrap_or("").to_string();

            debug!(
                "Command completed in {:?} with exit code {}",
                duration, exit_code
            );

            Ok(CommandResult {
                exit_code,
                stdout,
                stderr,
                duration,
            })
        } else {
            let error_msg = body["error"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            warn!("Command execution failed: {}", error_msg);

            Ok(CommandResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: error_msg,
                duration,
            })
        }
    }

    async fn create_snapshot(&self, session: &Session, name: &str) -> Result<SnapshotId> {
        info!("Creating snapshot '{}' for VM {}", name, session.vm_id);

        let payload = serde_json::json!({
            "name": name,
            "description": format!("Snapshot after step: {}", name),
        });

        // Send request with optional auth
        let mut request = self
            .client
            .post(self.snapshot_url(&session.vm_id))
            .json(&payload);

        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await.map_err(|e| {
            GitHubRunnerError::SnapshotFailed(format!("Snapshot request failed: {}", e))
        })?;

        if response.status().is_success() {
            let body: serde_json::Value = response.json().await.map_err(|e| {
                GitHubRunnerError::SnapshotFailed(format!(
                    "Failed to parse snapshot response: {}",
                    e
                ))
            })?;

            let snapshot_id = body["snapshot_id"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    format!(
                        "snapshot-{}",
                        self.snapshot_counter.fetch_add(1, Ordering::SeqCst)
                    )
                });

            info!("Created snapshot: {}", snapshot_id);
            Ok(SnapshotId(snapshot_id))
        } else {
            Err(GitHubRunnerError::SnapshotFailed(format!(
                "Snapshot creation failed with status: {}",
                response.status()
            )))
        }
    }

    async fn rollback(&self, session: &Session, snapshot_id: &SnapshotId) -> Result<()> {
        info!(
            "Rolling back VM {} to snapshot {}",
            session.vm_id, snapshot_id.0
        );

        // Send request with optional auth
        let mut request = self
            .client
            .post(self.rollback_url(&session.vm_id, &snapshot_id.0));

        if let Some(ref token) = self.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| GitHubRunnerError::RollbackFailed {
                snapshot_id: snapshot_id.0.clone(),
                reason: format!("Rollback request failed: {}", e),
            })?;

        if response.status().is_success() {
            info!("Successfully rolled back to snapshot {}", snapshot_id.0);
            Ok(())
        } else {
            Err(GitHubRunnerError::RollbackFailed {
                snapshot_id: snapshot_id.0.clone(),
                reason: format!("Rollback failed with status: {}", response.status()),
            })
        }
    }
}

/// Simulated VM executor for demonstration without real Firecracker
///
/// This executor simulates Firecracker VM execution by logging commands
/// and returning mock results. Useful for testing and demonstration.
pub struct SimulatedVmExecutor {
    /// Execution delay to simulate VM processing
    pub execution_delay: Duration,
    /// Commands that should fail (for testing)
    pub failing_commands: Vec<String>,
    /// Snapshot counter
    snapshot_counter: AtomicU64,
    /// Execution log
    execution_log: std::sync::Mutex<Vec<ExecutionLogEntry>>,
}

/// Log entry for simulated execution
#[derive(Debug, Clone)]
pub struct ExecutionLogEntry {
    pub vm_id: String,
    pub command: String,
    pub working_dir: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
}

impl Default for SimulatedVmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedVmExecutor {
    pub fn new() -> Self {
        Self {
            execution_delay: Duration::from_millis(100),
            failing_commands: Vec::new(),
            snapshot_counter: AtomicU64::new(0),
            execution_log: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Create an executor with specific failing commands
    pub fn with_failing_commands(failing_commands: Vec<String>) -> Self {
        Self {
            failing_commands,
            ..Self::new()
        }
    }

    /// Get the execution log
    pub fn get_log(&self) -> Vec<ExecutionLogEntry> {
        self.execution_log.lock().unwrap().clone()
    }
}

#[async_trait]
impl CommandExecutor for SimulatedVmExecutor {
    async fn execute(
        &self,
        session: &Session,
        command: &str,
        _timeout: Duration,
        working_dir: &str,
    ) -> Result<CommandResult> {
        info!(
            "[SIMULATED FIRECRACKER] VM {} executing: {}",
            session.vm_id, command
        );

        // Simulate execution delay
        tokio::time::sleep(self.execution_delay).await;

        let should_fail = self.failing_commands.iter().any(|c| command.contains(c));

        // Log the execution
        {
            let mut log = self.execution_log.lock().unwrap();
            log.push(ExecutionLogEntry {
                vm_id: session.vm_id.clone(),
                command: command.to_string(),
                working_dir: working_dir.to_string(),
                timestamp: chrono::Utc::now(),
                success: !should_fail,
            });
        }

        if should_fail {
            info!(
                "[SIMULATED FIRECRACKER] Command failed (configured to fail): {}",
                command
            );
            Ok(CommandResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: format!("Simulated failure for: {}", command),
                duration: self.execution_delay,
            })
        } else {
            info!("[SIMULATED FIRECRACKER] Command succeeded: {}", command);
            Ok(CommandResult {
                exit_code: 0,
                stdout: format!("Simulated output from Firecracker VM for: {}", command),
                stderr: String::new(),
                duration: self.execution_delay,
            })
        }
    }

    async fn create_snapshot(&self, session: &Session, name: &str) -> Result<SnapshotId> {
        let snapshot_id = format!(
            "fc-snapshot-{}-{}",
            session.vm_id,
            self.snapshot_counter.fetch_add(1, Ordering::SeqCst)
        );

        info!(
            "[SIMULATED FIRECRACKER] Created snapshot '{}' -> {}",
            name, snapshot_id
        );

        Ok(SnapshotId(snapshot_id))
    }

    async fn rollback(&self, session: &Session, snapshot_id: &SnapshotId) -> Result<()> {
        info!(
            "[SIMULATED FIRECRACKER] Rolled back VM {} to snapshot {}",
            session.vm_id, snapshot_id.0
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SessionId;
    use crate::session::SessionState;

    fn create_test_session() -> Session {
        Session {
            id: SessionId::new(),
            vm_id: "test-firecracker-vm".to_string(),
            vm_type: "terraphim-minimal".to_string(),
            started_at: chrono::Utc::now(),
            state: SessionState::Active,
            snapshots: Vec::new(),
            last_activity: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_simulated_executor_success() {
        let executor = SimulatedVmExecutor::new();
        let session = create_test_session();

        let result = executor
            .execute(
                &session,
                "cargo build",
                Duration::from_secs(300),
                "/workspace",
            )
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("cargo build"));

        let log = executor.get_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].command, "cargo build");
        assert!(log[0].success);
    }

    #[tokio::test]
    async fn test_simulated_executor_failure() {
        let executor = SimulatedVmExecutor::with_failing_commands(vec!["fail_this".to_string()]);
        let session = create_test_session();

        let result = executor
            .execute(
                &session,
                "fail_this command",
                Duration::from_secs(300),
                "/workspace",
            )
            .await
            .unwrap();

        assert!(!result.success());
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_simulated_snapshot() {
        let executor = SimulatedVmExecutor::new();
        let session = create_test_session();

        let snapshot_id = executor
            .create_snapshot(&session, "after-build")
            .await
            .unwrap();

        assert!(snapshot_id.0.contains("fc-snapshot"));
        assert!(snapshot_id.0.contains(&session.vm_id));
    }

    #[tokio::test]
    async fn test_simulated_rollback() {
        let executor = SimulatedVmExecutor::new();
        let session = create_test_session();

        let snapshot_id = SnapshotId("test-snapshot-123".to_string());
        let result = executor.rollback(&session, &snapshot_id).await;

        assert!(result.is_ok());
    }
}
