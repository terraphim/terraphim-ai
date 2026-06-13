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
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder};
use dashmap::DashMap;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::{Capability, ExecutionContext, ExecutionResult, SnapshotId, ValidationResult};
use crate::config::{BackendType, KgStrictness, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

const DEFAULT_IMAGE: &str = "python:3.11-slim";
const BACKEND_NAME: &str = "docker";

/// Default container memory limit in bytes (512 MiB).
const DEFAULT_MEMORY_BYTES: i64 = 512 * 1024 * 1024;
/// Default container PIDs limit.
const DEFAULT_PIDS_LIMIT: i64 = 256;

pub struct DockerExecutor {
    docker: Docker,
    /// Per-session container map. Each entry holds a `Mutex<Option<String>>`:
    /// the lock serialises creation for that session, and the inner `Option`
    /// is `None` until the container is created and published.
    session_to_container: DashMap<SessionId, Arc<Mutex<Option<String>>>>,
    image: String,
    /// HostConfig applied to every session container. Defaults to
    /// `default_host_config()` (permissive profile); override per executor
    /// via `with_host_config`.
    host_config: HostConfig,
    capabilities: Vec<Capability>,
    /// Knowledge graph validation strictness level from the RLM config.
    kg_strictness: KgStrictness,
}

/// Build the default `HostConfig` applied to every session container.
///
/// Permissive profile per design decision (2026-05-15):
/// - Memory cap: 512 MiB
/// - PIDs cap: 256
/// - All Linux capabilities dropped
/// - Network: `bridge` (outbound allowed for LLM bridge & pip use)
/// - Read-only rootfs: false (Python needs to write to /tmp)
fn default_host_config() -> HostConfig {
    HostConfig {
        memory: Some(DEFAULT_MEMORY_BYTES),
        pids_limit: Some(DEFAULT_PIDS_LIMIT),
        cap_drop: Some(vec!["ALL".to_string()]),
        network_mode: Some("bridge".to_string()),
        readonly_rootfs: Some(false),
        ..Default::default()
    }
}

fn unsupported(op: &'static str) -> RlmError {
    RlmError::NotSupported {
        backend: BACKEND_NAME.to_string(),
        op: op.to_string(),
    }
}

impl DockerExecutor {
    pub fn new(config: RlmConfig) -> Result<Self, RlmError> {
        let docker =
            Docker::connect_with_local_defaults().map_err(|e| RlmError::BackendInitFailed {
                backend: BACKEND_NAME.to_string(),
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
            docker,
            session_to_container: DashMap::new(),
            image: DEFAULT_IMAGE.to_string(),
            host_config: default_host_config(),
            capabilities,
            kg_strictness: config.kg_strictness,
        })
    }

    pub fn with_image(config: RlmConfig, image: &str) -> Result<Self, RlmError> {
        let mut executor = Self::new(config)?;
        executor.image = image.to_string();
        Ok(executor)
    }

    /// Override the per-container `HostConfig` (resource limits, network
    /// mode, capability drops, rootfs read-only flag). Replaces the entire
    /// default profile.
    pub fn with_host_config(mut self, host_config: HostConfig) -> Self {
        self.host_config = host_config;
        self
    }

    async fn ensure_container(&self, session_id: &SessionId) -> RlmResult<String> {
        let entry = self
            .session_to_container
            .entry(*session_id)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone();

        let mut guard = entry.lock().await;
        if let Some(id) = guard.as_ref() {
            return Ok(id.clone());
        }
        let new_id = self.create_container(session_id).await?;
        *guard = Some(new_id.clone());
        Ok(new_id)
    }

    /// Release the container associated with `session_id`, removing it from
    /// Docker and from the internal session map. Returns the container id if
    /// one was bound to this session, or `None` if no container existed.
    ///
    /// Mirrors `FirecrackerExecutor::release_session_vm`. Errors from
    /// `docker.remove_container` are logged but not propagated, so the
    /// session map is always cleaned up even if the daemon is unreachable.
    pub async fn release_session_container(&self, session_id: &SessionId) -> Option<String> {
        let removed = self.session_to_container.remove(session_id)?;
        let id = removed.1.lock().await.take()?;
        if let Err(e) = self.delete_container(&id).await {
            log::warn!(
                "release_session_container({}): failed to remove {}: {}",
                session_id,
                id,
                e
            );
        }
        Some(id)
    }

    async fn create_container(&self, session_id: &SessionId) -> RlmResult<String> {
        let container_name = format!("terraphim-rlm-{}", session_id);

        let config = ContainerCreateBody {
            image: Some(self.image.clone()),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            host_config: Some(self.host_config.clone()),
            ..Default::default()
        };

        let options = CreateContainerOptionsBuilder::new()
            .name(&container_name)
            .build();

        let create_response = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|e| RlmError::BackendInitFailed {
                backend: BACKEND_NAME.to_string(),
                message: format!("Failed to create container: {}", e),
            })?;

        if let Err(e) = self.docker.start_container(&create_response.id, None).await {
            let remove_opts = RemoveContainerOptionsBuilder::new().force(true).build();
            if let Err(remove_err) = self
                .docker
                .remove_container(&create_response.id, Some(remove_opts))
                .await
            {
                log::warn!(
                    "Failed to remove container {} after start failure: {}",
                    create_response.id,
                    remove_err
                );
            }
            return Err(RlmError::BackendInitFailed {
                backend: BACKEND_NAME.to_string(),
                message: format!("Failed to start container: {}", e),
            });
        }

        Ok(create_response.id)
    }

    async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        ctx: &ExecutionContext,
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

        match output {
            Ok(StartExecResults::Attached { mut output, .. }) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                let timeout_duration = Duration::from_millis(ctx.timeout_ms);

                let stream_future = async {
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
                };

                let timed_out = tokio::time::timeout(timeout_duration, stream_future)
                    .await
                    .is_err();

                let execution_time_ms = start.elapsed().as_millis() as u64;

                if timed_out {
                    return Ok(ExecutionResult::timeout(stdout, stderr)
                        .with_execution_time(execution_time_ms));
                }

                let exit_code = self
                    .docker
                    .inspect_exec(&exec.id)
                    .await
                    .ok()
                    .and_then(|inspect| inspect.exit_code)
                    .map(|c| i32::try_from(c).unwrap_or(-1))
                    .unwrap_or(-1);

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
            Ok(StartExecResults::Detached) => {
                let execution_time_ms = start.elapsed().as_millis() as u64;
                Ok(ExecutionResult {
                    stdout: String::new(),
                    stderr: "Exec detached (not captured)".to_string(),
                    exit_code: -1,
                    execution_time_ms,
                    output_truncated: false,
                    output_file_path: None,
                    timed_out: false,
                    metadata: HashMap::new(),
                })
            }
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

    /// Drain all session entries and return their (resolved) container ids.
    /// Used by `cleanup` and `Drop`.
    async fn drain_container_ids(&self) -> Vec<String> {
        let entries: Vec<_> = self
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        // Now empty the map.
        self.session_to_container.clear();

        let mut ids = Vec::with_capacity(entries.len());
        for entry in entries {
            if let Some(id) = entry.lock().await.take() {
                ids.push(id);
            }
        }
        ids
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
        self.exec_in_container(&container_id, cmd, ctx).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        let container_id = self.ensure_container(&ctx.session_id).await?;
        let bash_cmd = vec!["bash", "-c", cmd];
        self.exec_in_container(&container_id, bash_cmd, ctx).await
    }

    async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
        Ok(ValidationResult::valid(vec![]).with_strictness(self.kg_strictness))
    }

    async fn create_snapshot(
        &self,
        _session_id: &SessionId,
        _name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        Err(unsupported("create_snapshot"))
    }

    async fn restore_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("restore_snapshot"))
    }

    async fn list_snapshots(
        &self,
        _session_id: &SessionId,
    ) -> Result<Vec<SnapshotId>, Self::Error> {
        Err(unsupported("list_snapshots"))
    }

    async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
        Err(unsupported("delete_snapshot"))
    }

    async fn delete_session_snapshots(&self, _session_id: &SessionId) -> Result<(), Self::Error> {
        Err(unsupported("delete_session_snapshots"))
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
        let ids = self.drain_container_ids().await;
        let futures: Vec<_> = ids.iter().map(|id| self.delete_container(id)).collect();
        let results = futures::future::join_all(futures).await;
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                log::warn!("Failed to cleanup container {}: {}", ids[i], e);
            }
        }
        Ok(())
    }

    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        let _ = self.release_session_container(session_id).await;
        Ok(())
    }
}

