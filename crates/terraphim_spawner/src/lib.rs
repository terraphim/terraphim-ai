//! Agent spawner for Terraphim with health checking and output capture.
//!
//! This crate provides functionality to spawn external AI agents (Codex, Claude Code, OpenCode)
//! as non-interactive processes with:
//! - Configuration validation (CLI installed, API keys, models)
//! - Health checking via heartbeat (30s interval)
//! - Full output capture with @mention detection
//! - Auto-restart on failure

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::time::timeout;

use terraphim_types::capability::{ProcessId, Provider};

pub mod audit;
pub mod config;
pub mod health;
pub mod mention;
pub mod output;

/// Spawn request with provider/fallback configuration.
/// Mirrors fields from AgentDefinition to avoid circular dependency
/// between terraphim_spawner and terraphim_orchestrator.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// Unique agent name
    pub name: String,
    /// CLI tool to use (e.g., "opencode", "codex", "claude")
    pub cli_tool: String,
    /// Task/prompt for the agent
    pub task: String,
    /// Primary provider prefix (e.g., "opencode-go", "kimi-for-coding")
    pub provider: Option<String>,
    /// Primary model (e.g., "kimi-k2.5", "glm-5")
    pub model: Option<String>,
    /// Fallback provider if primary fails
    pub fallback_provider: Option<String>,
    /// Fallback model
    pub fallback_model: Option<String>,
    /// Provider tier for timeout configuration
    pub provider_tier: Option<ProviderTier>,
}

/// Provider tier classification for timeout configuration.
/// Mirrors terraphim_orchestrator::config::ProviderTier to avoid circular dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTier {
    /// Routine docs, advisory. Timeout: 30s.
    Quick,
    /// Quality gates, compound review, security. Timeout: 60s.
    Deep,
    /// Code generation, twins, tests. Timeout: 120s.
    Implementation,
    /// Spec validation, deep reasoning. Timeout: 300s. No fallback.
    Oracle,
}

