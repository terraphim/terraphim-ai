//! Routing strategies for provider selection.
//!
//! Provides fill-first and round-robin strategies for selecting providers,
//! with health-aware filtering and integration with existing Priority routing.
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_llm_proxy::routing::strategy::{
//!     RoutingStrategy, StrategyState, FillFirstStrategy, RoundRobinStrategy
//! };
//!
//! let mut state = StrategyState::new();
//!
//! // Fill-first uses provider order
//! let strategy = FillFirstStrategy;
//! let provider = strategy.select_provider(&candidates, &health, &mut state);
//!
//! // Round-robin distributes evenly
//! let strategy = RoundRobinStrategy;
//! let provider = strategy.select_provider(&candidates, &health, &mut state);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::debug;

use crate::config::Provider;
use crate::provider_health::{HealthStatus, ProviderHealthMonitor};

/// Routing strategy types.
///
/// Determines how providers are selected when multiple are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Use providers in order until failure (default).
    /// Fills first provider to capacity before using second.
    #[default]
    FillFirst,

    /// Distribute requests evenly across providers.
    /// Maintains a rotating index to ensure fair distribution.
    RoundRobin,

    /// Select provider with lowest latency.
    /// Requires performance metrics to be available.
    LatencyOptimized,

    /// Select cheapest provider for the request.
    /// Requires cost data to be available.
    CostOptimized,
}

impl std::fmt::Display for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingStrategy::FillFirst => write!(f, "fill_first"),
            RoutingStrategy::RoundRobin => write!(f, "round_robin"),
            RoutingStrategy::LatencyOptimized => write!(f, "latency_optimized"),
            RoutingStrategy::CostOptimized => write!(f, "cost_optimized"),
        }
    }
}

/// State maintained across strategy invocations.
///
/// Tracks round-robin index and last selection times for providers.
#[derive(Debug, Clone)]
pub struct StrategyState {
    /// Current index for round-robin distribution.
    pub round_robin_index: usize,

    /// Last selection time for each provider.
    pub last_selection: HashMap<String, Instant>,
}

impl Default for StrategyState {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategyState {
    /// Create a new strategy state.
    pub fn new() -> Self {
        Self {
            round_robin_index: 0,
            last_selection: HashMap::new(),
        }
    }

    /// Record a provider selection.
    pub fn record_selection(&mut self, provider_name: &str) {
        self.last_selection
            .insert(provider_name.to_string(), Instant::now());
    }

    /// Get time since last selection for a provider.
    pub fn time_since_selection(&self, provider_name: &str) -> Option<std::time::Duration> {
        self.last_selection
            .get(provider_name)
            .map(|instant| instant.elapsed())
    }
}

/// Provider candidate with health status.
#[derive(Debug, Clone)]
pub struct ProviderCandidate<'a> {
    /// The provider configuration.
    pub provider: &'a Provider,

    /// Current health status.
    pub health_status: HealthStatus,

    /// Whether the provider is currently healthy.
    pub is_healthy: bool,
}

impl<'a> ProviderCandidate<'a> {
    /// Create a new provider candidate.
    pub fn new(provider: &'a Provider, health_status: HealthStatus) -> Self {
        let is_healthy = matches!(health_status, HealthStatus::Healthy | HealthStatus::Unknown);
        Self {
            provider,
            health_status,
            is_healthy,
        }
    }

    /// Create a healthy candidate (default status).
    pub fn healthy(provider: &'a Provider) -> Self {
        Self {
            provider,
            health_status: HealthStatus::Healthy,
            is_healthy: true,
        }
    }
}

/// Fill-first routing strategy.
///
/// Selects providers in configuration order, skipping unhealthy providers.
/// This strategy is useful when you want to prioritize a primary provider
/// and only use fallbacks when the primary is unavailable.
#[derive(Debug, Clone, Default)]
pub struct FillFirstStrategy;

