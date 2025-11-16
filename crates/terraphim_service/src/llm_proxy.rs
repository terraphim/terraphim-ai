//! Terraphim LLM Proxy Service
//!
//! This module provides a unified proxy service for all LLM providers,
//! with automatic environment variable detection, fallback mechanisms,
//! and comprehensive logging for debugging proxy configurations.

use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmProxyError {
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication failed for provider: {provider}")]
    AuthError { provider: String },

    #[error("Rate limit exceeded for provider: {provider}")]
    RateLimitError { provider: String },

    #[error("Provider not supported: {provider}")]
    UnsupportedProvider { provider: String },
}

pub type Result<T> = std::result::Result<T, LlmProxyError>;

/// Configuration for LLM proxy settings
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub provider: String,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub max_retries: u32,
    pub enable_fallback: bool,
}

impl ProxyConfig {
    /// Create a new proxy configuration
    pub fn new(provider: String, model: String) -> Self {
        Self {
            provider,
            model,
            base_url: None,
            api_key: None,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_fallback: true,
        }
    }

    /// Set custom base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set API key
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = Some(key);
        self
    }

    /// Set timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable or disable fallback to direct endpoints
    pub fn with_fallback(mut self, enable: bool) -> Self {
        self.enable_fallback = enable;
        self
    }
}

/// LLM Proxy client with automatic configuration detection
#[derive(Debug)]
pub struct LlmProxyClient {
    client: Client,
    configs: HashMap<String, ProxyConfig>,
    pub default_provider: String,
}

impl LlmProxyClient {
    /// Create a new LLM proxy client
    pub fn new(default_provider: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("Terraphim/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| LlmProxyError::NetworkError(e.to_string()))?;

        let mut proxy = Self {
            client,
            configs: HashMap::new(),
            default_provider,
        };

        // Auto-detect and configure proxy settings from environment
        proxy.auto_configure_from_env()?;

        Ok(proxy)
    }

    /// Create a new LLM proxy client without auto-configuration (for testing)
    #[cfg(test)]
    pub fn new_no_auto_configure(default_provider: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("Terraphim/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| LlmProxyError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            configs: HashMap::new(),
            default_provider,
        })
    }

    /// Auto-configure proxy settings from environment variables
    fn auto_configure_from_env(&mut self) -> Result<()> {
        // Configure Anthropic with z.ai proxy
        if let Ok(anthropic_url) = env::var("ANTHROPIC_BASE_URL") {
            let config = ProxyConfig::new(
                "anthropic".to_string(),
                "claude-3-sonnet-20240229".to_string(),
            )
            .with_base_url(anthropic_url)
            .with_api_key(
                env::var("ANTHROPIC_AUTH_TOKEN")
                    .or_else(|_| env::var("ANTHROPIC_API_KEY"))
                    .unwrap_or_default(),
            )
            .with_fallback(true);

            self.configs.insert("anthropic".to_string(), config);
            log::info!("🔗 Auto-configured Anthropic proxy from environment");
        }

        // Configure OpenRouter
        if let Ok(openrouter_url) = env::var("OPENROUTER_BASE_URL") {
            let config = ProxyConfig::new(
                "openrouter".to_string(),
                "anthropic/claude-3.5-sonnet".to_string(),
            )
            .with_base_url(openrouter_url)
            .with_api_key(env::var("OPENROUTER_API_KEY").unwrap_or_default())
            .with_fallback(true);

            self.configs.insert("openrouter".to_string(), config);
            log::info!("🔗 Auto-configured OpenRouter proxy from environment");
        }

        // Configure Ollama
        if let Ok(ollama_url) = env::var("OLLAMA_BASE_URL") {
            let config = ProxyConfig::new("ollama".to_string(), "llama3.1".to_string())
                .with_base_url(ollama_url)
                .with_fallback(true);

            self.configs.insert("ollama".to_string(), config);
            log::info!("🔗 Auto-configured Ollama proxy from environment");
        }

        Ok(())
    }

    /// Add or update a proxy configuration
    pub fn configure(&mut self, config: ProxyConfig) {
        log::info!("🔧 Configuring proxy for provider: {}", config.provider);
        self.configs.insert(config.provider.clone(), config);
    }

    /// Get configuration for a specific provider
    pub fn get_config(&self, provider: &str) -> Option<&ProxyConfig> {
        self.configs.get(provider)
    }