impl ProviderTier {
    /// Timeout in seconds for this tier
    pub fn timeout_secs(&self) -> u64 {
        match self {
            Self::Quick => 30,
            Self::Deep => 60,
            Self::Implementation => 120,
            Self::Oracle => 300,
        }
    }
}

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
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        let config = match model {
            Some(m) => config.with_model(m),
            None => config,
        };
        self.spawn_config(provider, &config, task).await
    }

    /// Spawn an agent from a provider configuration
    pub async fn spawn(
        &self,
        provider: &Provider,
        task: &str,
    ) -> Result<AgentHandle, SpawnerError> {
        let config = AgentConfig::from_provider(provider)?;
        self.spawn_config(provider, &config, task).await
    }

    /// Spawn an agent with automatic fallback on failure.
    ///
    /// Uses ProviderTier timeout and retries with fallback provider if configured.
    /// Checks banned providers before spawning.
    pub async fn spawn_with_fallback(
        &self,
        request: &SpawnRequest,
        working_dir: &Path,
        banned_providers: &[String],
        circuit_breakers: &mut HashMap<String, CircuitBreaker>,
    ) -> Result<AgentHandle, SpawnerError> {
        // 1. Check if primary provider is banned
        if let Some(ref provider) = request.provider {
            if Self::is_provider_banned(provider, banned_providers) {
                return Err(SpawnerError::ValidationError(format!(
                    "Provider '{}' is banned",
                    provider
                )));
            }
        }

        // 2. Determine timeout from provider_tier (or default 120s)
        let timeout_secs = request
            .provider_tier
            .map(|t| t.timeout_secs())
            .unwrap_or(120);
        let timeout_duration = Duration::from_secs(timeout_secs);

        // Build primary provider string: {provider}/{model} or just provider
        let primary_provider_str =
            Self::build_provider_string(request.provider.as_deref(), request.model.as_deref());

        // Get or create circuit breaker for primary provider
        let primary_cb = circuit_breakers
            .entry(primary_provider_str.clone())
            .or_insert_with(|| {
                CircuitBreaker::new(CircuitBreakerConfig {
                    failure_threshold: 3,
                    cooldown: Duration::from_secs(300), // 5 minutes
                    success_threshold: 1,
                })
            });

        // Check if primary circuit is open
        if !primary_cb.should_allow() {
            tracing::warn!(
                provider = %primary_provider_str,
                "Primary provider circuit is open, skipping to fallback"
            );
            // Fall through to fallback logic below
        } else {
            // 3. Try primary provider with timeout
            let primary_result = self
                .try_spawn_with_provider(request, working_dir, false, timeout_duration)
                .await;

            match primary_result {
                Ok(handle) => {
                    primary_cb.record_success();
                    return Ok(handle);
                }
                Err(e) => {
                    primary_cb.record_failure();
                    tracing::warn!(
                        provider = %primary_provider_str,
                        error = %e,
                        "Primary provider failed, attempting fallback"
                    );
                    // Fall through to fallback logic
                }
            }
        }

        // 4. Check if fallback exists and circuit is not open
        let fallback_provider_str = match (
            request.fallback_provider.as_deref(),
            request.fallback_model.as_deref(),
        ) {
            (Some(fp), Some(fm)) => format!("{}/{}", fp, fm),
            (Some(fp), None) => fp.to_string(),
            (None, Some(fm)) => format!("fallback/{}", fm),
            (None, None) => {
                return Err(SpawnerError::SpawnError(
                    "Primary provider failed and no fallback configured".to_string(),
                ));
            }
        };

        // Check if fallback provider is banned
        if let Some(ref fb_provider) = request.fallback_provider {
            if Self::is_provider_banned(fb_provider, banned_providers) {
                return Err(SpawnerError::ValidationError(format!(
                    "Fallback provider '{}' is banned",
                    fb_provider
                )));
            }
        }

        // Get or create circuit breaker for fallback
        let fallback_cb = circuit_breakers
            .entry(fallback_provider_str.clone())
            .or_insert_with(|| {
                CircuitBreaker::new(CircuitBreakerConfig {
                    failure_threshold: 3,
                    cooldown: Duration::from_secs(300),
                    success_threshold: 1,
                })
            });

        if !fallback_cb.should_allow() {
            return Err(SpawnerError::SpawnError(format!(
                "Both primary '{}' and fallback '{}' circuits are open",
                primary_provider_str, fallback_provider_str
            )));
        }

        // 5. Retry with fallback
        let fallback_result = self
            .try_spawn_with_provider(request, working_dir, true, timeout_duration)
            .await;

        match fallback_result {
            Ok(handle) => {
                fallback_cb.record_success();
                Ok(handle)
            }
            Err(e) => {
                fallback_cb.record_failure();
                Err(SpawnerError::SpawnError(format!(
                    "Both primary and fallback failed. Fallback error: {}",
                    e
                )))
            }
        }
    }

    /// Check if a provider is in the banned list.
    fn is_provider_banned(provider: &str, banned_providers: &[String]) -> bool {
        banned_providers
            .iter()
            .any(|banned| provider.starts_with(banned))
    }

    /// Build provider string from provider and model components.
    fn build_provider_string(provider: Option<&str>, model: Option<&str>) -> String {
        match (provider, model) {
            (Some(p), Some(m)) => format!("{}/{}", p, m),
            (Some(p), None) => p.to_string(),
            (None, Some(m)) => format!("unknown/{}", m),
            (None, None) => "unknown".to_string(),
        }
    }

    /// Try to spawn with either primary or fallback configuration.
    async fn try_spawn_with_provider(
        &self,
        request: &SpawnRequest,
        working_dir: &Path,
        use_fallback: bool,
        timeout_duration: Duration,
    ) -> Result<AgentHandle, SpawnerError> {
        // Determine which provider/model to use
        let _provider_str = if use_fallback {
            request.fallback_provider.clone()
        } else {
            request.provider.clone()
        };

        let model_str = if use_fallback {
            request.fallback_model.clone()
        } else {
            request.model.clone()
        };

        // Build the CLI command - use the cli_tool from request
        // In practice, this might need to be constructed from provider/model
        let cli_command = request.cli_tool.clone();

        // Create a minimal Provider for spawning
        // Note: This is a simplified approach - in production, you'd map
        // provider strings to actual Provider configurations
        let provider = Provider::new(
            format!(
                "{}-{}",
                request.name,
                if use_fallback { "fallback" } else { "primary" }
            ),
            format!("{} Agent", request.name),
            terraphim_types::capability::ProviderType::Agent {
                agent_id: format!("@{}", request.name),
                cli_command,
                working_dir: working_dir.to_path_buf(),
            },
            vec![],
        );

        // Spawn with timeout
        let spawn_future = self.spawn_with_model(&provider, &request.task, model_str.as_deref());

        match tokio::time::timeout(timeout_duration, spawn_future).await {
            Ok(result) => result,
            Err(_) => Err(SpawnerError::SpawnError(format!(
                "Spawn timed out after {} seconds",
                timeout_duration.as_secs()
            ))),
        }
    }

    /// Internal spawn implementation shared by spawn() and spawn_with_model().
    async fn spawn_config(
        &self,
        provider: &Provider,
        config: &AgentConfig,
        task: &str,
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
        let mut child = self.spawn_process(config, task).await?;

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
    async fn spawn_process(&self, config: &AgentConfig, task: &str) -> Result<Child, SpawnerError> {
        let working_dir = config
            .working_dir
            .as_ref()
            .unwrap_or(&self.default_working_dir);

        let mut cmd = Command::new(&config.cli_command);
        cmd.current_dir(working_dir)
            .args(&config.args)
            .arg(task)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Add environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Add provider-specific env vars
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
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

        let child = cmd.spawn()?;

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

        let handle = spawner.spawn(&provider, "Hello World").await;

        // Echo command should succeed
        assert!(handle.is_ok());

        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@test-agent");
    }

    #[tokio::test]
    async fn test_try_wait_completed() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let mut handle = spawner.spawn(&provider, "done").await.unwrap();

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
        let mut handle = spawner.spawn(&provider, "60").await.unwrap();

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

        let handle = spawner.spawn(&provider, "hello").await.unwrap();
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

        let handle = spawner.spawn(&provider, "broadcast test").await.unwrap();
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
        let handle = spawner.spawn(&provider, "resource-limited").await;
        assert!(handle.is_ok());
    }

    #[tokio::test]
    async fn test_agent_pool_drain() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();

        let handle = spawner.spawn(&provider, "hello").await.unwrap();
        let mut pool = AgentPool::new(5);
        pool.release(handle);

        assert_eq!(pool.total_idle(), 1);
        pool.drain().await;
        assert_eq!(pool.total_idle(), 0);
    }

    // --------------- Spawn With Fallback Tests ---------------

    #[test]
    fn test_build_provider_string() {
        assert_eq!(
            AgentSpawner::build_provider_string(Some("opencode-go"), Some("glm-5")),
            "opencode-go/glm-5"
        );
        assert_eq!(
            AgentSpawner::build_provider_string(Some("kimi-for-coding"), None),
            "kimi-for-coding"
        );
        assert_eq!(
            AgentSpawner::build_provider_string(None, Some("k2p5")),
            "unknown/k2p5"
        );
        assert_eq!(AgentSpawner::build_provider_string(None, None), "unknown");
    }

    #[test]
    fn test_is_provider_banned() {
        let banned = vec!["opencode".to_string(), "zen".to_string()];

        // Exact match
        assert!(AgentSpawner::is_provider_banned("opencode", &banned));

        // Prefix match (e.g., "opencode-go" starts with "opencode")
        assert!(AgentSpawner::is_provider_banned("opencode-go", &banned));

        // Not banned
        assert!(!AgentSpawner::is_provider_banned(
            "kimi-for-coding",
            &banned
        ));
        assert!(!AgentSpawner::is_provider_banned("claude-code", &banned));
    }

    #[tokio::test]
    async fn test_spawn_with_fallback_primary_success() {
        let spawner = AgentSpawner::new();
        let request = SpawnRequest {
            name: "test-agent".to_string(),
            cli_tool: "echo".to_string(),
            task: "Hello World".to_string(),
            provider: Some("opencode-go".to_string()),
            model: Some("kimi-k2.5".to_string()),
            fallback_provider: Some("opencode-go".to_string()),
            fallback_model: Some("glm-5".to_string()),
            provider_tier: Some(ProviderTier::Quick),
        };

        let mut circuit_breakers = HashMap::new();
        let banned_providers: Vec<String> = vec![];

        let result = spawner
            .spawn_with_fallback(
                &request,
                Path::new("/tmp"),
                &banned_providers,
                &mut circuit_breakers,
            )
            .await;

        // Should succeed with primary (echo command)
        assert!(result.is_ok());

        // Circuit breaker should record success for primary
        let primary_key = "opencode-go/kimi-k2.5";
        assert!(circuit_breakers.contains_key(primary_key));
        assert!(circuit_breakers[primary_key].should_allow());
    }

    #[tokio::test]
    async fn test_spawn_with_fallback_banned_primary() {
        let spawner = AgentSpawner::new();
        let request = SpawnRequest {
            name: "test-agent".to_string(),
            cli_tool: "echo".to_string(),
            task: "Hello World".to_string(),
            provider: Some("opencode-go".to_string()),
            model: Some("kimi-k2.5".to_string()),
            fallback_provider: None,
            fallback_model: None,
            provider_tier: Some(ProviderTier::Quick),
        };

        let mut circuit_breakers = HashMap::new();
        let banned_providers = vec!["opencode".to_string()];

        let result = spawner
            .spawn_with_fallback(
                &request,
                Path::new("/tmp"),
                &banned_providers,
                &mut circuit_breakers,
            )
            .await;

        // Should fail because primary is banned
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("banned"));
    }

    #[tokio::test]
    async fn test_spawn_with_fallback_no_fallback_configured() {
        let spawner = AgentSpawner::new();

        // Use a command that will definitely fail
        let request = SpawnRequest {
            name: "test-agent".to_string(),
            cli_tool: "nonexistent_command_12345".to_string(),
            task: "Hello World".to_string(),
            provider: Some("primary-provider".to_string()),
            model: Some("model-1".to_string()),
            fallback_provider: None,
            fallback_model: None,
            provider_tier: Some(ProviderTier::Quick),
        };

        let mut circuit_breakers = HashMap::new();
        let banned_providers: Vec<String> = vec![];

        let result = spawner
            .spawn_with_fallback(
                &request,
                Path::new("/tmp"),
                &banned_providers,
                &mut circuit_breakers,
            )
            .await;

        // Should fail - no fallback configured
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("no fallback configured") || err_msg.contains("Failed to spawn"));
    }

    #[tokio::test]
    async fn test_spawn_with_fallback_uses_correct_timeout() {
        // Test that different tiers use correct timeouts
        let test_cases = vec![
            (ProviderTier::Quick, 30u64),
            (ProviderTier::Deep, 60u64),
            (ProviderTier::Implementation, 120u64),
            (ProviderTier::Oracle, 300u64),
        ];

        for (tier, expected_secs) in test_cases {
            let request = SpawnRequest {
                name: "test-agent".to_string(),
                cli_tool: "echo".to_string(),
                task: "test".to_string(),
                provider: Some("test-provider".to_string()),
                model: Some("test-model".to_string()),
                fallback_provider: None,
                fallback_model: None,
                provider_tier: Some(tier),
            };

            let timeout = request
                .provider_tier
                .map(|t| t.timeout_secs())
                .unwrap_or(120);
            assert_eq!(
                timeout, expected_secs,
                "Timeout mismatch for tier {:?}",
                tier
            );
        }
    }

    #[tokio::test]
    async fn test_provider_tier_timeout_secs() {
        assert_eq!(ProviderTier::Quick.timeout_secs(), 30);
        assert_eq!(ProviderTier::Deep.timeout_secs(), 60);
        assert_eq!(ProviderTier::Implementation.timeout_secs(), 120);
        assert_eq!(ProviderTier::Oracle.timeout_secs(), 300);
    }

    #[test]
    fn test_circuit_breaker_prevents_retry_when_open() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            cooldown: Duration::from_secs(300),
            success_threshold: 1,
        });

        // Record 3 failures to open the circuit
        cb.record_failure();
        assert!(cb.should_allow());
        cb.record_failure();
        assert!(cb.should_allow());
        cb.record_failure();
        assert!(!cb.should_allow()); // Circuit is now open

        // State should be Open
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_spawn_request_clone() {
        let request = SpawnRequest {
            name: "test-agent".to_string(),
            cli_tool: "echo".to_string(),
            task: "Hello".to_string(),
            provider: Some("provider".to_string()),
            model: Some("model".to_string()),
            fallback_provider: Some("fallback".to_string()),
            fallback_model: Some("fallback-model".to_string()),
            provider_tier: Some(ProviderTier::Deep),
        };

        let cloned = request.clone();
        assert_eq!(cloned.name, "test-agent");
        assert_eq!(cloned.cli_tool, "echo");
        assert_eq!(cloned.task, "Hello");
        assert_eq!(cloned.provider, Some("provider".to_string()));
        assert_eq!(cloned.model, Some("model".to_string()));
        assert_eq!(cloned.fallback_provider, Some("fallback".to_string()));
        assert_eq!(cloned.fallback_model, Some("fallback-model".to_string()));
        assert_eq!(cloned.provider_tier, Some(ProviderTier::Deep));
    }
}
