//! Routing engine that orchestrates the routing process.

use crate::{
    keyword::KeywordRouter,
    registry::ProviderRegistry,
    strategy::{CostOptimized, RoutingStrategy},
    types::{RoutingContext, RoutingDecision, RoutingError, RoutingReason},
};
use terraphim_types::capability::{Capability, Provider};

/// Main routing engine
pub struct RoutingEngine {
    keyword_router: KeywordRouter,
    registry: ProviderRegistry,
    strategy: Box<dyn RoutingStrategy>,
}

impl RoutingEngine {
    /// Create a new routing engine with default settings
    pub fn new() -> Self {
        Self {
            keyword_router: KeywordRouter::new(),
            registry: ProviderRegistry::new(),
            strategy: Box::new(CostOptimized::default()),
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
        }
    }

    /// Set the routing strategy
    pub fn with_strategy(mut self, strategy: Box<dyn RoutingStrategy>) -> Self {
        self.strategy = strategy;
        self
    }

    /// Add a provider to the registry
    pub fn add_provider(&mut self, provider: Provider) {
        self.registry.add_provider(provider);
    }

    /// Route a prompt to the best provider
    pub fn route(
        &self,
        prompt: &str,
        _context: &RoutingContext,
    ) -> Result<RoutingDecision, RoutingError> {
        // 1. Extract capabilities from prompt
        let capabilities = self.keyword_router.extract_capabilities(prompt);

        if capabilities.is_empty() {
            // No capabilities found - use fallback
            return self.fallback_decision();
        }

        // 2. Find providers that can fulfill these capabilities
        let candidates = self.registry.find_by_capabilities(&capabilities);

        if candidates.is_empty() {
            return Err(RoutingError::NoProviderFound(capabilities));
        }

        // 3. Apply routing strategy to select best provider
        let selected = self.strategy.select_provider(candidates);

        match selected {
            Some(provider) => {
                // Calculate confidence based on match quality
                let confidence = self.calculate_confidence(prompt, provider, &capabilities);

                Ok(RoutingDecision {
                    provider: provider.clone(),
                    matched_capabilities: capabilities.clone(),
                    confidence,
                    reason: RoutingReason::CapabilityMatch { capabilities },
                })
            }
            None => self.fallback_decision(),
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
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::CostOptimized;
    use std::path::PathBuf;
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
}
