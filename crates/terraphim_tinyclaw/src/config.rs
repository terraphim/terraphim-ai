use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root configuration for terraphim-tinyclaw.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration with environment variable expansion.
    pub fn from_file_with_env<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let expanded = expand_env_vars(&content);
        let config: Config = toml::from_str(&expanded)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.agent.validate()?;
        self.channels.validate()?;
        self.llm.validate()?;
        Ok(())
    }

    /// Default configuration file path.
    pub fn default_path() -> Option<PathBuf> {
        env_home::env_home_dir().map(|home| home.join(".config/terraphim/tinyclaw.toml"))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            llm: LlmConfig::default(),
            channels: ChannelsConfig::default(),
            tools: ToolsConfig::default(),
        }
    }
}

/// Agent behavior configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Maximum tool-calling iterations per message.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Workspace directory for files and sessions.
    pub workspace: PathBuf,

    /// Path to system prompt file (default: workspace/SYSTEM.md).
    pub system_prompt_file: Option<PathBuf>,

    /// Maximum messages per session before summarization.
    #[serde(default = "default_max_session_messages")]
    pub max_session_messages: usize,

    /// Default role to use on startup.
    pub default_role: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iterations(),
            workspace: PathBuf::from("."),
            system_prompt_file: None,
            max_session_messages: default_max_session_messages(),
            default_role: None,
        }
    }
}

impl AgentConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_iterations == 0 {
            anyhow::bail!("max_iterations must be greater than 0");
        }
        if self.max_session_messages == 0 {
            anyhow::bail!("max_session_messages must be greater than 0");
        }
        Ok(())
    }

    /// Get the system prompt file path, defaulting to workspace/SYSTEM.md.
    pub fn system_prompt_path(&self) -> PathBuf {
        self.system_prompt_file
            .clone()
            .unwrap_or_else(|| self.workspace.join("SYSTEM.md"))
    }
}

fn default_max_iterations() -> usize {
    20
}

fn default_max_session_messages() -> usize {
    200
}

/// Hybrid LLM configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    /// Proxy configuration for tool-calling and quality responses.
    pub proxy: ProxyConfig,

    /// Direct LLM configuration for compression and simple QA.
    pub direct: DirectLlmConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            direct: DirectLlmConfig::default(),
        }
    }
}

impl LlmConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.proxy.validate()?;
        Ok(())
    }
}

/// Proxy client configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// Base URL for the terraphim-llm-proxy.
    pub base_url: String,

    /// API key for proxy authentication.
    pub api_key: String,

    /// Request timeout in milliseconds.
    #[serde(default = "default_proxy_timeout")]
    pub timeout_ms: u64,

    /// Model override (optional - proxy decides if not set).
    pub model: Option<String>,

    /// Retry backoff after failure in seconds.
    #[serde(default = "default_retry_after")]
    pub retry_after_secs: u64,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3456".to_string(),
            api_key: String::new(),
            timeout_ms: default_proxy_timeout(),
            model: None,
            retry_after_secs: default_retry_after(),
        }
    }
}

impl ProxyConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.base_url.is_empty() {
            anyhow::bail!("proxy.base_url cannot be empty");
        }
        if self.api_key.is_empty() {
            log::warn!("proxy.api_key is empty - requests may fail");
        }
        Ok(())
    }
}

fn default_proxy_timeout() -> u64 {
    60_000
}

fn default_retry_after() -> u64 {
    60
}

/// Direct LLM configuration for cheap/local tasks.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DirectLlmConfig {
    /// Provider name (e.g., "ollama", "openai").
    pub provider: String,

    /// Model name (e.g., "llama3.2").
    pub model: String,

    /// Optional base URL override.
    pub base_url: Option<String>,
}

impl Default for DirectLlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "llama3.2".to_string(),
            base_url: Some("http://127.0.0.1:11434".to_string()),
        }
    }
}

/// Channel-specific configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChannelsConfig {
    #[cfg(feature = "telegram")]
    pub telegram: Option<TelegramConfig>,

    #[cfg(feature = "discord")]
    pub discord: Option<DiscordConfig>,
}

impl ChannelsConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        #[cfg(feature = "telegram")]
        if let Some(ref cfg) = self.telegram {
            cfg.validate()?;
        }

        #[cfg(feature = "discord")]
        if let Some(ref cfg) = self.discord {
            cfg.validate()?;
        }

        Ok(())
    }
}

