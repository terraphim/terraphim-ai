//! Configuration management for the LLM proxy

use crate::routing::{ModelExclusion, ModelMapping, RoutingStrategy};
use crate::webhooks::WebhookSettings;
use crate::{ProxyError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported configuration file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    pub proxy: ProxySettings,
    pub router: RouterSettings,
    pub providers: Vec<Provider>,
    #[serde(default)]
    pub security: SecuritySettings,
    /// OAuth authentication settings for provider login flows.
    #[serde(default)]
    pub oauth: OAuthSettings,
    /// Management API settings for runtime configuration.
    #[serde(default)]
    pub management: ManagementSettings,
    /// Webhook settings for event notifications.
    #[serde(default)]
    pub webhooks: WebhookSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxySettings {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouterSettings {
    pub default: String,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub think: Option<String>,
    /// Plan implementation routing - for tactical/implementation tasks
    #[serde(default)]
    pub plan_implementation: Option<String>,
    #[serde(default)]
    pub long_context: Option<String>,
    #[serde(default = "default_long_context_threshold")]
    pub long_context_threshold: usize,
    #[serde(default)]
    pub web_search: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    /// Model name mappings with glob pattern support.
    /// Maps incoming model names to target models (e.g., "claude-sonnet-4-5-*" -> "anthropic/claude-sonnet-4.5")
    #[serde(default)]
    pub model_mappings: Vec<ModelMapping>,
    /// Model exclusion patterns per provider.
    /// Filters out unwanted models (e.g., "*-preview", "*-beta").
    #[serde(default)]
    pub model_exclusions: Vec<ModelExclusion>,
    /// Routing strategy for provider selection.
    /// Determines how providers are chosen when multiple are available.
    #[serde(default)]
    pub strategy: RoutingStrategy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Provider {
    pub name: String,
    pub api_base_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub transformers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SecuritySettings {
    #[serde(default)]
    pub rate_limiting: RateLimitingSettings,
    #[serde(default)]
    pub ssrf_protection: SsrfProtectionSettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitingSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: usize,
    #[serde(default = "default_concurrent")]
    pub concurrent_requests: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SsrfProtectionSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub allow_localhost: bool,
    #[serde(default = "default_false")]
    pub allow_private_ips: bool,
}

impl Default for RateLimitingSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: 60,
            concurrent_requests: 10,
        }
    }
}

impl Default for SsrfProtectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_localhost: false,
            allow_private_ips: false,
        }
    }
}

// ============================================================================
// OAuth Settings
// ============================================================================

/// OAuth authentication settings for LLM provider login flows.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthSettings {
    /// Storage backend for tokens ("file" or "redis").
    #[serde(default = "default_storage_backend")]
    pub storage_backend: String,

    /// Redis URL for token storage (when storage_backend = "redis").
    pub redis_url: Option<String>,

    /// Custom storage path for token files.
    /// Defaults to `~/.terraphim-llm-proxy/auth/` if not set.
    #[serde(default)]
    pub storage_path: Option<String>,

    /// Claude (Anthropic) OAuth settings.
    #[serde(default)]
    pub claude: OAuthProviderSettings,

    /// Gemini (Google) OAuth settings.
    #[serde(default)]
    pub gemini: OAuthProviderSettings,

    /// OpenAI (Codex) OAuth settings.
    #[serde(default)]
    pub openai: OAuthProviderSettings,

    /// GitHub Copilot OAuth settings (device flow).
    #[serde(default)]
    pub copilot: OAuthProviderSettings,
}

impl Default for OAuthSettings {
    fn default() -> Self {
        Self {
            storage_backend: default_storage_backend(),
            redis_url: None,
            storage_path: None,
            claude: OAuthProviderSettings::default(),
            gemini: OAuthProviderSettings::default(),
            openai: OAuthProviderSettings::default(),
            copilot: OAuthProviderSettings::default(),
        }
    }
}

/// Per-provider OAuth configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthProviderSettings {
    /// Whether this OAuth provider is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Port for OAuth callback server.
    #[serde(default = "default_callback_port")]
    pub callback_port: u16,

    /// OAuth client ID (for providers that require it).
    pub client_id: Option<String>,

    /// OAuth client secret (for providers that require it).
    pub client_secret: Option<String>,

    /// Auth mode for Claude OAuth: "bearer" (direct Bearer token) or "api_key" (create API key).
    /// Default: "api_key". Only used for Claude provider.
    #[serde(default)]
    pub auth_mode: Option<String>,

    /// Override default OAuth scopes for this provider.
    #[serde(default)]
    pub scopes: Option<Vec<String>>,

    /// Anthropic beta header value (e.g. "oauth-2025-04-20").
    /// Only used for Claude provider in Bearer mode.
    #[serde(default)]
    pub anthropic_beta: Option<String>,
}

