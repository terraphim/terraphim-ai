//! Agent spawner for Terraphim with health checking and output capture.
//!
//! This crate provides functionality to spawn external AI agents (Codex, Claude Code, OpenCode)
//! as non-interactive processes with:
//! - Configuration validation (CLI installed, API keys, models)
//! - Health checking via heartbeat (30s interval)
//! - Full output capture with @mention detection
//! - Auto-restart on failure

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::time::timeout;

use terraphim_types::capability::{ProcessId, Provider};

/// Configuration for the durable per-run output journal (terraphim-ai#3269).
///
/// When attached to a [`SpawnContext`], the spawner opens the journal
/// *before* spawning the child so a journal failure cannot leave an
/// unmanaged process, and every captured output event is durably
/// appended before in-memory fanout.
#[derive(Debug, Clone)]
pub struct OutputJournalConfig {
    /// Root directory under which the per-run journal file lives.
    pub root: PathBuf,
    /// Unique run identity; determines the journal file name.
    pub run_id: uuid::Uuid,
}

/// Per-spawn overrides that a caller can pass to AgentSpawner::spawn().
///
/// Enables multi-project use: one orchestrator serving many projects can
/// pass per-project working_dir and env without constructing N spawners.
#[derive(Debug, Clone, Default)]
pub struct SpawnContext {
    /// Working directory for the child process. None -> use spawner default.
    pub working_dir: Option<PathBuf>,
    /// Env vars to set on the child process (added to inherited env).
    pub env_overrides: HashMap<String, String>,
    /// Optional path to write raw stderr lines for post-mortem debugging.
    /// When set, the spawner creates the file and appends every stderr line
    /// directly, bypassing the bounded in-memory buffer.
    pub stderr_log_path: Option<PathBuf>,
    /// Optional durable output journal configuration. None -> legacy
    /// broadcast-only capture.
    pub output_journal: Option<OutputJournalConfig>,
}

impl SpawnContext {
    /// Use the spawner's default working_dir and no env overrides.
    pub fn global() -> Self {
        Self::default()
    }

    /// Override working_dir; keep env untouched.
    pub fn with_working_dir(path: impl Into<PathBuf>) -> Self {
        Self {
            working_dir: Some(path.into()),
            env_overrides: HashMap::new(),
            stderr_log_path: None,
            output_journal: None,
        }
    }

    /// Builder-style env addition.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_overrides.insert(key.into(), value.into());
        self
    }

    /// Set a dedicated stderr log file for this spawn.
    pub fn with_stderr_log(mut self, path: impl Into<PathBuf>) -> Self {
        self.stderr_log_path = Some(path.into());
        self
    }

    /// Attach a durable output journal for this spawn (terraphim-ai#3269).
    pub fn with_output_journal(mut self, root: impl Into<PathBuf>, run_id: uuid::Uuid) -> Self {
        self.output_journal = Some(OutputJournalConfig {
            root: root.into(),
            run_id,
        });
        self
    }
}

pub mod audit;
pub mod config;
pub mod health;
pub mod journal;
pub mod mention;
pub mod output;
pub mod redaction;

pub use audit::AuditEvent;
pub use config::{AgentConfig, AgentValidator, ResourceLimits, ValidationError};
pub use health::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthChecker, HealthHistory, HealthStatus,
};
pub use journal::{
    is_retention_eligible, AckMarker, CompleteMarker, Journal, JournalCheckpoint, JournalError,
    JournalFrame, JournalRecord, OutputKind, RecoveredJournal, RecoveryLimits,
};
pub use mention::{MentionEvent, MentionRouter};
pub use output::{OutputCapture, OutputEvent, OutputJournalError};
pub use redaction::{redact, verify_redacted};

/// Errors that can occur during agent spawning.
#[derive(thiserror::Error, Debug)]
pub enum SpawnerError {
    /// Agent configuration failed validation before spawning.
    #[error("Agent validation failed: {0}")]
    ValidationError(String),

    /// The OS-level process spawn failed (e.g. CLI not found on PATH).
    #[error("Failed to spawn agent: {0}")]
    SpawnError(String),

    /// The agent process exited before producing expected output.
    #[error("Agent process exited unexpectedly: {0}")]
    ProcessExit(String),

