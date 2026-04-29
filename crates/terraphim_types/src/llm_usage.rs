//! LLM usage tracking types for cost monitoring across providers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model: String,
    pub provider: String,
    pub cost_usd: Option<f64>,
    pub latency_ms: u64,
}

impl LlmUsage {
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_usd = Some(cost_usd);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResult {
    pub content: String,
    pub usage: Option<LlmUsage>,
}

impl LlmResult {
    pub fn new(content: String) -> Self {
        Self {
            content,
            usage: None,
        }
    }

    pub fn with_usage(mut self, usage: LlmUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model_pattern: String,
    pub input_cost_per_million_tokens: f64,
    pub output_cost_per_million_tokens: f64,
}

impl ModelPricing {
    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million_tokens;
        let output_cost =
            (output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million_tokens;
        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_usage_total_tokens() {
        let usage = LlmUsage {
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4".to_string(),
            provider: "openrouter".to_string(),
            cost_usd: None,
            latency_ms: 1200,
        };
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn test_llm_usage_with_cost() {
        let usage = LlmUsage {
            input_tokens: 1000,
            output_tokens: 500,
            model: "gpt-4".to_string(),
            provider: "openrouter".to_string(),
            cost_usd: None,
            latency_ms: 800,
        };
        let usage = usage.with_cost(0.045);
        assert_eq!(usage.cost_usd, Some(0.045));
    }

    #[test]
    fn test_llm_result_new() {
        let result = LlmResult::new("hello".to_string());
        assert_eq!(result.content, "hello");
        assert!(result.usage.is_none());
    }

    #[test]
    fn test_llm_result_with_usage() {
        let usage = LlmUsage {
            input_tokens: 10,
            output_tokens: 5,
            model: "test".to_string(),
            provider: "test".to_string(),
            cost_usd: Some(0.001),
            latency_ms: 100,
        };
        let result = LlmResult::new("response".to_string()).with_usage(usage);
        assert_eq!(result.content, "response");
        assert!(result.usage.is_some());
        assert_eq!(result.usage.unwrap().total_tokens(), 15);
    }

    #[test]
    fn test_model_pricing_calculate_cost() {
        let pricing = ModelPricing {
            model_pattern: "openai/gpt-4*".to_string(),
            input_cost_per_million_tokens: 30.0,
            output_cost_per_million_tokens: 60.0,
        };
        let cost = pricing.calculate_cost(1_000_000, 500_000);
        assert!((cost - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_model_pricing_zero_tokens() {
        let pricing = ModelPricing {
            model_pattern: "test".to_string(),
            input_cost_per_million_tokens: 10.0,
            output_cost_per_million_tokens: 20.0,
        };
        assert_eq!(pricing.calculate_cost(0, 0), 0.0);
    }

    #[test]
    fn test_llm_usage_serialization_roundtrip() {
        let usage = LlmUsage {
            input_tokens: 100,
            output_tokens: 50,
            model: "gpt-4".to_string(),
            provider: "openrouter".to_string(),
            cost_usd: Some(0.015),
            latency_ms: 1200,
        };
        let json = serde_json::to_string(&usage).unwrap();
        let deserialized: LlmUsage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input_tokens, 100);
        assert_eq!(deserialized.output_tokens, 50);
        assert_eq!(deserialized.model, "gpt-4");
    }
}
