//! LLM Router Configuration Types
//!
//! Configuration types for intelligent LLM routing in Terraphim AI.

use serde::{Deserialize, Serialize};

/// Router configuration from Role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRouterConfig {
    /// Enable intelligent routing (default: true)
    #[serde(default)]
    pub enabled: bool,

    /// Routing mode: "library" (in-process) or "service" (HTTP proxy)
    #[serde(default)]
    pub mode: RouterMode,

    /// Proxy URL for service mode (default: http://127.0.0.1:3456)
    #[serde(default = "default_proxy_url")]
    pub proxy_url: Option<String>,

    /// Taxonomy path for pattern-based routing (default: docs/taxonomy)
    #[serde(default = "default_taxonomy_path")]
    pub taxonomy_path: Option<String>,

    /// Enable cost optimization phase
    #[serde(default)]
    pub cost_optimization_enabled: bool,

    /// Enable performance optimization phase
    #[serde(default)]
    pub performance_optimization_enabled: bool,

    /// Routing strategy preference
    #[serde(default)]
    pub strategy: RouterStrategy,
}

impl Default for LlmRouterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: RouterMode::Library,
            proxy_url: None,
            taxonomy_path: None,
            cost_optimization_enabled: false,
            performance_optimization_enabled: false,
            strategy: RouterStrategy::Balanced,
        }
    }
}

/// Router mode selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouterMode {
    /// In-process library routing (fast, single deployment)
    #[serde(rename = "library")]
    Library,

    /// External HTTP service (slower, separate deployment)
    #[serde(rename = "service")]
    Service,
}

/// Router strategy for preference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouterStrategy {
    /// Cost-first optimization
    #[serde(rename = "cost_first")]
    CostFirst,

    /// Quality-first (performance metrics)
    #[serde(rename = "quality_first")]
    QualityFirst,

    /// Balanced (cost + quality)
    #[serde(rename = "balanced")]
    Balanced,

    /// Static model selection (backward compatibility)
    #[serde(rename = "static")]
    Static,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_router_config_default() {
        let config = LlmRouterConfig::default();
        assert!(config.enabled);
        assert!(matches!(config.mode, RouterMode::Library));
        assert_eq!(config.strategy, RouterStrategy::Balanced);
        assert!(config.proxy_url.is_none());
        assert_eq!(config.taxonomy_path, None);
    }

    #[test]
    fn test_router_mode_serialization() {
        let modes = vec![RouterMode::Library, RouterMode::Service];

        for mode in modes {
            let serialized = serde_json::to_string(&mode).unwrap();
            let deserialized: RouterMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }

    #[test]
    fn test_router_strategy_serialization() {
        let strategies = vec![
            RouterStrategy::CostFirst,
            RouterStrategy::QualityFirst,
            RouterStrategy::Balanced,
            RouterStrategy::Static,
        ];

        for strategy in strategies {
            let serialized = serde_json::to_string(&strategy).unwrap();
            let deserialized: RouterStrategy = serde_json::from_str(&serialized).unwrap();
            assert_eq!(strategy, deserialized);
        }
    }
}
