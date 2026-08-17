use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Root configuration for terraphim-tinyclaw.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    /// Credential pool configuration. **Default: disabled.** When
    /// `credentials.enabled = false`, the existing env-var expansion path
    /// remains in effect (rollback = config flag, no code revert).
    #[serde(default)]
    pub credentials: CredentialsConfig,

    /// MCP (Model Context Protocol) configuration. **Default: disabled.**
    /// When `mcp.enabled = true`, the MCP server exposes the 9-tool channel
    /// bridge over stdio. When `mcp.server_command` is set, the client
    /// connects to an external MCP server.
    #[serde(default)]
    pub mcp: McpConfig,

    /// Agent memory bridge configuration. **Default: disabled.**
    /// When `memory.enabled = true`, memory tools are registered and
    /// memory context is injected into the system prompt.
    #[serde(default)]
    pub memory: MemoryConfig,

    /// RLM sandbox configuration (#3146). **Default: disabled.**
    /// When `sandbox.enabled = true`, `SandboxTool` (rlm_code / rlm_bash /
    /// rlm_query + session ops) is registered, wrapping `terraphim_rlm`.
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// Subagent configuration (#3145). **Default: disabled.**
    /// When `subagent.enabled = true`, `SubagentTool` (spawn/status/list/
    /// terminate/collect) is registered, wrapping `terraphim_spawner`.
    #[serde(default)]
    pub subagent: SubagentConfig,

    /// Browser automation configuration (#3148). **Default: disabled.**
    /// When `browser.enabled = true`, `BrowserTool` (navigate/extract/api)
    /// is registered. Uses reqwest directly (deployed terraphim-agent
    /// binary has web_operations disabled).
    #[serde(default)]
    pub browser: BrowserConfig,

    /// Scheduler configuration (#3147). **Default: disabled.**
    /// When `scheduler.enabled = true`, `ScheduleTool` (create/list/delete)
    /// is registered for the agent loop; the `schedule` CLI subcommand
    /// shares the same store.
    #[serde(default)]
    pub scheduler: SchedulerConfig,

    /// Home Assistant configuration. **Default: disabled.**
    /// When `homeassistant.enabled = true`, the four HA tools
    /// (ha_list_entities / ha_get_state / ha_list_services / ha_call_service)
    /// are registered over the HA REST API.
    #[serde(default)]
    pub homeassistant: HomeAssistantConfig,

    /// Vision configuration. **Default: disabled.**
    /// When `vision.enabled = true`, the `vision_analyze` tool registers and
    /// sends multimodal chat-completion requests to an OpenAI-compatible
    /// vision model endpoint.
    #[serde(default)]
    pub vision: VisionConfig,

    /// Image generation configuration. **Default: disabled.**
    /// When `image_gen.enabled = true`, the `image_generate` tool registers
    /// against an OpenAI-compatible image endpoint (DALL-E style).
    #[serde(default)]
    pub image_gen: ImageGenConfig,

    /// Text-to-speech configuration. **Default: disabled.**
    /// When `tts.enabled = true`, the `text_to_speech` tool registers.
    #[serde(default)]
    pub tts: TtsConfig,

    /// Mixture-of-Agents configuration. **Default: disabled.**
    /// When `moa.enabled = true`, the `mixture_of_agents` tool registers.
    #[serde(default)]
    pub moa: MoaConfig,

    /// RL training configuration. **Default: disabled.**
    /// When `rl.enabled = true`, the `rl_check_status` tool registers to poll
    /// a rollout server's status endpoint.
    #[serde(default)]
    pub rl: RlConfig,

    /// Post-turn evolution trigger configuration (#3228, T2). **Default:
    /// disabled.** When `evolution.enabled = true`, each completed turn is
    /// evaluated by deterministic heuristics (ported from AutoClaw's
    /// `evaluatePostTurn`) and admitted turns invoke a proposer subagent
    /// whose only legal outputs are `NOTHING_TO_SAVE` or an `evo.propose`
    /// payload (TACP spec 5.1).
    #[serde(default)]
    pub evolution: crate::agent::evo_trigger::EvolutionConfig,
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
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct LlmConfig {
    /// Proxy configuration for tool-calling and quality responses.
    pub proxy: ProxyConfig,

    /// Direct LLM configuration for compression and simple QA.
    pub direct: DirectLlmConfig,
}

impl LlmConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.proxy.validate()?;
        Ok(())
    }
}

impl std::fmt::Debug for LlmConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmConfig")
            .field("proxy", &self.proxy)
            .field("direct", &self.direct)
            .finish()
    }
}

/// Proxy client configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// Base URL for the terraphim-llm-proxy.
    pub base_url: String,

    /// API key for proxy authentication.
    #[serde(default)]
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

impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("base_url", &self.base_url)
            .field("api_key", &"***REDACTED***")
            .field("timeout_ms", &self.timeout_ms)
            .field("model", &self.model)
            .field("retry_after_secs", &self.retry_after_secs)
            .finish()
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

    #[cfg(feature = "slack")]
    pub slack: Option<SlackConfig>,
    // Note: matrix config disabled due to sqlite dependency conflict
    // #[cfg(feature = "matrix")]
    // pub matrix: Option<MatrixConfig>,
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

        #[cfg(feature = "slack")]
        if let Some(ref cfg) = self.slack {
            cfg.validate()?;
        }

        // Note: matrix validation disabled due to sqlite dependency conflict
        // #[cfg(feature = "matrix")]
        // if let Some(ref cfg) = self.matrix {
        //     cfg.validate()?;
        // }

        Ok(())
    }
}

/// Telegram channel configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    /// Bot token from @BotFather.
    pub token: String,

    /// List of allowed sender IDs (usernames or user IDs).
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl std::fmt::Debug for TelegramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramConfig")
            .field("token", &"***REDACTED***")
            .field("allow_from", &self.allow_from)
            .finish()
    }
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
    /// Returns true if allow_from contains `"*"` (wildcard) or the given sender_id.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        crate::channel::is_sender_allowed(&self.allow_from, sender_id)
    }
}

/// Discord channel configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct DiscordConfig {
    /// Bot token from Discord Developer Portal.
    pub token: String,

    /// List of allowed sender IDs (usernames or user IDs).
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl std::fmt::Debug for DiscordConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordConfig")
            .field("token", &"***REDACTED***")
            .field("allow_from", &self.allow_from)
            .finish()
    }
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
    /// Returns true if allow_from contains `"*"` (wildcard) or the given sender_id.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        crate::channel::is_sender_allowed(&self.allow_from, sender_id)
    }
}

/// Slack channel configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct SlackConfig {
    /// Bot token (xoxb-...) from Slack App settings.
    pub bot_token: String,

    /// App-level token (xapp-...) for Socket Mode connections.
    pub app_token: String,

    /// List of allowed sender IDs (Slack user IDs like "U01234567").
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl std::fmt::Debug for SlackConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlackConfig")
            .field("bot_token", &"***REDACTED***")
            .field("app_token", &"***REDACTED***")
            .field("allow_from", &self.allow_from)
            .finish()
    }
}

impl SlackConfig {
    /// Validate the Slack configuration.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bot_token.trim().is_empty() {
            anyhow::bail!("slack.bot_token cannot be empty");
        }
        if self.app_token.trim().is_empty() {
            anyhow::bail!("slack.app_token cannot be empty");
        }
        if self.allow_from.is_empty() {
            anyhow::bail!(
                "slack.allow_from cannot be empty - \
                 at least one user must be authorized for security"
            );
        }
        Ok(())
    }

    /// Check if a sender is allowed.
    /// Returns true if allow_from contains `"*"` (wildcard) or the given sender_id.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        crate::channel::is_sender_allowed(&self.allow_from, sender_id)
    }
}

/// Matrix channel configuration for WhatsApp bridge.
#[derive(Clone, Deserialize, Serialize)]
pub struct MatrixConfig {
    /// Matrix homeserver URL (e.g., `https://matrix.example.com`)
    pub homeserver_url: String,
    /// Matrix username
    pub username: String,
    /// Matrix password
    pub password: String,
    /// List of allowed sender IDs (Matrix MXIDs like "@user:example.com")
    /// Must be non-empty for security.
    pub allow_from: Vec<String>,
}

impl std::fmt::Debug for MatrixConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatrixConfig")
            .field("homeserver_url", &self.homeserver_url)
            .field("username", &self.username)
            .field("password", &"***REDACTED***")
            .field("allow_from", &self.allow_from)
            .finish()
    }
}

