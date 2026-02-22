//! Routing strategies for selecting the best provider.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

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
        let result = candidates.into_iter().min_by_key(|p| p.cost_level);
        tracing::debug!(
            strategy = "cost_optimized",
            selected_provider = result.map(|p| p.id.as_str()),
            "Strategy selection complete"
        );
        result
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
        let result = candidates.into_iter().min_by_key(|p| p.latency);
        tracing::debug!(
            strategy = "latency_optimized",
            selected_provider = result.map(|p| p.id.as_str()),
            "Strategy selection complete"
        );
        result
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
        let result = candidates.into_iter().max_by_key(|p| p.capabilities.len());
        tracing::debug!(
            strategy = "capability_first",
            selected_provider = result.map(|p| p.id.as_str()),
            "Strategy selection complete"
        );
        result
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

        let result = candidates.into_iter().nth(selected);
        tracing::debug!(
            strategy = "round_robin",
            selected_provider = result.map(|p| p.id.as_str()),
            index = index,
            "Strategy selection complete"
        );
        result
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}

/// Strategy: A/B testing -- probabilistically route between two strategies.
///
/// Uses a weight (0.0-1.0) to determine how often strategy A vs B is chosen.
/// A weight of 0.7 means 70% of requests go through strategy A.
pub struct WeightedStrategy {
    strategy_a: Box<dyn RoutingStrategy>,
    strategy_b: Box<dyn RoutingStrategy>,
    /// Weight for strategy A (0.0 = always B, 1.0 = always A)
    weight_a: f64,
    /// Counter for deterministic round-based selection
    counter: AtomicU64,
}

impl std::fmt::Debug for WeightedStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WeightedStrategy")
            .field("strategy_a", &self.strategy_a.name())
            .field("strategy_b", &self.strategy_b.name())
            .field("weight_a", &self.weight_a)
            .finish()
    }
}

impl WeightedStrategy {
    /// Create a new weighted strategy for A/B testing.
    ///
    /// `weight_a` is the fraction of requests routed to strategy A (0.0 to 1.0).
    pub fn new(
        strategy_a: Box<dyn RoutingStrategy>,
        strategy_b: Box<dyn RoutingStrategy>,
        weight_a: f64,
    ) -> Self {
        Self {
            strategy_a,
            strategy_b,
            weight_a: weight_a.clamp(0.0, 1.0),
            counter: AtomicU64::new(0),
        }
    }
}

impl RoutingStrategy for WeightedStrategy {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        // Deterministic: use counter mod 100 vs weight percentage
        let threshold = (self.weight_a * 100.0) as u64;
        let use_a = (count % 100) < threshold;

        let (chosen, name) = if use_a {
            (&self.strategy_a, self.strategy_a.name())
        } else {
            (&self.strategy_b, self.strategy_b.name())
        };

        tracing::debug!(
            strategy = "weighted",
            chosen_branch = name,
            branch = if use_a { "A" } else { "B" },
            counter = count,
            weight_a = self.weight_a,
            "A/B strategy selection"
        );

        chosen.select_provider(candidates)
    }

    fn name(&self) -> &'static str {
        "weighted"
    }
}

/// Strategy: Filter candidates by user preferences, then delegate to a base strategy.
///
/// Removes providers exceeding `max_cost` or `max_latency` before passing
/// to the inner strategy. If filtering eliminates all candidates, falls through
/// to the base strategy with unfiltered candidates.
pub struct PreferenceFilter {
    base: Box<dyn RoutingStrategy>,
    max_cost: Option<CostLevel>,
    max_latency: Option<Latency>,
}

impl PreferenceFilter {
    /// Create a preference filter wrapping a base strategy.
    pub fn new(base: Box<dyn RoutingStrategy>) -> Self {
        Self {
            base,
            max_cost: None,
            max_latency: None,
        }
    }

    /// Set maximum acceptable cost level.
    pub fn with_max_cost(mut self, cost: CostLevel) -> Self {
        self.max_cost = Some(cost);
        self
    }

