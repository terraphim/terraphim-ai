//! Agent configuration and validation

use std::collections::HashMap;
use std::path::PathBuf;
use terraphim_types::capability::{Provider, ProviderType};

/// Resource limits for spawned agent processes.
///
/// These are lightweight process-level limits applied via `setrlimit(2)`.
/// For full sandboxing (VM isolation), use the `terraphim_firecracker` crate.
#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    /// Maximum virtual memory (bytes). Maps to RLIMIT_AS.
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU time (seconds). Maps to RLIMIT_CPU.
    pub max_cpu_seconds: Option<u64>,
    /// Maximum file size the process can create (bytes). Maps to RLIMIT_FSIZE.
    pub max_file_size_bytes: Option<u64>,
    /// Maximum number of open file descriptors. Maps to RLIMIT_NOFILE.
    pub max_open_files: Option<u64>,
}

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
    /// Resource limits for the spawned process
    pub resource_limits: ResourceLimits,
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
                args: Self::infer_args(cli_command),
                working_dir: Some(working_dir.clone()),
                env_vars: HashMap::new(),
                required_api_keys: Self::infer_api_keys(cli_command),
                resource_limits: ResourceLimits::default(),
            }),
            ProviderType::Llm { .. } => Err(ValidationError::NotAnAgent(provider.id.clone())),
        }
    }

    /// Set the model for this agent, adding appropriate CLI flags.
    pub fn with_model(mut self, model: &str) -> Self {
        let model_args = Self::model_args(&self.cli_command, model);
        self.args.extend(model_args);
        self
    }

    /// Extract the binary name from a CLI command (handles full paths).
    fn cli_name(cli_command: &str) -> &str {
        std::path::Path::new(cli_command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(cli_command)
    }

    /// Infer CLI-specific arguments for non-interactive execution.
    ///
    /// Each CLI tool has its own subcommand/flag for non-interactive mode:
    /// - codex: `exec <prompt>` runs a single task and exits
    /// - claude: `-p <prompt>` prints output without interactive UI
    /// - opencode: `run --format json` runs a single task and outputs JSON
    fn infer_args(cli_command: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            "codex" => vec!["exec".to_string(), "--full-auto".to_string()],
            "claude" | "claude-code" => vec![
                "-p".to_string(),
                "--allowedTools".to_string(),
                "Bash,Read,Write,Edit,Glob,Grep".to_string(),
            ],
            "opencode" => vec![
                "run".to_string(),
                "--format".to_string(),
                "json".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    /// Generate model-specific CLI arguments.
    fn model_args(cli_command: &str, model: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            "codex" => vec!["-m".to_string(), model.to_string()],
            "claude" | "claude-code" => vec!["--model".to_string(), model.to_string()],
            "opencode" => vec!["-m".to_string(), model.to_string()],
            _ => vec![],
        }
    }

    /// Infer required API keys from CLI command.
    ///
    /// Note: codex uses OAuth (ChatGPT login) and does not require OPENAI_API_KEY.
    fn infer_api_keys(cli_command: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            "claude" | "claude-code" => vec!["ANTHROPIC_API_KEY".to_string()],
            "opencode" => vec!["OPENAI_API_KEY".to_string()],
            _ => Vec::new(),
        }
    }

    /// Validate that the provider is not banned.
    /// Returns error if provider starts with a banned prefix.
    pub fn validate_provider(&self, banned_providers: &[String]) -> Result<(), ValidationError> {
        // Check the model string for banned provider prefixes
        // Model strings look like "opencode/kimi-k2.5" or "opencode-go/kimi-k2.5"
        for arg in &self.args {
            for banned in banned_providers {
                // Check for exact prefix "banned/" but NOT "banned-/" (e.g., "opencode/" vs "opencode-go/")
                if arg.starts_with(&format!("{}/", banned))
                    && !arg.starts_with(&format!("{}-", banned))
                {
                    return Err(ValidationError::BannedProvider(banned.clone(), arg.clone()));
                }
            }
        }
        Ok(())
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

    #[error("Banned provider prefix '{0}' detected in model: {1}. See ADR-002.")]
    BannedProvider(String, String),
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

        // If the command is a full path, check the file directly
        let path = std::path::Path::new(cmd);
        if path.is_absolute() {
            if path.exists() {
                return Ok(());
            }
            return Err(ValidationError::CliNotFound(cmd.clone()));
        }

        // Otherwise check if command exists in PATH
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
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert!(limits.max_memory_bytes.is_none());
        assert!(limits.max_cpu_seconds.is_none());
        assert!(limits.max_file_size_bytes.is_none());
        assert!(limits.max_open_files.is_none());
    }

    #[test]
    fn test_infer_api_keys() {
        let keys = AgentConfig::infer_api_keys("claude");
        assert!(keys.contains(&"ANTHROPIC_API_KEY".to_string()));

        let keys = AgentConfig::infer_api_keys("opencode");
        assert!(keys.contains(&"OPENAI_API_KEY".to_string()));

        let keys = AgentConfig::infer_api_keys("unknown");
        assert!(keys.is_empty());
    }

    #[test]
    fn test_infer_args_opencode() {
        let args = AgentConfig::infer_args("opencode");
        assert_eq!(
            args,
            vec![
                "run".to_string(),
                "--format".to_string(),
                "json".to_string()
            ]
        );
    }

    #[test]
    fn test_model_args_opencode() {
        let args = AgentConfig::model_args("opencode", "opencode-go/kimi-k2.5");
        assert_eq!(
            args,
            vec!["-m".to_string(), "opencode-go/kimi-k2.5".to_string()]
        );
    }

    #[test]
    fn test_model_args_with_provider_prefix() {
        // Test that opencode accepts provider-prefixed model strings
        let args = AgentConfig::model_args("opencode", "kimi-for-coding/k2p5");
        assert_eq!(
            args,
            vec!["-m".to_string(), "kimi-for-coding/k2p5".to_string()]
        );

        // Test with opencode-go prefix
        let args = AgentConfig::model_args("opencode", "opencode-go/glm-5");
        assert_eq!(
            args,
            vec!["-m".to_string(), "opencode-go/glm-5".to_string()]
        );
    }

    #[test]
    fn test_validate_provider_rejects_opencode_prefix() {
        let config = AgentConfig {
            agent_id: "test".to_string(),
            cli_command: "opencode".to_string(),
            args: vec!["-m".to_string(), "opencode/kimi-k2.5".to_string()],
            working_dir: None,
            env_vars: HashMap::new(),
            required_api_keys: vec![],
            resource_limits: ResourceLimits::default(),
        };

        let banned = vec!["opencode".to_string()];
        let result = config.validate_provider(&banned);
        assert!(result.is_err());
        match result {
            Err(ValidationError::BannedProvider(provider, model)) => {
                assert_eq!(provider, "opencode");
                assert_eq!(model, "opencode/kimi-k2.5");
            }
            _ => panic!("Expected BannedProvider error"),
        }
    }

    #[test]
    fn test_validate_provider_allows_opencode_go_prefix() {
        let config = AgentConfig {
            agent_id: "test".to_string(),
            cli_command: "opencode".to_string(),
            args: vec!["-m".to_string(), "opencode-go/kimi-k2.5".to_string()],
            working_dir: None,
            env_vars: HashMap::new(),
            required_api_keys: vec![],
            resource_limits: ResourceLimits::default(),
        };

        let banned = vec!["opencode".to_string()];
        assert!(config.validate_provider(&banned).is_ok());
    }

    #[test]
    fn test_validate_provider_allows_kimi_for_coding() {
        let config = AgentConfig {
            agent_id: "test".to_string(),
            cli_command: "opencode".to_string(),
            args: vec!["-m".to_string(), "kimi-for-coding/k2p5".to_string()],
            working_dir: None,
            env_vars: HashMap::new(),
            required_api_keys: vec![],
            resource_limits: ResourceLimits::default(),
        };

        let banned = vec!["opencode".to_string()];
        assert!(config.validate_provider(&banned).is_ok());
    }

    #[test]
    fn test_validate_provider_allows_all_when_empty_banned_list() {
        let config = AgentConfig {
            agent_id: "test".to_string(),
            cli_command: "opencode".to_string(),
            args: vec!["-m".to_string(), "opencode/kimi-k2.5".to_string()],
            working_dir: None,
            env_vars: HashMap::new(),
            required_api_keys: vec![],
            resource_limits: ResourceLimits::default(),
        };

        let banned: Vec<String> = vec![];
        assert!(config.validate_provider(&banned).is_ok());
    }

    #[test]
    fn test_validate_provider_multiple_args() {
        let config = AgentConfig {
            agent_id: "test".to_string(),
            cli_command: "opencode".to_string(),
            args: vec![
                "-m".to_string(),
                "kimi-for-coding/k2p5".to_string(),
                "--fallback".to_string(),
                "opencode/gpt-4".to_string(),
            ],
            working_dir: None,
            env_vars: HashMap::new(),
            required_api_keys: vec![],
            resource_limits: ResourceLimits::default(),
        };

        let banned = vec!["opencode".to_string()];
        let result = config.validate_provider(&banned);
        assert!(result.is_err());
        match result {
            Err(ValidationError::BannedProvider(provider, model)) => {
                assert_eq!(provider, "opencode");
                assert_eq!(model, "opencode/gpt-4");
            }
            _ => panic!("Expected BannedProvider error for second occurrence"),
        }
    }
}