impl Drop for DockerExecutor {
    fn drop(&mut self) {
        // Snapshot the entry pointers so we can drain in the spawned task
        // without holding the DashMap reference here.
        let entries: Vec<_> = self
            .session_to_container
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        self.session_to_container.clear();

        if entries.is_empty() {
            return;
        }

        let docker = self.docker.clone();
        match tokio::runtime::Handle::try_current() {
            Ok(_handle) => {
                tokio::spawn(async move {
                    let mut ids = Vec::with_capacity(entries.len());
                    for entry in entries {
                        if let Some(id) = entry.lock().await.take() {
                            ids.push(id);
                        }
                    }
                    let remove_opts = RemoveContainerOptionsBuilder::new().force(true).build();
                    let futures: Vec<_> = ids
                        .iter()
                        .map(|id| docker.remove_container(id, Some(remove_opts.clone())))
                        .collect();
                    let results = futures::future::join_all(futures).await;
                    for (i, result) in results.into_iter().enumerate() {
                        if let Err(e) = result {
                            log::warn!("Drop: failed to remove container {}: {}", ids[i], e);
                        }
                    }
                });
            }
            Err(_) => {
                log::warn!(
                    "DockerExecutor::drop called outside tokio runtime; {} session entries not cleaned up",
                    entries.len()
                );
            }
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

    /// Container-running tests need the default image cached locally.
    /// We skip rather than auto-pull to keep test latency bounded and
    /// network access optional.
    fn is_default_image_present() -> bool {
        std::process::Command::new("docker")
            .args(["image", "inspect", DEFAULT_IMAGE])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn skip_unless_image_ready(test_name: &str) -> bool {
        if !is_docker_available() {
            eprintln!("Skipping {}: Docker not available", test_name);
            return false;
        }
        if !is_default_image_present() {
            eprintln!(
                "Skipping {}: image {} not present locally (run `docker pull {}` to enable)",
                test_name, DEFAULT_IMAGE, DEFAULT_IMAGE
            );
            return false;
        }
        true
    }

    #[test]
    fn test_with_host_config_overrides_default() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }
        let strict = HostConfig {
            memory: Some(64 * 1024 * 1024),
            pids_limit: Some(32),
            cap_drop: Some(vec!["ALL".to_string()]),
            network_mode: Some("none".to_string()),
            readonly_rootfs: Some(true),
            ..Default::default()
        };
        let exec = DockerExecutor::new(RlmConfig::minimal())
            .unwrap()
            .with_host_config(strict.clone());
        assert_eq!(exec.host_config.memory, strict.memory);
        assert_eq!(exec.host_config.network_mode, strict.network_mode);
        assert_eq!(exec.host_config.readonly_rootfs, strict.readonly_rootfs);
    }

    #[test]
    fn test_default_host_config_permissive_profile() {
        // Verify the design-decision values are wired into HostConfig.
        let hc = default_host_config();
        assert_eq!(hc.memory, Some(DEFAULT_MEMORY_BYTES));
        assert_eq!(hc.pids_limit, Some(DEFAULT_PIDS_LIMIT));
        assert_eq!(hc.cap_drop.as_deref(), Some(&["ALL".to_string()][..]));
        assert_eq!(hc.network_mode.as_deref(), Some("bridge"));
        assert_eq!(hc.readonly_rootfs, Some(false));
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

    #[tokio::test]
    async fn test_docker_snapshot_returns_not_supported() {
        // Snapshot ops do not require a running Docker daemon - they're pure
        // returns.
        let cfg = RlmConfig::minimal();
        // We cannot construct a DockerExecutor without a daemon, so gate.
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }
        let exec = DockerExecutor::new(cfg).unwrap();
        let session = SessionId::new();

        assert!(matches!(
            exec.create_snapshot(&session, "x").await,
            Err(RlmError::NotSupported { .. })
        ));
        assert!(matches!(
            exec.list_snapshots(&session).await,
            Err(RlmError::NotSupported { .. })
        ));
    }

    #[tokio::test]
    async fn test_docker_release_session_container_unknown_returns_none() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }
        let exec = DockerExecutor::new(RlmConfig::minimal()).unwrap();
        let unknown = SessionId::new();
        assert!(exec.release_session_container(&unknown).await.is_none());
    }

    #[tokio::test]
    async fn test_docker_release_session_container_removes() {
        if !skip_unless_image_ready("test_docker_release_session_container_removes") {
            return;
        }
        let exec = DockerExecutor::new(RlmConfig::minimal()).unwrap();
        let ctx = ExecutionContext {
            session_id: SessionId::new(),
            timeout_ms: 30_000,
            ..Default::default()
        };

        let result = exec.execute_command("echo hi", &ctx).await.unwrap();
        assert!(result.is_success(), "echo should succeed: {:?}", result);

        let released = exec.release_session_container(&ctx.session_id).await;
        assert!(released.is_some(), "expected a container to release");

        // Subsequent op should create a fresh container, not error.
        let result2 = exec.execute_command("echo again", &ctx).await.unwrap();
        assert!(result2.is_success());

        let _ = exec.release_session_container(&ctx.session_id).await;
    }

    #[tokio::test]
    async fn test_docker_concurrent_ensure_no_leak() {
        if !skip_unless_image_ready("test_docker_concurrent_ensure_no_leak") {
            return;
        }
        let exec = std::sync::Arc::new(DockerExecutor::new(RlmConfig::minimal()).unwrap());
        let session_id = SessionId::new();

        // Fire 8 concurrent calls with the same session id.
        let mut handles = Vec::new();
        for _ in 0..8 {
            let exec = exec.clone();
            let sid = session_id;
            handles.push(tokio::spawn(
                async move { exec.ensure_container(&sid).await },
            ));
        }
        let results: Vec<_> = futures::future::join_all(handles).await;
        let ids: Vec<String> = results.into_iter().map(|r| r.unwrap().unwrap()).collect();

        // All callers must see the same container id.
        let first = ids[0].clone();
        assert!(ids.iter().all(|id| id == &first));

        // Cleanup.
        let _ = exec.release_session_container(&session_id).await;
    }
}
