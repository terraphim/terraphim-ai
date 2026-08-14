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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, oneshot};
use tokio::time::Instant as TokioInstant;

use super::{
    Capability, ExecutionContext, ExecutionEnvironment, ExecutionResult, SnapshotId,
    ValidationResult,
};
use crate::config::{BackendType, RlmConfig};
use crate::error::{RlmError, RlmResult};
use crate::native_diagnostics::{Probe, ProbeResult, ValidatedNativeFailureEvidence};
use crate::types::SessionId;

const DEFAULT_IMAGE: &str = "python:3.11-slim";
/// Fixed diagnostics image for strict Rust/Cargo/Git probes.
pub const STRICT_DIAGNOSTICS_IMAGE: &str = "rust:1.96-bookworm";
const BACKEND_NAME: &str = "docker";

/// Default container memory limit in bytes (512 MiB).
const DEFAULT_MEMORY_BYTES: i64 = 512 * 1024 * 1024;
/// Default container PIDs limit.
const DEFAULT_PIDS_LIMIT: i64 = 256;

/// Strict diagnostics container memory limit in bytes (256 MiB).
const STRICT_DIAGNOSTICS_MEMORY_BYTES: i64 = 256 * 1024 * 1024;
/// Strict diagnostics PIDs limit.
const STRICT_DIAGNOSTICS_PIDS_LIMIT: i64 = 64;
/// Strict diagnostics CPU quota in Docker NanoCPUs (0.5 CPU).
const STRICT_DIAGNOSTICS_NANO_CPUS: i64 = 500_000_000;
/// Strict diagnostics tmpfs scratch size in bytes (64 MiB).
const STRICT_DIAGNOSTICS_TMPFS_BYTES: i64 = 64 * 1024 * 1024;
/// Bounded cleanup deadline used after the main strict lifecycle deadline fires.
const STRICT_DIAGNOSTICS_CLEANUP_TIMEOUT: Duration = Duration::from_secs(5);
/// Absolute preflight deadline for strict Docker diagnostics construction.
const STRICT_DIAGNOSTICS_PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(30);

const STRICT_CHECKOUT_TARGET: &str = "/workspace";
const STRICT_TMP_TARGET: &str = "/tmp";

fn strict_container_name(session_id: &SessionId) -> String {
    format!("terraphim-rlm-{}", session_id)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StrictRecoveryRecord {
    locator: String,
    locator_kind: StrictRecoveryLocatorKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StrictRecoveryLocatorKind {
    NameCreateInFlight,
    ContainerId,
}

impl StrictRecoveryRecord {
    fn create_in_flight(name: String) -> Self {
        Self {
            locator: name,
            locator_kind: StrictRecoveryLocatorKind::NameCreateInFlight,
        }
    }

    fn container_id(id: String) -> Self {
        Self {
            locator: id,
            locator_kind: StrictRecoveryLocatorKind::ContainerId,
        }
    }
}

#[derive(Debug)]
struct StrictWorkerState {
    completed: AtomicBool,
    completed_notify: tokio::sync::Notify,
}

impl StrictWorkerState {
    fn new() -> Self {
        Self {
            completed: AtomicBool::new(false),
            completed_notify: tokio::sync::Notify::new(),
        }
    }

    fn mark_completed(&self) {
        self.completed.store(true, Ordering::Release);
        self.completed_notify.notify_waiters();
    }

    fn is_completed(&self) -> bool {
        self.completed.load(Ordering::Acquire)
    }
}

/// Executes code in Docker containers, providing namespace-level isolation.
pub struct DockerExecutor {
    docker: Docker,
    session_to_container: Arc<DashMap<SessionId, Arc<Mutex<Option<String>>>>>,
    strict_recovery: Arc<DashMap<SessionId, StrictRecoveryRecord>>,
    strict_workers: Arc<DashMap<SessionId, Arc<StrictWorkerState>>>,
    strict_shutting_down: Arc<AtomicBool>,
    strict_spawn_lock: Arc<Mutex<()>>,
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
    /// Fixed diagnostics image was not present locally.
    #[error("strict Docker diagnostics image unavailable")]
    DiagnosticsImageUnavailable,
    /// Required fixed diagnostics tools failed preflight.
    #[error("strict Docker diagnostics tools unavailable")]
    DiagnosticsToolsUnavailable,
    /// Constructor did not produce the expected Docker backend.
    #[error("strict sandbox constructor did not produce Docker backend")]
    NonDockerBackend,
}

/// Opaque strict Docker-only diagnostics sandbox.
///
/// The inner Docker executor is intentionally private. Callers can use only the
/// fixed diagnostic methods and cannot mutate Docker host configuration,
/// execute raw commands, or extract the raw executor.
pub struct StrictDockerDiagnosticsSandbox {
    inner: DockerExecutor,
}

/// Validated limits for strict diagnostic probes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbeExecutionLimits {
    timeout_ms: u64,
    max_output_bytes: u64,
}

impl ProbeExecutionLimits {
    /// Maximum accepted strict probe timeout.
    pub const MAX_TIMEOUT_MS: u64 = 600_000;
    /// Maximum accepted inline probe output.
    pub const MAX_OUTPUT_BYTES: u64 = 1024 * 1024;

    /// Validate strict diagnostic execution limits.
    ///
    /// This timeout is the operation deadline for strict probe phases after a
    /// Docker create request has begun. For lifecycle safety, an in-flight
    /// create request is not cancelled on deadline; the worker waits until
    /// Docker returns a definitive container ID or error. If Docker returns an
    /// ID after the deadline, the worker deletes that ID under the separate
    /// strict cleanup timeout before reporting the operation timeout. This can
    /// make the call exceed `timeout_ms`, but prevents accepting a name-based
    /// `NotFound` race as cleanup proof.
    ///
    /// # Errors
    ///
    /// Returns an error when a limit is zero or exceeds the strict maximum.
    pub fn new(timeout_ms: u64, max_output_bytes: u64) -> Result<Self, ProbeExecutionLimitsError> {
        if timeout_ms == 0 || timeout_ms > Self::MAX_TIMEOUT_MS {
            return Err(ProbeExecutionLimitsError::InvalidTimeout);
        }
        if max_output_bytes == 0 || max_output_bytes > Self::MAX_OUTPUT_BYTES {
            return Err(ProbeExecutionLimitsError::InvalidMaxOutput);
        }
        Ok(Self {
            timeout_ms,
            max_output_bytes,
        })
    }

    /// Strict timeout in milliseconds.
    pub fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    /// Strict maximum captured output bytes.
    pub fn max_output_bytes(&self) -> u64 {
        self.max_output_bytes
    }
}

impl Default for ProbeExecutionLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            max_output_bytes: crate::DEFAULT_MAX_INLINE_OUTPUT_BYTES.min(Self::MAX_OUTPUT_BYTES),
        }
    }
}