impl FillFirstStrategy {
    /// Select the first healthy provider from candidates.
    ///
    /// Returns the first provider that is healthy, or None if all are unhealthy.
    pub fn select<'a>(&self, candidates: &[ProviderCandidate<'a>]) -> Option<&'a Provider> {
        for candidate in candidates {
            if candidate.is_healthy {
                debug!(
                    provider = %candidate.provider.name,
                    status = ?candidate.health_status,
                    "FillFirst: selected provider"
                );
                return Some(candidate.provider);
            }
            debug!(
                provider = %candidate.provider.name,
                status = ?candidate.health_status,
                "FillFirst: skipping unhealthy provider"
            );
        }

        debug!("FillFirst: no healthy providers available");
        None
    }

    /// Select provider with state tracking (for consistency with other strategies).
    pub fn select_with_state<'a>(
        &self,
        candidates: &[ProviderCandidate<'a>],
        state: &mut StrategyState,
    ) -> Option<&'a Provider> {
        let provider = self.select(candidates)?;
        state.record_selection(&provider.name);
        Some(provider)
    }
}

/// Round-robin routing strategy.
///
/// Distributes requests evenly across healthy providers.
/// Maintains a rotating index that wraps around when reaching the end.
#[derive(Debug, Clone, Default)]
pub struct RoundRobinStrategy;

impl RoundRobinStrategy {
    /// Select the next provider in round-robin order.
    ///
    /// Skips unhealthy providers and wraps around to the beginning
    /// when reaching the end of the candidate list.
    pub fn select<'a>(
        &self,
        candidates: &[ProviderCandidate<'a>],
        state: &mut StrategyState,
    ) -> Option<&'a Provider> {
        if candidates.is_empty() {
            return None;
        }

        // Get healthy candidates only
        let healthy_indices: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_healthy)
            .map(|(i, _)| i)
            .collect();

        if healthy_indices.is_empty() {
            debug!("RoundRobin: no healthy providers available");
            return None;
        }

        // Find the next index in our healthy list
        let start_index = state.round_robin_index;

        // Try to find a healthy provider starting from current index
        for offset in 0..healthy_indices.len() {
            let idx = (start_index + offset) % healthy_indices.len();
            let candidate_idx = healthy_indices[idx];
            let candidate = &candidates[candidate_idx];

            if candidate.is_healthy {
                // Update state for next call
                state.round_robin_index = (idx + 1) % healthy_indices.len();
                state.record_selection(&candidate.provider.name);

                debug!(
                    provider = %candidate.provider.name,
                    index = idx,
                    next_index = state.round_robin_index,
                    "RoundRobin: selected provider"
                );
                return Some(candidate.provider);
            }
        }

        debug!("RoundRobin: no healthy providers available");
        None
    }
}

/// Select a provider using the specified strategy.
///
/// This is the main entry point for strategy-based provider selection.
/// It filters candidates by health status and applies the selected strategy.
///
/// # Arguments
/// * `strategy` - The routing strategy to use
/// * `providers` - List of available providers
/// * `health_monitor` - Provider health monitor for status checks
/// * `state` - Mutable strategy state for round-robin tracking
///
/// # Returns
/// The selected provider, or None if no healthy providers are available
pub async fn select_provider<'a>(
    strategy: RoutingStrategy,
    providers: &'a [Provider],
    health_monitor: &ProviderHealthMonitor,
    state: &mut StrategyState,
) -> Option<&'a Provider> {
    // Build candidates with health status
    let mut candidates = Vec::with_capacity(providers.len());

    for provider in providers {
        let health_status = health_monitor
            .get_provider_health(&provider.name)
            .await
            .map(|h| h.status)
            .unwrap_or(HealthStatus::Unknown);

        candidates.push(ProviderCandidate::new(provider, health_status));
    }

    select_provider_from_candidates(strategy, &candidates, state)
}

/// Select a provider from pre-built candidates.
///
/// Lower-level function that works with already-prepared candidates.
pub fn select_provider_from_candidates<'a>(
    strategy: RoutingStrategy,
    candidates: &[ProviderCandidate<'a>],
    state: &mut StrategyState,
) -> Option<&'a Provider> {
    match strategy {
        RoutingStrategy::FillFirst => FillFirstStrategy.select_with_state(candidates, state),
        RoutingStrategy::RoundRobin => RoundRobinStrategy.select(candidates, state),
        RoutingStrategy::LatencyOptimized => {
            // Fall back to fill-first for now
            // TODO: Implement latency-based selection
            debug!("LatencyOptimized not fully implemented, falling back to FillFirst");
            FillFirstStrategy.select_with_state(candidates, state)
        }
        RoutingStrategy::CostOptimized => {
            // Fall back to fill-first for now
            // TODO: Implement cost-based selection
            debug!("CostOptimized not fully implemented, falling back to FillFirst");
            FillFirstStrategy.select_with_state(candidates, state)
        }
    }
}

