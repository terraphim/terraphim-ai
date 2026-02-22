//! Performance metrics and test results

use crate::performance::config::{PerformanceThresholds, PerformanceWeights};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Performance metrics for a model
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceMetrics {
    pub model_name: String,
    pub provider_name: String,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_tokens_per_sec: f64,
    pub success_rate: f64,
    pub error_rate: f64,
    pub last_tested: SystemTime,
    pub test_count: u32,
}

/// Result of a single performance test
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestResult {
    pub test_type: TestType,
    pub model_name: String,
    pub provider_name: String,
    pub success: bool,
    pub latency_ms: f64,
    pub throughput_tokens_per_sec: Option<f64>,
    pub tokens_processed: Option<usize>,
    pub error_message: Option<String>,
    pub timestamp: SystemTime,
}

/// Types of performance tests
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TestType {
    Latency,
    Throughput,
    Concurrent,
    Health,
}

impl PerformanceMetrics {
    /// Create new metrics from test results
    pub fn from_test_results(
        model_name: String,
        provider_name: String,
        results: Vec<TestResult>,
    ) -> Self {
        if results.is_empty() {
            return Self {
                model_name,
                provider_name,
                avg_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                throughput_tokens_per_sec: 0.0,
                success_rate: 0.0,
                error_rate: 1.0,
                last_tested: SystemTime::now(),
                test_count: 0,
            };
        }

        let successful_results: Vec<_> = results.iter().filter(|r| r.success).collect();

        let latencies: Vec<f64> = successful_results.iter().map(|r| r.latency_ms).collect();

        let throughputs: Vec<f64> = successful_results
            .iter()
            .filter_map(|r| r.throughput_tokens_per_sec)
            .collect();

        let avg_latency = if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().sum::<f64>() / latencies.len() as f64
        };

        let p95_latency = if latencies.is_empty() {
            0.0
        } else {
            let mut sorted_latencies = latencies.clone();
            sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let index =
                ((sorted_latencies.len() as f64 * 0.95) as usize).min(sorted_latencies.len() - 1);
            sorted_latencies[index]
        };

        let p99_latency = if latencies.is_empty() {
            0.0
        } else {
            let mut sorted_latencies = latencies.clone();
            sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let index =
                ((sorted_latencies.len() as f64 * 0.99) as usize).min(sorted_latencies.len() - 1);
            sorted_latencies[index]
        };

        let avg_throughput = if throughputs.is_empty() {
            0.0
        } else {
            throughputs.iter().sum::<f64>() / throughputs.len() as f64
        };

        let success_rate = results.len() as f64;
        let success_count = successful_results.len() as f64;
        let success_rate = if success_rate > 0.0 {
            success_count / success_rate
        } else {
            0.0
        };
        let error_rate = 1.0 - success_rate;

        let last_tested = results
            .iter()
            .map(|r| r.timestamp)
            .max()
            .unwrap_or(SystemTime::now());

        Self {
            model_name,
            provider_name,
            avg_latency_ms: avg_latency,
            p95_latency_ms: p95_latency,
            p99_latency_ms: p99_latency,
            throughput_tokens_per_sec: avg_throughput,
            success_rate,
            error_rate,
            last_tested,
            test_count: results.len() as u32,
        }
    }

    /// Calculate performance score based on weights
    pub fn calculate_score(&self, weights: &PerformanceWeights) -> f64 {
        // Normalize latency (lower is better, invert for scoring)
        let latency_score = if self.avg_latency_ms > 0.0 {
            (1000.0 / self.avg_latency_ms).min(1.0)
        } else {
            0.0
        };

        // Normalize throughput (higher is better)
        let throughput_score = (self.throughput_tokens_per_sec / 100.0).min(1.0);

        // Success rate is already normalized
        let reliability_score = self.success_rate;

        // Calculate weighted score
        latency_score * weights.latency
            + throughput_score * weights.throughput
            + reliability_score * weights.reliability
    }

    /// Check if metrics meet performance thresholds
    pub fn meets_thresholds(&self, thresholds: &PerformanceThresholds) -> bool {
        self.avg_latency_ms <= thresholds.max_latency_ms
            && self.throughput_tokens_per_sec >= thresholds.min_throughput_tokens_per_sec
            && self.success_rate >= thresholds.min_success_rate
    }

    /// Check if metrics are stale (need refreshing)
    pub fn is_stale(&self, max_age_hours: u64) -> bool {
        if let Ok(duration) = self.last_tested.elapsed() {
            duration.as_secs() > (max_age_hours * 3600)
        } else {
            true // Clock went backwards, consider stale
        }
    }

    /// Update metrics with new test result
    pub fn update_with_result(&mut self, result: TestResult) {
        // This is a simplified update - in practice, you'd want to maintain
        // a rolling window of results for more accurate metrics
        if result.success {
            // Simple exponential moving average update
            let alpha = 0.1; // Smoothing factor
            self.avg_latency_ms = alpha * result.latency_ms + (1.0 - alpha) * self.avg_latency_ms;

            if let Some(throughput) = result.throughput_tokens_per_sec {
                self.throughput_tokens_per_sec =
                    alpha * throughput + (1.0 - alpha) * self.throughput_tokens_per_sec;
            }
        }

        // Update success/error rates
        let total_tests = self.test_count as f64 + 1.0;
        let success_count =
            self.success_rate * self.test_count as f64 + if result.success { 1.0 } else { 0.0 };

        self.success_rate = success_count / total_tests;
        self.error_rate = 1.0 - self.success_rate;
        self.test_count += 1;
        self.last_tested = result.timestamp;
    }
}