impl MatrixConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.homeserver_url.is_empty() {
            anyhow::bail!("matrix.homeserver_url cannot be empty");
        }
        if self.username.is_empty() {
            anyhow::bail!("matrix.username cannot be empty");
        }
        if self.password.is_empty() {
            anyhow::bail!("matrix.password cannot be empty");
        }
        if self.allow_from.is_empty() {
            anyhow::bail!(
                "matrix.allow_from cannot be empty - \
                 at least one user must be authorized for security"
            );
        }
        Ok(())
    }

    /// Check if a sender is allowed.
    /// Returns true if allow_from contains `"*"` (wildcard) or the given sender_id.
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        crate::channel::is_sender_allowed(&self.allow_from, sender_id)
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
    /// Web search provider ("exa", "kimi_search").
    ///
    /// If not specified, falls back to environment variables
    /// (EXA_API_KEY or KIMI_API_KEY).
    pub search_provider: Option<String>,

    /// Web fetch mode ("raw", "readability").
    ///
    /// Defaults to "raw" if not specified.
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
    fn telegram_config_debug_redacts_token() {
        let cfg = TelegramConfig {
            token: "telegram-secret-12345".into(),
            allow_from: vec!["@alice".into()],
        };
        let out = format!("{:?}", cfg);
        assert!(!out.contains("telegram-secret-12345"));
        assert!(out.contains("***REDACTED***"));
        assert!(out.contains("@alice")); // non-secret field still rendered
    }

    #[test]
    fn discord_config_debug_redacts_token() {
        let cfg = DiscordConfig {
            token: "discord-secret-67890".into(),
            allow_from: vec!["bob".into()],
        };
        let out = format!("{:?}", cfg);
        assert!(!out.contains("discord-secret-67890"));
        assert!(out.contains("***REDACTED***"));
    }

    #[test]
    fn slack_config_debug_redacts_both_tokens() {
        let cfg = SlackConfig {
            bot_token: "xoxb-bot-secret".into(),
            app_token: "xapp-app-secret".into(),
            allow_from: vec!["U01234567".into()],
        };
        let out = format!("{:?}", cfg);
        assert!(!out.contains("xoxb-bot-secret"));
        assert!(!out.contains("xapp-app-secret"));
        // The redacted marker appears at least once per redacted field
        assert!(out.matches("***REDACTED***").count() >= 2);
    }

    #[test]
    fn matrix_config_debug_redacts_password_only() {
        let cfg = MatrixConfig {
            homeserver_url: "https://matrix.example.com".into(),
            username: "@user:example.com".into(),
            password: "matrix-secret-pw".into(),
            allow_from: vec!["@friend:example.com".into()],
        };
        let out = format!("{:?}", cfg);
        assert!(!out.contains("matrix-secret-pw"));
        assert!(out.contains("***REDACTED***"));
        // username + homeserver_url are not secrets; verify they render
        assert!(out.contains("@user:example.com"));
        assert!(out.contains("matrix.example.com"));
    }

    #[test]
    fn test_config_from_toml() {
        let toml = r#"
[agent]
workspace = "/tmp/tinyclaw"
max_iterations = 10

[llm.proxy]
base_url = "http://localhost:3456"

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
        // SAFETY: No other test in this binary reads or writes TEST_VAR.
        // Cargo runs tests in parallel threads by default; we accept this because
        // only this test touches this variable.
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
        let cfg = AgentConfig {
            max_iterations: 0,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());

        let cfg = AgentConfig {
            max_iterations: 1,
            ..Default::default()
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_slack_config_validate_valid() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test-token".to_string(),
            app_token: "xapp-test-token".to_string(),
            allow_from: vec!["U01234567".to_string()],
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_slack_config_validate_empty_bot_token() {
        let cfg = SlackConfig {
            bot_token: String::new(),
            app_token: "xapp-test-token".to_string(),
            allow_from: vec!["U01234567".to_string()],
        };
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("bot_token"));
    }

    #[test]
    fn test_slack_config_validate_empty_app_token() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test-token".to_string(),
            app_token: String::new(),
            allow_from: vec!["U01234567".to_string()],
        };
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("app_token"));
    }

    #[test]
    fn test_slack_config_validate_empty_allow_from() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test-token".to_string(),
            app_token: "xapp-test-token".to_string(),
            allow_from: vec![],
        };
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("allow_from"));
    }

    #[test]
    fn test_slack_config_is_allowed() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U111".to_string(), "U222".to_string()],
        };
        assert!(cfg.is_allowed("U111"));
        assert!(cfg.is_allowed("U222"));
        assert!(!cfg.is_allowed("U333"));
    }

    #[test]
    fn test_slack_config_is_allowed_wildcard() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["*".to_string()],
        };
        assert!(cfg.is_allowed("U111"));
        assert!(cfg.is_allowed("anyone"));
    }

    #[test]
    fn test_slack_config_validate_rejects_whitespace_only_tokens() {
        let cfg = SlackConfig {
            bot_token: "   ".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U111".to_string()],
        };
        assert!(
            cfg.validate().is_err(),
            "Whitespace-only bot_token should be rejected"
        );

        let cfg2 = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "  \t  ".to_string(),
            allow_from: vec!["U111".to_string()],
        };
        assert!(
            cfg2.validate().is_err(),
            "Whitespace-only app_token should be rejected"
        );
    }

    #[test]
    fn test_slack_config_is_allowed_case_sensitivity() {
        let cfg = SlackConfig {
            bot_token: "xoxb-test".to_string(),
            app_token: "xapp-test".to_string(),
            allow_from: vec!["U12345".to_string()],
        };
        assert!(cfg.is_allowed("U12345"), "Exact match should pass");
        assert!(
            !cfg.is_allowed("u12345"),
            "Lowercase variant should be rejected"
        );
    }

    #[test]
    fn test_proxy_config_debug_redacts_api_key() {
        let cfg = ProxyConfig {
            api_key: "super-secret-proxy-key".to_string(),
            base_url: "http://localhost:3456".to_string(),
            ..Default::default()
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("super-secret-proxy-key"),
            "api_key must not appear in ProxyConfig Debug output"
        );
        assert!(
            output.contains("***REDACTED***"),
            "Redaction marker must appear in ProxyConfig Debug output"
        );
    }

    #[test]
    fn test_telegram_config_debug_redacts_token() {
        let cfg = TelegramConfig {
            token: "secret-telegram-bot-token".to_string(),
            allow_from: vec!["user1".to_string()],
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("secret-telegram-bot-token"),
            "token must not appear in TelegramConfig Debug output"
        );
        assert!(output.contains("***REDACTED***"));
    }

    #[test]
    fn test_discord_config_debug_redacts_token() {
        let cfg = DiscordConfig {
            token: "secret-discord-bot-token".to_string(),
            allow_from: vec!["user1".to_string()],
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("secret-discord-bot-token"),
            "token must not appear in DiscordConfig Debug output"
        );
        assert!(output.contains("***REDACTED***"));
    }

    #[test]
    fn test_slack_config_debug_redacts_tokens() {
        let cfg = SlackConfig {
            bot_token: "xoxb-secret-bot-token".to_string(),
            app_token: "xapp-secret-app-token".to_string(),
            allow_from: vec!["U01234567".to_string()],
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("xoxb-secret-bot-token"),
            "bot_token must not appear in SlackConfig Debug output"
        );
        assert!(
            !output.contains("xapp-secret-app-token"),
            "app_token must not appear in SlackConfig Debug output"
        );
        assert!(output.contains("***REDACTED***"));
    }

    #[test]
    fn test_matrix_config_debug_redacts_password() {
        let cfg = MatrixConfig {
            homeserver_url: "https://matrix.example.com".to_string(),
            username: "bot_user".to_string(),
            password: "super-secret-matrix-password".to_string(),
            allow_from: vec!["@user:example.com".to_string()],
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("super-secret-matrix-password"),
            "password must not appear in MatrixConfig Debug output"
        );
        assert!(output.contains("***REDACTED***"));
        assert!(output.contains("matrix.example.com"));
        assert!(output.contains("bot_user"));
    }

    #[test]
    fn test_llm_config_debug_does_not_leak_api_key() {
        let cfg = LlmConfig {
            proxy: ProxyConfig {
                api_key: "secret-llm-api-key".to_string(),
                base_url: "http://localhost:3456".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let output = format!("{:?}", cfg);
        assert!(
            !output.contains("secret-llm-api-key"),
            "api_key must not appear in LlmConfig Debug output"
        );
        assert!(
            output.contains("***REDACTED***"),
            "Redaction marker must appear in LlmConfig Debug output"
        );
    }
}

// -----------------------------------------------------------------------------
// Credential pool configuration (Wave 1 of Hermes parity arc, epic #3160).
// -----------------------------------------------------------------------------

/// Credential pool configuration. When `enabled = false` (the default) the
/// existing env-var expansion path is used. When `enabled = true`, the
/// `HybridLlmRouter` consults the pool instead.
///
/// **Default behaviour: disabled.** Tinyclaw continues to honour `OPENROUTER_KEY`
/// etc. via the existing config expansion unless the operator explicitly
/// turns the pool on.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialsConfig {
    /// Master switch. `false` = use the legacy env-var path.
    /// `true` = use the credential pool.
    #[serde(default)]
    pub enabled: bool,

    /// Optional path to a `KEY=VALUE` env file (Hermes' `~/.hermes/.env`
    /// style). When set, an `EnvFileSource` is constructed at startup and
    /// used as the default source for the pool. When unset, an
    /// `EnvVarSource` is used (env-var lookups only).
    #[serde(default)]
    pub pool_file: Option<std::path::PathBuf>,

    /// Default cooldown applied by `report_throttle` when the caller does
    /// not supply one. Matches Hermes' 60-second default.
    #[serde(default = "default_credentials_cooldown_secs")]
    pub cooldown_secs: u64,

    /// Provider class the router should acquire from the pool (e.g.
    /// "openrouter"). When `None` or empty, the pool is not consulted even
    /// if `enabled = true`.
    #[serde(default)]
    pub provider_class: Option<String>,

    /// Pool entries as `provider=class:env_or_file` triples. Format:
    ///
    /// ```toml
    /// [[credentials.entries]]
    /// provider = "openrouter-primary"
    /// class = "openrouter"
    /// token_ref = { env_var = "OPENROUTER_KEY_1" }
    ///
    /// [[credentials.entries]]
    /// provider = "openrouter-fallback"
    /// class = "openrouter"
    /// token_ref = { file = "/etc/tinyclaw/openrouter-2.env" }
    /// ```
    ///
    /// Empty by default; pool becomes a no-op (every `acquire` returns
    /// `Exhausted`) unless entries are registered.
    #[serde(default)]
    pub entries: Vec<CredentialEntryConfig>,
}

