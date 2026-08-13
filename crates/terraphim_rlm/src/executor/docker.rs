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
use bollard::models::{
    ContainerCreateBody, HostConfig, Mount, MountBindOptions, MountBindOptionsPropagationEnum,
    MountTmpfsOptions, MountType,
};
use bollard::query_parameters::{CreateContainerOptionsBuilder, RemoveContainerOptionsBuilder};
use dashmap::DashMap;
use futures::StreamExt;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::{
    Capability, ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId,
    ValidationResult,
};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::types::SessionId;

const DEFAULT_IMAGE: &str = "python:3.11-slim";
const BACKEND_NAME: &str = "docker";

/// Default container memory limit in bytes (512 MiB).
const DEFAULT_MEMORY_BYTES: i64 = 512 * 1024 * 1024;
/// Default container PIDs limit.
const DEFAULT_PIDS_LIMIT: i64 = 256;

/// Strict diagnostics container memory limit in bytes (256 MiB).
const STRICT_DIAGNOSTICS_MEMORY_BYTES: i64 = 256 * 1024 * 1024;
/// Strict diagnostics PIDs limit.
const STRICT_DIAGNOSTICS_PIDS_LIMIT: i64 = 64;
/// Strict diagnostics tmpfs scratch size in bytes (64 MiB).
const STRICT_DIAGNOSTICS_TMPFS_BYTES: i64 = 64 * 1024 * 1024;

const STRICT_CHECKOUT_TARGET: &str = "/workspace";
const STRICT_TMP_TARGET: &str = "/tmp";

/// Executes code in Docker containers, providing namespace-level isolation.
pub struct DockerExecutor {
    docker: Docker,
    session_to_container: DashMap<SessionId, Arc<Mutex<Option<String>>>>,
    image: String,
    host_config: HostConfig,
    capabilities: Vec<Capability>,
    validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
}

/// Validated strict Docker diagnostics profile.
///
/// This private profile validates the checkout path up front and produces a
/// locked-down typed Docker `HostConfig` without exposing arbitrary host config
/// mutation to public strict sandbox callers.
#[derive(Clone, Eq, PartialEq)]
struct StrictDockerDiagnosticsProfile {
    checkout_path: PathBuf,
}

impl fmt::Debug for StrictDockerDiagnosticsProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StrictDockerDiagnosticsProfile")
            .field("checkout_path", &"<redacted>")
            .field("checkout_target", &STRICT_CHECKOUT_TARGET)
            .field("memory_bytes", &STRICT_DIAGNOSTICS_MEMORY_BYTES)
            .field("pids_limit", &STRICT_DIAGNOSTICS_PIDS_LIMIT)
            .field("tmpfs_bytes", &STRICT_DIAGNOSTICS_TMPFS_BYTES)
            .finish()
    }
}

/// Strict diagnostics profile validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
enum StrictDockerDiagnosticsProfileError {
    /// Checkout path was not an existing non-root directory after
    /// canonicalization.
    #[error("checkout path must be an existing non-root directory")]
    InvalidCheckout,
}

impl StrictDockerDiagnosticsProfile {
    /// Validate and construct a strict Docker diagnostics profile.
    ///
    /// # Errors
    ///
    /// Returns [`StrictDockerDiagnosticsProfileError::InvalidCheckout`] if the
    /// checkout path does not exist, is not a directory, cannot be
    /// canonicalized, or resolves to the filesystem root.
    fn new(checkout_path: impl AsRef<Path>) -> Result<Self, StrictDockerDiagnosticsProfileError> {
        let checkout_path = checkout_path
            .as_ref()
            .canonicalize()
            .map_err(|_| StrictDockerDiagnosticsProfileError::InvalidCheckout)?;

        if checkout_path.parent().is_none()
            || !checkout_path.is_dir()
            || checkout_path.to_str().is_none()
        {
            return Err(StrictDockerDiagnosticsProfileError::InvalidCheckout);
        }

        Ok(Self { checkout_path })
    }

