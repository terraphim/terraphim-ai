//! Router Configuration - Merges Role config with environment variables
//!
//! This module provides MergedRouterConfig that combines LLM router configuration
//! from Role extra fields with environment variable overrides.

use std::env;
use terraphim_config::llm_router::{LlmRouterConfig, RouterMode, RouterStrategy};

/// Merged router configuration from Role and environment
#[derive(Debug, Clone, Default)]
pub struct MergedRouterConfig {
    /// Enable intelligent routing
    pub enabled: bool,

    /// Routing mode
    pub mode: RouterMode,

    /// Proxy URL for service mode
    pub proxy_url: Option<String>,

    /// Taxonomy path
    pub taxonomy_path: Option<String>,

    /// Enable cost optimization
    pub cost_optimization_enabled: bool,

    /// Enable performance optimization
    pub performance_optimization_enabled: bool,

    /// Routing strategy
    pub strategy: RouterStrategy,
}

impl MergedRouterConfig {
    /// Create merged configuration from Role and environment
    pub fn from_role_and_env(role_config: Option<&LlmRouterConfig>) -> Self {
        let mut config = role_config.cloned().unwrap_or_default();

        // Override with environment variables
        if let Ok(url) = env::var("LLM_PROXY_URL") {
            config.proxy_url = Some(url);
        }

        if let Ok(path) = env::var("LLM_TAXONOMY_PATH") {
            config.taxonomy_path = Some(path);
        }

        if let Ok(val) = env::var("LLM_COST_OPTIMIZATION") {
            config.cost_optimization_enabled = val.parse().unwrap_or(false);
        }

        if let Ok(val) = env::var("LLM_PERFORMANCE_OPTIMIZATION") {
            config.performance_optimization_enabled = val.parse().unwrap_or(false);
        }

        if let Ok(val) = env::var("LLM_ROUTING_STRATEGY") {
            config.strategy = serde_json::from_str(&val).unwrap_or(RouterStrategy::Balanced);
        }

        config.into()
    }

    /// Check if routing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get default proxy URL based on mode
    pub fn get_proxy_url(&self) -> String {
        self.proxy_url.clone().unwrap_or_else(|| match self.mode {
            RouterMode::Service => "http://127.0.0.1:3456".to_string(),
            RouterMode::Library => panic!("Library mode should not use proxy URL"),
        })
    }
}

impl From<LlmRouterConfig> for MergedRouterConfig {
    fn from(config: LlmRouterConfig) -> Self {
        Self {
            enabled: config.enabled,
            mode: config.mode,
            proxy_url: config.proxy_url,
            taxonomy_path: config.taxonomy_path,
            cost_optimization_enabled: config.cost_optimization_enabled,
            performance_optimization_enabled: config.performance_optimization_enabled,
            strategy: config.strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merged_config_defaults() {
        let config = MergedRouterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, RouterMode::Library);
        assert!(config.proxy_url.is_none());
        assert_eq!(config.strategy, RouterStrategy::Balanced);
    }

    #[test]
    fn test_merged_config_from_role() {
        let role_config = LlmRouterConfig {
            enabled: true,
            mode: RouterMode::Service,
            proxy_url: Some("http://custom-proxy:8080".to_string()),
            strategy: RouterStrategy::CostFirst,
            ..Default::default()
        };

        let merged = MergedRouterConfig::from_role_and_env(Some(&role_config));

        assert!(merged.enabled);
        assert_eq!(merged.mode, RouterMode::Service);
        assert_eq!(
            merged.proxy_url,
            Some("http://custom-proxy:8080".to_string())
        );
        assert_eq!(merged.strategy, RouterStrategy::CostFirst);
    }

    #[test]
    fn test_env_overrides() {
        env::set_var("LLM_PROXY_URL", "http://env-proxy:9999");

        let role_config = LlmRouterConfig {
            enabled: true,
            mode: RouterMode::Service,
            strategy: RouterStrategy::Balanced,
            ..Default::default()
        };

        let merged = MergedRouterConfig::from_role_and_env(Some(&role_config));

        assert_eq!(merged.proxy_url, Some("http://env-proxy:9999".to_string()));

        env::remove_var("LLM_PROXY_URL");
    }
}
