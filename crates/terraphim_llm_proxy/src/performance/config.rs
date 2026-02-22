//! Performance testing configuration

use serde::{Deserialize, Serialize};

/// Performance testing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    pub enabled: bool,
    pub test_interval_minutes: u64,
    pub test_timeout_seconds: u64,
    pub max_concurrent_tests: usize,
    pub min_test_count: u32,
    pub performance_window_hours: u64,
    pub auto_fallback_enabled: bool,
    pub weights: PerformanceWeights,
    pub test_runs: usize,
    pub test_prompts: Vec<String>,
    pub thresholds: PerformanceThresholds,
    pub persistence_path: Option<String>,
    pub metrics_ttl_hours: u64,
}

/// Performance thresholds for model validation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceThresholds {
    pub max_latency_ms: f64,
    pub min_throughput_tokens_per_sec: f64,
    pub min_success_rate: f64,
}

/// Performance scoring weights
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceWeights {
    pub latency: f64,
    pub throughput: f64,
    pub reliability: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_interval_minutes: 60,
            test_timeout_seconds: 30,
            max_concurrent_tests: 3,
            min_test_count: 5,
            performance_window_hours: 24,
            auto_fallback_enabled: true,
            weights: PerformanceWeights::default(),
            test_runs: 5,
            test_prompts: vec![
                "Hello, how are you?".to_string(),
                "What is the capital of France?".to_string(),
                "Explain photosynthesis in one sentence.".to_string(),
                "Write a short poem about nature.".to_string(),
                "What are the primary colors?".to_string(),
            ],
            thresholds: PerformanceThresholds::default(),
            persistence_path: None,
            metrics_ttl_hours: 24,
        }
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_latency_ms: 5000.0,
            min_throughput_tokens_per_sec: 10.0,
            min_success_rate: 0.95,
        }
    }
}

impl Default for PerformanceWeights {
    fn default() -> Self {
        Self {
            latency: 0.4,
            throughput: 0.4,
            reliability: 0.2,
        }
    }
}

impl PerformanceConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), crate::error::ProxyError> {
        if self.test_interval_minutes == 0 {
            return Err(crate::error::ProxyError::ConfigError(
                "test_interval_minutes must be greater than 0".to_string(),
            ));
        }

        if self.test_timeout_seconds == 0 {
            return Err(crate::error::ProxyError::ConfigError(
                "test_timeout_seconds must be greater than 0".to_string(),
            ));
        }

        if self.max_concurrent_tests == 0 {
            return Err(crate::error::ProxyError::ConfigError(
                "max_concurrent_tests must be greater than 0".to_string(),
            ));
        }

        if self.min_test_count == 0 {
            return Err(crate::error::ProxyError::ConfigError(
                "min_test_count must be greater than 0".to_string(),
            ));
        }

        // Validate weights sum to 1.0
        let weight_sum = self.weights.latency + self.weights.throughput + self.weights.reliability;
        if (weight_sum - 1.0).abs() > 0.01 {
            return Err(crate::error::ProxyError::ConfigError(
                "Performance weights must sum to 1.0".to_string(),
            ));
        }

        Ok(())
    }
}
