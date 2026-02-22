//! Routing engine that orchestrates the routing process.

use crate::{
    keyword::KeywordRouter,
    registry::ProviderRegistry,
    strategy::{CostOptimized, RoutingStrategy, StrategyRegistry},
    types::{RoutingContext, RoutingDecision, RoutingError, RoutingReason},
};
use terraphim_types::capability::{Capability, Provider};
use tracing::{debug, info, info_span, warn};

/// Truncate prompt to first 50 chars for safe logging (privacy).
fn prompt_preview(prompt: &str) -> String {
    let truncated: String = prompt.chars().take(50).collect();
    if prompt.chars().count() > 50 {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

/// Hash a prompt for correlation without exposing content.
fn prompt_hash(prompt: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    hasher.finish()
}

/// Main routing engine
pub struct RoutingEngine {
    keyword_router: KeywordRouter,
    registry: ProviderRegistry,
    strategy: Box<dyn RoutingStrategy>,
    strategy_registry: StrategyRegistry,
}

impl std::fmt::Debug for RoutingEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutingEngine")
            .field("keyword_router", &self.keyword_router)
            .field("registry", &self.registry)
            .field("strategy", &self.strategy.name())
            .field("strategy_registry", &self.strategy_registry)
            .finish()
    }
}

impl RoutingEngine {
    /// Create a new routing engine with default settings
    pub fn new() -> Self {
        Self {
            keyword_router: KeywordRouter::new(),
            registry: ProviderRegistry::new(),
            strategy: Box::new(CostOptimized),
            strategy_registry: StrategyRegistry::new(),
        }
    }

    /// Create with custom components
    pub fn with_components(
        keyword_router: KeywordRouter,
        registry: ProviderRegistry,
        strategy: Box<dyn RoutingStrategy>,
    ) -> Self {
        Self {
            keyword_router,
            registry,
            strategy,
            strategy_registry: StrategyRegistry::new(),
        }
    }

    /// Set the default routing strategy
    pub fn with_strategy(mut self, strategy: Box<dyn RoutingStrategy>) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set a custom strategy registry (replaces the built-in one)
    pub fn with_strategy_registry(mut self, registry: StrategyRegistry) -> Self {
        self.strategy_registry = registry;
        self
    }

    /// Get a mutable reference to the strategy registry for registration
    pub fn strategy_registry_mut(&mut self) -> &mut StrategyRegistry {
        &mut self.strategy_registry
    }

    /// Add a provider to the registry
    pub fn add_provider(&mut self, provider: Provider) {
        self.registry.add_provider(provider);
    }

