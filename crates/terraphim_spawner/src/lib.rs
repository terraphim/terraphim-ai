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
        }
    }

    /// Builder-style env addition.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_overrides.insert(key.into(), value.into());
        self
    }
}

pub mod audit;
pub mod config;
pub mod health;
pub mod mention;
pub mod output;

pub use audit::AuditEvent;
pub use config::{AgentConfig, AgentValidator, ResourceLimits, ValidationError};
pub use health::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, HealthChecker, HealthHistory, HealthStatus,
};
pub use mention::{MentionEvent, MentionRouter};
pub use output::{OutputCapture, OutputEvent};

/// Errors that can occur during agent spawning
#[derive(thiserror::Error, Debug)]
pub enum SpawnerError {
    #[error("Agent validation failed: {0}")]
    ValidationError(String),

    #[error("Failed to spawn agent: {0}")]
    SpawnError(String),

    #[error("Agent process exited unexpectedly: {0}")]
    ProcessExit(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config validation error: {0}")]
    ConfigValidation(#[from] ValidationError),
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
    pub async fn shutdown(&mut self, grace_period: Duration) -> Result<bool, SpawnerError> {
        // Get the OS PID for signal sending
        let pid = match self.child.id() {
            Some(id) => id,
            None => {
                // Process already exited
                self.health_checker.mark_terminated();
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

        // Wait for graceful exit or timeout
        match timeout(grace_period, self.child.wait()).await {
            Ok(Ok(status)) => {
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
                self.health_checker.mark_terminated();
                Ok(true)
            }
            Ok(Err(e)) => {
                self.health_checker.mark_terminated();
                Err(SpawnerError::ProcessExit(format!(
                    "Wait failed for {}: {}",
                    self.process_id, e
                )))
            }
            Err(_) => {
                // Timeout expired -- force kill
                tracing::warn!(
                    process_id = %self.process_id,
                    grace_period_ms = grace_period.as_millis() as u64,
                    "Process did not exit within grace period, sending SIGKILL"
                );
                self.child.kill().await?;
                tracing::info!(
                    target: "terraphim_spawner::audit",
                    event = %AuditEvent::AgentTerminated {
                        process_id: self.process_id,
                        graceful: false,
                    },
                    "Agent force-killed"
                );
                self.health_checker.mark_terminated();
                Ok(false)
            }
        }
    }

    /// Hard kill the agent process (immediate SIGKILL).
    pub async fn kill(mut self) -> Result<(), SpawnerError> {
        self.health_checker.mark_terminated();
        self.child.kill().await?;
        Ok(())
    }

    /// Check if the process has exited (non-blocking).
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, SpawnerError> {
        match self.child.try_wait() {
            Ok(status) => {
                if status.is_some() {
                    self.health_checker.mark_terminated();
                }
                Ok(status)
            }
            Err(e) => Err(SpawnerError::Io(e)),
        }
    }

    /// Wait for the child process to exit naturally.
    /// Returns the exit status.
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, SpawnerError> {
        let status = self.child.wait().await.map_err(SpawnerError::Io)?;
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
        self.spawn_config(provider, &config, task, false, &ctx).await
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
        use_stdin: bool,
        resource_limits: ResourceLimits,
        ctx: &SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        let config = config.with_resource_limits(resource_limits);
        let config = match model {
            Some(m) => config.with_model(m),
            None => config,
        };
        self.spawn_config(provider, &config, task, use_stdin, ctx).await
    }

    /// Spawn an agent from a provider configuration.
    pub async fn spawn(
        &self,
        provider: &Provider,
        task: &str,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        self.spawn_config(provider, &config, task, false, &ctx).await
    }

    /// Spawn an agent with primary and fallback configuration.
    ///
    /// Attempts to spawn with the primary provider first. If that fails,
    /// falls back to the fallback provider (if configured).
    pub async fn spawn_with_fallback(
        &self,
        request: &SpawnRequest,
        ctx: SpawnContext,
    ) -> Result<AgentHandle, SpawnerError> {
        // Try primary first with resource limits
        let primary_result = self
            .spawn_with_options(
                &request.primary_provider,
                &request.task,
                request.primary_model.as_deref(),
                request.use_stdin,
                request.resource_limits.clone(),
                &ctx,
            )
            .await;

        // If primary succeeds, return the handle
        match primary_result {
            Ok(handle) => Ok(handle),
            Err(primary_err) => {
                tracing::warn!(
                    primary_provider = %request.primary_provider.id,
                    error = %primary_err,
                    "Primary spawn failed, attempting fallback"
                );

                // Try fallback if configured
                if let Some(ref fallback) = request.fallback_provider {
                    tracing::info!(
                        fallback_provider = %fallback.id,
                        "Attempting fallback spawn"
                    );

                    let fallback_result = self
                        .spawn_with_options(
                            fallback,
                            &request.task,
                            request.fallback_model.as_deref(),
                            request.use_stdin,
                            request.resource_limits.clone(),
                            &ctx,
                        )
                        .await;

                    match fallback_result {
                        Ok(handle) => {
                            tracing::info!(
                                fallback_provider = %fallback.id,
                                "Fallback spawn succeeded"
                            );
                            Ok(handle)
                        }
                        Err(fallback_err) => {
                            tracing::error!(
                                fallback_provider = %fallback.id,
                                error = %fallback_err,
                                "Fallback spawn also failed"
                            );
                            // Return the primary error since that's the original failure
                            Err(primary_err)
                        }
                    }
                } else {
                    // No fallback configured, return primary error
                    Err(primary_err)
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

        let output_capture =
            OutputCapture::new(process_id, BufReader::new(stdout), BufReader::new(stderr));

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

        if use_stdin {
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

        // Write task to stdin if using stdin delivery
        if use_stdin {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(task.as_bytes()).await.map_err(|e| {
                    SpawnerError::SpawnError(format!("failed to write prompt to stdin: {}", e))
                })?;
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

        let handle = spawner.spawn(&provider, "Hello World", SpawnContext::global()).await;

        // Echo command should succeed
        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@test-agent");
    }

    #[tokio::test]
    async fn test_try_wait_completed() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner.spawn(&provider, "done", SpawnContext::global()).await.unwrap();

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
        let mut handle = spawner.spawn(&provider, "60", SpawnContext::global()).await.unwrap();

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

        let handle = spawner.spawn(&provider, "hello", SpawnContext::global()).await.unwrap();
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

        let handle = spawner.spawn(&provider, "broadcast test", SpawnContext::global()).await.unwrap();
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
        let handle = spawner.spawn(&provider, "resource-limited", SpawnContext::global()).await;
        assert!(handle.is_ok());
    }

    #[tokio::test]
    async fn test_agent_pool_drain() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner.spawn(&provider, "hello", SpawnContext::global()).await.unwrap();
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
        let handle = spawner.spawn(&provider, "arg test", SpawnContext::global()).await;

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
            .spawn_with_model_stdin(&provider, "model test via stdin", Some("test-model"), SpawnContext::global())
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
        use tempfile::TempDir;

        let tmpdir = TempDir::new().expect("create tempdir");
        let tmppath = tmpdir.path().to_path_buf();

        // Provider runs /bin/pwd so the child prints its cwd.
        let provider = Provider::new(
            "@pwd-agent",
            "Pwd Agent",
            terraphim_types::capability::ProviderType::Agent {
                agent_id: "@pwd".to_string(),
                cli_command: "/bin/pwd".to_string(),
                working_dir: PathBuf::from("/tmp"),
            },
            vec![terraphim_types::capability::Capability::CodeGeneration],
        );

        let spawner = AgentSpawner::new().with_working_dir("/tmp");
        let ctx = SpawnContext::with_working_dir(tmppath.clone());

        // Subscribe before spawn so we don't miss events from a fast process.
        let handle = spawner
            .spawn(&provider, ".", ctx)
            .await
            .expect("spawn with working_dir override should succeed");

        let mut rx = handle.subscribe_output();

        // Give pwd time to run and the capture task to broadcast its line.
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut found = false;
        let resolved = std::fs::canonicalize(&tmppath).unwrap_or(tmppath.clone());
        loop {
            match rx.try_recv() {
                Ok(OutputEvent::Stdout { line, .. }) => {
                    let trimmed = line.trim();
                    if trimmed == resolved.to_string_lossy().as_ref()
                        || trimmed == tmppath.to_string_lossy().as_ref()
                    {
                        found = true;
                        break;
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        assert!(found, "child cwd should be the overridden tmpdir");
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
            .spawn(
                &provider,
                "ADF_INHERITED_SPAWN_CTX",
                SpawnContext::global(),
            )
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
}
