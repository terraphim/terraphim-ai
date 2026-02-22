//! Cost calculation and estimation

use crate::cost::{config::CostOptimizationStrategy, PricingDatabase};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cost calculator for estimating request costs
#[derive(Debug, Clone)]
pub struct CostCalculator {
    pricing_db: Arc<PricingDatabase>,
}

/// Cost estimate for a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost: f64,
    pub currency: String,
    pub confidence: f64, // 0.0 to 1.0
    pub cost_breakdown: CostBreakdown,
}

/// Detailed cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub request_cost: f64,
    pub discounts: Vec<DiscountBreakdown>,
}

/// Discount breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountBreakdown {
    pub description: String,
    pub discount_amount: f64,
    pub discount_percentage: f64,
}

/// Token usage estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub estimated: bool,
}

impl CostCalculator {
    /// Create a new cost calculator
    pub fn new(pricing_db: Arc<PricingDatabase>) -> Self {
        Self { pricing_db }
    }

    /// Estimate cost for a request
    pub async fn estimate_cost(
        &self,
        provider: &str,
        model: &str,
        token_usage: &TokenUsage,
    ) -> Option<CostEstimate> {
        let pricing = self.pricing_db.get_pricing(provider, model).await?;

        let mut input_cost =
            (token_usage.input_tokens as f64 / 1_000_000.0) * pricing.base_info.input_token_price;
        let mut output_cost =
            (token_usage.output_tokens as f64 / 1_000_000.0) * pricing.base_info.output_token_price;
        let request_cost = pricing.base_info.per_request_price.unwrap_or(0.0);

        // Apply discounts if applicable
        let mut discounts = Vec::new();
        let total_tokens = token_usage.input_tokens + token_usage.output_tokens;

        for discount_info in pricing.discounts.iter() {
            if total_tokens >= discount_info.min_tokens {
                let discount_amount =
                    (input_cost + output_cost) * (discount_info.discount_percentage / 100.0);
                discounts.push(DiscountBreakdown {
                    description: discount_info.description.clone(),
                    discount_amount,
                    discount_percentage: discount_info.discount_percentage,
                });

                input_cost *= 1.0 - discount_info.discount_percentage / 100.0;
                output_cost *= 1.0 - discount_info.discount_percentage / 100.0;
            }
        }

        let total_cost = input_cost + output_cost + request_cost;

        Some(CostEstimate {
            provider: provider.to_string(),
            model: model.to_string(),
            input_tokens: token_usage.input_tokens,
            output_tokens: token_usage.output_tokens,
            estimated_cost: total_cost,
            currency: pricing.base_info.currency.clone(),
            confidence: if token_usage.estimated { 0.7 } else { 0.95 },
            cost_breakdown: CostBreakdown {
                input_cost,
                output_cost,
                request_cost,
                discounts,
            },
        })
    }

    /// Find cheapest option for given requirements
    pub async fn find_cheapest_option(
        &self,
        providers: &[String],
        models: &[String],
        token_usage: &TokenUsage,
    ) -> Option<CostEstimate> {
        let mut cheapest: Option<CostEstimate> = None;

        for provider in providers {
            for model in models {
                if let Some(estimate) = self.estimate_cost(provider, model, token_usage).await {
                    match &cheapest {
                        None => cheapest = Some(estimate),
                        Some(current) if estimate.estimated_cost < current.estimated_cost => {
                            cheapest = Some(estimate);
                        }
                        _ => {}
                    }
                }
            }
        }

        cheapest
    }