impl Default for OAuthProviderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            callback_port: default_callback_port(),
            client_id: None,
            client_secret: None,
            auth_mode: None,
            scopes: None,
            anthropic_beta: None,
        }
    }
}

impl OAuthProviderSettings {
    /// Validate Claude-specific OAuth settings and log warnings.
    pub fn validate_claude_oauth(&self) {
        if !self.enabled {
            return;
        }

        if self.client_id.is_none() {
            tracing::warn!(
                "Claude OAuth enabled but no client_id configured. \
                 Set oauth.claude.client_id in config."
            );
        }

        match self.auth_mode.as_deref() {
            Some("bearer") => {
                tracing::info!("Claude OAuth: bearer token mode (direct API auth)");
                if self.anthropic_beta.is_none() {
                    tracing::info!(
                        "Claude OAuth: no anthropic_beta header configured. \
                         Set oauth.claude.anthropic_beta if needed."
                    );
                }
            }
            Some("api_key") => {
                tracing::info!("Claude OAuth: API key creation mode");
                // Check scopes include org:create_api_key
                if let Some(ref scopes) = self.scopes {
                    if !scopes.iter().any(|s| s == "org:create_api_key") {
                        tracing::warn!(
                            "Claude OAuth api_key mode may need 'org:create_api_key' scope"
                        );
                    }
                }
            }
            Some(other) => {
                tracing::warn!(
                    "Claude OAuth: unknown auth_mode '{}'. Valid values: 'bearer', 'api_key'",
                    other
                );
            }
            None => {
                tracing::debug!("Claude OAuth: no auth_mode set, OAuth tokens will not be used for API requests");
            }
        }
    }
}

fn default_storage_backend() -> String {
    "file".to_string()
}

fn default_callback_port() -> u16 {
    9999
}

// ============================================================================
// Management API Settings
// ============================================================================

/// Management API settings for runtime configuration and monitoring.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ManagementSettings {
    /// Whether the management API is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Secret key for management API authentication.
    /// Can be a literal value or environment variable reference (e.g., "$MANAGEMENT_SECRET").
    pub secret_key: Option<String>,

    /// Whether to allow management API access from non-localhost addresses.
    #[serde(default)]
    pub allow_remote: bool,

    /// Logging control settings.
    #[serde(default)]
    pub logging: ManagementLoggingSettings,
}

/// Logging control settings for the management API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManagementLoggingSettings {
    /// Default log level.
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Maximum number of log entries to retain for the logs endpoint.
    #[serde(default = "default_max_log_entries")]
    pub max_entries: usize,
}