/// TOML-friendly serialisation of `TokenRef`. Same shape as `TokenRef` but
/// uses `serde`'s `tag`-less externally-tagged enum so configs stay short.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TokenRefConfig {
    /// `token_ref = { env_var = "OPENROUTER_KEY" }`
    EnvVar {
        #[serde(rename = "env_var")]
        env_var: String,
    },
    /// `token_ref = { file = "/etc/tinyclaw/or.env" }`
    File { file: std::path::PathBuf },
}

impl From<TokenRefConfig> for crate::credentials::TokenRef {
    fn from(value: TokenRefConfig) -> Self {
        match value {
            TokenRefConfig::EnvVar { env_var } => {
                crate::credentials::TokenRef::EnvVar { name: env_var }
            }
            TokenRefConfig::File { file } => crate::credentials::TokenRef::File { path: file },
        }
    }
}

/// One credential entry in `CredentialsConfig.entries`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialEntryConfig {
    /// Provider identifier (e.g. `"openrouter-primary"`).
    pub provider: String,
    /// Provider class (e.g. `"openrouter"`). Multiple entries with the
    /// same class form a rotation pool.
    pub class: String,
    /// How to materialise the secret.
    pub token_ref: TokenRefConfig,
}

fn default_credentials_cooldown_secs() -> u64 {
    60
}