    /// Test proxy connectivity for a specific provider
    pub async fn test_connectivity(&self, provider: &str) -> Result<bool> {
        let config =
            self.configs
                .get(provider)
                .ok_or_else(|| LlmProxyError::UnsupportedProvider {
                    provider: provider.to_string(),
                })?;

        let test_url = match provider {
            "anthropic" => {
                if let Some(base_url) = &config.base_url {
                    format!("{}/v1/messages", base_url.trim_end_matches('/'))
                } else {
                    "https://api.anthropic.com/v1/messages".to_string()
                }
            }
            "openrouter" => {
                if let Some(base_url) = &config.base_url {
                    format!("{}/chat/completions", base_url.trim_end_matches('/'))
                } else {
                    "https://openrouter.ai/api/v1/chat/completions".to_string()
                }
            }
            "ollama" => {
                if let Some(base_url) = &config.base_url {
                    format!("{}/api/tags", base_url.trim_end_matches('/'))
                } else {
                    "http://127.0.0.1:11434/api/tags".to_string()
                }
            }
            _ => {
                return Err(LlmProxyError::UnsupportedProvider {
                    provider: provider.to_string(),
                })
            }
        };

        let start = std::time::Instant::now();

        let request = self.client.get(&test_url).timeout(Duration::from_secs(10));

        let request = if let Some(api_key) = &config.api_key {
            if !api_key.is_empty() {
                match provider {
                    "anthropic" => request.header("x-api-key", api_key),
                    _ => request.header("Authorization", format!("Bearer {}", api_key)),
                }
            } else {
                request
            }
        } else {
            request
        };

        match request.send().await {
            Ok(response) => {
                let duration = start.elapsed();
                let success = response.status().is_success() || response.status().as_u16() == 401; // 401 means connectivity but auth failed

                if success {
                    log::info!(
                        "✅ Proxy connectivity test for {} passed ({}ms)",
                        provider,
                        duration.as_millis()
                    );
                } else {
                    log::warn!(
                        "⚠️ Proxy connectivity test for {} failed: {}",
                        provider,
                        response.status()
                    );
                }

                Ok(success)
            }
            Err(e) => {
                log::error!("❌ Proxy connectivity test for {} failed: {}", provider, e);
                Err(LlmProxyError::NetworkError(e.to_string()))
            }
        }
    }

    /// Test connectivity for all configured providers
    pub async fn test_all_connectivity(&self) -> HashMap<String, Result<bool>> {
        let mut results = HashMap::new();

        for provider in self.configs.keys() {
            let result = self.test_connectivity(provider).await;
            results.insert(provider.clone(), result);
        }

        results
    }

    /// Get the effective base URL for a provider (proxy or direct)
    pub fn get_effective_url(&self, provider: &str) -> Option<String> {
        if let Some(config) = self.configs.get(provider) {
            if let Some(base_url) = &config.base_url {
                return Some(base_url.clone());
            }
        }

        // Return direct endpoint if no proxy configured
        match provider {
            "anthropic" => Some("https://api.anthropic.com".to_string()),
            "openrouter" => Some("https://openrouter.ai/api/v1".to_string()),
            "ollama" => Some("http://127.0.0.1:11434".to_string()),
            _ => None,
        }
    }

    /// Check if a provider is configured to use a proxy
    pub fn is_using_proxy(&self, provider: &str) -> bool {
        self.configs
            .get(provider)
            .map(|config| config.base_url.is_some())
            .unwrap_or(false)
    }

    /// Get list of configured providers
    pub fn configured_providers(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }

    /// Log current proxy configuration
    pub fn log_configuration(&self) {
        log::info!("📋 LLM Proxy Configuration:");

        for (provider, config) in &self.configs {
            let proxy_status = if config.base_url.is_some() {
                format!("Proxy: {}", config.base_url.as_ref().unwrap())
            } else {
                "Direct".to_string()
            };

            log::info!(
                "  {}: {} (Model: {}, Fallback: {})",
                provider,
                proxy_status,
                config.model,
                if config.enable_fallback {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );
        }

        log::info!("  Default provider: {}", self.default_provider);
    }
}

impl Default for LlmProxyClient {
    fn default() -> Self {
        Self::new("anthropic".to_string()).expect("Failed to create default LLM proxy client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_creation() {
        let config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
            .with_base_url("https://api.z.ai/api/anthropic".to_string())
            .with_timeout(Duration::from_secs(60));

        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-3-sonnet");
        assert!(config.base_url.is_some());
        assert_eq!(config.base_url.unwrap(), "https://api.z.ai/api/anthropic");
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_proxy_client_creation() {
        let client = LlmProxyClient::new_no_auto_configure("anthropic".to_string());
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.default_provider, "anthropic");
        assert!(client.configured_providers().is_empty()); // No auto-config in test
    }

    #[tokio::test]
    async fn test_effective_url_resolution() {
        let mut client = LlmProxyClient::new_no_auto_configure("anthropic".to_string()).unwrap();

        // Test direct URL (no proxy)
        assert_eq!(
            client.get_effective_url("anthropic"),
            Some("https://api.anthropic.com".to_string())
        );

        // Test proxy URL
        let config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
            .with_base_url("https://api.z.ai/api/anthropic".to_string());

        client.configure(config);
        assert_eq!(
            client.get_effective_url("anthropic"),
            Some("https://api.z.ai/api/anthropic".to_string())
        );
    }

    #[test]
    fn test_proxy_detection() {
        let mut client = LlmProxyClient::new_no_auto_configure("anthropic".to_string()).unwrap();

        // Initially no proxy
        assert!(!client.is_using_proxy("anthropic"));

        // Configure proxy
        let config = ProxyConfig::new("anthropic".to_string(), "claude-3-sonnet".to_string())
            .with_base_url("https://api.z.ai/api/anthropic".to_string());

        client.configure(config);
        assert!(client.is_using_proxy("anthropic"));
    }
}