impl Default for ManagementLoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            max_entries: default_max_log_entries(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_log_entries() -> usize {
    1000
}

impl ProxyConfig {
    /// Load configuration from TOML file with environment variable expansion
    pub fn load(path: &str) -> Result<Self> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(ProxyError::ConfigError(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path)?;

        // Parse TOML
        let mut config: ProxyConfig = toml::from_str(&content)?;

        // Expand environment variables
        config.expand_env_vars()?;

        Ok(config)
    }

    /// Load configuration with automatic format detection based on file extension.
    ///
    /// Supports `.toml`, `.yaml`, and `.yml` extensions.
    pub fn load_auto(path: &Path) -> Result<Self> {
        let format = Self::detect_format(path)?;
        Self::load_with_format(path, format)
    }

    /// Load configuration from a YAML file.
    pub fn load_yaml(path: &Path) -> Result<Self> {
        Self::load_with_format(path, ConfigFormat::Yaml)
    }

    /// Load configuration from a TOML file.
    pub fn load_toml(path: &Path) -> Result<Self> {
        Self::load_with_format(path, ConfigFormat::Toml)
    }

    /// Load configuration with explicit format specification.
    pub fn load_with_format(path: &Path, format: ConfigFormat) -> Result<Self> {
        if !path.exists() {
            return Err(ProxyError::ConfigError(format!(
                "Configuration file not found: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path)?;

        let mut config: ProxyConfig = match format {
            ConfigFormat::Toml => toml::from_str(&content)?,
            ConfigFormat::Yaml => serde_yaml::from_str(&content)
                .map_err(|e| ProxyError::ConfigError(format!("YAML parsing error: {}", e)))?,
        };

        config.expand_env_vars()?;
        Ok(config)
    }

    /// Save configuration with automatic format detection based on file extension.
    pub fn save_auto(&self, path: &Path) -> Result<()> {
        let format = Self::detect_format(path)?;
        self.save_with_format(path, format)
    }

    /// Save configuration to a YAML file.
    pub fn save_yaml(&self, path: &Path) -> Result<()> {
        self.save_with_format(path, ConfigFormat::Yaml)
    }

    /// Save configuration to a TOML file.
    pub fn save_toml(&self, path: &Path) -> Result<()> {
        self.save_with_format(path, ConfigFormat::Toml)
    }

    /// Save configuration with explicit format specification.
    pub fn save_with_format(&self, path: &Path, format: ConfigFormat) -> Result<()> {
        let content = match format {
            ConfigFormat::Toml => toml::to_string_pretty(self)
                .map_err(|e| ProxyError::ConfigError(format!("TOML serialization error: {}", e)))?,
            ConfigFormat::Yaml => serde_yaml::to_string(self)
                .map_err(|e| ProxyError::ConfigError(format!("YAML serialization error: {}", e)))?,
        };

        std::fs::write(path, content)?;
        Ok(())
    }

    /// Detect configuration format from file extension.
    pub fn detect_format(path: &Path) -> Result<ConfigFormat> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => Ok(ConfigFormat::Toml),
            Some("yaml") | Some("yml") => Ok(ConfigFormat::Yaml),
            Some(ext) => Err(ProxyError::ConfigError(format!(
                "Unsupported config file extension: .{}",
                ext
            ))),
            None => Err(ProxyError::ConfigError(
                "Config file has no extension, cannot detect format".to_string(),
            )),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate proxy settings
        if self.proxy.api_key.is_empty() {
            return Err(ProxyError::ConfigError(
                "API key not configured".to_string(),
            ));
        }

        if self.proxy.api_key.starts_with('$') {
            return Err(ProxyError::ConfigError(format!(
                "API key environment variable not resolved: {}",
                self.proxy.api_key
            )));
        }

        // Validate providers
        if self.providers.is_empty() {
            return Err(ProxyError::ConfigError(
                "No providers configured".to_string(),
            ));
        }

        for provider in &self.providers {
            if provider.api_key.is_empty() {
                return Err(ProxyError::ConfigError(format!(
                    "Provider '{}' has empty API key",
                    provider.name
                )));
            }

            if provider.api_key.starts_with('$') {
                return Err(ProxyError::ConfigError(format!(
                    "Provider '{}' API key not resolved: {}",
                    provider.name, provider.api_key
                )));
            }

            if provider.models.is_empty() {
                return Err(ProxyError::ConfigError(format!(
                    "Provider '{}' has no models configured",
                    provider.name
                )));
            }
        }

        // Validate routing settings
        if !self.provider_exists(&self.router.default) {
            return Err(ProxyError::ConfigError(format!(
                "Default router provider '{}' not found",
                self.router.default
            )));
        }

        Ok(())
    }

    fn expand_env_vars(&mut self) -> Result<()> {
        // Expand proxy API key
        self.proxy.api_key = expand_env_var(&self.proxy.api_key)?;

        // Expand provider API keys
        for provider in &mut self.providers {
            provider.api_key = expand_env_var(&provider.api_key)?;
        }

        Ok(())
    }

    fn provider_exists(&self, provider_model: &str) -> bool {
        // Format: "provider,model"
        if let Some((provider_name, _)) = provider_model.split_once(',') {
            self.providers.iter().any(|p| p.name == provider_name)
        } else {
            false
        }
    }
}

fn expand_env_var(value: &str) -> Result<String> {
    if let Some(var_name) = value.strip_prefix('$') {
        std::env::var(var_name).map_err(|_| {
            ProxyError::ConfigError(format!("Environment variable not set: {}", var_name))
        })
    } else {
        Ok(value.to_string())
    }
}

// Default values
fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3456
}

fn default_timeout() -> u64 {
    600000 // 10 minutes
}

fn default_long_context_threshold() -> usize {
    60000
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_requests_per_minute() -> usize {
    60
}

fn default_concurrent() -> usize {
    10
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_expand_env_var() {
        std::env::set_var("TEST_VAR", "test_value");
        assert_eq!(expand_env_var("$TEST_VAR").unwrap(), "test_value");
        assert_eq!(expand_env_var("literal").unwrap(), "literal");
    }

    #[test]
    fn test_detect_format_toml() {
        let path = Path::new("config.toml");
        assert_eq!(
            ProxyConfig::detect_format(path).unwrap(),
            ConfigFormat::Toml
        );
    }

    #[test]
    fn test_detect_format_yaml() {
        let path = Path::new("config.yaml");
        assert_eq!(
            ProxyConfig::detect_format(path).unwrap(),
            ConfigFormat::Yaml
        );

        let path = Path::new("config.yml");
        assert_eq!(
            ProxyConfig::detect_format(path).unwrap(),
            ConfigFormat::Yaml
        );
    }

    #[test]
    fn test_detect_format_unsupported() {
        let path = Path::new("config.json");
        assert!(ProxyConfig::detect_format(path).is_err());
    }

    #[test]
    fn test_detect_format_no_extension() {
        let path = Path::new("config");
        assert!(ProxyConfig::detect_format(path).is_err());
    }

    fn create_test_config() -> ProxyConfig {
        ProxyConfig {
            proxy: ProxySettings {
                host: "127.0.0.1".to_string(),
                port: 3456,
                api_key: "test-key".to_string(),
                timeout_ms: 60000,
            },
            router: RouterSettings {
                default: "openai,gpt-4".to_string(),
                background: None,
                think: None,
                plan_implementation: None,
                long_context: None,
                long_context_threshold: 60000,
                web_search: None,
                image: None,
                model_mappings: vec![],
                model_exclusions: vec![],
                strategy: RoutingStrategy::default(),
            },
            providers: vec![Provider {
                name: "openai".to_string(),
                api_base_url: "https://api.openai.com/v1".to_string(),
                api_key: "sk-test".to_string(),
                models: vec!["gpt-4".to_string()],
                transformers: vec![],
            }],
            security: SecuritySettings::default(),
            oauth: OAuthSettings::default(),
            management: ManagementSettings::default(),
            webhooks: WebhookSettings::default(),
        }
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = create_test_config();

        // Create temp file with .yaml extension
        let temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save as YAML
        config.save_yaml(&temp_path).unwrap();

        // Verify file was written
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("proxy:"));
        assert!(content.contains("host:"));

        // Load back
        let loaded = ProxyConfig::load_yaml(&temp_path).unwrap();
        assert_eq!(loaded.proxy.host, config.proxy.host);
        assert_eq!(loaded.proxy.port, config.proxy.port);
        assert_eq!(loaded.providers.len(), config.providers.len());

        // Cleanup happens automatically when temp_file is dropped
        drop(temp_file);
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = create_test_config();

        // Create temp file with .toml extension
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save as TOML
        config.save_toml(&temp_path).unwrap();

        // Verify file was written
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("[proxy]"));
        assert!(content.contains("host = "));

        // Load back
        let loaded = ProxyConfig::load_toml(&temp_path).unwrap();
        assert_eq!(loaded.proxy.host, config.proxy.host);
        assert_eq!(loaded.proxy.port, config.proxy.port);
        assert_eq!(loaded.providers.len(), config.providers.len());

        // Cleanup happens automatically
        drop(temp_file);
    }

    #[test]
    fn test_auto_detect_yaml() {
        let config = create_test_config();

        // Create temp file with .yaml extension
        let temp_file = NamedTempFile::with_suffix(".yaml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save with auto-detection (should use YAML)
        config.save_auto(&temp_path).unwrap();

        // Verify it's YAML format
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("proxy:"));
        assert!(!content.contains("[proxy]")); // Not TOML

        // Load with auto-detection
        let loaded = ProxyConfig::load_auto(&temp_path).unwrap();
        assert_eq!(loaded.proxy.host, config.proxy.host);
    }

    #[test]
    fn test_auto_detect_toml() {
        let config = create_test_config();

        // Create temp file with .toml extension
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save with auto-detection (should use TOML)
        config.save_auto(&temp_path).unwrap();

        // Verify it's TOML format
        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("[proxy]"));

        // Load with auto-detection
        let loaded = ProxyConfig::load_auto(&temp_path).unwrap();
        assert_eq!(loaded.proxy.host, config.proxy.host);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let path = Path::new("/nonexistent/config.yaml");
        assert!(ProxyConfig::load_auto(path).is_err());
    }

    #[test]
    fn test_router_settings_with_model_mappings() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[[router.model_mappings]]
from = "claude-sonnet-4-5-*"
to = "anthropic/claude-sonnet-4.5"
bidirectional = true

[[router.model_mappings]]
from = "claude-haiku-*"
to = "anthropic/claude-3.5-haiku"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        // Verify model mappings were parsed
        assert_eq!(config.router.model_mappings.len(), 2);

        // Check first mapping
        assert_eq!(config.router.model_mappings[0].from, "claude-sonnet-4-5-*");
        assert_eq!(
            config.router.model_mappings[0].to,
            "anthropic/claude-sonnet-4.5"
        );
        assert!(config.router.model_mappings[0].bidirectional);

        // Check second mapping
        assert_eq!(config.router.model_mappings[1].from, "claude-haiku-*");
        assert_eq!(
            config.router.model_mappings[1].to,
            "anthropic/claude-3.5-haiku"
        );
        assert!(!config.router.model_mappings[1].bidirectional); // default is false
    }

    #[test]
    fn test_router_settings_default_empty_mappings() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        // model_mappings should default to empty vec
        assert!(config.router.model_mappings.is_empty());
    }

    #[test]
    fn test_oauth_settings_defaults() {
        let settings = OAuthSettings::default();
        assert_eq!(settings.storage_backend, "file");
        assert!(settings.redis_url.is_none());
        assert!(!settings.claude.enabled);
        assert_eq!(settings.claude.callback_port, 9999);
        assert!(!settings.gemini.enabled);
        assert!(!settings.openai.enabled);
        assert!(!settings.copilot.enabled);
    }

    #[test]
    fn test_oauth_settings_from_toml() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[oauth]