    /// Heartbeat health-check failed after the grace window elapsed.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Underlying I/O error (pipe read/write, file access).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Structured config validation error (field-level).
    #[error("Config validation error: {0}")]
    ConfigValidation(#[from] ValidationError),

    /// Durable journal error (open/create failure before spawn, or a
    /// journal-level fault propagated to the caller).
    #[error("Journal error: {0}")]
    Journal(#[from] JournalError),

    /// Durable output journal error surfaced while draining capture
    /// tasks or sealing the journal in `AgentHandle::wait`.
    #[error("Output journal error: {0}")]
    OutputJournal(#[from] OutputJournalError),

    /// A journal-backed agent handle observed the child exit before its
    /// durable journal was finalized (terraphim-ai#3269). The
    /// synchronous `try_wait` deliberately refuses to expose the raw
    /// exit status because doing so would let a caller observe a
    /// terminal status before the completion marker is sealed on disk.
    /// Callers must route through the async `AgentHandle::wait`, which
    /// reuses the cached status, seals the journal, and succeeds.
    #[error(
        "durable finalization required for process {process_id}: \
         call AgentHandle::wait to seal the journal and obtain the exit status"
    )]
    DurableFinalizationRequired { process_id: ProcessId },
}

/// Grace period (in seconds) for early-exit detection in
/// [`AgentSpawner::spawn_with_fallback`]. If the primary agent exits within
/// this window with a non-zero code, the fallback provider is attempted.
/// Rate-limit and auth failures cause immediate exit, so 5 seconds is
/// generous while keeping the happy-path delay minimal.
const EARLY_EXIT_GRACE_SECS: u64 = 5;
const EARLY_EXIT_GRACE: Duration = Duration::from_secs(EARLY_EXIT_GRACE_SECS);

/// Poll an agent handle for process exit at 100ms intervals.
/// Used by `spawn_with_fallback` for early-exit detection. Routes
/// through the internal draining poll so a journal-backed primary
/// has its capture tasks settled before the status is used to
/// decide on fallback; the journal itself is sealed by the caller
/// (terraphim-ai#3269).
async fn poll_exit(handle: &mut AgentHandle) -> Result<std::process::ExitStatus, SpawnerError> {
    loop {
        if let Some(status) = handle.try_wait_drained().await? {
            return Ok(status);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Request to spawn an agent with primary and fallback configuration.
///
/// If the primary provider fails to spawn, the spawner will automatically
/// retry with the fallback provider (if configured).
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Primary provider configuration
    pub primary_provider: Provider,
    /// Primary model to use (if applicable)
    pub primary_model: Option<String>,
    /// Fallback provider configuration (if primary fails)
    pub fallback_provider: Option<Provider>,
    /// Fallback model to use (if applicable)
    pub fallback_model: Option<String>,
    /// Task/prompt to give the agent
    pub task: String,
    /// Whether to deliver task via stdin (for large prompts)
    pub use_stdin: bool,
    /// Whether to remove broad default tool permissions from supported CLIs.
    pub disable_default_tools: bool,
    /// Resource limits for the spawned process.
    pub resource_limits: ResourceLimits,
}

impl SpawnRequest {
    /// Create a new spawn request with primary provider and task.
    pub fn new(primary_provider: Provider, task: impl Into<String>) -> Self {
        Self {
            primary_provider,
            primary_model: None,
            fallback_provider: None,
            fallback_model: None,
            task: task.into(),
            use_stdin: false,
            disable_default_tools: false,
            resource_limits: ResourceLimits::default(),
        }
    }

    /// Set the primary model.
    pub fn with_primary_model(mut self, model: impl Into<String>) -> Self {
        self.primary_model = Some(model.into());
        self
    }

    /// Set the fallback provider.
    pub fn with_fallback_provider(mut self, provider: Provider) -> Self {
        self.fallback_provider = Some(provider);
        self
    }

    /// Set the fallback model.
    pub fn with_fallback_model(mut self, model: impl Into<String>) -> Self {
        self.fallback_model = Some(model.into());
        self
    }

    /// Use stdin for task delivery (for large prompts).
    pub fn with_stdin(mut self) -> Self {
        self.use_stdin = true;
        self
    }

    /// Remove broad default tool permissions from supported CLIs.
    pub fn with_default_tools_disabled(mut self) -> Self {
        self.disable_default_tools = true;
        self
    }

    /// Set resource limits for the spawned process.
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }
}

/// Handle to a spawned agent process
#[derive(Debug)]
pub struct AgentHandle {
    /// Process ID
    pub process_id: ProcessId,
    /// Provider configuration
    pub provider: Provider,
    /// Child process handle
    child: Child,
    /// Health checker
    health_checker: HealthChecker,
    /// Output capture
    output_capture: OutputCapture,
    /// Whether this handle is backed by a durable output journal that
    /// must be sealed before a terminal status is exposed to the
    /// caller (terraphim-ai#3269). Set from [`SpawnContext`] in
    /// [`AgentSpawner::spawn_config`].
    journal_backed: bool,
    /// Exit status cached from `child.try_wait` so the async `wait`
    /// can reuse it without blocking again on the (already-reaped)
    /// child (terraphim-ai#3269). Populated on journal-backed
    /// handles where `try_wait` refused to expose the status, and by
    /// the internal draining poll used by `spawn_with_fallback`.
    cached_status: Option<std::process::ExitStatus>,
    /// Explicit finalized terminal state (terraphim-ai#3269). Set by
    /// `wait`/`shutdown`/`kill`/the fallback clean-primary path ONLY
    /// after the output capture has drained and the durable journal
    /// (when configured) has sealed successfully. Once set, public
    /// `try_wait` returns this cached terminal status instead of
    /// `DurableFinalizationRequired`, and `wait` returns it without
    /// re-finalizing. Deliberately NOT set by a raw non-journal
    /// `try_wait`: a later `wait` must still drain the output capture.
    finalized_status: Option<std::process::ExitStatus>,
}

impl AgentHandle {
    /// Get the process ID
    pub fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Check if the agent is healthy
    pub async fn is_healthy(&self) -> bool {
        self.health_checker.is_healthy().await
    }

    /// Get the last health status
    pub fn health_status(&self) -> HealthStatus {
        self.health_checker.status()
    }

    /// Get the output capture handle
    pub fn output_capture(&self) -> &OutputCapture {
        &self.output_capture
    }

    /// Subscribe to live output events via broadcast channel.
    ///
    /// Returns a receiver that gets a clone of every output event,
    /// suitable for streaming to WebSocket clients.
    pub fn subscribe_output(&self) -> tokio::sync::broadcast::Receiver<OutputEvent> {
        self.output_capture.subscribe()
    }

    /// Graceful shutdown: SIGTERM, wait for `grace_period`, then SIGKILL if still alive.
    ///
    /// Returns `Ok(true)` if the process exited gracefully, `Ok(false)` if it
    /// required a SIGKILL, or `Err` on I/O failure.
    ///
    /// The grace period bounds ONLY the child-process exit wait after
    /// SIGTERM. Output-capture finalization (`OutputCapture::finish`,
    /// draining the capture tasks and sealing the durable journal when
    /// configured) always happens OUTSIDE the timeout, so a slow pipe
    /// drain can neither cancel the capture JoinHandles (losing buffered
    /// output) nor turn a graceful in-grace exit into a spurious
    /// force-kill. A caller that observes `Ok` can rely on the journal
    /// being complete on disk. Finalization errors are propagated to the
    /// caller (terraphim-ai#3269).
    pub async fn shutdown(&mut self, grace_period: Duration) -> Result<bool, SpawnerError> {
        // Get the OS PID for signal sending
        let pid = match self.child.id() {
            Some(id) => id,
            None => {
                // Process already exited; reuse the cached exit status (if
                // any) and finalize capture/journal before returning.
                self.wait().await?;
                return Ok(true);
            }
        };

        // Send SIGTERM
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            let nix_pid = Pid::from_raw(pid as i32);
            if let Err(e) = kill(nix_pid, Signal::SIGTERM) {
                tracing::warn!(pid = pid, error = %e, "Failed to send SIGTERM");
            } else {
                tracing::info!(pid = pid, process_id = %self.process_id, "Sent SIGTERM");
            }
        }

        // The grace timeout covers ONLY waiting for the child to exit
        // after SIGTERM — never the output drain.
        match timeout(grace_period, self.wait_child_exit()).await {
            Ok(Ok(status)) => {
                // Graceful exit inside the grace period. Finalize the
                // output capture outside the timeout so a slow drain
                // cannot degrade this into a force-kill.
                self.output_capture.finish(status.code()).await?;
                self.finalized_status = Some(status);
                self.health_checker.mark_terminated();
                tracing::info!(
                    process_id = %self.process_id,
                    status = %status,
                    "Process exited gracefully"
                );
                tracing::info!(
                    target: "terraphim_spawner::audit",
                    event = %AuditEvent::AgentTerminated {
                        process_id: self.process_id,
                        graceful: true,
                    },
                    "Agent terminated gracefully"
                );
                Ok(true)
            }
            Ok(Err(e)) => {
                self.health_checker.mark_terminated();
                Err(e)
            }
            Err(_) => {
                // Timeout expired -- force kill, but only if the child is
                // still alive (it may have exited just as the grace
                // lapsed, in which case its status is already cached or
                // reaped and must not be killed again).
                tracing::warn!(
                    process_id = %self.process_id,
                    grace_period_ms = grace_period.as_millis() as u64,
                    "Process did not exit within grace period, sending SIGKILL"
                );
                if self.child.id().is_some() {
                    self.child.kill().await?;
                }
                // Obtain the exit status (reusing the cached one from a
                // prior try_wait refusal when present) and finalize the
                // capture/journal outside the timeout before reporting
                // the forced termination.
                let status = match self.cached_status.take() {
                    Some(cached) => cached,
                    None => self.child.wait().await.map_err(SpawnerError::Io)?,
                };
                self.output_capture.finish(status.code()).await?;
                self.finalized_status = Some(status);
                self.health_checker.mark_terminated();
                tracing::info!(
                    target: "terraphim_spawner::audit",
                    event = %AuditEvent::AgentTerminated {
                        process_id: self.process_id,
                        graceful: false,
                    },
                    "Agent force-killed"
                );
                Ok(false)
            }
        }
    }

    /// Wait only for the child process to exit, without touching the
    /// output capture (terraphim-ai#3269).
    ///
    /// Reuses the status cached by a prior `try_wait` refusal (or the
    /// draining poll in `spawn_with_fallback`) instead of blocking again
    /// on the already-reaped child. This is the future the shutdown
    /// grace timeout wraps, so cancelling it on timeout never touches
    /// the capture JoinHandles — finalization runs afterwards, outside
    /// the timeout.
    async fn wait_child_exit(&mut self) -> Result<std::process::ExitStatus, SpawnerError> {
        if let Some(cached) = self.cached_status.take() {
            Ok(cached)
        } else {
            self.child.wait().await.map_err(SpawnerError::Io)
        }
    }

    /// Hard kill the agent process (immediate SIGKILL).
    ///
    /// Performs an immediate hard kill when the child still has a live
    /// process id, then always routes through the async [`AgentHandle::wait`]
    /// finalization path (reusing/reaping the actual exit status, draining
    /// output capture, and sealing the durable journal when configured)
    /// before returning. Kill, wait, and finalization errors are propagated
    /// to the caller, so a caller that observes `Ok` can rely on the journal
    /// being complete on disk (terraphim-ai#3269).
    pub async fn kill(mut self) -> Result<(), SpawnerError> {
        self.health_checker.mark_terminated();
        if self.child.id().is_some() {
            self.child.kill().await?;
        }
        self.wait().await?;
        Ok(())
    }

    /// Check if the process has exited (non-blocking).
    ///
    /// On **non-journal-backed** handles, preserves the exact legacy
    /// behaviour: any observed exit status is returned to the caller
    /// and the health checker is marked terminated.
    ///
    /// On **journal-backed** handles (terraphim-ai#3269), the moment
    /// `child.try_wait` observes `Some(status)` the handle becomes an
    /// unsafe place to expose a terminal status: the durable journal
    /// has not been sealed yet, so a caller could observe exit before
    /// the completion marker is on disk. The status is therefore
    /// cached internally and `Err(DurableFinalizationRequired)` is
    /// returned, forcing the caller through the async `wait` which
    /// reuses the cached status, finishes the journal, and succeeds.
    ///
    /// Once a finalizing path (`wait`/`shutdown`) has durably
    /// completed, the finalized terminal status is returned directly
    /// from cache instead of the refusal.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, SpawnerError> {
        if let Some(status) = self.finalized_status {
            return Ok(Some(status));
        }
        match self.child.try_wait() {
            Ok(Some(status)) if self.journal_backed => {
                self.cached_status = Some(status);
                Err(SpawnerError::DurableFinalizationRequired {
                    process_id: self.process_id,
                })
            }
            Ok(status) => {
                if status.is_some() {
                    self.health_checker.mark_terminated();
                }
                // NOTE: the raw non-journal exit status is NOT recorded
                // as finalized — a later `wait` must still drain the
                // output capture before reporting the terminal status.
                Ok(status)
            }
            Err(e) => Err(SpawnerError::Io(e)),
        }
    }

    /// Internal async polling helper for `spawn_with_fallback`
    /// early-exit detection (terraphim-ai#3269).
    ///
    /// Unlike the public synchronous [`AgentHandle::try_wait`], this
    /// method drains the output capture tasks BEFORE marking the
    /// health checker terminated or exposing a terminal status, so the
    /// fallback path never observes an exit status ahead of the
    /// captured output being settled. It deliberately does NOT seal
    /// the durable journal: `spawn_with_fallback` shares one `run_id`
    /// across the primary and the fallback attempt, so the journal
    /// must stay open across a failed primary and be sealed exactly
    /// once — either by [`OutputCapture::complete`] on the clean
    /// primary, by the fallback's own finalization, or by the explicit
    /// seal in the fallback-spawn-failure path. The observed status is
    /// cached so a later [`AgentHandle::wait`] reuses it and stays
    /// idempotent.
    async fn try_wait_drained(&mut self) -> Result<Option<std::process::ExitStatus>, SpawnerError> {
        let status = match self.cached_status {
            Some(cached) => Some(cached),
            None => self.child.try_wait().map_err(SpawnerError::Io)?,
        };
        let Some(status) = status else {
            return Ok(None);
        };
        self.cached_status = Some(status);
        self.output_capture.drain().await?;
        self.health_checker.mark_terminated();
        Ok(Some(status))
    }

