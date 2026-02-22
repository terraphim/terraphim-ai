//! Agent configuration and validation

use std::collections::HashMap;
use std::path::PathBuf;
use terraphim_types::capability::{Provider, ProviderType};

/// Configuration for an agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent identifier
    pub agent_id: String,
    /// CLI command to spawn the agent
    pub cli_command: String,
    /// Arguments to pass to the CLI
    pub args: Vec<String>,
    /// Working directory
    pub working_dir: Option<PathBuf>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Required API keys
    pub required_api_keys: Vec<String>,
}

impl AgentConfig {
    /// Create agent config from a provider
    pub fn from_provider(provider: &Provider) -> Result<Self, ValidationError> {
        match &provider.provider_type {
            ProviderType::Agent {
                agent_id,
                cli_command,
                working_dir,
            } => Ok(Self {
                agent_id: agent_id.clone(),
                cli_command: cli_command.clone(),
                args: Vec::new(),
                working_dir: Some(working_dir.clone()),
                env_vars: HashMap::new(),
                required_api_keys: Self::infer_api_keys(cli_command),
            }),
            ProviderType::Llm { .. } => Err(ValidationError::NotAnAgent(provider.id.clone())),
        }
    }

    /// Infer required API keys from CLI command
    fn infer_api_keys(cli_command: &str) -> Vec<String> {
        match cli_command {
            "claude" | "claude-code" => vec!["ANTHROPIC_API_KEY".to_string()],
            "opencode" | "codex" => vec!["OPENAI_API_KEY".to_string()],
            _ => Vec::new(),
        }
    }
}

/// Errors during agent validation
#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Provider {0} is not an agent")]
    NotAnAgent(String),

    #[error("CLI command not found: {0}")]
    CliNotFound(String),

    #[error("Required API key not set: {0}")]
    ApiKeyNotSet(String),

    #[error("Working directory does not exist: {0}")]
    WorkingDirNotFound(PathBuf),
}

/// Validator for agent configuration
pub struct AgentValidator {
    config: AgentConfig,
}

impl AgentValidator {
    /// Create a new validator
    pub fn new(config: &AgentConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Validate the agent configuration
    pub async fn validate(&self) -> Result<(), ValidationError> {
        // Check CLI command exists
        self.validate_cli().await?;

        // Check required API keys
        self.validate_api_keys().await?;

        // Check working directory
        self.validate_working_dir().await?;

        Ok(())
    }

    /// Validate CLI command exists
    async fn validate_cli(&self) -> Result<(), ValidationError> {
        let cmd = &self.config.cli_command;

        // Check if command exists in PATH
        let check = tokio::process::Command::new("which")
            .arg(cmd)
            .output()
            .await;

        match check {
            Ok(output) if output.status.success() => Ok(()),
            _ => Err(ValidationError::CliNotFound(cmd.clone())),
        }
    }

    /// Validate API keys are set
    async fn validate_api_keys(&self) -> Result<(), ValidationError> {
        for key in &self.config.required_api_keys {
            if std::env::var(key).is_err() {
                return Err(ValidationError::ApiKeyNotSet(key.clone()));
            }
        }
        Ok(())
    }

    /// Validate working directory exists
    async fn validate_working_dir(&self) -> Result<(), ValidationError> {
        if let Some(dir) = &self.config.working_dir {
            if !dir.exists() {
                return Err(ValidationError::WorkingDirNotFound(dir.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_api_keys() {
        let keys = AgentConfig::infer_api_keys("claude");
        assert!(keys.contains(&"ANTHROPIC_API_KEY".to_string()));

        let keys = AgentConfig::infer_api_keys("opencode");
        assert!(keys.contains(&"OPENAI_API_KEY".to_string()));

        let keys = AgentConfig::infer_api_keys("unknown");
        assert!(keys.is_empty());
    }
}
