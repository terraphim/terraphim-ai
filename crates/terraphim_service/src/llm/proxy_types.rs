//! LLM Proxy Types - Re-exports from terraphim-llm-proxy
//!
//! This crate re-exports the essential LLM router types from terraphim_llm-proxy
//! for use by terraphim_service. This allows terraphim_service to use
//! the production-ready routing types without adding terraphim_llm-proxy as a
//! direct dependency (which has path resolution issues).

pub use terraphim_llm_proxy::router::RouterAgent;
pub use terraphim_llm_proxy::router::{
    Priority, RouterMode, RouterStrategy, RoutingDecision, RoutingScenario,
};

/// Re-export router configuration types
pub use terraphim_llm_proxy::config::RouterConfig;

/// Re-export all router types for convenience
pub type LlmRouterConfig = terraphim_llm_proxy::config::RouterConfig;
pub type RouterMode = terraphim_llm_proxy::router::RouterMode;
pub type RouterStrategy = terraphim_llm_proxy::router::RouterStrategy;
pub type RoutingDecision = terraphim_llm_proxy::router::RoutingDecision;
pub type RoutingScenario = terraphim_llm_proxy::router::RoutingScenario;
pub type Priority = terraphim_llm_proxy::router::Priority;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_types_available() {
        // Verify re-exports exist
        let _config = RouterConfig::default();
        let _mode = RouterMode::Library;
        let _strategy = RouterStrategy::Balanced;

        // Verify types work
        assert_eq!(_config.enabled, true);
        assert_eq!(_config.mode, _mode);
        assert_eq!(_config.strategy, _strategy);
    }
}
