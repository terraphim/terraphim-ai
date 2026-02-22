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

pub mod config;
pub mod health;
pub mod mention;
pub mod output;

pub use config::{AgentConfig, AgentValidator, ValidationError};
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
                log::warn!("Failed to send SIGTERM to {}: {}", pid, e);
            } else {
                log::info!("Sent SIGTERM to process {} ({})", pid, self.process_id);
            }
        }

        // Wait for graceful exit or timeout
        match timeout(grace_period, self.child.wait()).await {
            Ok(Ok(status)) => {
                log::info!(
                    "Process {} exited gracefully with status: {}",
                    self.process_id,
                    status
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
                log::warn!(
                    "Process {} did not exit within {:?}, sending SIGKILL",
                    self.process_id,
                    grace_period
                );
                self.child.kill().await?;
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

    /// Spawn an agent from a provider configuration
    pub async fn spawn(
        &self,
        provider: &Provider,
        task: &str,
    ) -> Result<AgentHandle, SpawnerError> {
        // Validate the provider
        let config = AgentConfig::from_provider(provider)?;
        let validator = AgentValidator::new(&config);
        validator.validate().await?;

        // Spawn the agent process
        let process_id = ProcessId::new();
        let mut child = self.spawn_process(&config, task).await?;

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

        let child = cmd.spawn()?;

        Ok(child)
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
}
