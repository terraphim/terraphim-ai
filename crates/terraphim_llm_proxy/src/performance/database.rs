//! Performance metrics database

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::ProxyError;
use crate::performance::config::PerformanceConfig;
use crate::performance::metrics::PerformanceMetrics;

/// In-memory and optionally persistent performance database
pub struct PerformanceDatabase {
    metrics: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
    config: PerformanceConfig,
}

impl PerformanceDatabase {
    /// Create new performance database
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Add or update performance metrics for a model
    pub async fn add_metrics(
        &self,
        key: String,
        metrics: PerformanceMetrics,
    ) -> Result<(), ProxyError> {
        let mut db = self.metrics.write().await;
        db.insert(key.clone(), metrics);
        debug!("Added performance metrics for key: {}", key);
        Ok(())
    }

    /// Get performance metrics for a specific model
    pub async fn get_metrics(&self, key: &str) -> Result<Option<PerformanceMetrics>, ProxyError> {
        let db = self.metrics.read().await;
        Ok(db.get(key).cloned())
    }

    /// Get performance metrics for a specific provider and model
    pub async fn get_metrics_by_provider_model(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<Option<PerformanceMetrics>, ProxyError> {
        let key = format!("{}:{}", provider, model);
        self.get_metrics(&key).await
    }

    /// Store performance metrics
    pub async fn store_metrics(&self, metrics: &PerformanceMetrics) -> Result<(), ProxyError> {
        let key = format!("{}:{}", metrics.provider_name, metrics.model_name);
        self.add_metrics(key, metrics.clone()).await
    }

    /// Store a test result
    pub async fn store_test_result(
        &self,
        result: &crate::performance::metrics::TestResult,
    ) -> Result<(), ProxyError> {
        let key = format!("{}:{}", result.provider_name, result.model_name);
        self.update_with_test_result(&key, result.clone()).await
    }

    /// Get all performance metrics
    pub async fn get_all_metrics(&self) -> Result<HashMap<String, PerformanceMetrics>, ProxyError> {
        let db = self.metrics.read().await;
        Ok(db.clone())
    }

    /// Get metrics for a specific provider
    pub async fn get_provider_metrics(
        &self,
        provider_name: &str,
    ) -> Result<Vec<PerformanceMetrics>, ProxyError> {
        let db = self.metrics.read().await;
        let provider_metrics: Vec<_> = db
            .values()
            .filter(|m| m.provider_name == provider_name)
            .cloned()
            .collect();
        Ok(provider_metrics)
    }

    /// Get the best performing model for a provider
    pub async fn get_best_performing_model(
        &self,
        provider_name: &str,
    ) -> Result<Option<(String, String)>, ProxyError> {
        let provider_metrics = self.get_provider_metrics(provider_name).await?;

        if provider_metrics.is_empty() {
            return Ok(None);
        }

        // Find the model with the highest performance score
        let best_metrics = provider_metrics.iter().max_by(|a, b| {
            let score_a = a.calculate_score(&self.config.weights);
            let score_b = b.calculate_score(&self.config.weights);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(best) = best_metrics {
            Ok(Some((best.provider_name.clone(), best.model_name.clone())))
        } else {
            Ok(None)
        }
    }

    /// Remove metrics for a specific model
    pub async fn remove_metrics(&self, key: &str) -> Result<(), ProxyError> {
        let mut db = self.metrics.write().await;
        db.remove(key);
        debug!("Removed performance metrics for key: {}", key);
        Ok(())
    }

    /// Clean up expired metrics based on TTL
    pub async fn cleanup_expired_metrics(&self) -> Result<usize, ProxyError> {
        let mut db = self.metrics.write().await;
        let initial_count = db.len();

        db.retain(|_, metrics| !metrics.is_stale(self.config.metrics_ttl_hours));

        let removed_count = initial_count - db.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired performance metrics", removed_count);
        }

        Ok(removed_count)
    }

    /// Get models that meet performance thresholds
    pub async fn get_models_meeting_thresholds(&self) -> Result<Vec<String>, ProxyError> {
        let db = self.metrics.read().await;
        let valid_models: Vec<String> = db
            .iter()
            .filter(|(_, metrics)| metrics.meets_thresholds(&self.config.thresholds))
            .map(|(key, _)| key.clone())
            .collect();
        Ok(valid_models)
    }

    /// Get models that need testing (stale or never tested)
    pub async fn get_models_needing_testing(
        &self,
        model_keys: &[String],
    ) -> Result<Vec<String>, ProxyError> {
        let db = self.metrics.read().await;
        let mut needing_testing = Vec::new();

        for key in model_keys {
            match db.get(key) {
                Some(metrics) => {
                    if metrics.is_stale(self.config.test_interval_minutes / 60) {
                        needing_testing.push(key.clone());
                    }
                }
                None => {
                    // Never tested
                    needing_testing.push(key.clone());
                }
            }
        }

        Ok(needing_testing)
    }

    /// Update metrics with new test result
    pub async fn update_with_test_result(
        &self,
        key: &str,
        result: crate::performance::metrics::TestResult,
    ) -> Result<(), ProxyError> {
        let mut db = self.metrics.write().await;

        match db.get_mut(key) {
            Some(metrics) => {
                metrics.update_with_result(result);
                debug!("Updated existing metrics for key: {}", key);
            }
            None => {
                // Create new metrics from this single result
                let metrics = PerformanceMetrics::from_test_results(
                    result.model_name.clone(),
                    result.provider_name.clone(),
                    vec![result],
                );
                db.insert(key.to_string(), metrics);
                debug!("Created new metrics for key: {}", key);
            }
        }

        Ok(())
    }

    /// Save metrics to disk if persistence is configured
    pub async fn save_to_disk(&self) -> Result<(), ProxyError> {
        if let Some(path) = &self.config.persistence_path {
            let db = self.metrics.read().await;
            let json = serde_json::to_string_pretty(&*db)
                .map_err(|e| ProxyError::Internal(format!("Failed to serialize metrics: {}", e)))?;

            tokio::fs::write(path, json).await.map_err(|e| {
                ProxyError::Internal(format!("Failed to write metrics to {}: {}", path, e))
            })?;

            info!("Saved {} performance metrics to {}", db.len(), path);
        }
        Ok(())
    }

    /// Load metrics from disk if persistence is configured
    pub async fn load_from_disk(&self) -> Result<(), ProxyError> {
        if let Some(path) = &self.config.persistence_path {
            match tokio::fs::read_to_string(path).await {
                Ok(content) => {
                    let loaded_metrics: HashMap<String, PerformanceMetrics> =
                        serde_json::from_str(&content).map_err(|e| {
                            ProxyError::Internal(format!("Failed to deserialize metrics: {}", e))
                        })?;

                    let mut db = self.metrics.write().await;
                    db.extend(loaded_metrics);

                    info!("Loaded {} performance metrics from {}", db.len(), path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    debug!("No existing metrics file found at {}", path);
                }
                Err(e) => {
                    warn!("Failed to load metrics from {}: {}", path, e);
                }
            }
        }
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, ProxyError> {
        let db = self.metrics.read().await;
        let total_models = db.len();

        let mut providers = std::collections::HashSet::new();
        let mut models_meeting_thresholds = 0;
        let mut stale_models = 0;

        for metrics in db.values() {
            providers.insert(&metrics.provider_name);

            if metrics.meets_thresholds(&self.config.thresholds) {
                models_meeting_thresholds += 1;
            }

            if metrics.is_stale(self.config.metrics_ttl_hours) {
                stale_models += 1;
            }
        }

        Ok(DatabaseStats {
            total_models,
            total_providers: providers.len(),
            models_meeting_thresholds,
            stale_models,
            last_updated: SystemTime::now(),
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_models: usize,
    pub total_providers: usize,
    pub models_meeting_thresholds: usize,
    pub stale_models: usize,
    pub last_updated: SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::config::{PerformanceThresholds, PerformanceWeights};
    use std::time::Duration;

    fn create_test_config() -> PerformanceConfig {
        PerformanceConfig {
            enabled: true,
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
            ],
            thresholds: PerformanceThresholds::default(),
            persistence_path: None,
            metrics_ttl_hours: 24,
        }
    }

    fn create_test_metrics() -> PerformanceMetrics {
        PerformanceMetrics {
            model_name: "test-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 150.0,
            p95_latency_ms: 300.0,
            p99_latency_ms: 500.0,
            throughput_tokens_per_sec: 25.0,
            success_rate: 0.98,
            error_rate: 0.02,
            last_tested: SystemTime::now(),
            test_count: 10,
        }
    }

    #[tokio::test]
    async fn test_add_and_get_metrics() {
        let db = PerformanceDatabase::new(create_test_config());
        let metrics = create_test_metrics();
        let key = "test-provider:test-model";

        db.add_metrics(key.to_string(), metrics.clone())
            .await
            .unwrap();

        let retrieved = db.get_metrics(key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().model_name, "test-model");
    }

    #[tokio::test]
    async fn test_get_all_metrics() {
        let db = PerformanceDatabase::new(create_test_config());

        let metrics1 = create_test_metrics();
        let metrics2 = PerformanceMetrics {
            model_name: "test-model-2".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 200.0,
            p95_latency_ms: 400.0,
            p99_latency_ms: 600.0,
            throughput_tokens_per_sec: 30.0,
            success_rate: 0.95,
            error_rate: 0.05,
            last_tested: SystemTime::now(),
            test_count: 8,
        };

        db.add_metrics("test-provider:test-model".to_string(), metrics1)
            .await
            .unwrap();
        db.add_metrics("test-provider:test-model-2".to_string(), metrics2)
            .await
            .unwrap();

        let all_metrics = db.get_all_metrics().await.unwrap();
        assert_eq!(all_metrics.len(), 2);
    }

    #[tokio::test]
    async fn test_get_provider_metrics() {
        let db = PerformanceDatabase::new(create_test_config());

        let metrics1 = create_test_metrics();
        let metrics2 = PerformanceMetrics {
            model_name: "test-model-2".to_string(),
            provider_name: "other-provider".to_string(),
            avg_latency_ms: 200.0,
            p95_latency_ms: 400.0,
            p99_latency_ms: 600.0,
            throughput_tokens_per_sec: 30.0,
            success_rate: 0.95,
            error_rate: 0.05,
            last_tested: SystemTime::now(),
            test_count: 8,
        };

        db.add_metrics("test-provider:test-model".to_string(), metrics1)
            .await
            .unwrap();
        db.add_metrics("other-provider:test-model-2".to_string(), metrics2)
            .await
            .unwrap();

        let provider_metrics = db.get_provider_metrics("test-provider").await.unwrap();
        assert_eq!(provider_metrics.len(), 1);
        assert_eq!(provider_metrics[0].model_name, "test-model");
    }

    #[tokio::test]
    async fn test_remove_metrics() {
        let db = PerformanceDatabase::new(create_test_config());
        let metrics = create_test_metrics();
        let key = "test-provider:test-model";

        db.add_metrics(key.to_string(), metrics).await.unwrap();
        db.remove_metrics(key).await.unwrap();

        let retrieved = db.get_metrics(key).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_metrics() {
        let mut config = create_test_config();
        config.metrics_ttl_hours = 1; // 1 hour TTL

        let db = PerformanceDatabase::new(config);

        // Add fresh metrics
        let fresh_metrics = PerformanceMetrics {
            model_name: "fresh-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 100.0,
            p95_latency_ms: 200.0,
            p99_latency_ms: 300.0,
            throughput_tokens_per_sec: 25.0,
            success_rate: 0.98,
            error_rate: 0.02,
            last_tested: SystemTime::now(),
            test_count: 10,
        };

        // Add stale metrics (2 hours old)
        let stale_metrics = PerformanceMetrics {
            model_name: "stale-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 150.0,
            p95_latency_ms: 300.0,
            p99_latency_ms: 500.0,
            throughput_tokens_per_sec: 20.0,
            success_rate: 0.95,
            error_rate: 0.05,
            last_tested: SystemTime::now() - Duration::from_secs(7200), // 2 hours ago
            test_count: 5,
        };

        db.add_metrics("test-provider:fresh-model".to_string(), fresh_metrics)
            .await
            .unwrap();
        db.add_metrics("test-provider:stale-model".to_string(), stale_metrics)
            .await
            .unwrap();

        let removed_count = db.cleanup_expired_metrics().await.unwrap();
        assert_eq!(removed_count, 1);

        let all_metrics = db.get_all_metrics().await.unwrap();
        assert_eq!(all_metrics.len(), 1);
        assert!(all_metrics.contains_key("test-provider:fresh-model"));
        assert!(!all_metrics.contains_key("test-provider:stale-model"));
    }

    #[tokio::test]
    async fn test_get_best_performing_model() {
        let db = PerformanceDatabase::new(create_test_config());

        let good_metrics = PerformanceMetrics {
            model_name: "good-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 100.0, // Better latency
            p95_latency_ms: 200.0,
            p99_latency_ms: 300.0,
            throughput_tokens_per_sec: 50.0, // Better throughput
            success_rate: 0.99,
            error_rate: 0.01,
            last_tested: SystemTime::now(),
            test_count: 20,
        };

        let poor_metrics = PerformanceMetrics {
            model_name: "poor-model".to_string(),
            provider_name: "test-provider".to_string(),
            avg_latency_ms: 500.0, // Worse latency
            p95_latency_ms: 800.0,
            p99_latency_ms: 1200.0,
            throughput_tokens_per_sec: 10.0, // Worse throughput
            success_rate: 0.90,
            error_rate: 0.10,
            last_tested: SystemTime::now(),
            test_count: 5,
        };

        db.add_metrics("test-provider:good-model".to_string(), good_metrics)
            .await
            .unwrap();
        db.add_metrics("test-provider:poor-model".to_string(), poor_metrics)
            .await
            .unwrap();

        let best = db.get_best_performing_model("test-provider").await.unwrap();
        assert!(best.is_some());
        let (provider, model) = best.unwrap();
        assert_eq!(provider, "test-provider");
        assert_eq!(model, "good-model");
    }

    #[tokio::test]
    async fn test_database_stats() {
        let db = PerformanceDatabase::new(create_test_config());

        let metrics = create_test_metrics();
        db.add_metrics("test-provider:test-model".to_string(), metrics)
            .await
            .unwrap();

        let stats = db.get_stats().await.unwrap();
        assert_eq!(stats.total_models, 1);
        assert_eq!(stats.total_providers, 1);
        assert_eq!(stats.models_meeting_thresholds, 1); // Should meet default thresholds
        assert_eq!(stats.stale_models, 0); // Should not be stale
    }
}
