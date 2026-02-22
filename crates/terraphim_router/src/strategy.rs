//! Routing strategies for selecting the best provider.

use terraphim_types::capability::{CostLevel, Latency, Provider};

/// Trait for routing strategies
pub trait RoutingStrategy: Send + Sync {
    /// Select the best provider from candidates
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider>;

    /// Get strategy name
    fn name(&self) -> &'static str;
}

/// Strategy: Optimize for lowest cost
#[derive(Debug, Clone, Default)]
pub struct CostOptimized;

impl RoutingStrategy for CostOptimized {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        candidates.into_iter().min_by_key(|p| p.cost_level)
    }

    fn name(&self) -> &'static str {
        "cost_optimized"
    }
}

/// Strategy: Optimize for lowest latency
#[derive(Debug, Clone, Default)]
pub struct LatencyOptimized;

impl RoutingStrategy for LatencyOptimized {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        candidates.into_iter().min_by_key(|p| p.latency)
    }

    fn name(&self) -> &'static str {
        "latency_optimized"
    }
}

/// Strategy: Optimize for best capability match
#[derive(Debug, Clone, Default)]
pub struct CapabilityFirst;

impl RoutingStrategy for CapabilityFirst {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        // For now, just pick the first one with the most capabilities
        // In a real implementation, this would score by relevance
        candidates.into_iter().max_by_key(|p| p.capabilities.len())
    }

    fn name(&self) -> &'static str {
        "capability_first"
    }
}

/// Strategy: Round-robin for load balancing
#[derive(Debug)]
pub struct RoundRobin {
    index: std::sync::atomic::AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingStrategy for RoundRobin {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        if candidates.is_empty() {
            return None;
        }

        let index = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let selected = index % candidates.len();

        candidates.into_iter().nth(selected)
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use terraphim_types::capability::{Capability, ProviderType};

    fn create_test_provider(id: &str, cost: CostLevel, latency: Latency) -> Provider {
        Provider {
            id: id.to_string(),
            name: id.to_string(),
            provider_type: ProviderType::Llm {
                model_id: id.to_string(),
                api_endpoint: "https://example.com".to_string(),
            },
            capabilities: vec![Capability::CodeGeneration],
            cost_level: cost,
            latency,
            keywords: vec![],
        }
    }

    #[test]
    fn test_cost_optimized() {
        let strategy = CostOptimized;

        let providers = vec![
            create_test_provider("expensive", CostLevel::Expensive, Latency::Medium),
            create_test_provider("cheap", CostLevel::Cheap, Latency::Medium),
            create_test_provider("moderate", CostLevel::Moderate, Latency::Medium),
        ];

        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates);

        assert_eq!(selected.unwrap().id, "cheap");
    }

    #[test]
    fn test_latency_optimized() {
        let strategy = LatencyOptimized;

        let providers = vec![
            create_test_provider("slow", CostLevel::Moderate, Latency::Slow),
            create_test_provider("fast", CostLevel::Moderate, Latency::Fast),
            create_test_provider("medium", CostLevel::Moderate, Latency::Medium),
        ];

        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates);

        assert_eq!(selected.unwrap().id, "fast");
    }

    #[test]
    fn test_round_robin() {
        let strategy = RoundRobin::new();

        let providers = vec![
            create_test_provider("a", CostLevel::Cheap, Latency::Fast),
            create_test_provider("b", CostLevel::Cheap, Latency::Fast),
            create_test_provider("c", CostLevel::Cheap, Latency::Fast),
        ];

        // First call should return "a"
        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates.clone());
        assert_eq!(selected.unwrap().id, "a");

        // Second call should return "b"
        let selected = strategy.select_provider(candidates.clone());
        assert_eq!(selected.unwrap().id, "b");

        // Third call should return "c"
        let selected = strategy.select_provider(candidates.clone());
        assert_eq!(selected.unwrap().id, "c");

        // Fourth call should wrap around to "a"
        let selected = strategy.select_provider(candidates);
        assert_eq!(selected.unwrap().id, "a");
    }
}
