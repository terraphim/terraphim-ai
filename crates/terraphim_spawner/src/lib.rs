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
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};

use terraphim_types::capability::{ProcessId, Provider};

pub mod config;
pub mod health;
pub mod mention;
pub mod output;

pub use config::{AgentConfig, AgentValidator, ValidationError};
pub use health::{HealthChecker, HealthStatus};
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
    
    /// Kill the agent process
    pub async fn kill(mut self) -> Result<(), SpawnerError> {
        self.child.kill().await?;
        Ok(())
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
    
    /// Spawn an agent from a provider configuration
    pub async fn spawn(&self,
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
        let stdout = child.stdout.take().ok_or_else(|| 
            SpawnerError::SpawnError("Failed to capture stdout".to_string())
        )?;
        let stderr = child.stderr.take().ok_or_else(|| 
            SpawnerError::SpawnError("Failed to capture stderr".to_string())
        )?;
        
        let output_capture = OutputCapture::new(
            process_id,
            BufReader::new(stdout),
            BufReader::new(stderr),
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
    ) -> Result<Child, SpawnerError> {
        let working_dir = config.working_dir.as_ref()
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
    use terraphim_types::capability::{Capability, CostLevel, Latency, ProviderType};

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

    #[test]
    fn test_spawner_creation() {
        let spawner = AgentSpawner::new()
            .with_auto_restart(false)
            .with_working_dir("/workspace");
        
        assert!(!spawner.auto_restart);
        assert_eq!(spawner.default_working_dir, PathBuf::from("/workspace"));
    }

    #[tokio::test]
    async fn test_spawn_echo_agent() {
        let spawner = AgentSpawner::new();
        let provider = create_test_agent_provider();
        
        let handle = spawner.spawn(&provider, "Hello World"
        ).await;
        
        // Echo command should succeed
        assert!(handle.is_ok());
        
        let handle = handle.unwrap();
        assert_eq!(handle.provider.id, "@test-agent");
    }
}