impl Default for CredentialsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pool_file: None,
            cooldown_secs: default_credentials_cooldown_secs(),
            provider_class: None,
            entries: Vec::new(),
        }
    }
}

/// MCP (Model Context Protocol) configuration.
///
/// **Default behaviour: disabled.** The MCP server is only started when
/// `enabled = true`. The client is only used when `server_command` is set.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpConfig {
    /// Master switch for the MCP server.
    #[serde(default)]
    pub enabled: bool,

    /// Optional external MCP server command for the client to connect to.
    /// Example: `"npx -y @modelcontextprotocol/server-everything stdio"`.
    #[serde(default)]
    pub server_command: Option<String>,
}

/// Agent memory bridge configuration. When `enabled = false` (the default),
/// no memory tools are registered and no memory context is injected.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    /// Master switch. `false` = memory bridge disabled.
    #[serde(default)]
    pub enabled: bool,

    /// Optional role for scoped memory retrieval. When set, memory
    /// retrieve/apply calls include `--role <role>`.
    #[serde(default)]
    pub role: Option<String>,

    /// Path to the terraphim-agent binary. Defaults to "terraphim-agent"
    /// (resolved via PATH lookup).
    #[serde(default = "default_agent_binary")]
    pub binary: String,

    /// Timeout for terraphim-agent subprocess calls in seconds.
    #[serde(default = "default_memory_timeout")]
    pub timeout_secs: u64,

    /// Maximum characters of memory context injected into the system
    /// prompt per request. Prevents token-budget overflow.
    #[serde(default = "default_max_context_chars")]
    pub max_context_chars: usize,

    /// Session memory backend for the agent loop: `"jsonl"` (default;
    /// per-session JSON-line files, preserving the existing on-disk
    /// layout) or `"sqlite"` (keyed JSON via
    /// `terraphim_persistence::DeviceStorage`). Unknown values fall back
    /// to `"jsonl"`.
    #[serde(default = "default_memory_backend")]
    pub backend: String,

    /// Explicit opt-in for the `"sqlite"` session backend. **Default:
    /// `false`.**
    ///
    /// The sqlite path currently persists session state through
    /// `DeviceStorage` while session *tools* (session_history,
    /// session_send, …) still read the jsonl `SessionManager` — a known
    /// split-brain session state (#3227 review P1). When this flag is
    /// `false`, a requested `backend = "sqlite"` is rejected with a
    /// warning and the loop falls back to jsonl, so the split-brain can
    /// only occur when a user deliberately opts in. Set to `true` only
    /// if you accept that caveat.
    #[serde(default)]
    pub allow_sqlite_backend: bool,
}

fn default_agent_binary() -> String {
    "terraphim-agent".to_string()
}

fn default_memory_timeout() -> u64 {
    10
}

fn default_max_context_chars() -> usize {
    4000
}

fn default_memory_backend() -> String {
    "jsonl".to_string()
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            role: None,
            binary: default_agent_binary(),
            timeout_secs: default_memory_timeout(),
            max_context_chars: default_max_context_chars(),
            backend: default_memory_backend(),
            allow_sqlite_backend: false,
        }
    }
}

/// RLM sandbox configuration (#3146).
///
/// **Default behaviour: disabled.** When enabled, `SandboxTool` wraps
/// `terraphim_rlm` for isolated code/shell execution with backend fallback.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxConfig {
    /// Master switch. `false` = no sandbox tools registered.
    #[serde(default)]
    pub enabled: bool,

    /// Execution backend preference: `"local"` (default) or `"docker"`.
    /// Firecracker/E2B are compile-time options of terraphim_rlm and are
    /// not selectable here.
    #[serde(default = "default_sandbox_backend")]
    pub backend: String,

    /// Per-execution timeout in seconds (RLM time budget).
    #[serde(default = "default_sandbox_timeout")]
    pub timeout_secs: u64,

    /// Maximum output bytes surfaced per execution result.
    #[serde(default = "default_sandbox_max_output")]
    pub max_output_bytes: usize,
}