/// Telegram channel configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    /// Bot token from @BotFather.
    pub token: String,

    /// List of allowed sender IDs (usernames or user IDs).
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl TelegramConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.token.is_empty() {
            anyhow::bail!("telegram.token cannot be empty");
        }
        if self.allow_from.is_empty() {
            anyhow::bail!(
                "telegram.allow_from cannot be empty - \
                 at least one user must be authorized for security"
            );
        }
        Ok(())
    }

    /// Check if a sender is allowed.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        self.allow_from.contains(&sender_id.to_string())
    }
}

/// Discord channel configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DiscordConfig {
    /// Bot token from Discord Developer Portal.
    pub token: String,

    /// List of allowed sender IDs (usernames or user IDs).
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl DiscordConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.token.is_empty() {
            anyhow::bail!("discord.token cannot be empty");
        }
        if self.allow_from.is_empty() {
            anyhow::bail!(
                "discord.allow_from cannot be empty - \
                 at least one user must be authorized for security"
            );
        }
        Ok(())
    }

    /// Check if a sender is allowed.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        self.allow_from.contains(&sender_id.to_string())
    }
}

/// Tool configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToolsConfig {
    /// Shell tool configuration.
    pub shell: Option<ShellToolConfig>,

    /// Web tools configuration.
    pub web: Option<WebToolsConfig>,
}

/// Shell tool configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShellToolConfig {
    /// Timeout for shell commands in seconds.
    #[serde(default = "default_shell_timeout")]
    pub timeout_seconds: u64,

    /// Additional shell deny patterns.
    #[serde(default)]
    pub deny_patterns: Vec<String>,
}

fn default_shell_timeout() -> u64 {
    120
}

/// Web tools configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebToolsConfig {
    /// Web search provider ("brave", "searxng", "google").
    pub search_provider: Option<String>,

    /// Web fetch mode ("readability", "raw").
    pub fetch_mode: Option<String>,
}

/// Expand environment variables in a string.
/// Supports $VAR and ${VAR} syntax.
fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();

    // Expand ${VAR} syntax
    let re = regex::Regex::new(r"\$\{(\w+)\}").unwrap();
    result = re
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
        })
        .to_string();

    // Expand $VAR syntax
    let re2 = regex::Regex::new(r"\$(\w+)").unwrap();
    result = re2
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
        })
        .to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_toml() {
        let toml = r#"
[agent]
workspace = "/tmp/tinyclaw"
max_iterations = 10

[llm.proxy]
base_url = "http://localhost:3456"
api_key = "example_only_not_real"

[llm.direct]
provider = "ollama"
model = "llama3.2"
"#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.agent.max_iterations, 10);
        assert_eq!(config.agent.workspace, PathBuf::from("/tmp/tinyclaw"));
        assert_eq!(config.llm.proxy.base_url, "http://localhost:3456");
        assert_eq!(config.llm.direct.provider, "ollama");
    }

    #[test]
    fn test_config_rejects_empty_allow_from() {
        #[cfg(feature = "telegram")]
        {
            let cfg = TelegramConfig {
                token: "test-token".to_string(),
                allow_from: vec![],
            };
            assert!(cfg.validate().is_err());
        }
    }

    #[test]
    fn test_telegram_allows_specified_users() {
        let cfg = TelegramConfig {
            token: "test".to_string(),
            allow_from: vec!["user1".to_string(), "user2".to_string()],
        };
        assert!(cfg.is_allowed("user1"));
        assert!(cfg.is_allowed("user2"));
        assert!(!cfg.is_allowed("user3"));
    }

    #[test]
    fn test_env_var_expansion() {
        unsafe {
            std::env::set_var("TEST_VAR", "test_value");
        }
        let input = "key = \"$TEST_VAR\"";
        let expanded = expand_env_vars(input);
        assert!(expanded.contains("test_value"));
    }

    #[test]
    fn test_agent_config_defaults() {
        let cfg = AgentConfig::default();
        assert_eq!(cfg.max_iterations, 20);
        assert_eq!(cfg.max_session_messages, 200);
        assert!(cfg.system_prompt_file.is_none());
    }

    #[test]
    fn test_system_prompt_path_default() {
        let cfg = AgentConfig {
            workspace: PathBuf::from("/workspace"),
            system_prompt_file: None,
            ..Default::default()
        };
        assert_eq!(
            cfg.system_prompt_path(),
            PathBuf::from("/workspace/SYSTEM.md")
        );
    }

    #[test]
    fn test_config_validation() {
        let mut cfg = AgentConfig::default();
        cfg.max_iterations = 0;
        assert!(cfg.validate().is_err());

        cfg.max_iterations = 1;
        assert!(cfg.validate().is_ok());
    }
}
