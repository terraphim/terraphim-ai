//! Docker execution backend using container isolation.
//!
//! This module provides the `DockerExecutor` which implements the
//! `ExecutionEnvironment` trait using Docker containers for isolation.
//!
//! ## Features
//!
//! - Container isolation (PID, NET, IPC, Mount namespaces)
//! - Session-to-container affinity (one container per session)
//! - Python and bash execution via `docker exec`
//! - Automatic container cleanup on session end
//!
//! ## Requirements
//!
//! - Docker daemon running and accessible
//! - `bollard` crate available (via `docker-backend` feature)

use async_trait::async_trait;
use bollard::Docker;
use bollard::container::LogOutput;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::{CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder};
use futures::StreamExt;
use std::collections::HashMap;
use std::time::Instant;

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

const DEFAULT_IMAGE: &str = "python:3.11-slim";

#[allow(dead_code)]
pub struct DockerExecutor {
    config: RlmConfig,
    docker: Docker,
    session_to_container: parking_lot::RwLock<HashMap<SessionId, String>>,
    image: String,
    capabilities: Vec<Capability>,
}

impl DockerExecutor {
    pub fn new(config: RlmConfig) -> Result<Self, RlmError> {
        let docker =
            Docker::connect_with_local_defaults().map_err(|e| RlmError::BackendInitFailed {
                backend: "docker".to_string(),
                message: format!(
                    "Failed to connect to Docker daemon: {}. Is Docker running?",
                    e
                ),
            })?;

        let capabilities = vec![
            Capability::ContainerIsolation,
            Capability::PythonExecution,
            Capability::BashExecution,
            Capability::FileOperations,
        ];

        Ok(Self {
            config,
            docker,
            session_to_container: parking_lot::RwLock::new(HashMap::new()),
            image: DEFAULT_IMAGE.to_string(),
            capabilities,
        })
    }

    pub fn with_image(config: RlmConfig, image: &str) -> Result<Self, RlmError> {
        let mut executor = Self::new(config)?;
        executor.image = image.to_string();
        Ok(executor)
    }

    async fn ensure_container(&self, session_id: &SessionId) -> RlmResult<String> {
        if let Some(container_id) = self.session_to_container.read().get(session_id) {
            return Ok(container_id.clone());
        }

        let container_id = self.create_container(session_id).await?;
        self.session_to_container
            .write()
            .insert(*session_id, container_id.clone());

        Ok(container_id)
    }

    async fn create_container(&self, session_id: &SessionId) -> RlmResult<String> {
        let container_name = format!("terraphim-rlm-{}", session_id);

        let config = ContainerCreateBody {
            image: Some(self.image.clone()),
            cmd: Some(vec!["sleep".to_string(), "3600".to_string()]),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::new()
            .name(&container_name)
            .build();

        self.docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| RlmError::BackendInitFailed {
                backend: "docker".to_string(),
                message: format!("Failed to create container: {}", e),
            })
            .map(|c| c.id)
    }

    async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
    ) -> RlmResult<ExecutionResult> {
        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(container_id, exec_config)
            .await
            .map_err(|e| RlmError::ExecutionFailed {
                message: format!("Failed to create exec: {}", e),
                exit_code: None,
                stdout: None,
                stderr: None,
            })?;

        let start = Instant::now();

        let start_options = StartExecOptions {
            ..Default::default()
        };

        let output = self.docker.start_exec(&exec.id, Some(start_options)).await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match output {
            Ok(StartExecResults::Attached { mut output, .. }) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                let exit_code = 0;

                while let Some(Ok(msg)) = output.next().await {
                    match msg {
                        LogOutput::StdOut { message } => {
                            stdout.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdErr { message } => {
                            stderr.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::Console { message } => {
                            stdout.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdIn { .. } => {}
                    }
                }

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms,
                    output_truncated: false,
                    output_file_path: None,
                    timed_out: false,
                    metadata: HashMap::new(),
                })
            }
            Ok(StartExecResults::Detached) => Ok(ExecutionResult {
                stdout: String::new(),
                stderr: "Exec detached (not captured)".to_string(),
                exit_code: -1,
                execution_time_ms,
                output_truncated: false,
                output_file_path: None,
                timed_out: false,
                metadata: HashMap::new(),
            }),
            Err(e) => Err(RlmError::ExecutionFailed {
                message: format!("Exec failed: {}", e),
                exit_code: None,
                stdout: None,
                stderr: None,
            }),
        }
    }

    async fn delete_container(&self, container_id: &str) -> RlmResult<()> {
        let options = RemoveContainerOptionsBuilder::new().force(true).build();

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .map_err(|e| RlmError::Internal {
                message: format!("Failed to remove container {}: {}", container_id, e),
            })
    }
}

#[async_trait]
impl super::ExecutionEnvironment for DockerExecutor {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let container_id = self.ensure_container(&ctx.session_id).await?;
        let cmd = vec!["python3", "-c", code];
        self.exec_in_container(&container_id, cmd).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let container_id = self.ensure_container(&ctx.session_id).await?;
        let bash_cmd = vec!["bash", "-c", cmd];
        self.exec_in_container(&container_id, bash_cmd).await
    }

    async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult::valid(vec![]))
    }

    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Ok(SnapshotId::new(name, *session_id))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn list_snapshots(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<SnapshotId>, Self::Error> {
        Ok(vec![])
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), Self::Error> {
        Ok(())
    }

    fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Docker
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        match self.docker.ping().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        let containers: Vec<_> = self
            .session_to_container
            .write()
            .drain()
            .map(|(_, id)| id)
            .collect();

        for container_id in containers {
            if let Err(e) = self.delete_container(&container_id).await {
                log::warn!("Failed to cleanup container {}: {}", container_id, e);
            }
        }

        Ok(())
    }
}

impl Drop for DockerExecutor {
    fn drop(&mut self) {
        let containers: Vec<_> = self
            .session_to_container
            .write()
            .drain()
            .map(|(_, id)| id)
            .collect();

        for container_id in containers {
            let docker = self.docker.clone();
            let container_id = container_id.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new();
                if let Ok(rt) = rt {
                    rt.block_on(async {
                        let options = RemoveContainerOptionsBuilder::new().force(true).build();
                        let _ = docker.remove_container(&container_id, Some(options)).await;
                    });
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEnvironment;

    fn is_docker_available() -> bool {
        std::process::Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn test_docker_executor_requires_docker() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let config = RlmConfig::minimal();
        let executor = DockerExecutor::new(config);
        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_docker_executor_capabilities() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let config = RlmConfig::minimal();
        let executor = DockerExecutor::new(config).unwrap();

        assert!(executor.has_capability(Capability::ContainerIsolation));
        assert!(executor.has_capability(Capability::PythonExecution));
        assert!(executor.has_capability(Capability::BashExecution));
        assert!(!executor.has_capability(Capability::VmIsolation));
    }

    #[tokio::test]
    async fn test_docker_executor_health_check() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let config = RlmConfig::minimal();
        let executor = DockerExecutor::new(config).unwrap();
        let result = executor.health_check().await.unwrap();
        assert!(result);
    }
}