    /// Route a prompt to the best provider.
    ///
    /// If `context.strategy_override` is set and matches a registered strategy,
    /// that strategy is used for this request instead of the default.
    pub fn route(
        &self,
        prompt: &str,
        context: &RoutingContext,
    ) -> Result<RoutingDecision, RoutingError> {
        // Resolve per-request strategy override
        let override_strategy = context
            .strategy_override
            .as_deref()
            .and_then(|name| self.strategy_registry.get(name));

        let active_strategy: &dyn RoutingStrategy = match &override_strategy {
            Some(s) => {
                info!(
                    default_strategy = self.strategy.name(),
                    override_strategy = s.name(),
                    "Strategy override applied"
                );
                s.as_ref()
            }
            None => self.strategy.as_ref(),
        };

        let span = info_span!(
            "router.route",
            prompt_len = prompt.len(),
            prompt_hash = prompt_hash(prompt),
            prompt_preview = %prompt_preview(prompt),
            strategy = active_strategy.name(),
            selected_provider = tracing::field::Empty,
            confidence = tracing::field::Empty,
            reason = tracing::field::Empty,
        );
        let _guard = span.enter();

        // 1. Extract capabilities from prompt
        let capabilities = {
            let _cap_span = info_span!(
                "router.extract_capabilities",
                prompt_len = prompt.len(),
                capabilities_found = tracing::field::Empty,
            )
            .entered();
            let caps = self.keyword_router.extract_capabilities(prompt);
            tracing::Span::current().record("capabilities_found", caps.len());
            debug!(capabilities = ?caps, "Extracted capabilities from prompt");
            caps
        };

        if capabilities.is_empty() {
            info!("No capabilities extracted, using fallback");
            span.record("reason", "fallback_no_capabilities");
            return self.fallback_decision();
        }

        // 2. Find providers that can fulfill these capabilities
        let candidates = {
            let _find_span = info_span!(
                "router.find_providers",
                capabilities_count = capabilities.len(),
                candidates_found = tracing::field::Empty,
            )
            .entered();
            let cands = self.registry.find_by_capabilities(&capabilities);
            tracing::Span::current().record("candidates_found", cands.len());
            debug!(candidates_count = cands.len(), "Found matching providers");
            cands
        };

        if candidates.is_empty() {
            warn!(capabilities = ?capabilities, "No provider found for capabilities");
            return Err(RoutingError::NoProviderFound(capabilities));
        }

        // 3. Apply routing strategy to select best provider
        let selected = {
            let _sel_span = info_span!(
                "router.select_provider",
                strategy = active_strategy.name(),
                candidates_count = candidates.len(),
                selected_provider = tracing::field::Empty,
            )
            .entered();
            let sel = active_strategy.select_provider(candidates);
            if let Some(p) = &sel {
                tracing::Span::current().record("selected_provider", p.id.as_str());
            }
            sel
        };

        match selected {
            Some(provider) => {
                let confidence = self.calculate_confidence(prompt, provider, &capabilities);
                span.record("selected_provider", provider.id.as_str());
                span.record("confidence", confidence as f64);
                span.record("reason", "capability_match");

                info!(
                    provider_id = provider.id.as_str(),
                    provider_name = provider.name.as_str(),
                    confidence = confidence,
                    "Routing decision made"
                );

                Ok(RoutingDecision {
                    provider: provider.clone(),
                    matched_capabilities: capabilities.clone(),
                    confidence,
                    reason: RoutingReason::CapabilityMatch { capabilities },
                })
            }
            None => {
                span.record("reason", "fallback_no_selection");
                self.fallback_decision()
            }
        }
    }

    /// Calculate confidence score for a routing decision
    fn calculate_confidence(
        &self,
        prompt: &str,
        provider: &Provider,
        matched_caps: &[Capability],
    ) -> f32 {
        let mut score = 0.5f32; // Base score

        // Boost for keyword matches
        if provider.matches_keywords(prompt) {
            score += 0.3;
        }

        // Boost for capability coverage
        let coverage = matched_caps.len() as f32 / provider.capabilities.len().max(1) as f32;
        score += coverage * 0.2;

        score.min(1.0)
    }

    /// Get fallback decision when no good match
    fn fallback_decision(&self) -> Result<RoutingDecision, RoutingError> {
        // Try to get any provider as fallback
        let all = self.registry.all();

        if let Some(provider) = all.first() {
            Ok(RoutingDecision {
                provider: (*provider).clone(),
                matched_capabilities: vec![],
                confidence: 0.1,
                reason: RoutingReason::Fallback,
            })
        } else {
            Err(RoutingError::NoProviderFound(vec![]))
        }
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience wrapper for the routing engine
#[derive(Debug)]
pub struct Router {
    engine: RoutingEngine,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            engine: RoutingEngine::new(),
        }
    }

    /// Create from existing engine
    pub fn from_engine(engine: RoutingEngine) -> Self {
        Self { engine }
    }

    /// Add a provider
    pub fn add_provider(&mut self, provider: Provider) {
        self.engine.add_provider(provider);
    }

    /// Route a prompt
    pub fn route(
        &self,
        prompt: &str,
        context: &RoutingContext,
    ) -> Result<RoutingDecision, RoutingError> {
        self.engine.route(prompt, context)
    }