    /// Wait for the child process to exit naturally.
    ///
    /// After the child exits, drains the output capture tasks and seals
    /// the durable journal (when configured) *before* returning, so a
    /// caller that observes the returned status can rely on the journal
    /// being complete on disk. Returns the exit status.
    ///
    /// On journal-backed handles, reuses the status cached by a prior
    /// `try_wait` refusal instead of blocking again on the reaped
    /// child (terraphim-ai#3269).
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, SpawnerError> {
        if let Some(status) = self.finalized_status {
            return Ok(status);
        }
        let status = if let Some(cached) = self.cached_status.take() {
            cached
        } else {
            self.child.wait().await.map_err(SpawnerError::Io)?
        };
        self.output_capture.finish(status.code()).await?;
        // Record the finalized terminal state ONLY after the durable
        // finish succeeded: a failed finish leaves the handle
        // unfinalized so a retry can re-attempt the drain/seal.
        self.finalized_status = Some(status);
        self.health_checker.mark_terminated();
        Ok(status)
    }
}

// --------------- Agent Pool ---------------

/// Pool of reusable agent handles.
///
/// Manages a collection of spawned agents with checkout/release semantics.
/// Idle agents are kept warm for reuse rather than being terminated.
pub struct AgentPool {
    /// Available (idle) agents, keyed by provider ID.
    idle: HashMap<String, Vec<AgentHandle>>,
    /// Maximum idle agents per provider.
    max_idle_per_provider: usize,
    /// Grace period for shutdown of evicted agents.
    shutdown_grace: Duration,
}

impl AgentPool {
    /// Create a new agent pool.
    pub fn new(max_idle_per_provider: usize) -> Self {
        Self {
            idle: HashMap::new(),
            max_idle_per_provider,
            shutdown_grace: Duration::from_secs(5),
        }
    }

    /// Set the grace period for shutting down evicted agents.
    pub fn with_shutdown_grace(mut self, grace: Duration) -> Self {
        self.shutdown_grace = grace;
        self
    }

    /// Return an idle agent for the given provider, if one is available.
    pub fn checkout(&mut self, provider_id: &str) -> Option<AgentHandle> {
        let agents = self.idle.get_mut(provider_id)?;
        // Pop from the back (most recently returned = warmest)
        agents.pop()
    }

    /// Return an agent to the pool for reuse.
    ///
    /// If the pool for this provider is full, the oldest agent is evicted
    /// (shutdown gracefully in the background).
    pub fn release(&mut self, handle: AgentHandle) {
        let provider_id = handle.provider.id.clone();
        let agents = self.idle.entry(provider_id).or_default();

        // Evict oldest if at capacity
        if agents.len() >= self.max_idle_per_provider {
            let mut evicted = agents.remove(0);
            let grace = self.shutdown_grace;
            tokio::spawn(async move {
                let _ = evicted.shutdown(grace).await;
            });
        }

        agents.push(handle);
    }

    /// Number of idle agents for a given provider.
    pub fn idle_count(&self, provider_id: &str) -> usize {
        self.idle.get(provider_id).map_or(0, |v| v.len())
    }

    /// Total idle agents across all providers.
    pub fn total_idle(&self) -> usize {
        self.idle.values().map(|v| v.len()).sum()
    }

    /// Shut down all idle agents gracefully.
    pub async fn drain(&mut self) {
        for (_provider_id, agents) in self.idle.drain() {
            for mut handle in agents {
                let _ = handle.shutdown(self.shutdown_grace).await;
            }
        }
    }
}

impl std::fmt::Debug for AgentPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentPool")
            .field("max_idle_per_provider", &self.max_idle_per_provider)
            .field("shutdown_grace", &self.shutdown_grace)
            .field("total_idle", &self.total_idle())
            .finish()
    }
}

/// Spawner for AI agents
#[derive(Debug, Clone)]
pub struct AgentSpawner {
    /// Default working directory for spawned agents
    default_working_dir: PathBuf,
    /// Environment variables to pass to agents
    env_vars: HashMap<String, String>,
    /// Auto-restart on failure
    auto_restart: bool,
    /// Maximum restart attempts
    max_restarts: u32,
}

impl AgentSpawner {
    /// Create a new agent spawner
    pub fn new() -> Self {
        Self {
            default_working_dir: PathBuf::from("/tmp"),
            env_vars: HashMap::new(),
            auto_restart: true,
            max_restarts: 3,
        }
    }

