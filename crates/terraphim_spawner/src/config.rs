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
    /// Whether to deliver the task prompt via stdin instead of CLI arg
    pub use_stdin: bool,
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
                use_stdin: false,
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

    /// Set whether to deliver the task prompt via stdin.
    pub fn with_stdin(mut self, use_stdin: bool) -> Self {
        self.use_stdin = use_stdin;
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
    fn infer_args(cli_command: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            "codex" => vec!["exec".to_string(), "--full-auto".to_string()],
            "claude" | "claude-code" => vec![
                "-p".to_string(),
                "--allowedTools".to_string(),
                "Bash,Read,Write,Edit,Glob,Grep".to_string(),
            ],
            _ => Vec::new(),
        }
    }

    /// Normalise a model name for Claude CLI.
    ///
    /// Claude CLI requires the `claude-` prefix for versioned model names
    /// (e.g. `opus-4-6` -> `claude-opus-4-6`). Short aliases like `opus`
    /// or `sonnet` are passed through unchanged.
    fn normalise_claude_model(model: &str) -> String {
        if model.starts_with("claude-") {
            return model.to_string();
        }
        // Versioned names contain hyphens (e.g. "opus-4-6", "sonnet-4-6")
        // Short aliases do not (e.g. "opus", "sonnet", "haiku")
        if model.contains('-') {
            format!("claude-{}", model)
        } else {
            model.to_string()
        }
    }

    /// Generate model-specific CLI arguments.
    fn model_args(cli_command: &str, model: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            "codex" => vec!["-m".to_string(), model.to_string()],
            "claude" | "claude-code" => {
                let normalised = Self::normalise_claude_model(model);
                vec!["--model".to_string(), normalised]
            }
            _ => vec![],
        }
    }

    /// Infer required API keys from CLI command.
    ///
    /// Note: codex uses OAuth (ChatGPT login) and does not require OPENAI_API_KEY.
    fn infer_api_keys(cli_command: &str) -> Vec<String> {
        match Self::cli_name(cli_command) {
            // Claude CLI uses OAuth (browser flow), not API keys.
            // Do NOT require ANTHROPIC_API_KEY -- it poisons Claude CLI
            // by forcing API-key auth mode with an invalid value.
            "claude" | "claude-code" => Vec::new(),
            "opencode" => vec!["OPENAI_API_KEY".to_string()],
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
        // Claude CLI uses OAuth, not API keys -- should return empty
        let keys = AgentConfig::infer_api_keys("claude");
        assert!(
            keys.is_empty(),
            "claude uses OAuth, should not require API key"
        );

        let keys = AgentConfig::infer_api_keys("opencode");
        assert!(keys.contains(&"OPENAI_API_KEY".to_string()));

        let keys = AgentConfig::infer_api_keys("unknown");
        assert!(keys.is_empty());
    }

    #[test]
    fn test_infer_api_keys_full_path() {
        // Full paths should extract the binary name correctly
        let keys = AgentConfig::infer_api_keys("/home/alex/.local/bin/claude");
        assert!(keys.is_empty(), "claude via full path uses OAuth");

        let keys = AgentConfig::infer_api_keys("/home/alex/.bun/bin/opencode");
        assert!(keys.contains(&"OPENAI_API_KEY".to_string()));
    }

    #[test]
    fn test_normalise_claude_model() {
        // Already prefixed -- pass through
        assert_eq!(
            AgentConfig::normalise_claude_model("claude-opus-4-6"),
            "claude-opus-4-6"
        );
        assert_eq!(
            AgentConfig::normalise_claude_model("claude-sonnet-4-6"),
            "claude-sonnet-4-6"
        );

        // Versioned without prefix -- add prefix
        assert_eq!(
            AgentConfig::normalise_claude_model("opus-4-6"),
            "claude-opus-4-6"
        );
        assert_eq!(
            AgentConfig::normalise_claude_model("sonnet-4-6"),
            "claude-sonnet-4-6"
        );

        // Short aliases -- pass through (no hyphens)
        assert_eq!(AgentConfig::normalise_claude_model("opus"), "opus");
        assert_eq!(AgentConfig::normalise_claude_model("sonnet"), "sonnet");
        assert_eq!(AgentConfig::normalise_claude_model("haiku"), "haiku");
    }

    #[test]
    fn test_model_args_claude_normalises() {
        let args = AgentConfig::model_args("claude", "opus-4-6");
        assert_eq!(
            args,
            vec!["--model".to_string(), "claude-opus-4-6".to_string()]
        );

        let args = AgentConfig::model_args("claude", "claude-opus-4-6");
        assert_eq!(
            args,
            vec!["--model".to_string(), "claude-opus-4-6".to_string()]
        );

        let args = AgentConfig::model_args("claude", "sonnet");
        assert_eq!(args, vec!["--model".to_string(), "sonnet".to_string()]);
    }

    #[test]
    fn test_cli_name_extraction() {
        assert_eq!(
            AgentConfig::cli_name("/home/alex/.local/bin/claude"),
            "claude"
        );
        assert_eq!(
            AgentConfig::cli_name("/home/alex/.bun/bin/opencode"),
            "opencode"
        );
        assert_eq!(AgentConfig::cli_name("claude"), "claude");
        assert_eq!(AgentConfig::cli_name("/usr/bin/codex"), "codex");
    }
}