    /// Set maximum acceptable latency.
    pub fn with_max_latency(mut self, latency: Latency) -> Self {
        self.max_latency = Some(latency);
        self
    }
}

impl std::fmt::Debug for PreferenceFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreferenceFilter")
            .field("base", &self.base.name())
            .field("max_cost", &self.max_cost)
            .field("max_latency", &self.max_latency)
            .finish()
    }
}

impl RoutingStrategy for PreferenceFilter {
    fn select_provider<'a>(&self, candidates: Vec<&'a Provider>) -> Option<&'a Provider> {
        let filtered: Vec<&'a Provider> = candidates
            .iter()
            .copied()
            .filter(|p| {
                if let Some(max_cost) = self.max_cost {
                    if p.cost_level > max_cost {
                        return false;
                    }
                }
                if let Some(max_latency) = self.max_latency {
                    if p.latency > max_latency {
                        return false;
                    }
                }
                true
            })
            .collect();

        let filtered_count = filtered.len();
        tracing::debug!(
            strategy = "preference_filter",
            base = self.base.name(),
            max_cost = ?self.max_cost,
            max_latency = ?self.max_latency,
            original_count = candidates.len(),
            filtered_count = filtered_count,
            "Applied preference filters"
        );

        if filtered.is_empty() {
            // Fall through to base with unfiltered candidates
            tracing::debug!("Preference filter eliminated all candidates, using unfiltered");
            self.base.select_provider(candidates)
        } else {
            self.base.select_provider(filtered)
        }
    }

    fn name(&self) -> &'static str {
        "preference_filter"
    }
}

/// Registry of named strategies for runtime lookup.
///
/// Strategies are stored as factory functions so each lookup produces a fresh
/// instance (important for stateful strategies like `RoundRobin`).
pub struct StrategyRegistry {
    factories: HashMap<String, Box<dyn Fn() -> Box<dyn RoutingStrategy> + Send + Sync>>,
}

impl StrategyRegistry {
    /// Create a registry pre-populated with the four built-in strategies.
    pub fn new() -> Self {
        let mut reg = Self {
            factories: HashMap::new(),
        };
        reg.register("cost_optimized", || Box::new(CostOptimized));
        reg.register("latency_optimized", || Box::new(LatencyOptimized));
        reg.register("capability_first", || Box::new(CapabilityFirst));
        reg.register("round_robin", || Box::new(RoundRobin::new()));
        reg
    }