/// Safe validation error for strict diagnostic probe limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProbeExecutionLimitsError {
    /// Timeout was zero or above the strict maximum.
    #[error("probe timeout must be positive and within the strict maximum")]
    InvalidTimeout,
    /// Output limit was zero or above the strict maximum.
    #[error("probe output limit must be positive and within the strict maximum")]
    InvalidMaxOutput,
}

impl StrictDockerDiagnosticsSandbox {
    /// Execute one closed, read-only diagnostic probe in `/workspace`.
    ///
    /// # Errors
    ///
    /// Returns an RLM execution error if Docker exec fails or if fail-closed
    /// cleanup cannot be proven.
    pub(crate) async fn execute_probe(
        &self,
        _evidence: &ValidatedNativeFailureEvidence,
        probe: Probe,
        limits: ProbeExecutionLimits,
    ) -> RlmResult<ProbeResult> {
        let command = StrictDiagnosticCommand::for_probe(probe);
        let execution = self
            .inner
            .execute_strict_diagnostic_session(command.argv, limits)
            .await?;
        Ok(ProbeResult::from_execution(probe, execution))
    }
}

impl fmt::Debug for StrictDockerDiagnosticsSandbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StrictDockerDiagnosticsSandbox { backend: Docker }")
    }
}

#[derive(Debug, Eq, PartialEq)]
struct StrictDiagnosticCommand {
    argv: Vec<&'static str>,
}

impl StrictDiagnosticCommand {
    fn for_probe(probe: Probe) -> Self {
        match probe {
            Probe::CargoMetadataNoDeps => Self::cargo_metadata_no_deps(),
            Probe::GitDiffCheck => Self::git_diff_check(),
        }
    }

    fn cargo_metadata_no_deps() -> Self {
        Self {
            argv: vec!["cargo", "metadata", "--no-deps", "--format-version", "1"],
        }
    }

    fn git_diff_check() -> Self {
        Self {
            argv: vec!["git", "diff", "--check"],
        }
    }

    #[cfg(test)]
    fn create_exec_options_for_test(&self) -> CreateExecOptions<&str> {
        strict_diagnostic_create_exec_options(self.argv.clone())
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
    ensure_strict_diagnostics_preflight(&executor).await?;
    Ok(StrictDockerDiagnosticsSandbox { inner: executor })
}

async fn ensure_strict_diagnostics_preflight<P>(
    preflight: &P,
) -> Result<(), StrictDockerSandboxError>
where
    P: StrictDiagnosticsPreflight + Sync,
{
    let deadline = TokioInstant::now() + STRICT_DIAGNOSTICS_PREFLIGHT_TIMEOUT;
    ensure_strict_diagnostics_preflight_with_deadline(preflight, deadline).await
}

async fn ensure_strict_diagnostics_preflight_with_deadline<P>(
    preflight: &P,
    deadline: TokioInstant,
) -> Result<(), StrictDockerSandboxError>
where
    P: StrictDiagnosticsPreflight + Sync,
{
    if preflight.backend_type() != BackendType::Docker {
        return Err(StrictDockerSandboxError::NonDockerBackend);
    }
    if !strict_preflight_phase(
        deadline,
        preflight.docker_healthy(),
        StrictDockerSandboxError::DockerUnhealthy,
    )
    .await?
    {
        return Err(StrictDockerSandboxError::DockerUnhealthy);
    }
    if !strict_preflight_phase(
        deadline,
        preflight.diagnostics_image_available(),
        StrictDockerSandboxError::DiagnosticsImageUnavailable,
    )
    .await?
    {
        return Err(StrictDockerSandboxError::DiagnosticsImageUnavailable);
    }
    if !strict_preflight_phase(
        deadline,
        preflight.diagnostics_tools_available(deadline),
        StrictDockerSandboxError::DiagnosticsToolsUnavailable,
    )
    .await?
    {
        return Err(StrictDockerSandboxError::DiagnosticsToolsUnavailable);
    }
    Ok(())
}

async fn strict_preflight_phase<F>(
    deadline: TokioInstant,
    future: F,
    timeout_error: StrictDockerSandboxError,
) -> Result<bool, StrictDockerSandboxError>
where
    F: std::future::Future<Output = bool>,
{
    let Some(remaining) = deadline.checked_duration_since(TokioInstant::now()) else {
        return Err(timeout_error);
    };
    tokio::time::timeout(remaining, future)
        .await
        .map_err(|_| timeout_error)
}

#[async_trait]
trait StrictDiagnosticsPreflight {
    fn backend_type(&self) -> BackendType;
    async fn docker_healthy(&self) -> bool;
    async fn diagnostics_image_available(&self) -> bool;
    async fn diagnostics_tools_available(&self, deadline: TokioInstant) -> bool;
}

#[async_trait]
impl StrictDiagnosticsPreflight for DockerExecutor {
    fn backend_type(&self) -> BackendType {
        BackendType::Docker
    }

    async fn docker_healthy(&self) -> bool {
        matches!(self.health_check().await, Ok(true))
    }

    async fn diagnostics_image_available(&self) -> bool {
        self.docker
            .inspect_image(STRICT_DIAGNOSTICS_IMAGE)
            .await
            .is_ok()
    }