    /// Build the typed locked-down Docker host configuration.
    fn host_config(&self) -> HostConfig {
        strict_diagnostics_host_config(&self.checkout_path)
    }
}

/// Fail-closed strict Docker sandbox construction errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum StrictDockerSandboxError {
    /// Checkout path did not satisfy strict mount prerequisites.
    #[error("strict Docker checkout mount prerequisite failed")]
    InvalidCheckout,
    /// Docker backend construction failed.
    #[error("strict Docker backend initialization failed")]
    BackendInit,
    /// Docker backend was constructed but did not pass the required daemon
    /// health check.
    #[error("strict Docker backend health check failed")]
    DockerUnhealthy,
    /// Constructor did not produce the expected Docker backend.
    #[error("strict sandbox constructor did not produce Docker backend")]
    NonDockerBackend,
}

/// Opaque strict Docker-only diagnostics sandbox.
///
/// The inner Docker executor is intentionally private. Callers can use the
/// [`ExecutionEnvironment`] contract but cannot mutate Docker host
/// configuration or extract the raw executor.
pub struct StrictDockerDiagnosticsSandbox {
    inner: DockerExecutor,
}

impl fmt::Debug for StrictDockerDiagnosticsSandbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StrictDockerDiagnosticsSandbox { backend: Docker }")
    }
}

/// Construct the strict Docker-only diagnostics sandbox.
///
/// This factory instantiates Docker directly and does not use backend selection
/// or fallback. If Docker cannot be constructed or fails its daemon health
/// check, the error is returned and no local executor fallback is available.
///
/// # Errors
///
/// Returns an error when the checkout profile is invalid, Docker construction
/// fails, or the constructed backend is not Docker.
pub async fn strict_docker_diagnostics_sandbox(
    checkout_path: impl AsRef<Path>,
) -> Result<StrictDockerDiagnosticsSandbox, StrictDockerSandboxError> {
    let profile = StrictDockerDiagnosticsProfile::new(checkout_path)
        .map_err(|_| StrictDockerSandboxError::InvalidCheckout)?;
    let executor = DockerExecutor::new_strict_diagnostics(profile, None)
        .map_err(|_| StrictDockerSandboxError::BackendInit)?;
    ensure_strict_docker_healthy(&executor).await?;
    if executor.backend_type() != BackendType::Docker {
        return Err(StrictDockerSandboxError::NonDockerBackend);
    }
    Ok(StrictDockerDiagnosticsSandbox { inner: executor })
}

