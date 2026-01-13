//! LLM Proxy Types - Re-exports from terraphim_config
//!
//! This module re-exports the essential LLM router types from terraphim_config
//! for use by terraphim_service. This provides a clean interface for routing
//! configuration without external dependencies.

pub use terraphim_config::llm_router::LlmRouterConfig;
pub use terraphim_config::llm_router::RouterMode;
pub use terraphim_config::llm_router::RouterStrategy;

/// Re-export for convenience
pub type RouterConfig = terraphim_config::llm_router::LlmRouterConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_types_available() {
        // Verify re-exports exist
        let config = RouterConfig::default();
        let _mode = RouterMode::Library;
        let _strategy = RouterStrategy::Balanced;

        // Verify types work
        assert_eq!(config.enabled, true);
        assert_eq!(config.mode, _mode);
        assert_eq!(config.strategy, _strategy);
    }
}