storage_backend = "redis"
redis_url = "redis://localhost:6379"

[oauth.claude]
enabled = true
callback_port = 9998
client_id = "claude-client-id"

[oauth.gemini]
enabled = true
callback_port = 9997

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.oauth.storage_backend, "redis");
        assert_eq!(
            config.oauth.redis_url,
            Some("redis://localhost:6379".to_string())
        );
        assert!(config.oauth.claude.enabled);
        assert_eq!(config.oauth.claude.callback_port, 9998);
        assert_eq!(
            config.oauth.claude.client_id,
            Some("claude-client-id".to_string())
        );
        assert!(config.oauth.gemini.enabled);
        assert_eq!(config.oauth.gemini.callback_port, 9997);
        assert!(!config.oauth.copilot.enabled); // default
    }

    #[test]
    fn test_oauth_provider_settings_new_fields_default() {
        let settings = OAuthProviderSettings::default();
        assert!(settings.auth_mode.is_none());
        assert!(settings.scopes.is_none());
        assert!(settings.anthropic_beta.is_none());
    }

    #[test]
    fn test_oauth_provider_settings_claude_auth_mode_from_toml() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[oauth.claude]
enabled = true
callback_port = 54545
client_id = "claude-client-id"
auth_mode = "bearer"
scopes = ["user:inference", "user:profile"]
anthropic_beta = "oauth-2025-04-20"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.oauth.claude.auth_mode, Some("bearer".to_string()));
        assert_eq!(
            config.oauth.claude.scopes,
            Some(vec![
                "user:inference".to_string(),
                "user:profile".to_string()
            ])
        );
        assert_eq!(
            config.oauth.claude.anthropic_beta,
            Some("oauth-2025-04-20".to_string())
        );
    }

    #[test]
    fn test_oauth_provider_settings_api_key_mode_from_toml() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[oauth.claude]