impl TestResult {
    /// Create a successful latency test result
    pub fn latency_success(model_name: String, provider_name: String, latency_ms: f64) -> Self {
        Self {
            test_type: TestType::Latency,
            model_name,
            provider_name,
            success: true,
            latency_ms,
            throughput_tokens_per_sec: None,
            tokens_processed: None,
            error_message: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a successful throughput test result
    pub fn throughput_success(
        model_name: String,
        provider_name: String,
        latency_ms: f64,
        throughput_tokens_per_sec: f64,
        tokens_processed: usize,
    ) -> Self {
        Self {
            test_type: TestType::Throughput,
            model_name,
            provider_name,
            success: true,
            latency_ms,
            throughput_tokens_per_sec: Some(throughput_tokens_per_sec),
            tokens_processed: Some(tokens_processed),
            error_message: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a failed test result
    pub fn failure(
        test_type: TestType,
        model_name: String,
        provider_name: String,
        error_message: String,
    ) -> Self {
        Self {
            test_type,
            model_name,
            provider_name,
            success: false,
            latency_ms: 0.0,
            throughput_tokens_per_sec: None,
            tokens_processed: None,
            error_message: Some(error_message),
            timestamp: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics {
            model_name: "test-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 150.5,
            p95_latency_ms: 300.0,
            p99_latency_ms: 500.0,
            throughput_tokens_per_sec: 25.7,
            success_rate: 0.98,
            error_rate: 0.02,
            last_tested: SystemTime::now(),
            test_count: 10,
        };

        assert_eq!(metrics.model_name, "test-model");
        assert_eq!(metrics.avg_latency_ms, 150.5);
        assert_eq!(metrics.success_rate, 0.98);
    }

    #[test]
    fn test_performance_score_calculation() {
        let weights = PerformanceWeights {
            latency: 0.4,
            throughput: 0.4,
            reliability: 0.2,
        };

        let good_metrics = PerformanceMetrics {
            model_name: "good-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 100.0, // Good latency
            p95_latency_ms: 200.0,
            p99_latency_ms: 300.0,
            throughput_tokens_per_sec: 50.0, // Good throughput
            success_rate: 0.99,              // Good reliability
            error_rate: 0.01,
            last_tested: SystemTime::now(),
            test_count: 20,
        };

        let score = good_metrics.calculate_score(&weights);
        assert!(score > 0.5); // Should have a decent score

        let poor_metrics = PerformanceMetrics {
            model_name: "poor-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 2000.0, // Poor latency
            p95_latency_ms: 3000.0,
            p99_latency_ms: 4000.0,
            throughput_tokens_per_sec: 5.0, // Poor throughput
            success_rate: 0.85,             // Poor reliability
            error_rate: 0.15,
            last_tested: SystemTime::now(),
            test_count: 5,
        };

        let poor_score = poor_metrics.calculate_score(&weights);
        assert!(score > poor_score); // Good should be better than poor
    }

    #[test]
    fn test_threshold_validation() {
        let thresholds = PerformanceThresholds {
            max_latency_ms: 1000.0,
            min_throughput_tokens_per_sec: 20.0,
            min_success_rate: 0.95,
        };

        let good_metrics = PerformanceMetrics {
            model_name: "good-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 500.0, // Below threshold
            p95_latency_ms: 800.0,
            p99_latency_ms: 900.0,
            throughput_tokens_per_sec: 30.0, // Above threshold
            success_rate: 0.98,              // Above threshold
            error_rate: 0.02,
            last_tested: SystemTime::now(),
            test_count: 10,
        };

        assert!(good_metrics.meets_thresholds(&thresholds));

        let bad_metrics = PerformanceMetrics {
            model_name: "bad-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 1500.0, // Above threshold
            p95_latency_ms: 2000.0,
            p99_latency_ms: 2500.0,
            throughput_tokens_per_sec: 10.0, // Below threshold
            success_rate: 0.90,              // Below threshold
            error_rate: 0.10,
            last_tested: SystemTime::now(),
            test_count: 5,
        };

        assert!(!bad_metrics.meets_thresholds(&thresholds));
    }

    #[test]
    fn test_metrics_from_test_results() {
        let results = vec![
            TestResult::latency_success(
                "test-model".to_string(),
                "test-provider".to_string(),
                100.0,
            ),
            TestResult::latency_success(
                "test-model".to_string(),
                "test-provider".to_string(),
                200.0,
            ),
            TestResult::latency_success(
                "test-model".to_string(),
                "test-provider".to_string(),
                150.0,
            ),
            TestResult::failure(
                TestType::Latency,
                "test-model".to_string(),
                "test-provider".to_string(),
                "Network error".to_string(),
            ),
        ];

        let metrics = PerformanceMetrics::from_test_results(
            "test-model".to_string(),
            "test-provider".to_string(),
            results,
        );

        assert_eq!(metrics.model_name, "test-model");
        assert_eq!(metrics.provider_name, "test-provider");
        assert_eq!(metrics.test_count, 4);
        assert_eq!(metrics.success_rate, 0.75); // 3/4 successful
        assert_eq!(metrics.error_rate, 0.25); // 1/4 failed
        assert_eq!(metrics.avg_latency_ms, 150.0); // (100+200+150)/3
    }

    #[test]
    fn test_staleness_check() {
        let metrics = PerformanceMetrics {
            model_name: "test-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 100.0,
            p95_latency_ms: 200.0,
            p99_latency_ms: 300.0,
            throughput_tokens_per_sec: 25.0,
            success_rate: 0.98,
            error_rate: 0.02,
            last_tested: SystemTime::now() - std::time::Duration::from_secs(7200), // 2 hours ago
            test_count: 10,
        };

        assert!(metrics.is_stale(1)); // Stale for 1 hour threshold (2 hours old)
        assert!(!metrics.is_stale(3)); // Not stale for 3 hour threshold
    }
}
