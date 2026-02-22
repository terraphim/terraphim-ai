//! Cost management configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for cost management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Enable cost-aware routing
    pub enabled: bool,

    /// Default budget limits (in USD)
    pub default_budget_limit: f64,

    /// Budget warning threshold (percentage)
    pub budget_warning_threshold: f64,

    /// Budget critical threshold (percentage)
    pub budget_critical_threshold: f64,

    /// Cost optimization strategy
    pub optimization_strategy: CostOptimizationStrategy,

    /// Pricing information for different providers and models
    pub pricing: HashMap<String, HashMap<String, PricingInfo>>,

    /// Currency for cost calculations
    pub currency: String,

    /// Enable automatic budget enforcement
    pub auto_budget_enforcement: bool,
}

impl Default for CostConfig {
    fn default() -> Self {
        let mut pricing = HashMap::new();

        // Add some default pricing for common models
        let openai_pricing = HashMap::from([
            (
                "gpt-4".to_string(),
                PricingInfo {
                    input_token_price: 0.03,
                    output_token_price: 0.06,
                    per_request_price: None,
                    currency: "USD".to_string(),
                    last_updated: chrono::Utc::now(),
                },
            ),
            (
                "gpt-4-turbo".to_string(),
                PricingInfo {
                    input_token_price: 0.01,
                    output_token_price: 0.03,
                    per_request_price: None,
                    currency: "USD".to_string(),
                    last_updated: chrono::Utc::now(),
                },
            ),
            (
                "gpt-3.5-turbo".to_string(),
                PricingInfo {
                    input_token_price: 0.0015,
                    output_token_price: 0.002,
                    per_request_price: None,
                    currency: "USD".to_string(),
                    last_updated: chrono::Utc::now(),
                },
            ),
        ]);

        pricing.insert("openai".to_string(), openai_pricing);

        Self {
            enabled: true,
            default_budget_limit: 100.0,
            budget_warning_threshold: 0.8,
            budget_critical_threshold: 0.95,
            optimization_strategy: CostOptimizationStrategy::Balanced,
            pricing,
            currency: "USD".to_string(),
            auto_budget_enforcement: true,
        }
    }
}

/// Cost optimization strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostOptimizationStrategy {
    /// Prioritize lowest cost
    LowestCost,
    /// Balance between cost and performance
    Balanced,
    /// Prioritize performance within budget constraints
    PerformanceWithinBudget,
    /// Prioritize cost for non-critical requests, performance for critical ones
    Adaptive,
}

/// Pricing information for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    /// Price per 1M input tokens
    pub input_token_price: f64,

    /// Price per 1M output tokens
    pub output_token_price: f64,

    /// Fixed price per request (if applicable)
    pub per_request_price: Option<f64>,

    /// Currency code
    pub currency: String,

    /// When this pricing was last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl PricingInfo {
    /// Calculate cost for a given number of tokens
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_token_price;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_token_price;
        let request_cost = self.per_request_price.unwrap_or(0.0);

        input_cost + output_cost + request_cost
    }

    /// Check if pricing is stale (older than specified hours)
    pub fn is_stale(&self, hours: u64) -> bool {
        let age = chrono::Utc::now() - self.last_updated;
        age.num_hours() > hours as i64
    }
}