enabled = true
client_id = "claude-client-id"
auth_mode = "api_key"
scopes = ["org:create_api_key", "user:profile", "user:inference"]

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.oauth.claude.auth_mode, Some("api_key".to_string()));
        assert_eq!(config.oauth.claude.scopes.as_ref().unwrap().len(), 3);
        assert!(config.oauth.claude.anthropic_beta.is_none());
    }

    #[test]
    fn test_management_settings_defaults() {
        let settings = ManagementSettings::default();
        assert!(!settings.enabled);
        assert!(settings.secret_key.is_none());
        assert!(!settings.allow_remote);
        assert_eq!(settings.logging.level, "info");
        assert_eq!(settings.logging.max_entries, 1000);
    }

    #[test]
    fn test_management_settings_from_toml() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[management]
enabled = true
secret_key = "my-secret"
allow_remote = true

[management.logging]
level = "debug"
max_entries = 5000

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert!(config.management.enabled);
        assert_eq!(config.management.secret_key, Some("my-secret".to_string()));
        assert!(config.management.allow_remote);
        assert_eq!(config.management.logging.level, "debug");
        assert_eq!(config.management.logging.max_entries, 5000);
    }

    #[test]
    fn test_webhooks_settings_from_toml() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[webhooks]
enabled = true
url = "https://hooks.example.com/llm"
secret = "webhook-secret"
events = ["circuit_breaker", "config_updated"]
retry_count = 5
timeout_seconds = 10

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert!(config.webhooks.enabled);
        assert_eq!(config.webhooks.url, "https://hooks.example.com/llm");
        assert_eq!(config.webhooks.secret, "webhook-secret");
        assert_eq!(config.webhooks.events.len(), 2);
        assert!(config
            .webhooks
            .events
            .contains(&"circuit_breaker".to_string()));
        assert!(config
            .webhooks
            .events
            .contains(&"config_updated".to_string()));
        assert_eq!(config.webhooks.retry_count, 5);
        assert_eq!(config.webhooks.timeout_seconds, 10);
    }

    #[test]
    fn test_router_settings_with_strategy() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"