    /// Get cost-optimized recommendations based on strategy
    pub async fn get_recommendations(
        &self,
        providers: &[String],
        models: &[String],
        token_usage: &TokenUsage,
        strategy: &CostOptimizationStrategy,
        budget_limit: Option<f64>,
    ) -> Vec<CostEstimate> {
        let mut estimates = Vec::new();

        for provider in providers {
            for model in models {
                if let Some(estimate) = self.estimate_cost(provider, model, token_usage).await {
                    // Apply budget filter if specified
                    if let Some(limit) = budget_limit {
                        if estimate.estimated_cost > limit {
                            continue;
                        }
                    }
                    estimates.push(estimate);
                }
            }
        }

        // Sort based on strategy
        match strategy {
            CostOptimizationStrategy::LowestCost => {
                estimates.sort_by(|a, b| a.estimated_cost.partial_cmp(&b.estimated_cost).unwrap());
            }
            CostOptimizationStrategy::Balanced => {
                // Sort by cost but consider confidence
                estimates.sort_by(|a, b| {
                    let a_score = a.estimated_cost / a.confidence;
                    let b_score = b.estimated_cost / b.confidence;
                    a_score.partial_cmp(&b_score).unwrap()
                });
            }
            CostOptimizationStrategy::PerformanceWithinBudget => {
                // Within budget, prioritize higher confidence (better performance estimate)
                estimates.sort_by(|a, b| {
                    b.confidence
                        .partial_cmp(&a.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            CostOptimizationStrategy::Adaptive => {
                // Adaptive strategy based on token usage
                if token_usage.total_tokens > 10000 {
                    // For large requests, prioritize cost
                    estimates
                        .sort_by(|a, b| a.estimated_cost.partial_cmp(&b.estimated_cost).unwrap());
                } else {
                    // For small requests, prioritize confidence
                    estimates.sort_by(|a, b| {
                        b.confidence
                            .partial_cmp(&a.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }

        estimates
    }

    /// Estimate tokens from text (rough estimation)
    pub fn estimate_tokens_from_text(&self, text: &str) -> u32 {
        // Rough estimation: ~4 characters per token for English
        // This is a simplified estimation - in practice, you'd use a proper tokenizer
        (text.len() as f32 / 4.0).ceil() as u32
    }

    /// Estimate tokens from messages
    pub fn estimate_tokens_from_messages(&self, messages: &[serde_json::Value]) -> TokenUsage {
        let mut total_chars = 0;

        for message in messages {
            if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                total_chars += content.len();
            }

            // Add some overhead for message structure
            total_chars += 50; // Rough estimate for JSON overhead
        }

        let input_tokens = self.estimate_tokens_from_text(&" ".repeat(total_chars));

        // Estimate output tokens based on input (rough heuristic)
        let output_tokens = (input_tokens as f32 * 0.75).ceil() as u32;

        TokenUsage {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated: true,
        }
    }

    /// Calculate cost savings between two options
    pub fn calculate_savings(
        &self,
        original: &CostEstimate,
        alternative: &CostEstimate,
    ) -> CostSavings {
        let cost_difference = original.estimated_cost - alternative.estimated_cost;
        let savings_percentage = if original.estimated_cost > 0.0 {
            (cost_difference / original.estimated_cost) * 100.0
        } else {
            0.0
        };

        CostSavings {
            absolute_savings: cost_difference,
            percentage_savings: savings_percentage,
            original_cost: original.estimated_cost,
            alternative_cost: alternative.estimated_cost,
            currency: original.currency.clone(),
        }
    }
}

/// Cost savings information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSavings {
    pub absolute_savings: f64,
    pub percentage_savings: f64,
    pub original_cost: f64,
    pub alternative_cost: f64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::{CostConfig, PricingInfo};
    use std::collections::HashMap;

    fn create_test_pricing_db() -> Arc<PricingDatabase> {
        let mut config = CostConfig::default();

        // Add test pricing
        let mut test_pricing = HashMap::new();
        test_pricing.insert(
            "test-model".to_string(),
            PricingInfo {
                input_token_price: 1.0,  // $1 per million tokens
                output_token_price: 2.0, // $2 per million tokens
                per_request_price: Some(0.01),
                currency: "USD".to_string(),
                last_updated: chrono::Utc::now(),
            },
        );

        config
            .pricing
            .insert("test-provider".to_string(), test_pricing);

        Arc::new(PricingDatabase::new(config))
    }

    #[tokio::test]
    async fn test_cost_estimation() {
        let pricing_db = create_test_pricing_db();
        let calculator = CostCalculator::new(pricing_db);

        let token_usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            estimated: false,
        };

        let estimate = calculator
            .estimate_cost("test-provider", "test-model", &token_usage)
            .await;
        assert!(estimate.is_some());

        let estimate = estimate.unwrap();
        assert_eq!(estimate.provider, "test-provider");
        assert_eq!(estimate.model, "test-model");
        assert_eq!(estimate.input_tokens, 1000);
        assert_eq!(estimate.output_tokens, 500);

        // Expected: (1000/1M * 1.0) + (500/1M * 2.0) + 0.01 = 0.001 + 0.001 + 0.01 = 0.012
        assert!((estimate.estimated_cost - 0.012).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_token_estimation() {
        let pricing_db = create_test_pricing_db();
        let calculator = CostCalculator::new(pricing_db);

        let text = "Hello, world! This is a test message.";
        let tokens = calculator.estimate_tokens_from_text(text);
        assert!(tokens > 0);
        assert!(tokens < text.len() as u32); // Should be less than character count
    }

    #[tokio::test]
    async fn test_cost_savings() {
        let pricing_db = create_test_pricing_db();
        let calculator = CostCalculator::new(pricing_db);

        let original = CostEstimate {
            provider: "test-provider".to_string(),
            model: "expensive-model".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            estimated_cost: 0.10,
            currency: "USD".to_string(),
            confidence: 0.9,
            cost_breakdown: CostBreakdown {
                input_cost: 0.05,
                output_cost: 0.04,
                request_cost: 0.01,
                discounts: Vec::new(),
            },
        };

        let alternative = CostEstimate {
            provider: "test-provider".to_string(),
            model: "cheap-model".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            estimated_cost: 0.05,
            currency: "USD".to_string(),
            confidence: 0.8,
            cost_breakdown: CostBreakdown {
                input_cost: 0.025,
                output_cost: 0.02,
                request_cost: 0.005,
                discounts: Vec::new(),
            },
        };

        let savings = calculator.calculate_savings(&original, &alternative);
        assert_eq!(savings.absolute_savings, 0.05);
        assert_eq!(savings.percentage_savings, 50.0);
    }
}