    /// Set the routing strategy on the inner engine
    pub fn with_strategy(mut self, strategy: Box<dyn RoutingStrategy>) -> Self {
        self.engine = self.engine.with_strategy(strategy);
        self
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::capability::{CostLevel, Latency, ProviderType};

    fn create_test_provider(id: &str, cost: CostLevel, caps: Vec<Capability>) -> Provider {
        Provider {
            id: id.to_string(),
            name: id.to_string(),
            provider_type: ProviderType::Llm {
                model_id: id.to_string(),
                api_endpoint: "https://example.com".to_string(),
            },
            capabilities: caps,
            cost_level: cost,
            latency: Latency::Medium,
            keywords: vec![],
        }
    }

    #[test]
    fn test_route_by_capability() {
        let mut engine = RoutingEngine::new();

        // Add providers
        engine.add_provider(create_test_provider(
            "cheap-coder",
            CostLevel::Cheap,
            vec![Capability::CodeGeneration],
        ));

        engine.add_provider(create_test_provider(
            "expensive-thinker",
            CostLevel::Expensive,
            vec![Capability::DeepThinking],
        ));

        // Route a coding task
        let decision = engine
            .route(
                "Implement a function to parse JSON",
                &RoutingContext::default(),
            )
            .unwrap();

        assert_eq!(decision.provider.id, "cheap-coder");
        assert!(decision
            .matched_capabilities
            .contains(&Capability::CodeGeneration));
    }

    #[test]
    fn test_no_provider_found() {
        let engine = RoutingEngine::new();

        // No providers registered
        let result = engine.route("Implement a function", &RoutingContext::default());

        assert!(result.is_err());
    }

    #[test]
    fn test_fallback() {
        let mut engine = RoutingEngine::new();

        // Add only one provider
        engine.add_provider(create_test_provider(
            "only-provider",
            CostLevel::Moderate,
            vec![Capability::CodeGeneration],
        ));

        // Route something that doesn't match well
        let decision = engine
            .route("Hello world", &RoutingContext::default())
            .unwrap();

        // Should fallback to the only provider
        assert_eq!(decision.provider.id, "only-provider");
        assert_eq!(decision.reason, RoutingReason::Fallback);
    }

    #[test]
    fn test_strategy_override_via_context() {
        let mut engine = RoutingEngine::new(); // default = CostOptimized

        // Two providers: cheap-slow vs expensive-fast, both have CodeGeneration
        engine.add_provider(Provider {
            id: "cheap-slow".to_string(),
            name: "Cheap Slow".to_string(),
            provider_type: ProviderType::Llm {
                model_id: "cheap".to_string(),
                api_endpoint: "http://localhost".to_string(),
            },
            capabilities: vec![Capability::CodeGeneration],
            cost_level: CostLevel::Cheap,
            latency: Latency::Slow,
            keywords: vec![],
        });
        engine.add_provider(Provider {
            id: "expensive-fast".to_string(),
            name: "Expensive Fast".to_string(),
            provider_type: ProviderType::Llm {
                model_id: "expensive".to_string(),
                api_endpoint: "http://localhost".to_string(),
            },
            capabilities: vec![Capability::CodeGeneration],
            cost_level: CostLevel::Expensive,
            latency: Latency::Fast,
            keywords: vec![],
        });

        // Default strategy (CostOptimized) -> picks "cheap-slow"
        let decision = engine
            .route(
                "Implement a function to parse JSON",
                &RoutingContext::default(),
            )
            .unwrap();
        assert_eq!(decision.provider.id, "cheap-slow");

        // Override to LatencyOptimized -> picks "expensive-fast"
        let context = RoutingContext {
            strategy_override: Some("latency_optimized".to_string()),
            ..Default::default()
        };
        let decision = engine
            .route("Implement a function to parse JSON", &context)
            .unwrap();
        assert_eq!(decision.provider.id, "expensive-fast");
    }

    #[test]
    fn test_strategy_override_unknown_falls_back_to_default() {
        let mut engine = RoutingEngine::new(); // default = CostOptimized

        engine.add_provider(create_test_provider(
            "cheap-coder",
            CostLevel::Cheap,
            vec![Capability::CodeGeneration],
        ));

        // Unknown override -> uses default CostOptimized
        let context = RoutingContext {
            strategy_override: Some("nonexistent_strategy".to_string()),
            ..Default::default()
        };
        let decision = engine
            .route("Implement a function to parse JSON", &context)
            .unwrap();
        assert_eq!(decision.provider.id, "cheap-coder");
    }
}