strategy = "round_robin"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert!(matches!(
            config.router.strategy,
            RoutingStrategy::RoundRobin
        ));
    }

    #[test]
    fn test_router_settings_with_model_exclusions() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[[router.model_exclusions]]
provider = "openrouter"
patterns = ["*-preview", "*-beta"]

[[router.model_exclusions]]
provider = "deepseek"
patterns = ["*-test"]

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.router.model_exclusions.len(), 2);
        assert_eq!(config.router.model_exclusions[0].provider, "openrouter");
        assert_eq!(config.router.model_exclusions[0].patterns.len(), 2);
        assert_eq!(config.router.model_exclusions[1].provider, "deepseek");
        assert_eq!(config.router.model_exclusions[1].patterns.len(), 1);
    }

    #[test]
    fn test_backwards_compatibility_minimal_config() {
        // Minimal config from before schema extensions - should still work
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"

[router]
default = "openai,gpt-4"

[[providers]]
name = "openai"
api_base_url = "https://api.openai.com/v1"
api_key = "sk-test"
models = ["gpt-4"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();

        // All new fields should have defaults
        assert!(!config.oauth.claude.enabled);
        assert!(!config.management.enabled);
        assert!(!config.webhooks.enabled);
        assert!(config.router.model_exclusions.is_empty());
        assert!(matches!(config.router.strategy, RoutingStrategy::FillFirst));
    }

    #[test]
    fn test_full_config_roundtrip_with_new_sections() {
        let config = create_test_config();

        // Create temp file with .toml extension
        let temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save as TOML
        config.save_toml(&temp_path).unwrap();

        // Load back
        let loaded = ProxyConfig::load_toml(&temp_path).unwrap();

        // Verify all sections loaded correctly
        assert_eq!(loaded.proxy.host, config.proxy.host);
        assert_eq!(loaded.oauth.storage_backend, config.oauth.storage_backend);
        assert_eq!(loaded.management.enabled, config.management.enabled);
        assert_eq!(loaded.webhooks.enabled, config.webhooks.enabled);
    }

    #[test]
    fn test_minimax_m2_5_anthropic_provider_config() {
        let toml_content = r#"
[proxy]
host = "127.0.0.1"
port = 3456
api_key = "test-key"
timeout_ms = 30000

[router]
default = "minimax,MiniMax-M2.5"
think = "minimax,MiniMax-M2.5"
long_context_threshold = 12000

[[providers]]
name = "minimax"
api_base_url = "https://api.minimax.io/anthropic"
api_key = "$MINIMAX_API_KEY"
models = ["MiniMax-M2.5", "MiniMax-M2.1"]
transformers = ["anthropic"]
"#;

        let config: ProxyConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.router.default, "minimax,MiniMax-M2.5");
        assert_eq!(config.router.think.as_deref(), Some("minimax,MiniMax-M2.5"));
        assert_eq!(config.providers.len(), 1);

        let minimax = &config.providers[0];
        assert_eq!(minimax.name, "minimax");
        assert_eq!(minimax.api_base_url, "https://api.minimax.io/anthropic");
        assert!(minimax.models.contains(&"MiniMax-M2.5".to_string()));
    }
}