/// Filter providers by health status.
///
/// Returns only providers that are healthy or have unknown status.
pub fn filter_healthy_providers<'a>(
    providers: &'a [Provider],
    healthy_names: &[String],
) -> Vec<&'a Provider> {
    providers
        .iter()
        .filter(|p| healthy_names.contains(&p.name))
        .collect()
}

/// Create provider candidates without health monitor.
///
/// Useful for testing or when health monitoring is not available.
pub fn create_candidates_without_health(providers: &[Provider]) -> Vec<ProviderCandidate<'_>> {
    providers.iter().map(ProviderCandidate::healthy).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_providers() -> Vec<Provider> {
        vec![
            Provider {
                name: "provider1".to_string(),
                api_base_url: "http://localhost:8001".to_string(),
                api_key: "key1".to_string(),
                models: vec!["model1".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "provider2".to_string(),
                api_base_url: "http://localhost:8002".to_string(),
                api_key: "key2".to_string(),
                models: vec!["model2".to_string()],
                transformers: vec![],
            },
            Provider {
                name: "provider3".to_string(),
                api_base_url: "http://localhost:8003".to_string(),
                api_key: "key3".to_string(),
                models: vec!["model3".to_string()],
                transformers: vec![],
            },
        ]
    }

    #[test]
    fn test_fill_first_order() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();

        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);

        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "provider1");
    }

    #[test]
    fn test_fill_first_skip_unhealthy() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Healthy),
            ProviderCandidate::new(&providers[2], HealthStatus::Healthy),
        ];

        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);

        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "provider2");
    }

    #[test]
    fn test_fill_first_skip_multiple_unhealthy() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[2], HealthStatus::Healthy),
        ];

        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);

        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "provider3");
    }

    #[test]
    fn test_fill_first_all_unhealthy() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[2], HealthStatus::Unhealthy),
        ];

        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);

        assert!(selected.is_none());
    }

    #[test]
    fn test_fill_first_degraded_allowed() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Degraded),
            ProviderCandidate::new(&providers[2], HealthStatus::Healthy),
        ];

        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);

        // Degraded is NOT considered healthy
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "provider3");
    }

    #[test]
    fn test_round_robin_distribution() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();

        let strategy = RoundRobinStrategy;
        let mut state = StrategyState::new();

        // First selection
        let selected1 = strategy.select(&candidates, &mut state);
        assert!(selected1.is_some());
        assert_eq!(selected1.unwrap().name, "provider1");

        // Second selection
        let selected2 = strategy.select(&candidates, &mut state);
        assert!(selected2.is_some());
        assert_eq!(selected2.unwrap().name, "provider2");

        // Third selection
        let selected3 = strategy.select(&candidates, &mut state);
        assert!(selected3.is_some());
        assert_eq!(selected3.unwrap().name, "provider3");
    }

    #[test]
    fn test_round_robin_wraps() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();

        let strategy = RoundRobinStrategy;
        let mut state = StrategyState::new();

        // Select all three
        strategy.select(&candidates, &mut state);
        strategy.select(&candidates, &mut state);
        strategy.select(&candidates, &mut state);

        // Fourth selection should wrap around to first
        let selected = strategy.select(&candidates, &mut state);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name, "provider1");
    }

    #[test]
    fn test_round_robin_skip_unhealthy() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Healthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[2], HealthStatus::Healthy),
        ];

        let strategy = RoundRobinStrategy;
        let mut state = StrategyState::new();

        // First selection - should be provider1
        let selected1 = strategy.select(&candidates, &mut state);
        assert_eq!(selected1.unwrap().name, "provider1");

        // Second selection - should skip provider2 and select provider3
        let selected2 = strategy.select(&candidates, &mut state);
        assert_eq!(selected2.unwrap().name, "provider3");

        // Third selection - should wrap back to provider1
        let selected3 = strategy.select(&candidates, &mut state);
        assert_eq!(selected3.unwrap().name, "provider1");
    }

    #[test]
    fn test_round_robin_all_unhealthy() {
        let providers = create_test_providers();
        let candidates = vec![
            ProviderCandidate::new(&providers[0], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[1], HealthStatus::Unhealthy),
            ProviderCandidate::new(&providers[2], HealthStatus::Unhealthy),
        ];

        let strategy = RoundRobinStrategy;
        let mut state = StrategyState::new();

        let selected = strategy.select(&candidates, &mut state);
        assert!(selected.is_none());
    }

    #[test]
    fn test_round_robin_empty_candidates() {
        let candidates: Vec<ProviderCandidate> = vec![];
        let strategy = RoundRobinStrategy;
        let mut state = StrategyState::new();

        let selected = strategy.select(&candidates, &mut state);
        assert!(selected.is_none());
    }

    #[test]
    fn test_strategy_with_priority() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();

        // FillFirst respects provider order (priority)
        let strategy = FillFirstStrategy;
        let selected = strategy.select(&candidates);
        assert_eq!(selected.unwrap().name, "provider1");
    }

    #[test]
    fn test_strategy_state_record_selection() {
        let mut state = StrategyState::new();

        state.record_selection("provider1");
        assert!(state.time_since_selection("provider1").is_some());
        assert!(state.time_since_selection("provider2").is_none());
    }

    #[test]
    fn test_routing_strategy_default() {
        let strategy = RoutingStrategy::default();
        assert_eq!(strategy, RoutingStrategy::FillFirst);
    }

    #[test]
    fn test_routing_strategy_display() {
        assert_eq!(RoutingStrategy::FillFirst.to_string(), "fill_first");
        assert_eq!(RoutingStrategy::RoundRobin.to_string(), "round_robin");
        assert_eq!(
            RoutingStrategy::LatencyOptimized.to_string(),
            "latency_optimized"
        );
        assert_eq!(RoutingStrategy::CostOptimized.to_string(), "cost_optimized");
    }

    #[test]
    fn test_routing_strategy_serialization() {
        let strategy = RoutingStrategy::RoundRobin;

        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"round_robin\"");

        let deserialized: RoutingStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, strategy);
    }

    #[test]
    fn test_select_provider_from_candidates_fill_first() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();
        let mut state = StrategyState::new();

        let selected =
            select_provider_from_candidates(RoutingStrategy::FillFirst, &candidates, &mut state);
        assert_eq!(selected.unwrap().name, "provider1");
    }

    #[test]
    fn test_select_provider_from_candidates_round_robin() {
        let providers = create_test_providers();
        let candidates: Vec<_> = providers.iter().map(ProviderCandidate::healthy).collect();
        let mut state = StrategyState::new();

        let selected1 =
            select_provider_from_candidates(RoutingStrategy::RoundRobin, &candidates, &mut state);
        assert_eq!(selected1.unwrap().name, "provider1");

        let selected2 =
            select_provider_from_candidates(RoutingStrategy::RoundRobin, &candidates, &mut state);
        assert_eq!(selected2.unwrap().name, "provider2");
    }

    #[test]
    fn test_create_candidates_without_health() {
        let providers = create_test_providers();
        let candidates = create_candidates_without_health(&providers);

        assert_eq!(candidates.len(), 3);
        assert!(candidates.iter().all(|c| c.is_healthy));
        assert!(candidates
            .iter()
            .all(|c| c.health_status == HealthStatus::Healthy));
    }

    #[test]
    fn test_provider_candidate_healthy() {
        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            models: vec!["model".to_string()],
            transformers: vec![],
        };

        let candidate = ProviderCandidate::healthy(&provider);
        assert!(candidate.is_healthy);
        assert_eq!(candidate.health_status, HealthStatus::Healthy);
    }

    #[test]
    fn test_provider_candidate_unknown_is_healthy() {
        let provider = Provider {
            name: "test".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            models: vec!["model".to_string()],
            transformers: vec![],
        };

        // Unknown status should be treated as healthy (optimistic)
        let candidate = ProviderCandidate::new(&provider, HealthStatus::Unknown);
        assert!(candidate.is_healthy);
    }

    #[test]
    fn test_filter_healthy_providers() {
        let providers = create_test_providers();
        let healthy_names = vec!["provider1".to_string(), "provider3".to_string()];

        let filtered = filter_healthy_providers(&providers, &healthy_names);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "provider1");
        assert_eq!(filtered[1].name, "provider3");
    }
}