fn default_sandbox_backend() -> String {
    "local".to_string()
}

fn default_sandbox_timeout() -> u64 {
    120
}

fn default_sandbox_max_output() -> usize {
    64 * 1024
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_sandbox_backend(),
            timeout_secs: default_sandbox_timeout(),
            max_output_bytes: default_sandbox_max_output(),
        }
    }
}

/// Subagent configuration (#3145).
///
/// **Default behaviour: disabled.** When enabled, `SubagentTool` wraps
/// `terraphim_spawner`'s AgentPool for isolated subagent lifecycle.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SubagentConfig {
    /// Master switch. `false` = no subagent tools registered.
    #[serde(default)]
    pub enabled: bool,

    /// Provider id used to spawn agents (maps to a Provider in
    /// terraphim_types::capability, e.g. `"claude-code"`).
    #[serde(default = "default_subagent_provider")]
    pub provider: String,

    /// Optional default model for spawned agents.
    #[serde(default)]
    pub model: Option<String>,

    /// Timeout for waiting on spawned agents in seconds.
    #[serde(default = "default_subagent_timeout")]
    pub timeout_secs: u64,
}

fn default_subagent_provider() -> String {
    "claude-code".to_string()
}

fn default_subagent_timeout() -> u64 {
    600
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_subagent_provider(),
            model: None,
            timeout_secs: default_subagent_timeout(),
        }
    }
}

/// Browser automation configuration (#3148).
///
/// **Default behaviour: disabled.** When enabled, `BrowserTool` provides
/// navigate / extract / api operations over reqwest. Browser-native ops
/// (click/type/screenshot) report `BackendUnavailable` because the
/// deployed terraphim-agent binary has web_operations compiled out.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrowserConfig {
    /// Master switch. `false` = no browser tools registered.
    #[serde(default)]
    pub enabled: bool,

    /// HTTP timeout in seconds for browser operations.
    #[serde(default = "default_browser_timeout")]
    pub timeout_secs: u64,

    /// Maximum response bytes captured per operation.
    #[serde(default = "default_browser_max_bytes")]
    pub max_bytes: usize,

    /// Optional proxy URL (e.g. `http://proxy:8080`).
    #[serde(default)]
    pub proxy: Option<String>,
}

fn default_browser_timeout() -> u64 {
    30
}

fn default_browser_max_bytes() -> usize {
    512 * 1024
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_secs: default_browser_timeout(),
            max_bytes: default_browser_max_bytes(),
            proxy: None,
        }
    }
}

/// Scheduler configuration (Hermes-parity cron surface, #3147).
///
/// Enables the `schedule` tool for the agent loop plus the
/// `terraphim-tinyclaw schedule` CLI subcommand. Jobs are persisted via
/// `terraphim_persistence::DeviceStorage` (same store type as the
/// dashboard cron CRUD).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchedulerConfig {
    /// Master switch. `false` = no schedule tool registered.
    #[serde(default)]
    pub enabled: bool,

    /// Storage key for the schedule job index document.
    #[serde(default = "default_scheduler_store_key")]
    pub store_key: String,
}

fn default_scheduler_store_key() -> String {
    "tinyclaw_schedules".to_string()
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            store_key: default_scheduler_store_key(),
        }
    }
}

/// Home Assistant configuration (Hermes parity).
///
/// **Default behaviour: disabled.** When `enabled = true` and `token` is set,
/// the HA tools register and talk to the HA REST API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HomeAssistantConfig {
    /// Master switch. `false` = no HA tools registered.
    #[serde(default)]
    pub enabled: bool,

    /// Base URL of the Home Assistant instance.
    #[serde(default = "default_hass_url")]
    pub url: String,

    /// Long-lived access token.
    #[serde(default)]
    pub token: String,
}

fn default_hass_url() -> String {
    "http://homeassistant.local:8123".to_string()
}

impl Default for HomeAssistantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: default_hass_url(),
            token: String::new(),
        }
    }
}

impl HomeAssistantConfig {
    /// Whether the HA tools are usable (enabled + token present).
    pub fn available(&self) -> bool {
        self.enabled && !self.token.is_empty()
    }
}

/// Vision configuration (Hermes parity).
///
/// **Default behaviour: disabled.** OpenAI-compatible multimodal endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VisionConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_vision_model")]
    pub model: String,

    #[serde(default = "default_vision_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub api_key: String,
}

fn default_vision_model() -> String {
    "google/gemini-3-flash-preview".to_string()
}

fn default_vision_base_url() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: default_vision_model(),
            base_url: default_vision_base_url(),
            api_key: String::new(),
        }
    }
}

impl VisionConfig {
    pub fn available(&self) -> bool {
        self.enabled && !self.api_key.is_empty()
    }
}