    /// Register a named strategy factory.
    pub fn register<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> Box<dyn RoutingStrategy> + Send + Sync + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
    }

    /// Look up and instantiate a strategy by name.
    pub fn get(&self, name: &str) -> Option<Box<dyn RoutingStrategy>> {
        self.factories.get(name).map(|f| f())
    }

    /// List registered strategy names.
    pub fn names(&self) -> Vec<&str> {
        self.factories.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for StrategyRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrategyRegistry")
            .field("strategies", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_weighted_strategy_all_a() {
        let strategy = WeightedStrategy::new(
            Box::new(CostOptimized),
            Box::new(LatencyOptimized),
            1.0, // Always use A
        );

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-fast", CostLevel::Expensive, Latency::Fast),
        ];

        // With weight 1.0, always uses CostOptimized -> "cheap-slow"
        for _ in 0..10 {
            let candidates: Vec<&Provider> = providers.iter().collect();
            let selected = strategy.select_provider(candidates);
            assert_eq!(selected.unwrap().id, "cheap-slow");
        }
    }

    #[test]
    fn test_weighted_strategy_all_b() {
        let strategy = WeightedStrategy::new(
            Box::new(CostOptimized),
            Box::new(LatencyOptimized),
            0.0, // Always use B
        );

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-fast", CostLevel::Expensive, Latency::Fast),
        ];

        // With weight 0.0, always uses LatencyOptimized -> "expensive-fast"
        for _ in 0..10 {
            let candidates: Vec<&Provider> = providers.iter().collect();
            let selected = strategy.select_provider(candidates);
            assert_eq!(selected.unwrap().id, "expensive-fast");
        }
    }

    #[test]
    fn test_weighted_strategy_split() {
        let strategy = WeightedStrategy::new(
            Box::new(CostOptimized),
            Box::new(LatencyOptimized),
            0.5, // 50/50 split
        );

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-fast", CostLevel::Expensive, Latency::Fast),
        ];

        let mut a_count = 0;
        let mut b_count = 0;
        for _ in 0..100 {
            let candidates: Vec<&Provider> = providers.iter().collect();
            let selected = strategy.select_provider(candidates).unwrap();
            if selected.id == "cheap-slow" {
                a_count += 1;
            } else {
                b_count += 1;
            }
        }

        // 50/50 split should give exactly 50 each (deterministic counter-based)
        assert_eq!(a_count, 50);
        assert_eq!(b_count, 50);
    }

    #[test]
    fn test_preference_filter_cost() {
        let strategy =
            PreferenceFilter::new(Box::new(LatencyOptimized)).with_max_cost(CostLevel::Moderate);

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-fast", CostLevel::Expensive, Latency::Fast),
            create_test_provider("moderate-medium", CostLevel::Moderate, Latency::Medium),
        ];

        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates);

        // Expensive is filtered out, then LatencyOptimized picks "moderate-medium" (Medium < Slow)
        assert_eq!(selected.unwrap().id, "moderate-medium");
    }

    #[test]
    fn test_preference_filter_latency() {
        let strategy =
            PreferenceFilter::new(Box::new(CostOptimized)).with_max_latency(Latency::Medium);

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-fast", CostLevel::Expensive, Latency::Fast),
            create_test_provider("moderate-medium", CostLevel::Moderate, Latency::Medium),
        ];

        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates);

        // Slow is filtered out, then CostOptimized picks "moderate-medium" (Moderate < Expensive)
        assert_eq!(selected.unwrap().id, "moderate-medium");
    }

    #[test]
    fn test_preference_filter_fallthrough() {
        // Filter requires max_latency=Fast, but both providers are Slow
        let strategy =
            PreferenceFilter::new(Box::new(CostOptimized)).with_max_latency(Latency::Fast);

        let providers = vec![
            create_test_provider("cheap-slow", CostLevel::Cheap, Latency::Slow),
            create_test_provider("expensive-slow", CostLevel::Expensive, Latency::Slow),
        ];

        let candidates: Vec<&Provider> = providers.iter().collect();
        let selected = strategy.select_provider(candidates);

        // All filtered out -> falls through to unfiltered CostOptimized -> "cheap-slow"
        assert_eq!(selected.unwrap().id, "cheap-slow");
    }

    #[test]
    fn test_strategy_registry_builtins() {
        let registry = StrategyRegistry::new();

        assert!(registry.get("cost_optimized").is_some());
        assert!(registry.get("latency_optimized").is_some());
        assert!(registry.get("capability_first").is_some());
        assert!(registry.get("round_robin").is_some());
        assert!(registry.get("nonexistent").is_none());

        let names = registry.names();
        assert_eq!(names.len(), 4);
    }

    #[test]
    fn test_strategy_registry_custom() {
        let mut registry = StrategyRegistry::new();
        registry.register("my_strategy", || Box::new(CostOptimized));

        let strategy = registry.get("my_strategy").unwrap();
        assert_eq!(strategy.name(), "cost_optimized");

        let names = registry.names();
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn test_strategy_registry_returns_fresh_instances() {
        let registry = StrategyRegistry::new();

        // RoundRobin is stateful -- verify we get independent instances
        let rr1 = registry.get("round_robin").unwrap();
        let rr2 = registry.get("round_robin").unwrap();

        let providers = vec![
            create_test_provider("a", CostLevel::Cheap, Latency::Fast),
            create_test_provider("b", CostLevel::Cheap, Latency::Fast),
        ];

        // Both should start at index 0 (independent state)
        let sel1 = rr1.select_provider(providers.iter().collect());
        let sel2 = rr2.select_provider(providers.iter().collect());
        assert_eq!(sel1.unwrap().id, "a");
        assert_eq!(sel2.unwrap().id, "a");
    }
}