    async fn diagnostics_tools_available(&self, deadline: TokioInstant) -> bool {
        ensure_strict_diagnostics_tools_available(self, deadline).await
    }
}

async fn ensure_strict_diagnostics_tools_available(
    executor: &DockerExecutor,
    deadline: TokioInstant,
) -> bool {
    for argv in [vec!["cargo", "--version"], vec!["git", "--version"]] {
        let Some(remaining) = deadline.checked_duration_since(TokioInstant::now()) else {
            return false;
        };
        let timeout_ms = u64::try_from(remaining.as_millis())
            .unwrap_or(u64::MAX)
            .clamp(1, ProbeExecutionLimits::MAX_TIMEOUT_MS);
        let limits = ProbeExecutionLimits::new(timeout_ms, 16 * 1024).unwrap_or_default();
        let Ok(result) = executor
            .execute_strict_diagnostic_session(argv, limits)
            .await
        else {
            return false;
        };
        if result.exit_code != 0 || result.timed_out {
            return false;
        }
    }
    true
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
        nano_cpus: Some(STRICT_DIAGNOSTICS_NANO_CPUS),
        cap_drop: Some(vec!["ALL".to_string()]),
        cap_add: None,
        network_mode: Some("none".to_string()),
        readonly_rootfs: Some(true),
        privileged: Some(false),
        security_opt: Some(vec!["no-new-privileges".to_string()]),
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
            session_to_container: Arc::new(DashMap::new()),
            strict_recovery: Arc::new(DashMap::new()),
            strict_workers: Arc::new(DashMap::new()),
            strict_shutting_down: Arc::new(AtomicBool::new(false)),
            strict_spawn_lock: Arc::new(Mutex::new(())),
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
        executor.image = STRICT_DIAGNOSTICS_IMAGE.to_string();
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
        let container_name = strict_container_name(session_id);

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

    async fn exec_in_container(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        self.exec_in_container_with_workdir(container_id, cmd, None, ctx)
            .await
    }

    async fn execute_strict_diagnostic_session(
        &self,
        cmd: Vec<&str>,
        limits: ProbeExecutionLimits,
    ) -> RlmResult<ExecutionResult> {
        let _spawn_guard = self.strict_spawn_lock.lock().await;
        if self.strict_shutting_down.load(Ordering::Acquire) {
            return Err(strict_diagnostics_shutting_down());
        }
        if !self.strict_workers.is_empty() {
            return Err(strict_diagnostics_busy());
        }
        self.retry_strict_recovery_records().await?;
        if !self.strict_recovery.is_empty() || !self.strict_workers.is_empty() {
            return Err(strict_diagnostics_busy());
        }

        let session_id = SessionId::new();
        let deadline = TokioInstant::now() + Duration::from_millis(limits.timeout_ms());
        let (tx, rx) = oneshot::channel();
        let command = cmd.into_iter().map(str::to_string).collect();
        let worker_state = Arc::new(StrictWorkerState::new());
        self.strict_workers.insert(session_id, worker_state.clone());
        let worker = StrictDiagnosticLifecycleWorker {
            backend: DockerStrictLifecycleBackend {
                docker: self.docker.clone(),
                host_config: self.host_config.clone(),
            },
            recovery: self.strict_recovery.clone(),
            workers: self.strict_workers.clone(),
            worker_state,
            session_id,
            container_name: strict_container_name(&session_id),
            command,
            limits,
            deadline,
            result_tx: tx,
        };
        tokio::spawn(worker.run());
        drop(_spawn_guard);

        rx.await.map_err(|_| strict_cleanup_failed())?
    }

    async fn retry_strict_recovery_records(&self) -> RlmResult<()> {
        let backend = DockerStrictLifecycleBackend {
            docker: self.docker.clone(),
            host_config: self.host_config.clone(),
        };
        retry_strict_recovery_records_with_backend(
            &backend,
            &self.strict_recovery,
            &self.strict_workers,
        )
        .await
    }

    async fn strict_shutdown(&self, wait: Duration) -> RlmResult<()> {
        self.strict_shutting_down.store(true, Ordering::Release);
        let deadline = TokioInstant::now() + wait;
        let workers: Vec<_> = self
            .strict_workers
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        for worker in workers {
            loop {
                let completed = worker.completed_notify.notified();
                if worker.is_completed() {
                    break;
                }
                let Some(remaining) = deadline.checked_duration_since(TokioInstant::now()) else {
                    return Err(strict_cleanup_failed());
                };
                tokio::time::timeout(remaining, completed)
                    .await
                    .map_err(|_| strict_cleanup_failed())?;
            }
        }

        self.retry_strict_recovery_records().await
    }

    async fn strict_shutdown_for_cleanup(&self) -> RlmResult<()> {
        self.strict_shutdown(STRICT_DIAGNOSTICS_CLEANUP_TIMEOUT)
            .await
    }

    #[cfg(test)]
    async fn strict_shutdown_for_test(&self, wait: Duration) -> RlmResult<()> {
        self.strict_shutdown(wait).await
    }

    async fn exec_in_container_with_workdir(
        &self,
        container_id: &str,
        cmd: Vec<&str>,
        working_dir: Option<&str>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd),
            working_dir,
            ..Default::default()
        };