/// Image generation configuration (Hermes parity).
///
/// **Default behaviour: disabled.** OpenAI-compatible image endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageGenConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_image_model")]
    pub model: String,

    #[serde(default = "default_image_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub api_key: String,

    /// Enable the provider-side content safety checker. Defaults to true.
    #[serde(default = "default_true")]
    pub safety_checker: bool,
}

fn default_image_model() -> String {
    "fal-ai/flux-2-pro".to_string()
}

fn default_image_base_url() -> String {
    "https://fal.run".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for ImageGenConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: default_image_model(),
            base_url: default_image_base_url(),
            api_key: String::new(),
            safety_checker: true,
        }
    }
}

impl ImageGenConfig {
    pub fn available(&self) -> bool {
        self.enabled && !self.api_key.is_empty()
    }
}

/// Text-to-speech configuration (Hermes parity).
///
/// **Default behaviour: disabled.** Providers: `edge` (shells out to
/// `edge-tts` CLI) and `openai` (OpenAI-compatible `/v1/audio/speech`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtsConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_tts_provider")]
    pub provider: String,

    #[serde(default)]
    pub voice: String,

    #[serde(default = "default_tts_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub api_key: String,

    #[serde(default = "default_tts_output_dir")]
    pub output_dir: String,
}

fn default_tts_provider() -> String {
    "edge".to_string()
}

fn default_tts_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_tts_output_dir() -> String {
    "voice-memos".to_string()
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_tts_provider(),
            voice: String::new(),
            base_url: default_tts_base_url(),
            api_key: String::new(),
            output_dir: default_tts_output_dir(),
        }
    }
}

impl TtsConfig {
    pub fn available(&self) -> bool {
        // Edge TTS needs no key; OpenAI provider needs a key.
        if !self.enabled {
            return false;
        }
        self.provider.to_lowercase() == "edge" || !self.api_key.is_empty()
    }
}

/// Mixture-of-Agents configuration (Hermes parity).
///
/// **Default behaviour: disabled.** Ensemble of reference models + aggregator.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MoaConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub base_url: String,

    #[serde(default)]
    pub api_key: String,

    #[serde(default = "default_moa_reference_models")]
    pub reference_models: Vec<String>,

    #[serde(default = "default_moa_aggregator_model")]
    pub aggregator_model: String,
}

fn default_moa_reference_models() -> Vec<String> {
    vec![
        "openai/gpt-5.2-pro".to_string(),
        "anthropic/claude-opus-4.5".to_string(),
        "google/gemini-3-pro-preview".to_string(),
    ]
}

fn default_moa_aggregator_model() -> String {
    "anthropic/claude-opus-4.5".to_string()
}

impl Default for MoaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_vision_base_url(),
            api_key: String::new(),
            reference_models: default_moa_reference_models(),
            aggregator_model: default_moa_aggregator_model(),
        }
    }
}

impl MoaConfig {
    pub fn available(&self) -> bool {
        self.enabled && !self.api_key.is_empty() && !self.reference_models.is_empty()
    }
}

/// RL training configuration (Hermes parity, partial).
///
/// **Default behaviour: disabled.** The full veRL training orchestration from
/// Hermes `rl_training_tool.py` is a deliberate non-goal (deeply coupled to
/// Python/ray/wandb). This config exposes a monitorable `rl_check_status` tool
/// that polls a rollout server's status endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RlConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_rl_server_url")]
    pub rollout_server_url: String,
}

fn default_rl_server_url() -> String {
    "http://localhost:8000".to_string()
}

impl Default for RlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rollout_server_url: default_rl_server_url(),
        }
    }
}

impl RlConfig {
    pub fn available(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod credentials_config_tests {
    use super::*;

    #[test]
    fn credentials_config_default_is_disabled() {
        let cfg = CredentialsConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.pool_file.is_none());
        assert_eq!(cfg.cooldown_secs, 60);
        assert!(cfg.entries.is_empty());
    }

    #[test]
    fn credentials_config_round_trip() {
        let toml = r#"
enabled = true
pool_file = "/etc/tinyclaw/creds.env"
cooldown_secs = 30
provider_class = "openrouter"

[[entries]]
provider = "openrouter-primary"
class = "openrouter"
token_ref = { env_var = "OPENROUTER_KEY_1" }

[[entries]]
provider = "openrouter-fallback"
class = "openrouter"
token_ref = { file = "/etc/tinyclaw/or-2.env" }
"#;
        let cfg: CredentialsConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(
            cfg.pool_file.as_deref(),
            Some(std::path::Path::new("/etc/tinyclaw/creds.env"))
        );
        assert_eq!(cfg.cooldown_secs, 30);
        assert_eq!(cfg.provider_class.as_deref(), Some("openrouter"));
        assert_eq!(cfg.entries.len(), 2);
        assert_eq!(cfg.entries[0].provider, "openrouter-primary");
        assert_eq!(cfg.entries[1].provider, "openrouter-fallback");
    }

    #[test]
    fn credentials_config_missing_section_uses_defaults() {
        let toml = r#"
[agent]
max_iterations = 10
workspace = "/tmp/tinyclaw-test"
[llm]
[llm.proxy]
base_url = "http://x"
[llm.direct]
provider = "ollama"
model = "llama3"
"#;
        // Top-level Config defaults `credentials` when missing.
        let cfg: Config = toml::from_str(toml).expect("parse");
        assert!(!cfg.credentials.enabled);
        assert!(cfg.credentials.entries.is_empty());
        // Memory config also defaults when missing.
        assert!(!cfg.memory.enabled);
        assert!(cfg.memory.role.is_none());
    }
}

#[cfg(test)]
mod memory_config_tests {
    use super::*;