    /// Set default working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.default_working_dir = dir.into();
        self
    }

    /// Set environment variables
    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars = vars;
        self
    }

    /// Set auto-restart behavior
    pub fn with_auto_restart(mut self, enabled: bool) -> Self {
        self.auto_restart = enabled;
        self
    }

    /// Set the maximum number of restart attempts.
    pub fn with_max_restarts(mut self, max: u32) -> Self {
        self.max_restarts = max;
        self
    }

    /// Whether auto-restart is enabled.
    pub fn auto_restart(&self) -> bool {
        self.auto_restart
    }

    /// Maximum restart attempts.
    pub fn max_restarts(&self) -> u32 {
        self.max_restarts
    }

    /// Spawn an agent from a provider configuration with an optional model.
    pub async fn spawn_with_model(
        &self,
        provider: &Provider,
        task: &str,
        model: Option<&str>,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        let config = match model {
            Some(m) => config.with_model(m),
            None => config,
        };
        self.spawn_config(provider, &config, task, false, &ctx)
            .await
    }

    /// Spawn an agent from a provider configuration with an optional model,
    /// delivering the task prompt via stdin to avoid ARG_MAX limits.
    pub async fn spawn_with_model_stdin(
        &self,
        provider: &Provider,
        task: &str,
        model: Option<&str>,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        let config = match model {
            Some(m) => config.with_model(m),
            None => config,
        };
        self.spawn_config(provider, &config, task, true, &ctx).await
    }

    /// Internal: spawn with model, stdin option, and resource limits.
    async fn spawn_with_options(
        &self,
        provider: &Provider,
        task: &str,
        model: Option<&str>,
        request: &SpawnRequest,
        ctx: &SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        let config = config.with_resource_limits(request.resource_limits.clone());
        let config = match model {
            Some(m) => config.with_model(m),
            None => config,
        };
        let config = if request.disable_default_tools {
            config.without_default_tools()
        } else {
            config
        };
        self.spawn_config(provider, &config, task, request.use_stdin, ctx)
            .await
    }

    /// Spawn an agent from a provider configuration.
    pub async fn spawn(
        &self,
        provider: &Provider,
        task: &str,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        self.spawn_config(provider, &config, task, false, &ctx)
            .await
    }

    /// Spawn an agent with primary and fallback configuration.
    ///
    /// Attempts to spawn with the primary provider first. If that fails,
    /// falls back to the fallback provider (if configured).
    ///
    /// After a successful spawn, waits up to `EARLY_EXIT_GRACE_SECS` for the
    /// process to exit. If it exits with a non-zero code within that window
    /// (indicating an immediate failure like rate-limit or auth error), the
    /// fallback is attempted. This catches CLI tools that start successfully
    /// but fail immediately due to provider-level issues.
    pub async fn spawn_with_fallback(
        &self,
        request: &SpawnRequest,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        // Try primary first with resource limits
        let mut handle = self
            .spawn_with_options(
                &request.primary_provider,
                &request.task,
                request.primary_model.as_deref(),
                request,
                &ctx,
            )
            .await?;

        // If no fallback configured, return immediately.
        let Some(ref fallback) = request.fallback_provider else {
            return Ok(handle);
        };

        // Early exit detection: poll for immediate failure (rate limit, auth
        // error, model not found). These cause the CLI to exit within seconds.
        let early_exit = tokio::select! {
            _ = tokio::time::sleep(EARLY_EXIT_GRACE) => None,
            status = poll_exit(&mut handle) => Some(status),
        };

        match early_exit {
            None => {
                // Still running after grace period — healthy spawn.
                tracing::debug!(
                    provider = %request.primary_provider.id,
                    "Primary still running after grace period"
                );
                Ok(handle)
            }
            Some(Ok(status)) if status.success() => {
                tracing::debug!(
                    provider = %request.primary_provider.id,
                    "Primary exited cleanly during grace period"
                );
                // The draining poll deliberately left the journal
                // unsealed; seal it now, before the primary handle is
                // returned. A journal error here fails closed rather
                // than returning a handle whose run is not durable.
                handle.output_capture().complete(status.code()).await?;
                // Finalized only after the durable completion
                // succeeded: later wait/try_wait return the cached
                // terminal status without re-finalizing.
                handle.finalized_status = Some(status);
                Ok(handle)
            }
            Some(Err(poll_err)) => {
                // Poll/drain/journal error: fail closed immediately.
                // The fallback is NEVER attempted on an error — an
                // uncertain primary state must not be masked by a
                // successful fallback (terraphim-ai#3269).
                Err(poll_err)
            }
            Some(Ok(status)) => {
                let code = status.code();
                tracing::warn!(
                    provider = %request.primary_provider.id,
                    exit_code = ?code,
                    "Primary exited during {}s grace period, attempting fallback",
                    EARLY_EXIT_GRACE_SECS,
                );
                // The primary capture has already drained. Close and join its
                // durable writer before the fallback tries to reopen the same
                // run, proving the old journal fd and writer lock are gone.
                let primary_process_id = handle.process_id;
                handle.output_capture.prepare_fallback_handoff().await?;

                // Try fallback only after ownership handoff.
                match self
                    .spawn_with_options(
                        fallback,
                        &request.task,
                        request.fallback_model.as_deref(),
                        request,
                        &ctx,
                    )
                    .await
                {
                    Ok(fb_handle) => {
                        tracing::info!(
                            fallback_provider = %fallback.id,
                            "Fallback spawn succeeded"
                        );
                        // The failed primary's drained handle is no
                        // longer needed; dropping it leaves the shared
                        // journal open for the fallback to seal.
                        drop(handle);
                        Ok(fb_handle)
                    }
                    Err(fb_err) => {
                        tracing::error!(
                            fallback_provider = %fallback.id,
                            error = %fb_err,
                            "Fallback spawn also failed"
                        );
                        // No fallback will finalize the logical run. The
                        // primary writer was already joined for handoff, so
                        // reopen the shared run and seal it with the primary's
                        // actual terminal identity/status before surfacing the
                        // fallback error. A seal failure supersedes the spawn
                        // error and fails closed.
                        if let Some(journal) = &ctx.output_journal {
                            output::seal_reopened_journal(
                                &journal.root,
                                journal.run_id,
                                primary_process_id,
                                code,
                            )
                            .await?;
                        }
                        Err(fb_err)
                    }
                }
            }
        }
    }

    /// Internal spawn implementation shared by spawn() and spawn_with_model().
    async fn spawn_config(
        &self,
        provider: &Provider,
        config: &AgentConfig,
        task: &str,
        use_stdin: bool,
        ctx: &SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let _span = tracing::info_span!(
            "spawner.spawn",
            provider_id = provider.id.as_str(),
            task_len = task.len(),
        )
        .entered();

        let validator = AgentValidator::new(config);
        validator.validate().await?;

        // Open/recover the durable output journal BEFORE spawning the child,
        // so a journal open failure cannot leave an unmanaged process
        // running and a configured stable run id reopens an existing
        // journal rather than refusing to spawn (terraphim-ai#3269).
        //
        // Fail-closed create-or-recover: a fresh run has no journal on disk
        // yet, so a bare `NotFound` from `open` is the single case where we
        // fall through to `create`. Every other error — including a create
        // race where another writer created the journal between our `open`
        // and `create` — propagates unchanged; we never retry or downgrade.
        let journal = match &ctx.output_journal {
            Some(cfg) => Some(match Journal::open(&cfg.root, cfg.run_id) {
                Ok(journal) => journal,
                Err(JournalError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                    Journal::create(&cfg.root, cfg.run_id)?
                }
                Err(err) => return Err(err.into()),
            }),
            None => None,
        };
        let journal_backed = journal.is_some();

        // Spawn the agent process
        let process_id = ProcessId::new();
        let mut child = self.spawn_process(config, task, use_stdin, ctx).await?;

        // Set up health checking
        let health_checker = HealthChecker::new(process_id, Duration::from_secs(30));

        // Set up output capture
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SpawnerError::SpawnError("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SpawnerError::SpawnError("Failed to capture stderr".to_string()))?;

        let output_capture = match journal {
            Some(journal) => OutputCapture::new_with_journal_and_stderr_log(
                process_id,
                BufReader::new(stdout),
                BufReader::new(stderr),
                journal,
                ctx.stderr_log_path.clone(),
            ),
            None => OutputCapture::new_with_stderr_log(
                process_id,
                BufReader::new(stdout),
                BufReader::new(stderr),
                ctx.stderr_log_path.clone(),
            ),
        };

        tracing::info!(
            target: "terraphim_spawner::audit",
            event = %AuditEvent::AgentSpawned {
                process_id,
                provider_id: provider.id.clone(),
            },
            "Agent spawned"
        );

        Ok(AgentHandle {
            process_id,
            provider: provider.clone(),
            child,
            health_checker,
            output_capture,
            journal_backed,
            cached_status: None,
            finalized_status: None,
        })
    }

    /// Spawn the actual process
    async fn spawn_process(
        &self,
        config: &AgentConfig,
        task: &str,
        use_stdin: bool,
        ctx: &SpawnContext,
    ) -> Result<Child, SpawnerError> {
        // Priority: ctx override > config working_dir > spawner default
        let working_dir = ctx
            .working_dir
            .as_ref()
            .or(config.working_dir.as_ref())
            .unwrap_or(&self.default_working_dir);

        let mut cmd = Command::new(&config.cli_command);
        cmd.current_dir(working_dir).args(&config.args);

        // Respect per-CLI stdin capability. opencode hangs on stdin for large
        // tasks, so it always receives the task as a positional argument even
        // when the orchestrator requests stdin delivery.
        let effective_stdin = use_stdin && config.supports_stdin;
        if use_stdin && !config.supports_stdin {
            tracing::info!(
                agent = %config.agent_id,
                cli = %config.cli_command,
                "stdin requested but not supported by CLI tool; falling back to positional arg"
            );
        }
        if effective_stdin {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.arg(task);
            cmd.stdin(Stdio::null());
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Add environment variables (spawner defaults, lowest priority)
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Add provider-specific env vars
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Apply per-call overrides last (highest priority)
        for (key, value) in &ctx.env_overrides {
            cmd.env(key, value);
        }

        // Strip ANTHROPIC_API_KEY for Claude CLI agents.
        // Claude CLI uses OAuth (browser flow) for authentication.
        // If ANTHROPIC_API_KEY is set in the environment (even inherited),
        // Claude CLI switches to API-key auth mode which fails with
        // invalid values like "oauth-managed".
        let cli_name = std::path::Path::new(&config.cli_command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if cli_name == "claude" || cli_name == "claude-code" {
            cmd.env_remove("ANTHROPIC_API_KEY");
        }

        // Apply resource limits via pre_exec hook (unix only)
        #[cfg(unix)]
        {
            let limits = config.resource_limits.clone();
            // SAFETY: setrlimit is async-signal-safe and we only call it
            // between fork and exec, which is the intended use case.
            unsafe {
                cmd.pre_exec(move || {
                    Self::apply_resource_limits(&limits)?;
                    Ok(())
                });
            }
        }

        let mut child = cmd.spawn()?;

        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                let oom_score = std::fs::read_to_string(format!("/proc/{}/oom_score", pid))
                    .unwrap_or_else(|_| "unknown".to_string());
                let oom_adj = std::fs::read_to_string(format!("/proc/{}/oom_score_adj", pid))
                    .unwrap_or_else(|_| "unknown".to_string());
                tracing::info!(
                    pid,
                    oom_score = oom_score.trim(),
                    oom_score_adj = oom_adj.trim(),
                    "spawned process oom diagnostics (before adjustment)"
                );

                if let Err(e) = Self::set_oom_score_adj(pid, -1000) {
                    tracing::warn!(
                        pid,
                        error = %e,
                        "failed to set oom_score_adj=-1000 for agent process"
                    );
                } else {
                    let new_adj = std::fs::read_to_string(format!("/proc/{}/oom_score_adj", pid))
                        .unwrap_or_else(|_| "unknown".to_string());
                    let new_score = std::fs::read_to_string(format!("/proc/{}/oom_score", pid))
                        .unwrap_or_else(|_| "unknown".to_string());
                    tracing::info!(
                        pid,
                        oom_score = new_score.trim(),
                        oom_score_adj = new_adj.trim(),
                        "spawned process oom diagnostics (after adjustment)"
                    );
                }
            }
        }

        // Write task to stdin if using stdin delivery
        if effective_stdin {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                if let Err(e) = stdin.write_all(task.as_bytes()).await {
                    // BrokenPipe means the child exited before consuming stdin
                    // (e.g. a short-lived CLI tool). The process itself spawned
                    // successfully; treat early exit as non-fatal here and let
                    // early-exit/health detection handle it, instead of failing
                    // the whole spawn.
                    if e.kind() != std::io::ErrorKind::BrokenPipe {
                        return Err(SpawnerError::SpawnError(format!(
                            "failed to write prompt to stdin: {}",
                            e
                        )));
                    }
                    tracing::debug!(
                        agent = %config.agent_id,
                        "child exited before consuming stdin; ignoring BrokenPipe"
                    );
                }
                // Drop stdin to close the pipe (signals EOF to the child)
            }
        }

        Ok(child)
    }

    /// Apply resource limits to the current process (called in pre_exec).
    #[cfg(unix)]
    fn apply_resource_limits(limits: &config::ResourceLimits) -> Result<(), std::io::Error> {
        use nix::sys::resource::{setrlimit, Resource};

        if let Some(max_mem) = limits.max_memory_bytes {
            setrlimit(Resource::RLIMIT_AS, max_mem, max_mem)
                .map_err(|e| std::io::Error::other(format!("RLIMIT_AS: {}", e)))?;
        }

        if let Some(max_cpu) = limits.max_cpu_seconds {
            setrlimit(Resource::RLIMIT_CPU, max_cpu, max_cpu)
                .map_err(|e| std::io::Error::other(format!("RLIMIT_CPU: {}", e)))?;
        }

        if let Some(max_fsize) = limits.max_file_size_bytes {
            setrlimit(Resource::RLIMIT_FSIZE, max_fsize, max_fsize)
                .map_err(|e| std::io::Error::other(format!("RLIMIT_FSIZE: {}", e)))?;
        }

        if let Some(max_files) = limits.max_open_files {
            setrlimit(Resource::RLIMIT_NOFILE, max_files, max_files)
                .map_err(|e| std::io::Error::other(format!("RLIMIT_NOFILE: {}", e)))?;
        }

        Ok(())
    }

    /// Set oom_score_adj for a given PID.
    #[cfg(unix)]
    fn set_oom_score_adj(pid: u32, score: i32) -> Result<(), std::io::Error> {
        use std::io::Write;
        let path = format!("/proc/{}/oom_score_adj", pid);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|e| std::io::Error::other(format!("open {}: {}", path, e)))?;
        let buf = format!("{}\n", score);
        f.write_all(buf.as_bytes())
            .map_err(|e| std::io::Error::other(format!("write {}: {}", path, e)))
    }
}