        self.exec_in_container_with_config(container_id, exec_config, ctx)
            .await
    }

    async fn exec_in_container_with_config(
        &self,
        container_id: &str,
        exec_config: CreateExecOptions<&str>,
        ctx: &ExecutionContext,
    ) -> RlmResult<ExecutionResult> {
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

struct StrictDiagnosticLifecycleWorker<B>
where
    B: StrictDiagnosticLifecycleBackend,
{
    backend: B,
    recovery: Arc<DashMap<SessionId, StrictRecoveryRecord>>,
    workers: Arc<DashMap<SessionId, Arc<StrictWorkerState>>>,
    worker_state: Arc<StrictWorkerState>,
    session_id: SessionId,
    container_name: String,
    command: Vec<String>,
    limits: ProbeExecutionLimits,
    deadline: TokioInstant,
    result_tx: oneshot::Sender<RlmResult<ExecutionResult>>,
}

impl<B> StrictDiagnosticLifecycleWorker<B>
where
    B: StrictDiagnosticLifecycleBackend,
{
    async fn run(self) {
        let result = self.run_inner().await;
        self.worker_state.mark_completed();
        self.workers.remove(&self.session_id);
        let _ = self.result_tx.send(result);
    }

    async fn run_inner(&self) -> RlmResult<ExecutionResult> {
        self.recovery.insert(
            self.session_id,
            StrictRecoveryRecord::create_in_flight(self.container_name.clone()),
        );

        let result = async {
            let container_id = self.create_phase().await?;
            self.phase(self.backend.start_strict_container(&container_id))
                .await?;
            let exec_config = strict_diagnostic_create_exec_options_owned(self.command.clone());
            let result = self
                .exec_phase(self.backend.exec_strict_container(
                    &container_id,
                    exec_config,
                    self.limits,
                ))
                .await?;
            Ok(result)
        }
        .await;

        if self.recovery.get(&self.session_id).is_none() {
            return result;
        }

        let cleanup_result = self.cleanup().await;
        strict_probe_cleanup_outcome(result, cleanup_result)
    }

    async fn create_phase(&self) -> RlmResult<String> {
        let create_result = self
            .backend
            .create_strict_container(&self.container_name)
            .await;
        let deadline_elapsed = self
            .deadline
            .checked_duration_since(TokioInstant::now())
            .is_none();

        match create_result {
            Ok(container_id) => {
                self.recovery.insert(
                    self.session_id,
                    StrictRecoveryRecord::container_id(container_id.clone()),
                );
                if deadline_elapsed {
                    let cleanup_result = self.cleanup().await;
                    return strict_probe_cleanup_outcome(
                        Err(strict_probe_timed_out()),
                        cleanup_result,
                    )
                    .map(|_| container_id);
                }
                Ok(container_id)
            }
            Err(error) => {
                self.recovery.remove(&self.session_id);
                if deadline_elapsed {
                    Err(strict_probe_timed_out())
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn phase<F, T>(&self, future: F) -> RlmResult<T>
    where
        F: std::future::Future<Output = RlmResult<T>>,
    {
        let Some(remaining) = self.deadline.checked_duration_since(TokioInstant::now()) else {
            return Err(strict_probe_timed_out());
        };
        tokio::time::timeout(remaining, future)
            .await
            .map_err(|_| strict_probe_timed_out())?
    }

    async fn exec_phase<F>(&self, future: F) -> RlmResult<ExecutionResult>
    where
        F: std::future::Future<Output = RlmResult<ExecutionResult>>,
    {
        let Some(remaining) = self.deadline.checked_duration_since(TokioInstant::now()) else {
            return Ok(strict_probe_timeout_result());
        };
        tokio::time::timeout(remaining, future)
            .await
            .unwrap_or_else(|_| Ok(strict_probe_timeout_result()))
    }

    async fn cleanup(&self) -> RlmResult<()> {
        let Some(record) = self
            .recovery
            .get(&self.session_id)
            .map(|entry| entry.clone())
        else {
            return Err(strict_cleanup_failed());
        };
        let cleanup = tokio::time::timeout(
            STRICT_DIAGNOSTICS_CLEANUP_TIMEOUT,
            self.backend.strict_delete_container(&record),
        )
        .await
        .map_err(|_| strict_cleanup_failed())?;
        cleanup?;
        self.recovery.remove(&self.session_id);
        Ok(())
    }
}

#[async_trait]
trait StrictDiagnosticLifecycleBackend: Clone + Send + Sync + 'static {
    async fn create_strict_container(&self, container_name: &str) -> RlmResult<String>;
    async fn start_strict_container(&self, container_id: &str) -> RlmResult<()>;
    async fn exec_strict_container(
        &self,
        container_id: &str,
        exec_config: CreateExecOptions<String>,
        limits: ProbeExecutionLimits,
    ) -> RlmResult<ExecutionResult>;
    async fn strict_delete_container(&self, record: &StrictRecoveryRecord) -> RlmResult<()>;
}

async fn retry_strict_recovery_records_with_backend<B>(
    backend: &B,
    recovery: &DashMap<SessionId, StrictRecoveryRecord>,
    workers: &DashMap<SessionId, Arc<StrictWorkerState>>,
) -> RlmResult<()>
where
    B: StrictDiagnosticLifecycleBackend,
{
    if !workers.is_empty() {
        return Err(strict_diagnostics_busy());
    }
    let records: Vec<_> = recovery
        .iter()
        .map(|entry| (*entry.key(), entry.value().clone()))
        .collect();

    for (session_id, record) in records {
        if record.locator_kind != StrictRecoveryLocatorKind::ContainerId {
            return Err(strict_cleanup_failed());
        }
        let cleanup = tokio::time::timeout(
            STRICT_DIAGNOSTICS_CLEANUP_TIMEOUT,
            backend.strict_delete_container(&record),
        )
        .await
        .map_err(|_| strict_cleanup_failed())?;
        cleanup?;
        recovery.remove(&session_id);
    }
    Ok(())
}

#[derive(Clone)]
struct DockerStrictLifecycleBackend {
    docker: Docker,
    host_config: HostConfig,
}

#[async_trait]
impl StrictDiagnosticLifecycleBackend for DockerStrictLifecycleBackend {
    async fn create_strict_container(&self, container_name: &str) -> RlmResult<String> {
        let config = strict_diagnostic_container_create_body(self.host_config.clone());
        let options = CreateContainerOptionsBuilder::new()
            .name(container_name)
            .build();
        let create_response = self
            .docker
            .create_container(Some(options), config)
            .await
            .map_err(|_| strict_backend_init_failed("create"))?;
        Ok(create_response.id)
    }

    async fn start_strict_container(&self, container_id: &str) -> RlmResult<()> {
        self.docker
            .start_container(container_id, None)
            .await
            .map_err(|_| strict_backend_init_failed("start"))
    }

    async fn exec_strict_container(
        &self,
        container_id: &str,
        exec_config: CreateExecOptions<String>,
        limits: ProbeExecutionLimits,
    ) -> RlmResult<ExecutionResult> {
        exec_in_container_with_config_for_docker(&self.docker, container_id, exec_config, limits)
            .await
    }

    async fn strict_delete_container(&self, record: &StrictRecoveryRecord) -> RlmResult<()> {
        strict_delete_container_with_docker(&self.docker, record).await
    }
}

async fn strict_delete_container_with_docker(
    docker: &Docker,
    record: &StrictRecoveryRecord,
) -> RlmResult<()> {
    let options = RemoveContainerOptionsBuilder::new().force(true).build();
    match docker
        .remove_container(&record.locator, Some(options))
        .await
    {
        Ok(()) => Ok(()),
        Err(_) => {
            log::warn!("strict diagnostic cleanup failed");
            Err(strict_cleanup_failed())
        }
    }
}

fn strict_probe_cleanup_outcome(
    result: RlmResult<ExecutionResult>,
    cleanup_result: RlmResult<()>,
) -> RlmResult<ExecutionResult> {
    match (result, cleanup_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(_)) | (Err(_), Err(_)) => Err(strict_cleanup_failed()),
    }
}

fn strict_cleanup_failed() -> RlmError {
    RlmError::Internal {
        message: "strict diagnostic cleanup failed".to_string(),
    }
}

fn strict_diagnostics_busy() -> RlmError {
    RlmError::StrictDiagnosticsBusy
}

fn strict_diagnostics_shutting_down() -> RlmError {
    RlmError::StrictDiagnosticsShuttingDown
}

fn strict_diagnostic_container_create_body(host_config: HostConfig) -> ContainerCreateBody {
    ContainerCreateBody {
        image: Some(STRICT_DIAGNOSTICS_IMAGE.to_string()),
        cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
        host_config: Some(host_config),
        ..Default::default()
    }
}

fn strict_probe_timeout_result() -> ExecutionResult {
    ExecutionResult::timeout(String::new(), String::new())
}

fn strict_probe_timed_out() -> RlmError {
    RlmError::ExecutionFailed {
        message: "strict diagnostic execution timed out".to_string(),
        exit_code: None,
        stdout: None,
        stderr: None,
    }
}

fn strict_backend_init_failed(action: &str) -> RlmError {
    RlmError::BackendInitFailed {
        backend: BACKEND_NAME.to_string(),
        message: format!("strict diagnostic container {action} failed"),
    }
}

#[cfg(test)]
fn strict_diagnostic_create_exec_options(cmd: Vec<&str>) -> CreateExecOptions<&str> {
    CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd),
        working_dir: Some(STRICT_CHECKOUT_TARGET),
        ..Default::default()
    }
}

fn strict_diagnostic_create_exec_options_owned(cmd: Vec<String>) -> CreateExecOptions<String> {
    CreateExecOptions {
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        cmd: Some(cmd),
        working_dir: Some(STRICT_CHECKOUT_TARGET.to_string()),
        ..Default::default()
    }
}

async fn exec_in_container_with_config_for_docker(
    docker: &Docker,
    container_id: &str,
    exec_config: CreateExecOptions<String>,
    limits: ProbeExecutionLimits,
) -> RlmResult<ExecutionResult> {
    let exec = docker
        .create_exec(container_id, exec_config)
        .await
        .map_err(|_| RlmError::ExecutionFailed {
            message: "strict diagnostic exec setup failed".to_string(),
            exit_code: None,
            stdout: None,
            stderr: None,
        })?;

    let start = Instant::now();
    let output = docker
        .start_exec(&exec.id, Some(StartExecOptions::default()))
        .await;

    match output {
        Ok(StartExecResults::Attached { mut output, .. }) => {
            let mut output_accumulator = BoundedOutputAccumulator::new(limits.max_output_bytes());

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

            let exit_code = docker
                .inspect_exec(&exec.id)
                .await
                .map_err(|_| RlmError::ExecutionFailed {
                    message: "strict diagnostic exec inspect failed".to_string(),
                    exit_code: None,
                    stdout: None,
                    stderr: None,
                })?
                .exit_code
                .map(|c| i32::try_from(c).unwrap_or(-1))
                .unwrap_or(-1);

            let (stdout, stderr, output_truncated, output_file_path) = output_accumulator.finish();
            Ok(ExecutionResult {
                stdout,
                stderr,
                exit_code,
                execution_time_ms: start.elapsed().as_millis() as u64,
                output_truncated,
                output_file_path,
                timed_out: false,
                metadata: HashMap::new(),
            })
        }
        Ok(StartExecResults::Detached) => Err(RlmError::ExecutionFailed {
            message: "strict diagnostic exec detached".to_string(),
            exit_code: None,
            stdout: None,
            stderr: None,
        }),
        Err(_) => Err(RlmError::ExecutionFailed {
            message: "strict diagnostic exec start failed".to_string(),
            exit_code: None,
            stdout: None,
            stderr: None,
        }),
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
        self.strict_shutdown_for_cleanup()
            .await
            .map_err(|_| strict_cleanup_failed())
    }

    async fn end_session(&self, session_id: &SessionId) -> Result<(), Self::Error> {
        let _ = self.release_session_container(session_id).await;
        Ok(())
    }
}

impl Drop for DockerExecutor {
    fn drop(&mut self) {
        // Drop can only attempt best-effort cleanup for legacy session
        // containers. Strict diagnostics lifecycle state is intentionally left
        // untouched here: a synchronous destructor cannot await in-flight
        // Docker create settlement or prove cleanup before Tokio runtime or
        // process shutdown.
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
    use bollard::models::{MountBindOptionsPropagationEnum, MountType};
    use std::error::Error;
    use std::fs;
    use std::sync::Mutex as StdMutex;
    use tokio::sync::Notify;

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
        assert_eq!(
            host_config.security_opt.as_deref(),
            Some(&["no-new-privileges".to_string()][..])
        );
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

    struct FakeStrictPreflight {
        backend_type: BackendType,
        healthy: bool,
        image_available: bool,
        tools_available: bool,
        block_health: bool,
        block_image: bool,
        block_tools: bool,
    }

    #[async_trait]
    impl StrictDiagnosticsPreflight for FakeStrictPreflight {
        fn backend_type(&self) -> BackendType {
            self.backend_type
        }

        async fn docker_healthy(&self) -> bool {
            if self.block_health {
                std::future::pending::<()>().await;
            }
            self.healthy
        }

        async fn diagnostics_image_available(&self) -> bool {
            if self.block_image {
                std::future::pending::<()>().await;
            }
            self.image_available
        }

        async fn diagnostics_tools_available(&self, _deadline: TokioInstant) -> bool {
            if self.block_tools {
                std::future::pending::<()>().await;
            }
            self.tools_available
        }
    }

    #[tokio::test]
    async fn strict_preflight_accepts_docker_with_fixed_image_and_tools() {
        let preflight = FakeStrictPreflight {
            backend_type: BackendType::Docker,
            healthy: true,
            image_available: true,
            tools_available: true,
            block_health: false,
            block_image: false,
            block_tools: false,
        };

        ensure_strict_diagnostics_preflight(&preflight)
            .await
            .expect("preflight passes");
    }

    #[tokio::test]
    async fn strict_preflight_fails_closed_for_backend_health_image_and_tools() {
        for (preflight, expected) in [
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Local,
                    healthy: true,
                    image_available: true,
                    tools_available: true,
                    block_health: false,
                    block_image: false,
                    block_tools: false,
                },
                StrictDockerSandboxError::NonDockerBackend,
            ),
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: false,
                    image_available: true,
                    tools_available: true,
                    block_health: false,
                    block_image: false,
                    block_tools: false,
                },
                StrictDockerSandboxError::DockerUnhealthy,
            ),
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: true,
                    image_available: false,
                    tools_available: true,
                    block_health: false,
                    block_image: false,
                    block_tools: false,
                },
                StrictDockerSandboxError::DiagnosticsImageUnavailable,
            ),
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: true,
                    image_available: true,
                    tools_available: false,
                    block_health: false,
                    block_image: false,
                    block_tools: false,
                },
                StrictDockerSandboxError::DiagnosticsToolsUnavailable,
            ),
        ] {
            let error = ensure_strict_diagnostics_preflight(&preflight)
                .await
                .expect_err("preflight rejects");

            assert_eq!(error, expected);
            assert!(error.source().is_none());
        }
    }

    #[tokio::test]
    async fn strict_preflight_deadline_wraps_blocked_health_image_and_tools() {
        for (preflight, expected) in [
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: true,
                    image_available: true,
                    tools_available: true,
                    block_health: true,
                    block_image: false,
                    block_tools: false,
                },
                StrictDockerSandboxError::DockerUnhealthy,
            ),
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: true,
                    image_available: true,
                    tools_available: true,
                    block_health: false,
                    block_image: true,
                    block_tools: false,
                },
                StrictDockerSandboxError::DiagnosticsImageUnavailable,
            ),
            (
                FakeStrictPreflight {
                    backend_type: BackendType::Docker,
                    healthy: true,
                    image_available: true,
                    tools_available: true,
                    block_health: false,
                    block_image: false,
                    block_tools: true,
                },
                StrictDockerSandboxError::DiagnosticsToolsUnavailable,
            ),
        ] {
            let error = ensure_strict_diagnostics_preflight_with_deadline(
                &preflight,
                TokioInstant::now() + Duration::from_millis(10),
            )
            .await
            .expect_err("blocked preflight phase times out");

            assert_eq!(error, expected);
            assert!(error.source().is_none());
        }
    }

    #[test]
    fn strict_container_create_body_uses_profile_host_config_and_no_env() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");

        let body = strict_diagnostic_container_create_body(profile.host_config());

        assert_eq!(body.host_config, Some(profile.host_config()));
        assert_eq!(body.env, None);
        assert_eq!(body.image.as_deref(), Some(STRICT_DIAGNOSTICS_IMAGE));
    }

    #[test]
    fn strict_diagnostic_command_compiler_emits_fixed_argv_templates() {
        assert_eq!(
            StrictDiagnosticCommand::cargo_metadata_no_deps().argv,
            vec!["cargo", "metadata", "--no-deps", "--format-version", "1"]
        );
        assert_eq!(
            StrictDiagnosticCommand::git_diff_check().argv,
            vec!["git", "diff", "--check"]
        );
    }

    #[test]
    fn strict_probe_limits_preserve_validated_values() {
        let limits = ProbeExecutionLimits::new(12_345, 65_536).expect("valid limits");

        assert_eq!(limits.timeout_ms(), 12_345);
        assert_eq!(limits.max_output_bytes(), 65_536);
    }

    #[test]
    fn strict_probe_limits_reject_zero_and_oversized_values() {
        assert_eq!(
            ProbeExecutionLimits::new(0, 1024).expect_err("zero timeout"),
            ProbeExecutionLimitsError::InvalidTimeout
        );
        assert_eq!(
            ProbeExecutionLimits::new(ProbeExecutionLimits::MAX_TIMEOUT_MS + 1, 1024)
                .expect_err("oversized timeout"),
            ProbeExecutionLimitsError::InvalidTimeout
        );
        assert_eq!(
            ProbeExecutionLimits::new(1000, 0).expect_err("zero output"),
            ProbeExecutionLimitsError::InvalidMaxOutput
        );
        assert_eq!(
            ProbeExecutionLimits::new(1000, ProbeExecutionLimits::MAX_OUTPUT_BYTES + 1)
                .expect_err("oversized output"),
            ProbeExecutionLimitsError::InvalidMaxOutput
        );
    }

    #[test]
    fn strict_diagnostic_create_exec_options_use_argv_workspace_and_no_env() {
        let command = StrictDiagnosticCommand::cargo_metadata_no_deps();
        let options = command.create_exec_options_for_test();
        let value = serde_json::to_value(&options).expect("options serialize");

        assert_eq!(
            value.get("Cmd").expect("cmd"),
            &serde_json::json!(["cargo", "metadata", "--no-deps", "--format-version", "1"])
        );
        assert_eq!(
            value.get("WorkingDir").and_then(|value| value.as_str()),
            Some(STRICT_CHECKOUT_TARGET)
        );
        assert!(value.get("Env").is_none_or(serde_json::Value::is_null));
        assert!(value.get("Cmd").is_some());
        assert_ne!(value.get("Cmd"), Some(&serde_json::json!("cargo test")));
        assert!(
            !value
                .get("Cmd")
                .and_then(|value| value.as_array())
                .is_some_and(|cmd| cmd.iter().any(|part| part == "sh" || part == "-c"))
        );
    }

    fn strict_test_execution_result(exit_code: i32, timed_out: bool) -> ExecutionResult {
        ExecutionResult {
            stdout: String::new(),
            stderr: String::new(),
            exit_code,
            execution_time_ms: 1,
            output_truncated: false,
            output_file_path: None,
            timed_out,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn strict_probe_cleanup_returns_success_nonzero_and_timeout_when_cleaned() {
        let success =
            strict_probe_cleanup_outcome(Ok(strict_test_execution_result(0, false)), Ok(()))
                .expect("success result");
        assert_eq!(success.exit_code, 0);

        let nonzero =
            strict_probe_cleanup_outcome(Ok(strict_test_execution_result(101, false)), Ok(()))
                .expect("nonzero still returns execution result");
        assert_eq!(nonzero.exit_code, 101);

        let timeout =
            strict_probe_cleanup_outcome(Ok(strict_test_execution_result(-1, true)), Ok(()))
                .expect("timeout still returns execution result");
        assert!(timeout.timed_out);
    }

    #[test]
    fn strict_probe_cleanup_preserves_exec_error_when_cleaned() {
        let error = strict_probe_cleanup_outcome(
            Err(RlmError::ExecutionFailed {
                message: "exec failed".to_string(),
                exit_code: None,
                stdout: None,
                stderr: None,
            }),
            Ok(()),
        )
        .expect_err("exec error returned after cleanup");

        assert!(matches!(error, RlmError::ExecutionFailed { .. }));
    }

    #[test]
    fn strict_probe_cleanup_fails_closed_when_cleanup_is_not_proven() {
        for result in [
            Ok(strict_test_execution_result(0, false)),
            Ok(strict_test_execution_result(101, false)),
            Err(RlmError::ExecutionFailed {
                message: "exec failed".to_string(),
                exit_code: None,
                stdout: None,
                stderr: None,
            }),
        ] {
            let error = strict_probe_cleanup_outcome(result, Err(strict_cleanup_failed()))
                .expect_err("cleanup failure wins");
            assert!(matches!(error, RlmError::Internal { .. }));
            assert!(!format!("{error:?}").contains("exec failed"));
        }
    }

    #[derive(Clone, Default)]
    struct FakeStrictLifecycleBackend {
        created_id: Arc<String>,
        events: Arc<StdMutex<Vec<String>>>,
        create_entered: Arc<Notify>,
        create_release: Arc<Notify>,
        pause_create: bool,
        fail_create: bool,
        block_start: bool,
        block_exec: bool,
        fail_delete: bool,
        block_delete: bool,
    }

    impl FakeStrictLifecycleBackend {
        fn new(created_id: &str) -> Self {
            Self {
                created_id: Arc::new(created_id.to_string()),
                events: Arc::new(StdMutex::new(Vec::new())),
                create_entered: Arc::new(Notify::new()),
                create_release: Arc::new(Notify::new()),
                pause_create: false,
                fail_create: false,
                block_start: false,
                block_exec: false,
                fail_delete: false,
                block_delete: false,
            }
        }

        fn pause_create(mut self) -> Self {
            self.pause_create = true;
            self
        }

        fn block_start(mut self) -> Self {
            self.block_start = true;
            self
        }

        fn block_exec(mut self) -> Self {
            self.block_exec = true;
            self
        }

        fn fail_create(mut self) -> Self {
            self.fail_create = true;
            self
        }

        fn fail_delete(mut self) -> Self {
            self.fail_delete = true;
            self
        }

        fn block_delete(mut self) -> Self {
            self.block_delete = true;
            self
        }

        fn events(&self) -> Vec<String> {
            self.events.lock().expect("events lock").clone()
        }
    }

    #[async_trait]
    impl StrictDiagnosticLifecycleBackend for FakeStrictLifecycleBackend {
        async fn create_strict_container(&self, container_name: &str) -> RlmResult<String> {
            self.events
                .lock()
                .expect("events lock")
                .push(format!("create:{container_name}"));
            self.create_entered.notify_waiters();
            if self.pause_create {
                self.create_release.notified().await;
            }
            if self.fail_create {
                return Err(strict_backend_init_failed("create"));
            }
            Ok((*self.created_id).clone())
        }

        async fn start_strict_container(&self, container_id: &str) -> RlmResult<()> {
            self.events
                .lock()
                .expect("events lock")
                .push(format!("start:{container_id}"));
            if self.block_start {
                std::future::pending::<()>().await;
            }
            Ok(())
        }

        async fn exec_strict_container(
            &self,
            container_id: &str,
            _exec_config: CreateExecOptions<String>,
            _limits: ProbeExecutionLimits,
        ) -> RlmResult<ExecutionResult> {
            self.events
                .lock()
                .expect("events lock")
                .push(format!("exec:{container_id}"));
            if self.block_exec {
                std::future::pending::<()>().await;
            }
            Ok(strict_test_execution_result(0, false))
        }

        async fn strict_delete_container(&self, record: &StrictRecoveryRecord) -> RlmResult<()> {
            self.events.lock().expect("events lock").push(format!(
                "delete:{}:{:?}",
                record.locator, record.locator_kind
            ));
            if self.block_delete {
                std::future::pending::<()>().await;
            }
            if self.fail_delete {
                return Err(strict_cleanup_failed());
            }
            Ok(())
        }
    }

    fn fake_worker(
        backend: FakeStrictLifecycleBackend,
        recovery: Arc<DashMap<SessionId, StrictRecoveryRecord>>,
        workers: Arc<DashMap<SessionId, Arc<StrictWorkerState>>>,
        deadline: TokioInstant,
    ) -> (
        StrictDiagnosticLifecycleWorker<FakeStrictLifecycleBackend>,
        oneshot::Receiver<RlmResult<ExecutionResult>>,
        SessionId,
    ) {
        let session_id = SessionId::new();
        let (tx, rx) = oneshot::channel();
        let worker_state = Arc::new(StrictWorkerState::new());
        workers.insert(session_id, worker_state.clone());
        let worker = StrictDiagnosticLifecycleWorker {
            backend,
            recovery,
            workers,
            worker_state,
            session_id,
            container_name: strict_container_name(&session_id),
            command: vec!["cargo".to_string(), "--version".to_string()],
            limits: ProbeExecutionLimits::new(100, 1024).expect("valid limits"),
            deadline,
            result_tx: tx,
        };
        (worker, rx, session_id)
    }

    #[tokio::test]
    async fn strict_worker_caller_cancellation_during_create_still_deletes_created_id_once() {
        let backend = FakeStrictLifecycleBackend::new("created-after-cancel").pause_create();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_secs(1),
        );
        let handle = tokio::spawn(worker.run());

        backend.create_entered.notified().await;
        drop(rx);
        backend.create_release.notify_waiters();
        handle.await.expect("worker finishes");

        assert_eq!(
            backend.events(),
            vec![
                format!("create:{}", strict_container_name(&session_id)),
                "start:created-after-cancel".to_string(),
                "exec:created-after-cancel".to_string(),
                "delete:created-after-cancel:ContainerId".to_string(),
            ]
        );
        assert!(recovery.is_empty());
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_timeout_during_start_deletes_bound_id_and_reports_timeout() {
        let backend = FakeStrictLifecycleBackend::new("created-before-timeout").block_start();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, _session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_millis(10),
        );
        tokio::spawn(worker.run());

        let error = rx
            .await
            .expect("worker sends result")
            .expect_err("start timeout fails");

        assert_eq!(
            error.to_string(),
            "Code execution failed: strict diagnostic execution timed out"
        );
        let events = backend.events();
        assert!(events[0].starts_with("create:terraphim-rlm-"));
        assert!(
            events
                .iter()
                .any(|event| event == "delete:created-before-timeout:ContainerId")
        );
        assert!(recovery.is_empty());
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_timeout_during_exec_deletes_bound_id_and_returns_typed_timeout_result() {
        let backend = FakeStrictLifecycleBackend::new("created-before-exec-timeout").block_exec();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, _session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_millis(10),
        );
        tokio::spawn(worker.run());

        let result = rx
            .await
            .expect("worker sends result")
            .expect("exec timeout returns typed execution result after cleanup");

        assert!(result.timed_out);
        assert_eq!(result.exit_code, -1);
        let evidence = crate::native_diagnostics::ValidatedNativeFailureEvidence::validate(
            crate::native_diagnostics::NativeFailureEvidenceInput {
                owner: "terraphim".to_string(),
                repo: "terraphim-ai".to_string(),
                commit_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
                run_id: 1,
                job_id: 2,
                failing_step: Some("native diagnostics".to_string()),
                verdict: crate::native_diagnostics::NativeVerdict::Failure,
                redacted_log_tail: "probe timed out".to_string(),
                max_evidence_bytes: 4096,
            },
        )
        .expect("valid evidence");
        let probe_result = ProbeResult::from_execution(Probe::CargoMetadataNoDeps, result);
        let diagnosis = crate::native_diagnostics::Diagnosis::from_evidence_and_probes(
            &evidence,
            &[probe_result],
        );
        assert_eq!(
            diagnosis.kind(),
            &crate::native_diagnostics::DiagnosisKind::Timeout
        );
        let events = backend.events();
        assert!(events[0].starts_with("create:terraphim-rlm-"));
        assert!(
            events
                .iter()
                .any(|event| event == "delete:created-before-exec-timeout:ContainerId")
        );
        assert!(recovery.is_empty());
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_create_id_after_deadline_deletes_id_once_without_starting() {
        let backend = FakeStrictLifecycleBackend::new("created-too-late").pause_create();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_millis(10),
        );
        tokio::spawn(worker.run());

        backend.create_entered.notified().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        backend.create_release.notify_waiters();

        let error = rx
            .await
            .expect("worker sends result")
            .expect_err("late create reports timeout after cleanup");

        assert_eq!(
            error.to_string(),
            "Code execution failed: strict diagnostic execution timed out"
        );
        let events = backend.events();
        assert_eq!(
            events,
            vec![
                format!("create:{}", strict_container_name(&session_id)),
                "delete:created-too-late:ContainerId".to_string(),
            ]
        );
        assert!(recovery.is_empty());
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_create_error_after_deadline_clears_name_without_delete() {
        let backend = FakeStrictLifecycleBackend::new("not-created")
            .pause_create()
            .fail_create();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_millis(10),
        );
        tokio::spawn(worker.run());

        backend.create_entered.notified().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        backend.create_release.notify_waiters();

        let error = rx
            .await
            .expect("worker sends result")
            .expect_err("late create error reports timeout");

        assert_eq!(
            error.to_string(),
            "Code execution failed: strict diagnostic execution timed out"
        );
        assert!(!recovery.contains_key(&session_id));
        assert!(
            !backend
                .events()
                .iter()
                .any(|event| event.starts_with("delete:"))
        );
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_delete_failure_leaves_recovery_record() {
        let backend = FakeStrictLifecycleBackend::new("cleanup-fails").fail_delete();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_secs(1),
        );
        tokio::spawn(worker.run());

        let error = rx
            .await
            .expect("worker sends result")
            .expect_err("cleanup failure wins");

        assert_eq!(
            error.to_string(),
            "Internal error: strict diagnostic cleanup failed"
        );
        assert_eq!(
            recovery.get(&session_id).map(|entry| entry.clone()),
            Some(StrictRecoveryRecord::container_id(
                "cleanup-fails".to_string()
            ))
        );
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_cleanup_timeout_leaves_recovery_record() {
        let backend = FakeStrictLifecycleBackend::new("cleanup-timeout").block_delete();
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_secs(1),
        );
        tokio::spawn(worker.run());

        let error = rx
            .await
            .expect("worker sends result")
            .expect_err("cleanup timeout wins");

        assert_eq!(
            error.to_string(),
            "Internal error: strict diagnostic cleanup failed"
        );
        assert_eq!(
            recovery.get(&session_id).map(|entry| entry.clone()),
            Some(StrictRecoveryRecord::container_id(
                "cleanup-timeout".to_string()
            ))
        );
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_worker_success_deletes_once_and_removes_recovery_record() {
        let backend = FakeStrictLifecycleBackend::new("cleanup-once");
        let recovery = Arc::new(DashMap::new());
        let workers = Arc::new(DashMap::new());
        let (worker, rx, session_id) = fake_worker(
            backend.clone(),
            recovery.clone(),
            workers.clone(),
            TokioInstant::now() + Duration::from_secs(1),
        );
        tokio::spawn(worker.run());

        let result = rx
            .await
            .expect("worker sends result")
            .expect("worker succeeds");

        assert_eq!(result.exit_code, 0);
        assert_eq!(
            backend
                .events()
                .iter()
                .filter(|event| event.starts_with("delete:"))
                .count(),
            1
        );
        assert!(!recovery.contains_key(&session_id));
        assert!(workers.is_empty());
    }

    #[tokio::test]
    async fn strict_recovery_retry_refuses_active_worker_without_deleting_name() {
        let backend = FakeStrictLifecycleBackend::new("active-worker");
        let recovery = DashMap::new();
        let workers = DashMap::new();
        let session_id = SessionId::new();
        recovery.insert(
            session_id,
            StrictRecoveryRecord::create_in_flight(strict_container_name(&session_id)),
        );
        workers.insert(session_id, Arc::new(StrictWorkerState::new()));

        let error = retry_strict_recovery_records_with_backend(&backend, &recovery, &workers)
            .await
            .expect_err("active worker blocks recovery retry");

        assert!(matches!(error, RlmError::StrictDiagnosticsBusy));
        assert!(recovery.contains_key(&session_id));
        assert!(backend.events().is_empty());
    }

    #[tokio::test]
    async fn strict_recovery_retry_refuses_stale_name_record_and_retains_it() {
        let backend = FakeStrictLifecycleBackend::new("stale-name");
        let recovery = DashMap::new();
        let workers = DashMap::new();
        let session_id = SessionId::new();
        recovery.insert(
            session_id,
            StrictRecoveryRecord::create_in_flight(strict_container_name(&session_id)),
        );

        let error = retry_strict_recovery_records_with_backend(&backend, &recovery, &workers)
            .await
            .expect_err("stale name fails closed");

        assert!(matches!(error, RlmError::Internal { .. }));
        assert!(recovery.contains_key(&session_id));
        assert!(backend.events().is_empty());
    }

    #[tokio::test]
    async fn strict_recovery_retry_deletes_confirmed_id_once_and_clears_record() {
        let backend = FakeStrictLifecycleBackend::new("unused");
        let recovery = DashMap::new();
        let workers = DashMap::new();
        let session_id = SessionId::new();
        recovery.insert(
            session_id,
            StrictRecoveryRecord::container_id("confirmed-id".to_string()),
        );

        retry_strict_recovery_records_with_backend(&backend, &recovery, &workers)
            .await
            .expect("confirmed ID recovery succeeds");
        retry_strict_recovery_records_with_backend(&backend, &recovery, &workers)
            .await
            .expect("second retry has no work");

        assert!(!recovery.contains_key(&session_id));
        assert_eq!(
            backend.events(),
            vec!["delete:confirmed-id:ContainerId".to_string()]
        );
    }

    #[tokio::test]
    async fn strict_executor_refuses_new_probe_when_worker_is_active() {
        let exec = DockerExecutor::new(RlmConfig::minimal(), None).expect("docker client handle");
        exec.strict_workers
            .insert(SessionId::new(), Arc::new(StrictWorkerState::new()));

        let error = exec
            .execute_strict_diagnostic_session(
                vec!["cargo", "--version"],
                ProbeExecutionLimits::default(),
            )
            .await
            .expect_err("active worker cap fails closed");

        assert!(matches!(error, RlmError::StrictDiagnosticsBusy));
    }

    #[tokio::test]
    async fn strict_shutdown_reports_unresolved_name_and_retains_record() {
        let exec = DockerExecutor::new(RlmConfig::minimal(), None).expect("docker client handle");
        let session_id = SessionId::new();
        exec.strict_recovery.insert(
            session_id,
            StrictRecoveryRecord::create_in_flight(strict_container_name(&session_id)),
        );

        let error = exec
            .strict_shutdown_for_test(Duration::from_millis(10))
            .await
            .expect_err("unresolved name cannot be proven clean");

        assert!(matches!(error, RlmError::Internal { .. }));
        assert!(exec.strict_recovery.contains_key(&session_id));
    }

    #[test]
    fn docker_executor_drop_leaves_strict_recovery_records_untouched() {
        let exec = DockerExecutor::new(RlmConfig::minimal(), None).expect("docker client handle");
        let recovery = exec.strict_recovery.clone();
        let session_id = SessionId::new();
        recovery.insert(
            session_id,
            StrictRecoveryRecord::container_id("recoverable-id".to_string()),
        );

        drop(exec);

        assert_eq!(
            recovery.get(&session_id).map(|entry| entry.clone()),
            Some(StrictRecoveryRecord::container_id(
                "recoverable-id".to_string()
            ))
        );
    }

    #[test]
    fn strict_container_create_body_serializes_locked_down_host_config() {
        let checkout = tempfile::tempdir().expect("checkout tempdir");
        let canonical_checkout = checkout.path().canonicalize().expect("canonical checkout");
        let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).expect("strict profile");

        let body = strict_diagnostic_container_create_body(profile.host_config());
        let value = serde_json::to_value(&body).expect("strict container request serializes");

        assert_eq!(
            value.get("Image").and_then(|v| v.as_str()),
            Some(STRICT_DIAGNOSTICS_IMAGE)
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
        assert_eq!(
            host_config.get("SecurityOpt"),
            Some(&serde_json::json!(["no-new-privileges"]))
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
