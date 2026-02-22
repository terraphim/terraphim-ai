//! Performance testing engine
//!
//! Provides automated latency and throughput testing for LLM models.

use crate::error::{ProxyError, Result};
use crate::performance::config::{PerformanceConfig, PerformanceThresholds};
use crate::performance::database::PerformanceDatabase;
use crate::performance::metrics::{PerformanceMetrics, TestResult, TestType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Performance testing interface
#[async_trait]
pub trait PerformanceTestProvider: Send + Sync {
    /// Test latency for a specific model
    async fn test_latency(&self, model: &str, prompt: &str) -> Result<Duration>;

    /// Test throughput for a specific model
    async fn test_throughput(&self, model: &str, prompt: &str) -> Result<f64>;

    /// Get available models for testing
    async fn get_available_models(&self) -> Result<Vec<String>>;
}

/// Performance testing engine
pub struct PerformanceTester {
    pub config: PerformanceConfig,
    database: Arc<PerformanceDatabase>,
    providers: HashMap<String, Box<dyn PerformanceTestProvider>>,
}

impl PerformanceTester {
    /// Create a new performance tester
    pub fn new(config: PerformanceConfig, database: Arc<PerformanceDatabase>) -> Self {
        Self {
            config,
            database,
            providers: HashMap::new(),
        }
    }

    /// Register a performance test provider
    pub fn register_provider<P>(&mut self, name: String, provider: P)
    where
        P: PerformanceTestProvider + 'static,
    {
        self.providers.insert(name, Box::new(provider));
    }

    /// Run latency test for a model
    pub async fn run_latency_test(&self, provider: &str, model: &str) -> Result<Vec<TestResult>> {
        let test_provider = self
            .providers
            .get(provider)
            .ok_or_else(|| ProxyError::ConfigError(format!("Provider {} not found", provider)))?;

        let mut results = Vec::with_capacity(self.config.test_runs);

        info!(
            "Running latency test for {}:{} ({} runs)",
            provider, model, self.config.test_runs
        );

        for i in 0..self.config.test_runs {
            let test_prompt = &self.config.test_prompts[i % self.config.test_prompts.len()];

            match test_provider.test_latency(model, test_prompt).await {
                Ok(latency) => {
                    let result = TestResult::latency_success(
                        model.to_string(),
                        provider.to_string(),
                        latency.as_millis() as f64,
                    );
                    results.push(result);
                    debug!(
                        "Latency test {}/{}: {}ms",
                        i + 1,
                        self.config.test_runs,
                        latency.as_millis()
                    );
                }
                Err(e) => {
                    let result = TestResult::failure(
                        TestType::Latency,
                        model.to_string(),
                        provider.to_string(),
                        e.to_string(),
                    );
                    results.push(result);
                    warn!(
                        "Latency test {}/{} failed: {}",
                        i + 1,
                        self.config.test_runs,
                        e
                    );
                }
            }

            // Add delay between tests
            if i < self.config.test_runs - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Store results in database
        for result in &results {
            if let Err(e) = self.database.store_test_result(result).await {
                error!("Failed to store test result: {}", e);
            }
        }

        // Update performance metrics
        if !results.is_empty() {
            let metrics = PerformanceMetrics::from_test_results(
                model.to_string(),
                provider.to_string(),
                results.clone(),
            );

            if let Err(e) = self.database.store_metrics(&metrics).await {
                error!("Failed to store performance metrics: {}", e);
            }
        }

        let success_count = results.iter().filter(|r| r.success).count();
        info!(
            "Latency test completed for {}:{}: {}/{} successful",
            provider, model, success_count, self.config.test_runs
        );

        Ok(results)
    }

    /// Run throughput test for a model
    pub async fn run_throughput_test(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Vec<TestResult>> {
        let test_provider = self
            .providers
            .get(provider)
            .ok_or_else(|| ProxyError::ConfigError(format!("Provider {} not found", provider)))?;

        let mut results = Vec::with_capacity(self.config.test_runs);

        info!(
            "Running throughput test for {}:{} ({} runs)",
            provider, model, self.config.test_runs
        );

        for i in 0..self.config.test_runs {
            let test_prompt = &self.config.test_prompts[i % self.config.test_prompts.len()];

            match test_provider.test_throughput(model, test_prompt).await {
                Ok(throughput) => {
                    // Estimate latency based on a typical response time
                    let estimated_latency = 1000.0; // 1 second default
                    let tokens_processed = (throughput * estimated_latency / 1000.0) as usize;

                    let result = TestResult::throughput_success(
                        model.to_string(),
                        provider.to_string(),
                        estimated_latency,
                        throughput,
                        tokens_processed,
                    );
                    results.push(result);
                    debug!(
                        "Throughput test {}/{}: {:.2} tokens/s",
                        i + 1,
                        self.config.test_runs,
                        throughput
                    );
                }
                Err(e) => {
                    let result = TestResult::failure(
                        TestType::Throughput,
                        model.to_string(),
                        provider.to_string(),
                        e.to_string(),
                    );
                    results.push(result);
                    warn!(
                        "Throughput test {}/{} failed: {}",
                        i + 1,
                        self.config.test_runs,
                        e
                    );
                }
            }

            // Add delay between tests
            if i < self.config.test_runs - 1 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Store results in database
        for result in &results {
            if let Err(e) = self.database.store_test_result(result).await {
                error!("Failed to store test result: {}", e);
            }
        }

        // Update performance metrics
        if !results.is_empty() {
            let metrics = PerformanceMetrics::from_test_results(
                model.to_string(),
                provider.to_string(),
                results.clone(),
            );

            if let Err(e) = self.database.store_metrics(&metrics).await {
                error!("Failed to store performance metrics: {}", e);
            }
        }

        let success_count = results.iter().filter(|r| r.success).count();
        info!(
            "Throughput test completed for {}:{}: {}/{} successful",
            provider, model, success_count, self.config.test_runs
        );

        Ok(results)
    }

    /// Get performance ranking for models
    pub async fn get_performance_ranking(
        &self,
        provider: &str,
        models: &[String],
    ) -> Result<Vec<(String, f64)>> {
        let mut rankings = Vec::with_capacity(models.len());

        for model in models {
            if let Some(metrics) = self
                .database
                .get_metrics_by_provider_model(provider, model)
                .await?
            {
                let score = metrics.calculate_score(&self.config.weights);
                rankings.push((model.clone(), score));
            }
        }

        // Sort by score (descending)
        rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(rankings)
    }

    /// Check if model meets performance thresholds
    pub async fn meets_thresholds(
        &self,
        provider: &str,
        model: &str,
        thresholds: &PerformanceThresholds,
    ) -> Result<bool> {
        if let Some(metrics) = self
            .database
            .get_metrics_by_provider_model(provider, model)
            .await?
        {
            Ok(metrics.meets_thresholds(thresholds))
        } else {
            Ok(false)
        }
    }

    /// Get performance metrics for a model
    pub async fn get_metrics(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Option<PerformanceMetrics>> {
        self.database
            .get_metrics_by_provider_model(provider, model)
            .await
    }

    /// Check if metrics need refreshing
    pub async fn needs_refresh(&self, provider: &str, model: &str) -> Result<bool> {
        if let Some(metrics) = self
            .database
            .get_metrics_by_provider_model(provider, model)
            .await?
        {
            Ok(metrics.is_stale(self.config.performance_window_hours))
        } else {
            Ok(true) // No metrics exist, need testing
        }
    }
}

/// Mock performance test provider for testing
pub struct MockPerformanceTestProvider {
    latency_base: Duration,
    throughput_base: f64,
    error_rate: f64,
}

impl MockPerformanceTestProvider {
    pub fn new(latency_base: Duration, throughput_base: f64, error_rate: f64) -> Self {
        Self {
            latency_base,
            throughput_base,
            error_rate,
        }
    }
}

#[async_trait]
impl PerformanceTestProvider for MockPerformanceTestProvider {
    async fn test_latency(&self, _model: &str, _prompt: &str) -> Result<Duration> {
        // Simulate network delay
        tokio::time::sleep(self.latency_base).await;

        // Simulate occasional errors
        if rand::random::<f64>() < self.error_rate {
            return Err(ProxyError::TestError("Simulated test failure".to_string()));
        }

        // Add some randomness
        let variance = self.latency_base.as_millis() as f64 * 0.2;
        let random_offset = (rand::random::<f64>() - 0.5) * 2.0 * variance;
        let final_latency = self.latency_base.as_millis() as f64 + random_offset;

        Ok(Duration::from_millis(final_latency as u64))
    }

    async fn test_throughput(&self, _model: &str, _prompt: &str) -> Result<f64> {
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Simulate occasional errors
        if rand::random::<f64>() < self.error_rate {
            return Err(ProxyError::TestError("Simulated test failure".to_string()));
        }

        // Add some randomness
        let variance = self.throughput_base * 0.1;
        let random_offset = (rand::random::<f64>() - 0.5) * 2.0 * variance;
        let final_throughput = self.throughput_base + random_offset;

        Ok(final_throughput.max(0.0))
    }

    async fn get_available_models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "gpt-3.5-turbo".to_string(),
            "gpt-4".to_string(),
            "claude-3-haiku".to_string(),
            "claude-3-sonnet".to_string(),
        ])
    }
}