    #[test]
    fn memory_config_default_is_disabled() {
        let cfg = MemoryConfig::default();
        assert!(!cfg.enabled);
        assert!(cfg.role.is_none());
        assert_eq!(cfg.binary, "terraphim-agent");
        assert_eq!(cfg.timeout_secs, 10);
        assert_eq!(cfg.max_context_chars, 4000);
    }

    #[test]
    fn memory_config_round_trip() {
        let toml = r#"
enabled = true
role = "Terraphim Engineer"
binary = "/usr/local/bin/terraphim-agent"
timeout_secs = 30
max_context_chars = 8000
"#;
        let cfg: MemoryConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.role.as_deref(), Some("Terraphim Engineer"));
        assert_eq!(cfg.binary, "/usr/local/bin/terraphim-agent");
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.max_context_chars, 8000);
    }

    #[test]
    fn memory_config_missing_section_uses_defaults() {
        let toml = r#"
[agent]
max_iterations = 10
workspace = "/tmp/tinyclaw-test"
[llm]
[llm.proxy]
base_url = "http://x"
[llm.direct]
provider = "ollama"
model = "llama3"
"#;
        let cfg: Config = toml::from_str(toml).expect("parse");
        assert!(!cfg.memory.enabled);
        assert!(cfg.memory.role.is_none());
        assert_eq!(cfg.memory.binary, "terraphim-agent");
    }

    #[test]
    fn memory_config_partial_override() {
        let toml = r#"
enabled = true
"#;
        let cfg: MemoryConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert!(cfg.role.is_none());
        assert_eq!(cfg.binary, "terraphim-agent");
        assert_eq!(cfg.timeout_secs, 10);
    }

    #[test]
    fn memory_config_sqlite_gate_defaults_closed() {
        // #3227 review P1: the sqlite backend must be opt-in so the
        // split-brain session state can never be entered silently.
        let cfg = MemoryConfig::default();
        assert!(!cfg.allow_sqlite_backend);
        assert_eq!(cfg.backend, "jsonl");

        // Omitted from TOML → still false (serde default).
        let cfg: MemoryConfig = toml::from_str("enabled = true\n").expect("parse");
        assert!(!cfg.allow_sqlite_backend);
    }

    #[test]
    fn memory_config_sqlite_gate_parses_explicit_opt_in() {
        let toml = r#"
enabled = true
backend = "sqlite"
allow_sqlite_backend = true
"#;
        let cfg: MemoryConfig = toml::from_str(toml).expect("parse");
        assert_eq!(cfg.backend, "sqlite");
        assert!(cfg.allow_sqlite_backend);
    }
}

#[cfg(test)]
mod parity_tools_config_tests {
    use super::*;

    #[test]
    fn sandbox_config_defaults() {
        let cfg = SandboxConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.backend, "local");
        assert_eq!(cfg.timeout_secs, 120);
        assert_eq!(cfg.max_output_bytes, 64 * 1024);
    }

    #[test]
    fn sandbox_config_parse() {
        let toml = r#"
enabled = true
backend = "docker"
timeout_secs = 30
"#;
        let cfg: SandboxConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.backend, "docker");
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.max_output_bytes, 64 * 1024);
    }

    #[test]
    fn subagent_config_defaults() {
        let cfg = SubagentConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.provider, "claude-code");
        assert!(cfg.model.is_none());
        assert_eq!(cfg.timeout_secs, 600);
    }

    #[test]
    fn browser_config_defaults_and_parse() {
        let cfg = BrowserConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.timeout_secs, 30);

        let toml = r#"
enabled = true
max_bytes = 1024
proxy = "http://localhost:8080"
"#;
        let cfg: BrowserConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.max_bytes, 1024);
        assert_eq!(cfg.proxy.as_deref(), Some("http://localhost:8080"));
        assert_eq!(cfg.timeout_secs, 30);
    }

    #[test]
    fn scheduler_config_defaults_and_parse() {
        let cfg = SchedulerConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.store_key, "tinyclaw_schedules");

        let toml = r#"
enabled = true
store_key = "custom_schedules"
"#;
        let cfg: SchedulerConfig = toml::from_str(toml).expect("parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.store_key, "custom_schedules");
    }
}