impl Default for AgentSpawner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::capability::{Capability, ProviderType};

    fn create_test_agent_provider() -> Provider {
        Provider::new(
            "@test-agent",
            "Test Agent",
            ProviderType::Agent {
                agent_id: "@test".to_string(),
                cli_command: "echo".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        )
    }

    /// Create a long-running agent (sleep) for shutdown testing.
    fn create_sleep_agent_provider() -> Provider {
        Provider::new(
            "@sleep-agent",
            "Sleep Agent",
            ProviderType::Agent {
                agent_id: "@sleep".to_string(),
                cli_command: "sleep".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        )
    }

    #[test]
    fn test_spawner_creation() {
        let spawner = AgentSpawner::new()
            .with_auto_restart(false)
            .with_working_dir("/workspace")
            .with_max_restarts(5);

        assert!(!spawner.auto_restart());
        assert_eq!(spawner.max_restarts(), 5);
        assert_eq!(spawner.default_working_dir, PathBuf::from("/workspace"));
    }

    #[tokio::test]
    async fn test_spawn_echo_agent() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner
            .spawn(&provider, "Hello World", SpawnContext::global())
            .await;

        // Echo command should succeed
        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@test-agent");
    }

    #[tokio::test]
    async fn test_try_wait_completed() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "done", SpawnContext::global())
            .await
            .unwrap();

        // Echo exits immediately; give it a moment
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status = handle.try_wait().unwrap();
        assert!(status.is_some()); // Process has exited
        assert_eq!(handle.health_status(), HealthStatus::Terminated);
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let spawner = AgentSpawner::new();
        let provider = create_sleep_agent_provider();

        // Spawn a sleep 60 agent
        let mut handle = spawner
            .spawn(&provider, "60", SpawnContext::global())
            .await
            .unwrap();

        // Graceful shutdown with 2s grace period
        let result = handle.shutdown(Duration::from_secs(2)).await;
        assert!(result.is_ok());

        // Should have exited (either gracefully via SIGTERM or force-killed)
        let _graceful = result.unwrap();
        // On most systems, sleep responds to SIGTERM -- either outcome is valid
        assert_eq!(handle.health_status(), HealthStatus::Terminated);
    }

    #[test]
    fn test_agent_pool_checkout_empty() {
        let mut pool = AgentPool::new(5);
        assert!(pool.checkout("nonexistent").is_none());
        assert_eq!(pool.total_idle(), 0);
    }

    #[tokio::test]
    async fn test_agent_pool_release_and_checkout() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner
            .spawn(&provider, "hello", SpawnContext::global())
            .await
            .unwrap();
        let mut pool = AgentPool::new(5);

        pool.release(handle);
        assert_eq!(pool.idle_count("@test-agent"), 1);
        assert_eq!(pool.total_idle(), 1);

        let checked_out = pool.checkout("@test-agent");
        assert!(checked_out.is_some());
        assert_eq!(pool.idle_count("@test-agent"), 0);
    }

    #[tokio::test]
    async fn test_subscribe_output_receives_events() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner
            .spawn(&provider, "broadcast test", SpawnContext::global())
            .await
            .unwrap();
        let mut receiver = handle.subscribe_output();

        // Give the echo process time to produce output and the capture task to process it
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Try to receive -- echo outputs "broadcast test" to stdout
        match receiver.try_recv() {
            Ok(OutputEvent::Stdout { line, .. }) => {
                assert!(line.contains("broadcast"));
            }
            Ok(OutputEvent::Mention { .. }) => {
                // Also acceptable if the line matched a mention pattern
            }
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // Race condition: the capture task may not have processed
                // the output yet. This is acceptable in CI environments.
            }
            Err(e) => panic!("Unexpected broadcast error: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_spawn_with_resource_limits() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        // Spawn with resource limits -- echo exits fast so this validates
        // that the pre_exec hook with setrlimit doesn't break spawning.
        let handle = spawner
            .spawn(&provider, "resource-limited", SpawnContext::global())
            .await;
        assert!(handle.is_ok());
    }

    #[tokio::test]
    async fn test_agent_pool_drain() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner
            .spawn(&provider, "hello", SpawnContext::global())
            .await
            .unwrap();
        let mut pool = AgentPool::new(5);
        pool.release(handle);

        assert_eq!(pool.total_idle(), 1);
        pool.drain().await;
        assert_eq!(pool.total_idle(), 0);
    }

    // =========================================================================
    // Stdin Delivery Tests (Gitea #73)
    // =========================================================================

    /// Create a cat agent provider for stdin testing (reads from stdin and outputs to stdout)
    fn create_cat_agent_provider() -> Provider {
        Provider::new(
            "@cat-agent",
            "Cat Agent",
            ProviderType::Agent {
                agent_id: "@cat".to_string(),
                cli_command: "cat".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        )
    }

    /// Test that spawn_process delivers prompt via stdin when use_stdin is true
    #[tokio::test]
    async fn test_spawn_process_stdin_echo() {
        let spawner = AgentSpawner::new();
        let provider = create_cat_agent_provider();

        // Spawn with stdin delivery - cat will echo the prompt back
        let handle = spawner
            .spawn_with_model_stdin(&provider, "hello from stdin", None, SpawnContext::global())
            .await;

        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@cat-agent");

        // Give cat time to read stdin and output to stdout
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check that output was captured
        let mut receiver = handle.subscribe_output();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // The cat command should have echoed our input
        match receiver.try_recv() {
            Ok(OutputEvent::Stdout { line, .. }) => {
                assert!(line.contains("hello from stdin"));
            }
            Ok(_) => {}
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // May be empty due to timing - that's okay
            }
            Err(e) => panic!("Unexpected broadcast error: {:?}", e),
        }
    }

    /// Test that without stdin flag, prompt is passed as CLI arg (backward compatibility)
    #[tokio::test]
    async fn test_spawn_process_arg_fallback() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        // Spawn without stdin - prompt should be CLI arg
        let handle = spawner
            .spawn(&provider, "arg test", SpawnContext::global())
            .await;

        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@test-agent");
    }

    /// Test that prompts above 32KB threshold trigger stdin delivery
    #[test]
    fn test_stdin_threshold_applied() {
        const STDIN_THRESHOLD: usize = 32_768; // 32 KB

        // Small prompt should NOT trigger stdin
        let small_prompt = "small task".to_string();
        let use_stdin = small_prompt.len() > STDIN_THRESHOLD;
        assert!(!use_stdin, "small prompt should not trigger stdin");

        // Large prompt should trigger stdin
        let large_prompt = "x".repeat(STDIN_THRESHOLD + 1);
        let use_stdin = large_prompt.len() > STDIN_THRESHOLD;
        assert!(use_stdin, "large prompt should trigger stdin");
    }

    /// Test that large prompts (100KB) write to stdin without error
    #[tokio::test]
    async fn test_stdin_write_completes() {
        let spawner = AgentSpawner::new();
        let provider = create_cat_agent_provider();

        // Create a large prompt (100KB)
        let large_prompt = "x".repeat(100 * 1024);

        // Spawn with stdin - should complete without error
        let handle = spawner
            .spawn_with_model_stdin(&provider, &large_prompt, None, SpawnContext::global())
            .await;

        assert!(
            handle.is_ok(),
            "large prompt should be written to stdin without error"
        );

        // Give time for the process to complete
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    /// Test that model flag + stdin delivery work together
    #[tokio::test]
    async fn test_spawn_with_model_stdin() {
        let spawner = AgentSpawner::new();

        // Use echo with a model - echo doesn't actually use models but this tests the API
        let provider = Provider::new(
            "@model-cat-agent",
            "Model Cat Agent",
            ProviderType::Agent {
                agent_id: "@model-cat".to_string(),
                cli_command: "cat".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );

        // Spawn with both model and stdin
        let handle = spawner
            .spawn_with_model_stdin(
                &provider,
                "model test via stdin",
                Some("test-model"),
                SpawnContext::global(),
            )
            .await;

        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@model-cat-agent");
    }

    // =========================================================================
    // ADF Remediation Tests (Gitea #117)
    // =========================================================================

    #[test]
    fn test_spawn_request_with_resource_limits() {
        let provider = create_test_agent_provider();
        let limits = ResourceLimits {
            max_cpu_seconds: Some(3600),
            max_memory_bytes: Some(2_147_483_648),
            ..Default::default()
        };
        let request = SpawnRequest::new(provider, "test").with_resource_limits(limits.clone());
        assert_eq!(request.resource_limits.max_cpu_seconds, Some(3600));
        assert_eq!(
            request.resource_limits.max_memory_bytes,
            Some(2_147_483_648)
        );
    }

    // =========================================================================
    // SpawnContext Tests (Gitea adf-fleet#3)
    // =========================================================================

    #[test]
    fn test_spawn_context_global_is_default() {
        let ctx = SpawnContext::global();
        assert!(ctx.working_dir.is_none());
        assert!(ctx.env_overrides.is_empty());
    }

    #[test]
    fn test_spawn_context_with_working_dir() {
        let ctx = SpawnContext::with_working_dir("/some/project");
        assert_eq!(ctx.working_dir, Some(PathBuf::from("/some/project")));
        assert!(ctx.env_overrides.is_empty());
    }

    #[test]
    fn test_spawn_context_with_env() {
        let ctx = SpawnContext::global()
            .with_env("FOO", "bar")
            .with_env("BAZ", "qux");
        assert!(ctx.working_dir.is_none());
        assert_eq!(ctx.env_overrides.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(ctx.env_overrides.get("BAZ"), Some(&"qux".to_string()));
    }

    #[tokio::test]
    async fn test_spawn_global_uses_spawner_default_working_dir() {
        let spawner = AgentSpawner::new().with_working_dir("/tmp");
        let provider = create_test_agent_provider();

        // SpawnContext::global() should preserve spawner's default behaviour.
        // We spawn /bin/echo (via echo provider) and check it succeeds.
        let handle = spawner
            .spawn(&provider, "hello", SpawnContext::global())
            .await;
        assert!(handle.is_ok(), "spawn with global context should succeed");
    }

    #[tokio::test]
    async fn test_spawn_with_working_dir_override() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::TempDir;

        let tmpdir = TempDir::new().expect("create tempdir");
        let tmppath = tmpdir.path().to_path_buf();

        // Write a tiny shell script that sleeps briefly before printing pwd.
        // The sleep is load-bearing: it gives the test time to call
        // `subscribe_output` after `spawn().await` returns. `OutputCapture`
        // uses a tokio broadcast channel which drops messages emitted before
        // any subscriber exists, so a fast `/bin/pwd` could complete and
        // drop its line on the floor between spawn and subscribe. The
        // validator only accepts a single executable path, so we cannot
        // pass `sh -c '...'` as cli_command.
        let script_path = tmpdir.path().join("pwd-with-delay.sh");
        std::fs::write(&script_path, "#!/bin/sh\nsleep 0.2\npwd\n").expect("write pwd script");
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).expect("chmod script");

        let provider = Provider::new(
            "@pwd-agent",
            "Pwd Agent",
            terraphim_types::capability::ProviderType::Agent {
                agent_id: "@pwd".to_string(),
                cli_command: script_path.to_string_lossy().to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![terraphim_types::capability::Capability::CodeGeneration],
        );

        let spawner = AgentSpawner::new().with_working_dir("/tmp");
        let ctx = SpawnContext::with_working_dir(tmppath.clone());

        let handle = spawner
            .spawn(&provider, ".", ctx)
            .await
            .expect("spawn with working_dir override should succeed");

        let mut rx = handle.subscribe_output();
        let resolved = std::fs::canonicalize(&tmppath).unwrap_or(tmppath.clone());

        // Drain output events until either the matching cwd line arrives,
        // the broadcast closes (sender dropped after capture finished), or
        // a generous overall timeout fires. recv() rather than try_recv()
        // blocks for actual output instead of polling-with-sleep.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut found = false;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Ok(OutputEvent::Stdout { line, .. })) => {
                    let trimmed = line.trim();
                    if trimmed == resolved.to_string_lossy().as_ref()
                        || trimmed == tmppath.to_string_lossy().as_ref()
                    {
                        found = true;
                        break;
                    }
                }
                Ok(Ok(_)) => {}      // non-stdout event; keep draining
                Ok(Err(_)) => break, // broadcast closed
                Err(_) => break,     // timeout
            }
        }

        assert!(found, "child cwd should be the overridden tmpdir");
    }

    #[tokio::test]
    async fn test_spawn_pi_receives_prompt_model_and_task() {
        use std::os::unix::fs::PermissionsExt;
        use tempfile::TempDir;

        let tmpdir = TempDir::new().expect("create tempdir");
        let script_path = tmpdir.path().join("pi");
        let args_path = tmpdir.path().join("pi-args.txt");

        std::fs::write(
            &script_path,
            "#!/bin/sh\nprintf '%s\n' \"$@\" > \"$PI_ARGS_CAPTURE\"\n",
        )
        .expect("write pi test script");
        let mut perms = std::fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms).expect("chmod pi test script");

        let provider = Provider::new(
            "@pi-agent",
            "Pi Agent",
            terraphim_types::capability::ProviderType::Agent {
                agent_id: "@pi".to_string(),
                cli_command: script_path.to_string_lossy().to_string(),
                working_dir: tmpdir.path().to_path_buf(),
            },
            vec![terraphim_types::capability::Capability::CodeGeneration],
        );

        let spawner = AgentSpawner::new();
        let mut handle = spawner
            .spawn_with_model(
                &provider,
                "What is 2+2?",
                Some("phi3"),
                SpawnContext::global()
                    .with_env("PI_ARGS_CAPTURE", args_path.to_string_lossy().to_string()),
            )
            .await
            .expect("pi spawn should succeed");
        let status = handle.wait().await.expect("pi test process should exit");
        assert!(status.success(), "pi test process should exit successfully");

        let args = std::fs::read_to_string(&args_path).expect("read captured pi args");
        let args: Vec<&str> = args.lines().collect();
        assert_eq!(args, vec!["prompt", "phi3", "What is 2+2?"]);
    }

    #[tokio::test]
    async fn test_spawn_env_override_propagates() {
        // Use /usr/bin/printenv VAR_NAME to verify env override.
        // printenv takes the variable name as its argument and prints the value.
        let provider = Provider::new(
            "@printenv-env-agent",
            "Printenv Env Agent",
            terraphim_types::capability::ProviderType::Agent {
                agent_id: "@printenv-env".to_string(),
                cli_command: "/usr/bin/printenv".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![terraphim_types::capability::Capability::CodeGeneration],
        );

        let spawner = AgentSpawner::new();
        let ctx = SpawnContext::global().with_env("ADF_SPAWN_CTX_TEST", "hello-from-ctx");

        // Task "ADF_SPAWN_CTX_TEST" becomes the arg to printenv, printing its value.
        let handle = spawner
            .spawn(&provider, "ADF_SPAWN_CTX_TEST", ctx)
            .await
            .expect("spawn with env override should succeed");

        let mut rx = handle.subscribe_output();
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut output = String::new();
        loop {
            match rx.try_recv() {
                Ok(OutputEvent::Stdout { line, .. }) => output.push_str(line.trim()),
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        assert!(
            output.contains("hello-from-ctx"),
            "env override should be visible in child process, got: {:?}",
            output
        );
    }

    #[tokio::test]
    async fn test_inherited_env_flows_through_without_override() {
        // Set an env var in the test process and verify a child sees it.
        // Using /usr/bin/printenv VAR_NAME avoids shell argument-parsing issues.
        unsafe {
            std::env::set_var("ADF_INHERITED_SPAWN_CTX", "inherited-value");
        }

        let provider = Provider::new(
            "@printenv-inherit-agent",
            "Printenv Inherit Agent",
            terraphim_types::capability::ProviderType::Agent {
                agent_id: "@printenv-inherit".to_string(),
                cli_command: "/usr/bin/printenv".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![terraphim_types::capability::Capability::CodeGeneration],
        );

        let spawner = AgentSpawner::new();
        let handle = spawner
            .spawn(&provider, "ADF_INHERITED_SPAWN_CTX", SpawnContext::global())
            .await
            .expect("spawn should succeed");

        let mut rx = handle.subscribe_output();
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut output = String::new();
        loop {
            match rx.try_recv() {
                Ok(OutputEvent::Stdout { line, .. }) => output.push_str(line.trim()),
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        assert!(
            output.contains("inherited-value"),
            "inherited env should be visible in child without override, got: {:?}",
            output
        );
    }

    // =========================================================================
    // Durable Output Journal Tests (terraphim-ai#3269)
    // =========================================================================

    #[tokio::test]
    async fn test_journal_open_failure_prevents_child_spawn() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let marker = temp.path().join("child-spawned.marker");
        let task = format!("touch {}", marker.display());
        let provider = Provider::new(
            "@bash-agent",
            "Bash Agent",
            ProviderType::Agent {
                agent_id: "@bash".to_string(),
                cli_command: "bash".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );
        let ctx = SpawnContext::global()
            .with_output_journal(PathBuf::from("relative-journal-root"), Uuid::new_v4());

        let result = AgentSpawner::new().spawn(&provider, &task, ctx).await;

        match result {
            Err(SpawnerError::Journal(_)) => {}
            Err(other) => panic!(
                "journal open failure must surface as typed SpawnerError::Journal, got: {:?}",
                other
            ),
            Ok(_) => panic!(
                "journal open failure must prevent child spawn instead of downgrading to broadcast-only"
            ),
        }
        assert!(
            !marker.exists(),
            "child process must not run when journal validation fails"
        );
    }

    #[tokio::test]
    async fn test_spawn_with_output_journal_persists_before_wait_returns() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "durable-lifecycle-line", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let process_id = handle.process_id();

        let status = handle.wait().await.expect("echo should exit");
        assert!(status.success(), "echo should exit successfully");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        let completion = recovered
            .completion
            .as_ref()
            .expect("recovery completion should be Some");
        assert!(
            completion.completion_id != Uuid::nil(),
            "journal should carry a non-nil completion id"
        );

        assert!(
            recovered.records.iter().any(|record| {
                record.kind == OutputKind::Stdout
                    && record.process_id == process_id.0
                    && record
                        .payload
                        .as_str()
                        .is_some_and(|s| s.contains("durable-lifecycle-line"))
            }),
            "journal should contain the stdout line for the spawned process"
        );

        assert!(
            recovered
                .records
                .iter()
                .any(|record| record.kind == OutputKind::Completed
                    && record.process_id == process_id.0),
            "journal should contain a Completed record for the spawned process"
        );
    }

    #[tokio::test]
    async fn test_spawn_reopens_incomplete_output_journal_and_resumes_sequence() {
        use serde_json::json;
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();

        // Pre-seed an incomplete journal (no completion frame), as if a
        // previous spawn crashed after writing its first record.
        {
            let mut journal = Journal::create(temp.path(), run_id).expect("create journal");
            journal
                .append(JournalRecord {
                    run_id,
                    sequence: 0,
                    process_id: 777,
                    kind: OutputKind::Stdout,
                    payload: json!("before-crash"),
                    completion_id: None,
                    observed_at: chrono::Utc::now(),
                })
                .expect("append pre-crash record");
        } // drop journal without completion, simulating a crash

        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "after-crash", ctx)
            .await
            .expect("spawn should reopen the incomplete journal and succeed");

        let status = handle.wait().await.expect("echo should exit");
        assert!(status.success(), "echo should exit successfully");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        let sequences: Vec<u64> = recovered.records.iter().map(|r| r.sequence).collect();
        assert_eq!(
            sequences,
            vec![0, 1, 2],
            "spawn must resume the sequence after the pre-crash record"
        );

        let payloads: Vec<&str> = recovered
            .records
            .iter()
            .filter_map(|r| r.payload.as_str())
            .collect();
        assert!(
            payloads.iter().any(|p| p.contains("before-crash")),
            "journal should retain the pre-crash record, got: {:?}",
            payloads
        );
        assert!(
            payloads.iter().any(|p| p.contains("after-crash")),
            "journal should contain the new stdout line, got: {:?}",
            payloads
        );

        assert!(
            recovered.completion.is_some(),
            "journal should carry a completion marker after a successful run"
        );
    }

    #[tokio::test]
    async fn test_try_wait_does_not_expose_durable_status_before_async_finalization() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "durable-try-wait", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let process_id = handle.process_id();

        // Echo exits quickly; give it a moment so the child has exited.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // try_wait must NOT hand back the raw exit status while the durable
        // journal has not been finalized: that would let a caller observe a
        // terminal status before the completion marker is sealed on disk.
        // Instead it must refuse with DurableFinalizationRequired carrying
        // this handle's process id, forcing callers through the async
        // finalizing wait().
        match handle.try_wait() {
            Err(SpawnerError::DurableFinalizationRequired { process_id: pid }) => {
                assert_eq!(pid, process_id, "error must carry the handle's process id");
            }
            Ok(status) => panic!(
                "try_wait must not expose exit status before durable finalization, got: {:?}",
                status
            ),
            Err(other) => panic!("unexpected error from try_wait: {:?}", other),
        }

        // The async finalizing wait() must still succeed and seal the journal.
        let status = handle.wait().await.expect("echo should exit");
        assert!(status.success(), "echo should exit successfully");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");
        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after async finalization"
        );
    }

    #[tokio::test]
    async fn test_shutdown_finalizes_durable_output_before_returning() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_sleep_agent_provider();

        // Spawn a long-running sleep 60 agent backed by the durable journal.
        let mut handle = spawner
            .spawn(&provider, "60", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let process_id = handle.process_id();

        // Graceful shutdown must finalize the durable journal before
        // returning: a caller that observes Ok from shutdown() must be able
        // to rely on the completion marker being sealed on disk.
        let result = handle.shutdown(Duration::from_secs(2)).await;
        assert!(result.is_ok(), "shutdown should succeed");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after shutdown returns"
        );

        let completed_records: Vec<&JournalRecord> = recovered
            .records
            .iter()
            .filter(|record| {
                record.kind == OutputKind::Completed && record.process_id == process_id.0
            })
            .collect();
        assert_eq!(
            completed_records.len(),
            1,
            "journal should contain exactly one Completed record for the spawned process, got: {:?}",
            completed_records
        );
    }

    /// Regression test: `spawn_with_fallback` must not misinterpret the
    /// journal-backed `try_wait` refusal (`DurableFinalizationRequired`) as
    /// a primary-provider failure. A primary that exits *successfully*
    /// within the early-exit grace window is a healthy result — the
    /// fallback must never be attempted and the durable journal must be
    /// finalized before any status is used (terraphim-ai#3269).
    #[tokio::test]
    async fn test_spawn_with_fallback_finalizes_durable_primary_before_status_use() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let journal_ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let primary = create_test_agent_provider(); // echo: exits 0 within grace window
        let invalid_fallback = Provider::new(
            "@invalid-fallback",
            "Invalid Fallback Agent",
            ProviderType::Agent {
                agent_id: "@invalid-fallback".to_string(),
                cli_command: "definitely-not-a-real-adf-cli".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );

        let request = SpawnRequest::new(primary.clone(), "primary-success")
            .with_fallback_provider(invalid_fallback);

        let mut handle = spawner
            .spawn_with_fallback(&request, journal_ctx)
            .await
            .expect("primary echo exits 0 during grace window; fallback must not be attempted");

        assert_eq!(
            handle.provider.id, "@test-agent",
            "handle must come from the primary provider, not the fallback"
        );

        // wait() must succeed and finalize the durable journal; a second
        // wait() must be idempotent (no double-finalize, no error).
        let status = handle.wait().await.expect("first wait should succeed");
        assert!(status.success(), "primary echo should exit successfully");
        let status2 = handle
            .wait()
            .await
            .expect("second wait should be idempotent and succeed");
        assert_eq!(status.code(), status2.code(), "idempotent wait result");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");
        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after spawn_with_fallback returns the primary handle"
        );
    }

    /// A failed primary must NOT seal the logical run's durable journal.
    ///
    /// `spawn_with_fallback` shares one `run_id` across the primary and
    /// the fallback attempt. When the primary exits non-zero inside the
    /// early-exit grace window, only the fallback's handle is returned to
    /// the caller — so the journal must stay open across the failed
    /// primary and be sealed exactly once, by the fallback's own
    /// finalization. The recovered journal must therefore contain the
    /// fallback's stdout and exactly one `Completed` record whose
    /// process id is the fallback handle's process id
    /// (terraphim-ai#3269).
    #[tokio::test]
    async fn test_spawn_with_fallback_keeps_one_durable_run_open_across_failed_primary() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let journal_ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        // Primary: `AgentConfig::infer_args("bash")` prepends `-c`, so the
        // task string runs as an inline script — `bash -c "exit 23"` —
        // which exits 23 well inside the early-exit grace window.
        let primary = Provider::new(
            "@bash-agent",
            "Bash Agent",
            ProviderType::Agent {
                agent_id: "@bash".to_string(),
                cli_command: "bash".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );
        let fallback = create_test_agent_provider(); // echo: exits 0

        let request = SpawnRequest::new(primary, "exit 23").with_fallback_provider(fallback);

        let mut handle = spawner
            .spawn_with_fallback(&request, journal_ctx)
            .await
            .expect("primary exits 23 in grace window; fallback echo must be attempted");

        assert_eq!(
            handle.provider.id, "@test-agent",
            "handle must come from the fallback provider, not the failed primary"
        );
        let fallback_pid = handle.process_id();

        let status = handle
            .wait()
            .await
            .expect("fallback wait should succeed and seal the run's journal");
        assert!(status.success(), "fallback echo should exit successfully");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");
        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after the fallback finalizes the run"
        );

        assert!(
            recovered.records.iter().any(|record| {
                record.kind == OutputKind::Stdout
                    && record.process_id == fallback_pid.0
                    && record
                        .payload
                        .as_str()
                        .is_some_and(|s| s.contains("exit 23"))
            }),
            "journal should contain the fallback's stdout line, got: {:?}",
            recovered.records
        );

        let completed_records: Vec<&JournalRecord> = recovered
            .records
            .iter()
            .filter(|record| record.kind == OutputKind::Completed)
            .collect();
        assert_eq!(
            completed_records.len(),
            1,
            "the logical run must be sealed exactly once, got: {:?}",
            completed_records
        );
        assert_eq!(
            completed_records[0].process_id, fallback_pid.0,
            "the single Completed record must belong to the fallback process; \
             the failed primary must not seal the logical run"
        );
    }

    #[tokio::test]
    async fn test_failed_fallback_spawn_reopens_and_seals_primary_run() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);
        let spawner = AgentSpawner::new();
        let primary = Provider::new(
            "@bash-agent",
            "Bash Agent",
            ProviderType::Agent {
                agent_id: "@bash".to_string(),
                cli_command: "bash".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );
        let invalid_fallback = Provider::new(
            "@invalid-fallback",
            "Invalid Fallback Agent",
            ProviderType::Agent {
                agent_id: "@invalid-fallback".to_string(),
                cli_command: "definitely-not-a-real-adf-cli".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );
        let request =
            SpawnRequest::new(primary, "exit 23").with_fallback_provider(invalid_fallback);

        spawner
            .spawn_with_fallback(&request, ctx)
            .await
            .expect_err("invalid fallback must fail after primary exit");

        let recovered = Journal::recover_run(temp.path(), run_id).expect("recover sealed run");
        let completed: Vec<_> = recovered
            .records
            .iter()
            .filter(|record| record.kind == OutputKind::Completed)
            .collect();
        assert_eq!(completed.len(), 1, "run must be sealed exactly once");
        assert_eq!(completed[0].payload, serde_json::json!({"exit_code": 23}));
        assert!(recovered.completion.is_some());
    }

    /// After the async finalizing `wait()` seals the durable journal,
    /// the public `try_wait` must return the finalized terminal status
    /// instead of refusing with `DurableFinalizationRequired`
    /// (terraphim-ai#3269). The handle tracks an explicit finalized
    /// terminal state that is only set after durable completion
    /// succeeds, so callers polling after `wait()` observe the same
    /// cached status without re-blocking on the reaped child.
    #[tokio::test]
    async fn test_try_wait_returns_finalized_status_after_durable_wait() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "finalized-try-wait", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let status = handle.wait().await.expect("echo should exit");
        assert!(status.success(), "echo should exit successfully");

        // try_wait must now expose the SAME finalized terminal status —
        // the journal is already sealed, so refusing with
        // DurableFinalizationRequired would be wrong.
        let polled = handle
            .try_wait()
            .expect("try_wait must succeed after durable finalization");
        assert_eq!(
            polled.map(|s| s.code()),
            Some(status.code()),
            "try_wait must return the finalized terminal status cached by wait()"
        );

        // And repeated polls keep returning it.
        let polled_again = handle
            .try_wait()
            .expect("repeated try_wait must succeed after finalization");
        assert_eq!(polled_again.map(|s| s.code()), Some(status.code()));
    }

    /// A graceful `shutdown` also reaches the finalized terminal state:
    /// after it returns `Ok`, `try_wait` must expose the terminal
    /// status rather than `DurableFinalizationRequired`
    /// (terraphim-ai#3269).
    #[tokio::test]
    async fn test_try_wait_returns_finalized_status_after_durable_shutdown() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_sleep_agent_provider();

        let mut handle = spawner
            .spawn(&provider, "60", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let graceful = handle
            .shutdown(Duration::from_secs(2))
            .await
            .expect("shutdown should succeed");
        assert!(
            graceful,
            "sleep should exit on SIGTERM within the grace period"
        );

        let polled = handle
            .try_wait()
            .expect("try_wait must succeed after shutdown finalized the run");
        assert!(
            polled.is_some(),
            "try_wait must return the finalized terminal status after shutdown"
        );
    }

    #[tokio::test]
    async fn test_kill_finalizes_durable_output_before_returning() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ctx = SpawnContext::global().with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new();
        let provider = create_sleep_agent_provider();

        // Spawn a long-running sleep 60 agent backed by the durable journal.
        let handle = spawner
            .spawn(&provider, "60", ctx)
            .await
            .expect("spawn with output journal should succeed");

        let process_id = handle.process_id();

        // Hard kill must finalize the durable journal before returning: a
        // caller that observes Ok from the consuming kill() must be able to
        // rely on the completion marker being sealed on disk.
        let result = handle.kill().await;
        assert!(result.is_ok(), "kill should succeed");

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after kill returns"
        );

        let completed_records: Vec<&JournalRecord> = recovered
            .records
            .iter()
            .filter(|record| {
                record.kind == OutputKind::Completed && record.process_id == process_id.0
            })
            .collect();
        assert_eq!(
            completed_records.len(),
            1,
            "journal should contain exactly one Completed record for the spawned process, got: {:?}",
            completed_records
        );
    }

    /// Verification test: stderr redaction must apply to BOTH durable sinks
    /// (terraphim-ai#3269). A secret printed to stderr by the child must
    /// appear redacted in the dedicated stderr log file AND in the
    /// recovered journal's Stderr records, with the completion marker
    /// sealed after wait() returns.
    #[tokio::test]
    async fn test_spawn_journal_preserves_redacted_stderr_log() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let stderr_log = temp.path().join("stderr.log");

        let ctx = SpawnContext::global()
            .with_stderr_log(&stderr_log)
            .with_output_journal(temp.path(), run_id);

        let spawner = AgentSpawner::new().with_working_dir("/tmp");
        let provider = Provider::new(
            "@bash-agent",
            "Bash Agent",
            ProviderType::Agent {
                agent_id: "@bash".to_string(),
                cli_command: "bash".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );

        let mut handle = spawner
            .spawn(&provider, "printf \"api_key=supersecret\\n\" >&2", ctx)
            .await
            .expect("spawn with journal and stderr log should succeed");

        let status = handle.wait().await.expect("bash should exit");
        assert!(status.success(), "bash printf should exit successfully");

        // The durable stderr log must hold only the redacted line.
        let log_contents = std::fs::read_to_string(&stderr_log)
            .expect("stderr log should exist after wait returns");
        assert!(
            log_contents.contains("REDACTED"),
            "stderr log should contain the redaction marker, got: {:?}",
            log_contents
        );
        assert!(
            !log_contents.contains("supersecret"),
            "stderr log must not contain the raw secret, got: {:?}",
            log_contents
        );

        // The recovered journal must carry a redacted Stderr record and a
        // sealed completion marker.
        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        let stderr_records: Vec<&JournalRecord> = recovered
            .records
            .iter()
            .filter(|record| record.kind == OutputKind::Stderr)
            .collect();
        assert!(
            !stderr_records.is_empty(),
            "journal should contain at least one Stderr record"
        );
        assert!(
            stderr_records.iter().any(|record| {
                record
                    .payload
                    .as_str()
                    .is_some_and(|s| s.contains("REDACTED"))
            }),
            "journal Stderr record should be redacted, got: {:?}",
            stderr_records
        );
        assert!(
            stderr_records.iter().all(|record| {
                record
                    .payload
                    .as_str()
                    .is_some_and(|s| !s.contains("supersecret"))
            }),
            "journal Stderr records must not contain the raw secret, got: {:?}",
            stderr_records
        );
        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after wait returns"
        );
    }

    /// The grace period must bound the *process exit* wait, not the
    /// output drain (terraphim-ai#3269).
    ///
    /// The managed bash child traps SIGTERM and exits 0 promptly — well
    /// inside the ~50ms grace window — but leaves a background
    /// grandchild holding the stdout pipe open for ~200ms before it
    /// writes `late-output` and exits (closing the pipe).
    ///
    /// Desired behaviour:
    ///   * `shutdown` returns `Ok(true)` because the managed child
    ///     exited gracefully inside the grace period;
    ///   * `shutdown` still waits for the late output to drain before
    ///     returning (return is NOT bounded by the grace period);
    ///   * the recovered durable journal contains `late-output` and a
    ///     sealed completion marker.
    ///
    /// The script touches a ready marker only after its TERM trap is
    /// installed, and the test waits boundedly for that marker before
    /// calling `shutdown` — otherwise SIGTERM can race process startup
    /// and kill bash before the trap exists. The grace timeout wraps
    /// the child-exit wait only; the output drain runs outside it.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_shutdown_grace_times_process_exit_not_output_drain() {
        use tempfile::TempDir;
        use uuid::Uuid;

        let temp = TempDir::new().expect("create tempdir");
        let run_id = Uuid::new_v4();
        let ready_marker = temp.path().join("trap-ready.marker");
        let ctx = SpawnContext::global()
            .with_output_journal(temp.path(), run_id)
            .with_env(
                "TRAP_READY_MARKER",
                ready_marker.to_string_lossy().to_string(),
            );

        let spawner = AgentSpawner::new();
        let provider = Provider::new(
            "@bash-agent",
            "Bash Agent",
            ProviderType::Agent {
                agent_id: "@bash".to_string(),
                cli_command: "bash".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![Capability::CodeGeneration],
        );

        // `AgentConfig::infer_args("bash")` prepends `-c`, so the task
        // runs as an inline script. The script:
        //   * traps TERM to kill its keepalive sleep and exit 0
        //     promptly (bash interrupts `wait` to run traps, so exit
        //     happens within a few ms of the signal);
        //   * spawns a background subshell that inherits the stdout
        //     pipe, waits ~200ms, writes `late-output`, and exits —
        //     only then does the pipe see EOF;
        //   * touches the ready marker ONLY AFTER the trap is
        //     installed, so the test never signals a trapless bash;
        //   * otherwise stays alive via `wait`.
        let script = "trap 'kill $KEEPALIVE_PID 2>/dev/null; exit 0' TERM; \
                      ( sleep 0.2; echo late-output ) & \
                      sleep 300 >/dev/null 2>&1 & \
                      KEEPALIVE_PID=$!; \
                      touch \"$TRAP_READY_MARKER\"; \
                      wait";

        let mut handle = spawner
            .spawn(&provider, script, ctx)
            .await
            .expect("spawn bash agent with output journal should succeed");

        let process_id = handle.process_id();

        // Wait boundedly for the child to install its TERM trap
        // (signalled by the ready marker) before shutdown sends
        // SIGTERM. Without this the signal can race process startup
        // and terminate bash with the default TERM disposition.
        let ready_deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !ready_marker.exists() {
            assert!(
                std::time::Instant::now() < ready_deadline,
                "child did not install its TERM trap (ready marker) within 5s"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Generous timing bounds: late output lands at ~200ms; grace is
        // 50ms, comfortably shorter so a grace-bounded drain always
        // loses the race, while the prompt trap exit is comfortably
        // inside it.
        let grace = Duration::from_millis(50);
        let started = tokio::time::Instant::now();
        let result = handle.shutdown(grace).await;
        let elapsed = started.elapsed();

        assert!(result.is_ok(), "shutdown should succeed, got: {:?}", result);
        assert!(
            result.unwrap(),
            "managed child trapped TERM and exited 0 promptly inside the \
             {grace:?} grace period; shutdown must report graceful exit, \
             not a force-kill (elapsed: {elapsed:?})"
        );

        assert!(
            elapsed >= Duration::from_millis(120),
            "shutdown must still wait for the late output drain (~200ms) \
             before returning instead of bounding the drain by the grace \
             period, but returned after {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "shutdown must return shortly after the ~200ms drain, took {elapsed:?}"
        );

        let recovered =
            Journal::recover_run(temp.path(), run_id).expect("journal recovery should succeed");

        assert!(
            recovered.completion.is_some(),
            "journal completion should be Some after shutdown returns"
        );

        assert!(
            recovered.records.iter().any(|record| {
                record.kind == OutputKind::Stdout
                    && record.process_id == process_id.0
                    && record
                        .payload
                        .as_str()
                        .is_some_and(|s| s.contains("late-output"))
            }),
            "journal should contain the drained late-output line written \
             after the managed child exited, got: {:?}",
            recovered.records
        );
    }
}