async fn ensure_strict_docker_healthy<E>(executor: &E) -> Result<(), StrictDockerSandboxError>
where
    E: ExecutionEnvironment<Error = RlmError>,
{
    match executor.health_check().await {
        Ok(true) => Ok(()),
        Ok(false) | Err(_) => Err(StrictDockerSandboxError::DockerUnhealthy),
    }
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

fn strict_diagnostics_host_config(checkout_path: &Path) -> HostConfig {
    HostConfig {
        memory: Some(STRICT_DIAGNOSTICS_MEMORY_BYTES),
        pids_limit: Some(STRICT_DIAGNOSTICS_PIDS_LIMIT),
        cap_drop: Some(vec!["ALL".to_string()]),
        cap_add: None,
        network_mode: Some("none".to_string()),
        readonly_rootfs: Some(true),
        privileged: Some(false),
        devices: None,
        device_cgroup_rules: None,
        device_requests: None,
        binds: None,
        volumes_from: None,
        mounts: Some(vec![
            Mount {
                target: Some(STRICT_TMP_TARGET.to_string()),
                source: None,
                typ: Some(MountType::TMPFS),
                read_only: Some(false),
                tmpfs_options: Some(MountTmpfsOptions {
                    size_bytes: Some(STRICT_DIAGNOSTICS_TMPFS_BYTES),
                    mode: None,
                    options: Some(vec![
                        vec!["noexec".to_string()],
                        vec!["nosuid".to_string()],
                        vec!["nodev".to_string()],
                    ]),
                }),
                ..Default::default()
            },
            Mount {
                target: Some(STRICT_CHECKOUT_TARGET.to_string()),
                source: Some(
                    checkout_path
                        .to_str()
                        .expect("strict profile rejects non-UTF-8 paths")
                        .to_string(),
                ),
                typ: Some(MountType::BIND),
                read_only: Some(true),
                bind_options: Some(MountBindOptions {
                    propagation: Some(MountBindOptionsPropagationEnum::RPRIVATE),
                    create_mountpoint: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ]),
        ..Default::default()
    }
}

fn unsupported(op: &'static str) -> RlmError {
    RlmError::NotSupported {
        backend: BACKEND_NAME.to_string(),
        op: op.to_string(),
    }
}

struct BoundedOutputAccumulator {
    stdout: String,
    stderr: String,
    output_truncated: bool,
    max_output_bytes: usize,
}

impl BoundedOutputAccumulator {
    fn new(max_output_bytes: u64) -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            output_truncated: false,
            max_output_bytes: usize::try_from(max_output_bytes).unwrap_or(usize::MAX),
        }
    }

    fn append_stdout(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.append_text(OutputChannel::Stdout, &text);
    }

    fn append_stderr(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.append_text(OutputChannel::Stderr, &text);
    }

    fn append_text(&mut self, channel: OutputChannel, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.output_truncated {
            return;
        }

        let mut retained_any = false;
        for ch in text.chars() {
            let ch_len = ch.len_utf8();
            if self.retained_bytes().saturating_add(ch_len) > self.max_output_bytes {
                self.output_truncated = true;
                break;
            }

            match channel {
                OutputChannel::Stdout => self.stdout.push(ch),
                OutputChannel::Stderr => self.stderr.push(ch),
            }
            retained_any = true;
        }

        if !retained_any || self.retained_bytes() < text.len() {
            self.output_truncated = true;
        }
    }

    fn retained_bytes(&self) -> usize {
        self.stdout.len().saturating_add(self.stderr.len())
    }

    fn finish(self) -> (String, String, bool, Option<String>) {
        (self.stdout, self.stderr, self.output_truncated, None)
    }
}

#[derive(Clone, Copy)]
enum OutputChannel {
    Stdout,
    Stderr,
}

impl DockerExecutor {
    /// Connect to the local Docker daemon and build a `DockerExecutor` with the
    /// default image and host configuration.
    pub fn new(
        _config: RlmConfig,
        validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
    ) -> Result<Self, RlmError> {
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
            validator,
        })
    }

    /// Build a `DockerExecutor` using a non-default container image.
    pub fn with_image(config: RlmConfig, image: &str) -> Result<Self, RlmError> {
        let mut executor = Self::new(config, None)?;
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

    /// Connect to Docker and build an executor with the strict diagnostics
    /// profile. This constructor does not consult backend fallback selection.
    fn new_strict_diagnostics(
        profile: StrictDockerDiagnosticsProfile,
        validator: Option<Arc<crate::validator::KnowledgeGraphValidator>>,
    ) -> Result<Self, RlmError> {
        let mut executor = Self::new(RlmConfig::minimal(), validator)?;
        executor.host_config = profile.host_config();
        Ok(executor)
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

        let config = self.container_create_body();

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

    fn container_create_body(&self) -> ContainerCreateBody {
        ContainerCreateBody {
            image: Some(self.image.clone()),
            cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            host_config: Some(self.host_config.clone()),
            ..Default::default()
        }
    }

    #[cfg(test)]
    fn strict_container_create_body_for_test(
        profile: &StrictDockerDiagnosticsProfile,
    ) -> ContainerCreateBody {
        Self {
            docker: Docker::connect_with_local_defaults()
                .expect("constructing Docker client handle should not contact daemon"),
            session_to_container: DashMap::new(),
            image: DEFAULT_IMAGE.to_string(),
            host_config: profile.host_config(),
            capabilities: Vec::new(),
            validator: None,
        }
        .container_create_body()
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
                let mut output_accumulator = BoundedOutputAccumulator::new(ctx.max_output_bytes);
                let timeout_duration = Duration::from_millis(ctx.timeout_ms);

                let stream_future = async {
                    while let Some(Ok(msg)) = output.next().await {
                        match msg {
                            LogOutput::StdOut { message } => {
                                output_accumulator.append_stdout(&message);
                            }
                            LogOutput::StdErr { message } => {
                                output_accumulator.append_stderr(&message);
                            }
                            LogOutput::Console { message } => {
                                output_accumulator.append_stdout(&message);
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
                    let (stdout, stderr, output_truncated, output_file_path) =
                        output_accumulator.finish();
                    let mut result = ExecutionResult::timeout(stdout, stderr)
                        .with_execution_time(execution_time_ms);
                    result.output_truncated = output_truncated;
                    result.output_file_path = output_file_path;
                    return Ok(result);
                }

                let exit_code = self
                    .docker
                    .inspect_exec(&exec.id)
                    .await
                    .ok()
                    .and_then(|inspect| inspect.exit_code)
                    .map(|c| i32::try_from(c).unwrap_or(-1))
                    .unwrap_or(-1);

                let (stdout, stderr, output_truncated, output_file_path) =
                    output_accumulator.finish();

                Ok(ExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    execution_time_ms,
                    output_truncated,
                    output_file_path,
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

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        match self.validator.as_ref() {
            Some(validator) if !input.trim().is_empty() => {
                let vr = validator.validate(input)?;
                Ok(ValidationResult::from_validator_result(
                    &vr,
                    crate::config::KgStrictness::Normal,
                ))
            }
            _ => Ok(ValidationResult::valid(Vec::new())),
        }
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

#[async_trait]
impl ExecutionEnvironment for StrictDockerDiagnosticsSandbox {
    type Error = RlmError;

    async fn execute_code(
        &self,
        code: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.inner.execute_code(code, ctx).await
    }

    async fn execute_command(
        &self,
        cmd: &str,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionResult, Self::Error> {
        self.inner.execute_command(cmd, ctx).await
    }

    async fn validate(&self, input: &str) -> Result<ValidationResult, Self::Error> {
        self.inner.validate(input).await
    }

    async fn create_snapshot(
        &self,
        session_id: &SessionId,
        name: &str,
    ) -> Result<SnapshotId, Self::Error> {
        self.inner.create_snapshot(session_id, name).await
    }

    async fn restore_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        self.inner.restore_snapshot(id).await
    }

    async fn list_snapshots(&self, session_id: &SessionId) -> Result<Vec<SnapshotId>, Self::Error> {
        self.inner.list_snapshots(session_id).await
    }

    async fn delete_snapshot(&self, id: &SnapshotId) -> Result<(), Self::Error> {
        self.inner.delete_snapshot(id).await
    }

    async fn delete_session_snapshots(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        self.inner.delete_session_snapshots(session_id).await
    }

    fn capabilities(&self) -> &[Capability] {
        self.inner.capabilities()
    }

    fn backend_type(&self) -> BackendType {
        self.inner.backend_type()
    }

    async fn health_check(&self) -> Result<bool, Self::Error> {
        self.inner.health_check().await
    }

    async fn cleanup(&self) -> Result<(), Self::Error> {
        self.inner.cleanup().await
    }

    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        self.inner.end_session(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEnvironment;
    use bollard::models::{MountBindOptionsPropagationEnum, MountType};
    use std::error::Error;
    use std::fs;

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
        let exec = DockerExecutor::new(RlmConfig::minimal(), None)
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

    struct FakeHealthExecutor {
        result: Result<bool, RlmError>,
    }

    #[async_trait]
    impl ExecutionEnvironment for FakeHealthExecutor {
        type Error = RlmError;

        async fn execute_code(
            &self,
            _code: &str,
            _ctx: &ExecutionContext,
        ) -> Result<ExecutionResult, Self::Error> {
            Ok(ExecutionResult::success(""))
        }

        async fn execute_command(
            &self,
            _cmd: &str,
            _ctx: &ExecutionContext,
        ) -> Result<ExecutionResult, Self::Error> {
            Ok(ExecutionResult::success(""))
        }

        async fn validate(&self, _input: &str) -> Result<ValidationResult, Self::Error> {
            Ok(ValidationResult::valid(Vec::new()))
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
            Ok(Vec::new())
        }

        async fn delete_snapshot(&self, _id: &SnapshotId) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn delete_session_snapshots(
            &self,
            _session_id: &SessionId,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn capabilities(&self) -> &[Capability] {
            &[]
        }

        fn backend_type(&self) -> BackendType {
            BackendType::Docker
        }

        async fn health_check(&self) -> Result<bool, Self::Error> {
            match &self.result {
                Ok(healthy) => Ok(*healthy),
                Err(error) => Err(clone_backend_init_error(error)),
            }
        }

        async fn cleanup(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn clone_backend_init_error(error: &RlmError) -> RlmError {
        match error {
            RlmError::BackendInitFailed { backend, message } => RlmError::BackendInitFailed {
                backend: backend.clone(),
                message: message.clone(),
            },
            _ => RlmError::BackendInitFailed {
                backend: "docker".to_string(),
                message: "test health failure".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn strict_health_gate_passes_only_true_health() {
        let executor = FakeHealthExecutor { result: Ok(true) };

        ensure_strict_docker_healthy(&executor)
            .await
            .expect("healthy docker accepted");
    }

    #[tokio::test]
    async fn strict_health_gate_fails_closed_on_false_health() {
        let executor = FakeHealthExecutor { result: Ok(false) };

        let error = ensure_strict_docker_healthy(&executor)
            .await
            .expect_err("unhealthy docker rejected");

        assert!(matches!(error, StrictDockerSandboxError::DockerUnhealthy));
    }

    #[tokio::test]
    async fn strict_health_gate_fails_closed_on_health_error_without_source_leak() {
        let sensitive = "unix:///var/run/docker.sock?token=secret-token";
        let executor = FakeHealthExecutor {
            result: Err(RlmError::BackendInitFailed {
                backend: "docker".to_string(),
                message: sensitive.to_string(),
            }),
        };

        let error = ensure_strict_docker_healthy(&executor)
            .await
            .expect_err("health error rejected");

        assert!(matches!(error, StrictDockerSandboxError::DockerUnhealthy));
        assert!(!format!("{error:?}").contains(sensitive));
        assert!(!error.to_string().contains(sensitive));
        assert!(error.source().is_none());
    }

    #[test]
    fn strict_sandbox_errors_do_not_expose_sources() {
        for error in [
            StrictDockerSandboxError::InvalidCheckout,
            StrictDockerSandboxError::BackendInit,
            StrictDockerSandboxError::DockerUnhealthy,
            StrictDockerSandboxError::NonDockerBackend,
        ] {
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn strict_diagnostics_profile_builds_locked_down_host_config() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let canonical_checkout = checkout.path().canonicalize().expect("canonical checkout");

        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");
        let host_config = profile.host_config();

        assert_eq!(host_config.network_mode.as_deref(), Some("none"));
        assert_eq!(host_config.readonly_rootfs, Some(true));
        assert_eq!(
            host_config.cap_drop.as_deref(),
            Some(&["ALL".to_string()][..])
        );
        assert_eq!(host_config.cap_add, None);
        assert_eq!(host_config.privileged, Some(false));
        assert_eq!(host_config.devices, None);
        assert_eq!(host_config.device_cgroup_rules, None);
        assert_eq!(host_config.device_requests, None);
        assert_eq!(host_config.binds, None);
        assert_eq!(host_config.volumes_from, None);
        assert_eq!(host_config.memory, Some(STRICT_DIAGNOSTICS_MEMORY_BYTES));
        assert_eq!(host_config.pids_limit, Some(STRICT_DIAGNOSTICS_PIDS_LIMIT));

        let mounts = host_config.mounts.as_ref().expect("typed mounts");
        assert_eq!(mounts.len(), 2);

        let scratch = mounts
            .iter()
            .find(|mount| mount.target.as_deref() == Some("/tmp"))
            .expect("/tmp scratch mount");
        assert_eq!(scratch.typ, Some(MountType::TMPFS));
        assert_eq!(scratch.source, None);
        assert_eq!(scratch.read_only, Some(false));
        let tmpfs = scratch.tmpfs_options.as_ref().expect("tmpfs options");
        assert_eq!(tmpfs.size_bytes, Some(STRICT_DIAGNOSTICS_TMPFS_BYTES));
        assert_eq!(
            tmpfs.options.as_ref().expect("tmpfs flags"),
            &vec![
                vec!["noexec".to_string()],
                vec!["nosuid".to_string()],
                vec!["nodev".to_string()]
            ]
        );

        let checkout_mount = mounts
            .iter()
            .find(|mount| mount.target.as_deref() == Some("/workspace"))
            .expect("checkout bind mount");
        assert_eq!(checkout_mount.typ, Some(MountType::BIND));
        assert_eq!(
            checkout_mount.source.as_deref(),
            Some(canonical_checkout.to_str().expect("utf-8 checkout path"))
        );
        assert_eq!(checkout_mount.read_only, Some(true));
        let bind_options = checkout_mount.bind_options.as_ref().expect("bind options");
        assert_eq!(
            bind_options.propagation,
            Some(MountBindOptionsPropagationEnum::RPRIVATE)
        );
        assert_eq!(bind_options.create_mountpoint, Some(false));
    }

    #[test]
    fn strict_diagnostics_profile_rejects_unsafe_checkout_paths() {
        let missing = tempfile::tempdir()
            .expect("parent tempdir")
            .path()
            .join("missing");
        let missing_error =
            StrictDockerDiagnosticsProfile::new(&missing).expect_err("missing path rejected");
        assert_eq!(
            missing_error,
            StrictDockerDiagnosticsProfileError::InvalidCheckout
        );

        let file_dir = tempfile::tempdir().expect("file tempdir");
        let file_path = file_dir.path().join("file");
        fs::write(&file_path, "not a directory").expect("test file");
        let file_error =
            StrictDockerDiagnosticsProfile::new(&file_path).expect_err("file path rejected");
        assert_eq!(
            file_error,
            StrictDockerDiagnosticsProfileError::InvalidCheckout
        );

        let root_error =
            StrictDockerDiagnosticsProfile::new("/").expect_err("filesystem root rejected");
        assert_eq!(
            root_error,
            StrictDockerDiagnosticsProfileError::InvalidCheckout
        );
    }

    #[test]
    fn strict_diagnostics_profile_debug_redacts_checkout_path() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");
        let canonical_checkout = checkout
            .path()
            .canonicalize()
            .expect("canonical checkout")
            .display()
            .to_string();

        let debug = format!("{profile:?}");

        assert!(debug.contains("checkout_path: \"<redacted>\""));
        assert!(!debug.contains(&canonical_checkout));
    }

    #[test]
    fn strict_container_create_body_uses_profile_host_config_and_no_env() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");

        let body = DockerExecutor::strict_container_create_body_for_test(&profile);

        assert_eq!(body.host_config, Some(profile.host_config()));
        assert_eq!(body.env, None);
        assert_eq!(body.image.as_deref(), Some(DEFAULT_IMAGE));
    }

    #[test]
    fn strict_container_create_body_serializes_locked_down_host_config() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let canonical_checkout = checkout.path().canonicalize().expect("canonical checkout");
        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");

        let body = DockerExecutor::strict_container_create_body_for_test(&profile);
        let value = serde_json::to_value(&body).expect("strict container request serializes");

        assert_eq!(
            value.get("Image").and_then(|v| v.as_str()),
            Some(DEFAULT_IMAGE)
        );
        assert!(value.get("Env").is_none_or(serde_json::Value::is_null));

        let host_config = value
            .get("HostConfig")
            .and_then(|v| v.as_object())
            .expect("serialized HostConfig object");
        assert_eq!(
            host_config.get("NetworkMode").and_then(|v| v.as_str()),
            Some("none")
        );
        assert_eq!(
            host_config.get("ReadonlyRootfs").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            host_config.get("Privileged").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            host_config.get("CapDrop").and_then(|v| v.as_array()),
            Some(&vec![serde_json::Value::String("ALL".to_string())])
        );
        assert!(
            host_config
                .get("CapAdd")
                .is_none_or(serde_json::Value::is_null)
        );
        assert!(
            host_config
                .get("Devices")
                .is_none_or(serde_json::Value::is_null)
        );
        assert!(
            host_config
                .get("DeviceCgroupRules")
                .is_none_or(serde_json::Value::is_null)
        );
        assert!(
            host_config
                .get("DeviceRequests")
                .is_none_or(serde_json::Value::is_null)
        );
        assert!(
            host_config
                .get("Binds")
                .is_none_or(serde_json::Value::is_null)
        );
        assert!(
            host_config
                .get("VolumesFrom")
                .is_none_or(serde_json::Value::is_null)
        );
        assert_eq!(
            host_config.get("Memory").and_then(|v| v.as_i64()),
            Some(STRICT_DIAGNOSTICS_MEMORY_BYTES)
        );
        assert_eq!(
            host_config.get("PidsLimit").and_then(|v| v.as_i64()),
            Some(STRICT_DIAGNOSTICS_PIDS_LIMIT)
        );

        let mounts = host_config
            .get("Mounts")
            .and_then(|v| v.as_array())
            .expect("typed mounts");
        assert_eq!(mounts.len(), 2);

        let scratch = mounts
            .iter()
            .find(|mount| mount.get("Target").and_then(|v| v.as_str()) == Some(STRICT_TMP_TARGET))
            .expect("tmpfs scratch mount");
        assert_eq!(scratch.get("Type").and_then(|v| v.as_str()), Some("tmpfs"));
        assert!(scratch.get("Source").is_none_or(serde_json::Value::is_null));
        assert_eq!(
            scratch.get("ReadOnly").and_then(|v| v.as_bool()),
            Some(false)
        );
        let tmpfs = scratch
            .get("TmpfsOptions")
            .and_then(|v| v.as_object())
            .expect("tmpfs options");
        assert_eq!(
            tmpfs.get("SizeBytes").and_then(|v| v.as_i64()),
            Some(STRICT_DIAGNOSTICS_TMPFS_BYTES)
        );
        assert_eq!(
            tmpfs.get("Options"),
            Some(&serde_json::json!([["noexec"], ["nosuid"], ["nodev"]]))
        );

        let checkout_mount = mounts
            .iter()
            .find(|mount| {
                mount.get("Target").and_then(|v| v.as_str()) == Some(STRICT_CHECKOUT_TARGET)
            })
            .expect("checkout bind mount");
        assert_eq!(
            checkout_mount.get("Type").and_then(|v| v.as_str()),
            Some("bind")
        );
        assert_eq!(
            checkout_mount.get("ReadOnly").and_then(|v| v.as_bool()),
            Some(true)
        );
        let source = checkout_mount
            .get("Source")
            .and_then(|v| v.as_str())
            .expect("checkout source exists");
        let source_canonical = Path::new(source)
            .canonicalize()
            .unwrap_or_else(|_| panic!("checkout source must canonicalize"));
        assert!(
            source_canonical == canonical_checkout,
            "checkout source must be the canonical test checkout"
        );
        let bind_options = checkout_mount
            .get("BindOptions")
            .and_then(|v| v.as_object())
            .expect("bind options");
        assert_eq!(
            bind_options.get("Propagation").and_then(|v| v.as_str()),
            Some("rprivate")
        );
        assert_eq!(
            bind_options
                .get("CreateMountpoint")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn bounded_output_accumulator_retains_combined_output_up_to_max_bytes() {
        let mut output = BoundedOutputAccumulator::new(5);

        output.append_stdout(b"abc");
        output.append_stderr(b"de");
        output.append_stdout(b"fgh");

        let (stdout, stderr, truncated, output_file_path) = output.finish();
        assert_eq!(stdout, "abc");
        assert_eq!(stderr, "de");
        assert_eq!(stdout.len() + stderr.len(), 5);
        assert!(truncated);
        assert_eq!(output_file_path, None);
    }

    #[test]
    fn bounded_output_accumulator_preserves_channel_separation_after_stderr_reaches_limit() {
        let mut output = BoundedOutputAccumulator::new(4);

        output.append_stderr(b"err");
        output.append_stdout(b"!");
        output.append_stderr(b"discarded");

        let (stdout, stderr, truncated, output_file_path) = output.finish();
        assert_eq!(stdout, "!");
        assert_eq!(stderr, "err");
        assert_eq!(stdout.len() + stderr.len(), 4);
        assert!(truncated);
        assert_eq!(output_file_path, None);
    }

    #[test]
    fn bounded_output_accumulator_with_zero_max_retains_no_bytes_and_truncates_on_output() {
        let mut output = BoundedOutputAccumulator::new(0);

        output.append_stdout(b"a");
        output.append_stderr(b"b");

        let (stdout, stderr, truncated, output_file_path) = output.finish();
        assert_eq!(stdout, "");
        assert_eq!(stderr, "");
        assert!(truncated);
        assert_eq!(output_file_path, None);
    }

    #[test]
    fn bounded_output_accumulator_handles_multibyte_utf8_without_slicing_panic() {
        let mut output = BoundedOutputAccumulator::new(3);

        output.append_stdout("é".as_bytes());
        output.append_stderr("€".as_bytes());

        let (stdout, stderr, truncated, output_file_path) = output.finish();
        assert_eq!(stdout, "é");
        assert_eq!(stderr, "");
        assert!(stdout.is_char_boundary(stdout.len()));
        assert!(stderr.is_char_boundary(stderr.len()));
        assert!(stdout.len() + stderr.len() <= 3);
        assert!(truncated);
        assert_eq!(output_file_path, None);
    }

    #[test]
    fn bounded_output_accumulator_discards_all_output_after_first_truncation() {
        let mut output = BoundedOutputAccumulator::new(3);

        output.append_stdout("é".as_bytes());
        output.append_stderr("€".as_bytes());
        output.append_stdout(b"x");

        let (stdout, stderr, truncated, output_file_path) = output.finish();
        assert_eq!(stdout, "é");
        assert_eq!(stderr, "");
        assert!(stdout.len() + stderr.len() <= 3);
        assert!(truncated);
        assert_eq!(output_file_path, None);
    }

    #[test]
    fn test_docker_executor_requires_docker() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let config = RlmConfig::minimal();
        let executor = DockerExecutor::new(config, None);
        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_docker_executor_capabilities() {
        if !is_docker_available() {
            eprintln!("Skipping test: Docker not available");
            return;
        }

        let config = RlmConfig::minimal();
        let executor = DockerExecutor::new(config, None).unwrap();

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
        let executor = DockerExecutor::new(config, None).unwrap();
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
        let exec = DockerExecutor::new(cfg, None).unwrap();
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
        let exec = DockerExecutor::new(RlmConfig::minimal(), None).unwrap();
        let unknown = SessionId::new();
        assert!(exec.release_session_container(&unknown).await.is_none());
    }

    #[tokio::test]
    async fn test_docker_release_session_container_removes() {
        if !skip_unless_image_ready("test_docker_release_session_container_removes") {
            return;
        }
        let exec = DockerExecutor::new(RlmConfig::minimal(), None).unwrap();
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
        let exec = std::sync::Arc::new(DockerExecutor::new(RlmConfig::minimal(), None).unwrap());
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
