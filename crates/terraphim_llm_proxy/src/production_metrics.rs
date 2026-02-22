//! Production Metrics Layer
//!
//! This module provides production-grade metrics collection, aggregation,
//! and export capabilities for monitoring and observability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::info;

use crate::metrics::{ProviderMetrics, RequestContext, RoutingMetrics};

/// Aggregated metrics for production monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// ISO 8601 timestamp when metrics were collected
    pub timestamp: String,
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// P95 response time in milliseconds
    pub p95_response_time_ms: u64,
    /// P99 response time in milliseconds
    pub p99_response_time_ms: u64,
    /// Total tokens processed
    pub total_tokens: u64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// Provider-specific metrics
    pub provider_metrics: HashMap<String, ProviderStats>,
    /// Routing metrics
    pub routing_metrics: RoutingStats,
    /// Session metrics
    pub session_metrics: SessionStats,
    /// System health indicators
    pub system_health: SystemHealth,
}

/// Provider-specific statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub provider_name: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub total_tokens: u64,
    pub error_rate: f64,
    pub last_used: Option<u64>,
    pub is_healthy: bool,
}

/// Routing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingStats {
    pub total_routing_decisions: u64,
    pub fallback_used: u64,
    pub avg_decision_time_ms: f64,
    pub scenario_distribution: HashMap<String, u64>,
    pub provider_distribution: HashMap<String, u64>,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub active_sessions: u64,
    pub max_sessions: u64,
    pub cache_hit_rate: f64,
    pub avg_session_duration_minutes: f64,
    pub total_sessions_created: u64,
    pub total_sessions_expired: u64,
}

/// System health indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub uptime_seconds: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub last_health_check: u64,
    pub health_issues: Vec<String>,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Production metrics collector
pub struct ProductionMetricsCollector {
    /// Internal metrics storage
    metrics: Arc<Mutex<InternalMetrics>>,
    /// Start time for uptime calculation
    start_time: Instant,
    /// Metrics collection interval
    collection_interval: Duration,
    /// Request history for percentile calculations
    request_history: Arc<Mutex<Vec<RequestSample>>>,
    /// Max history size for percentile calculations
    max_history_size: usize,
}

/// Internal metrics storage
#[derive(Debug)]
struct InternalMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_response_time_ms: u64,
    total_tokens: u64,
    provider_stats: HashMap<String, ProviderInternalStats>,
    routing_stats: RoutingInternalStats,
    session_stats: SessionInternalStats,
    error_samples: Vec<ErrorSample>,
    last_minute_requests: Vec<u64>, // For requests per second calculation
}

/// Internal provider statistics
#[derive(Debug)]
struct ProviderInternalStats {
    #[allow(dead_code)]
    name: String,
    requests: u64,
    successes: u64,
    failures: u64,
    response_times_ms: Vec<u64>,
    tokens: u64,
    last_used: Option<Instant>,
    consecutive_failures: u32,
}

/// Internal routing statistics
#[derive(Debug)]
struct RoutingInternalStats {
    total_decisions: u64,
    fallback_count: u64,
    decision_times_ms: Vec<u64>,
    scenario_counts: HashMap<String, u64>,
    provider_counts: HashMap<String, u64>,
}

/// Internal session statistics
#[derive(Debug)]
struct SessionInternalStats {
    active_sessions: u64,
    max_sessions: u64,
    sessions_created: u64,
    sessions_expired: u64,
    cache_hits: u64,
    cache_misses: u64,
    session_durations: Vec<u64>,
}

/// Sample of a request for metric calculations
#[derive(Debug, Clone)]
struct RequestSample {
    #[allow(dead_code)]
    timestamp: Instant,
    response_time_ms: u64,
    #[allow(dead_code)]
    success: bool,
    #[allow(dead_code)]
    provider: String,
    #[allow(dead_code)]
    tokens: u64,
}

/// Sample of an error for monitoring
#[derive(Debug, Clone)]
struct ErrorSample {
    timestamp: Instant,
    #[allow(dead_code)]
    error_type: String,
    #[allow(dead_code)]
    provider: String,
    #[allow(dead_code)]
    context: String,
}

impl ProductionMetricsCollector {
    /// Create a new production metrics collector
    pub fn new(collection_interval_secs: u64, max_history_size: usize) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(InternalMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                total_response_time_ms: 0,
                total_tokens: 0,
                provider_stats: HashMap::new(),
                routing_stats: RoutingInternalStats {
                    total_decisions: 0,
                    fallback_count: 0,
                    decision_times_ms: Vec::new(),
                    scenario_counts: HashMap::new(),
                    provider_counts: HashMap::new(),
                },
                session_stats: SessionInternalStats {
                    active_sessions: 0,
                    max_sessions: 0,
                    sessions_created: 0,
                    sessions_expired: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    session_durations: Vec::new(),
                },
                error_samples: Vec::new(),
                last_minute_requests: Vec::new(),
            })),
            start_time: Instant::now(),
            collection_interval: Duration::from_secs(collection_interval_secs),
            request_history: Arc::new(Mutex::new(Vec::new())),
            max_history_size,
        }
    }

    /// Record a completed request
    pub fn record_request(&self, context: &RequestContext, provider_metrics: &ProviderMetrics) {
        let response_time_ms = provider_metrics
            .response_time_ms
            .unwrap_or_else(|| context.elapsed().as_millis() as u64);
        let success = provider_metrics.status == crate::metrics::RequestStatus::Success;
        let token_count = provider_metrics.token_count.unwrap_or(0);

        // Update internal metrics
        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_requests += 1;

            if success {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }

            metrics.total_response_time_ms += response_time_ms;
            metrics.total_tokens += token_count as u64;

            // Update provider stats
            let provider_stat = metrics
                .provider_stats
                .entry(provider_metrics.provider.clone())
                .or_insert_with(|| ProviderInternalStats {
                    name: provider_metrics.provider.clone(),
                    requests: 0,
                    successes: 0,
                    failures: 0,
                    response_times_ms: Vec::new(),
                    tokens: 0,
                    last_used: None,
                    consecutive_failures: 0,
                });

            provider_stat.requests += 1;
            if success {
                provider_stat.successes += 1;
                provider_stat.consecutive_failures = 0;
            } else {
                provider_stat.failures += 1;
                provider_stat.consecutive_failures += 1;
            }

            provider_stat.response_times_ms.push(response_time_ms);
            provider_stat.tokens += token_count as u64;
            provider_stat.last_used = Some(Instant::now());

            // Keep only recent response times (last 1000)
            if provider_stat.response_times_ms.len() > 1000 {
                provider_stat.response_times_ms.drain(0..500);
            }

            // Record error samples for failed requests
            if !success {
                if let Some(error_type) = &provider_metrics.error_type {
                    metrics.error_samples.push(ErrorSample {
                        timestamp: Instant::now(),
                        error_type: error_type.clone(),
                        provider: provider_metrics.provider.clone(),
                        context: format!("Request ID: {}", context.request_id),
                    });

                    // Keep only recent error samples
                    if metrics.error_samples.len() > 100 {
                        metrics.error_samples.drain(0..50);
                    }
                }
            }
        }

        // Add to request history for percentile calculations
        {
            let mut history = self.request_history.lock().unwrap();
            history.push(RequestSample {
                timestamp: Instant::now(),
                response_time_ms,
                success,
                provider: provider_metrics.provider.clone(),
                tokens: token_count as u64,
            });

            // Keep history within bounds
            if history.len() > self.max_history_size {
                history.drain(0..self.max_history_size / 2);
            }
        }

        // Update last minute requests for RPS calculation
        {
            let mut metrics = self.metrics.lock().unwrap();
            let now = Instant::now();

            // Remove requests older than 1 minute
            while let Some(&front_time) = metrics.last_minute_requests.first() {
                if now.duration_since(self.start_time + Duration::from_secs(front_time))
                    > Duration::from_secs(60)
                {
                    metrics.last_minute_requests.remove(0);
                } else {
                    break;
                }
            }

            metrics
                .last_minute_requests
                .push(now.duration_since(self.start_time).as_secs());
        }
    }

    /// Record a routing decision
    pub fn record_routing(&self, routing_metrics: &RoutingMetrics) {
        let mut metrics = self.metrics.lock().unwrap();

        metrics.routing_stats.total_decisions += 1;
        metrics
            .routing_stats
            .decision_times_ms
            .push(routing_metrics.decision_time_ms);

        if routing_metrics.fallback_used {
            metrics.routing_stats.fallback_count += 1;
        }

        // Update scenario distribution
        *metrics
            .routing_stats
            .scenario_counts
            .entry(routing_metrics.scenario.clone())
            .or_insert(0) += 1;

        // Update provider distribution
        *metrics
            .routing_stats
            .provider_counts
            .entry(routing_metrics.provider.clone())
            .or_insert(0) += 1;

        // Keep only recent decision times
        if metrics.routing_stats.decision_times_ms.len() > 1000 {
            metrics.routing_stats.decision_times_ms.drain(0..500);
        }
    }

    /// Record session metrics
    pub fn record_session_created(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.session_stats.sessions_created += 1;
    }

    /// Record session expiration
    pub fn record_session_expired(&self, duration_minutes: u64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.session_stats.sessions_expired += 1;
        metrics
            .session_stats
            .session_durations
            .push(duration_minutes);

        // Keep only recent session durations
        if metrics.session_stats.session_durations.len() > 1000 {
            metrics.session_stats.session_durations.drain(0..500);
        }
    }

    /// Record session cache hit
    pub fn record_cache_hit(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.session_stats.cache_hits += 1;
    }

    /// Record session cache miss
    pub fn record_cache_miss(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.session_stats.cache_misses += 1;
    }

    /// Update active session count
    pub fn update_active_sessions(&self, active_sessions: u64, max_sessions: u64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.session_stats.active_sessions = active_sessions;
        metrics.session_stats.max_sessions = max_sessions;
    }

    /// Get aggregated metrics
    pub fn get_aggregated_metrics(&self) -> AggregatedMetrics {
        let metrics = self.metrics.lock().unwrap();
        let request_history = self.request_history.lock().unwrap();

        let total_requests = metrics.total_requests;
        let successful_requests = metrics.successful_requests;
        let failed_requests = metrics.failed_requests;

        // Calculate response time percentiles
        let mut response_times: Vec<u64> = request_history
            .iter()
            .map(|sample| sample.response_time_ms)
            .collect();
        response_times.sort_unstable();

        let avg_response_time_ms = if total_requests > 0 {
            metrics.total_response_time_ms as f64 / total_requests as f64
        } else {
            0.0
        };

        let p95_response_time_ms = if !response_times.is_empty() {
            response_times[(response_times.len() as f64 * 0.95) as usize]
        } else {
            0
        };

        let p99_response_time_ms = if !response_times.is_empty() {
            response_times[(response_times.len() as f64 * 0.99) as usize]
        } else {
            0
        };

        // Calculate requests per second
        let requests_per_second = metrics.last_minute_requests.len() as f64 / 60.0;

        // Calculate error rate
        let error_rate = if total_requests > 0 {
            failed_requests as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        };

        // Aggregate provider metrics
        let mut provider_metrics = HashMap::new();
        for (provider_name, provider_stat) in &metrics.provider_stats {
            let provider_error_rate = if provider_stat.requests > 0 {
                provider_stat.failures as f64 / provider_stat.requests as f64 * 100.0
            } else {
                0.0
            };

            let avg_provider_response_time = if provider_stat.requests > 0 {
                provider_stat.response_times_ms.iter().sum::<u64>() as f64
                    / provider_stat.requests as f64
            } else {
                0.0
            };

            provider_metrics.insert(
                provider_name.clone(),
                ProviderStats {
                    provider_name: provider_name.clone(),
                    total_requests: provider_stat.requests,
                    successful_requests: provider_stat.successes,
                    failed_requests: provider_stat.failures,
                    avg_response_time_ms: avg_provider_response_time,
                    total_tokens: provider_stat.tokens,
                    error_rate: provider_error_rate,
                    last_used: provider_stat
                        .last_used
                        .map(|t| t.duration_since(self.start_time).as_secs()),
                    is_healthy: provider_stat.consecutive_failures < 5,
                },
            );
        }

        // Aggregate routing metrics
        let avg_decision_time = if metrics.routing_stats.total_decisions > 0 {
            metrics.routing_stats.decision_times_ms.iter().sum::<u64>() as f64
                / metrics.routing_stats.total_decisions as f64
        } else {
            0.0
        };

        let routing_metrics = RoutingStats {
            total_routing_decisions: metrics.routing_stats.total_decisions,
            fallback_used: metrics.routing_stats.fallback_count,
            avg_decision_time_ms: avg_decision_time,
            scenario_distribution: metrics.routing_stats.scenario_counts.clone(),
            provider_distribution: metrics.routing_stats.provider_counts.clone(),
        };

        // Aggregate session metrics
        let total_cache_operations =
            metrics.session_stats.cache_hits + metrics.session_stats.cache_misses;
        let cache_hit_rate = if total_cache_operations > 0 {
            (metrics.session_stats.cache_hits as f64 / total_cache_operations as f64
                * 100.0
                * 100.0)
                .round()
                / 100.0
        } else {
            0.0
        };

        let avg_session_duration = if !metrics.session_stats.session_durations.is_empty() {
            metrics.session_stats.session_durations.iter().sum::<u64>() as f64
                / metrics.session_stats.session_durations.len() as f64
        } else {
            0.0
        };

        let session_metrics = SessionStats {
            active_sessions: metrics.session_stats.active_sessions,
            max_sessions: metrics.session_stats.max_sessions,
            cache_hit_rate,
            avg_session_duration_minutes: avg_session_duration,
            total_sessions_created: metrics.session_stats.sessions_created,
            total_sessions_expired: metrics.session_stats.sessions_expired,
        };

        // System health assessment
        let health_issues = Vec::new();
        let status = if error_rate >= 50.0 || avg_response_time_ms > 5000.0 {
            HealthStatus::Unhealthy
        } else if error_rate >= 5.0 || avg_response_time_ms > 2000.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let system_health = SystemHealth {
            status,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            memory_usage_mb: self.get_memory_usage(),
            cpu_usage_percent: self.get_cpu_usage(),
            disk_usage_percent: self.get_disk_usage(),
            last_health_check: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            health_issues,
        };

        AggregatedMetrics {
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_requests,
            successful_requests,
            failed_requests,
            avg_response_time_ms,
            p95_response_time_ms,
            p99_response_time_ms,
            total_tokens: metrics.total_tokens,
            requests_per_second,
            error_rate,
            provider_metrics,
            routing_metrics,
            session_metrics,
            system_health,
        }
    }

    /// Start background metrics collection
    pub async fn start_collection_loop(&self) {
        let interval_duration = self.collection_interval;
        let metrics = Arc::clone(&self.metrics);

        let mut interval = interval(interval_duration);

        loop {
            interval.tick().await;

            // Clean up old data
            {
                let mut m = metrics.lock().unwrap();

                // Clean old error samples
                let cutoff = Instant::now() - Duration::from_secs(3600); // 1 hour
                m.error_samples.retain(|sample| sample.timestamp > cutoff);

                // Clean old last minute requests
                let minute_ago = Instant::now() - Duration::from_secs(60);
                let current_time = self.start_time;
                m.last_minute_requests.retain(|&timestamp| {
                    current_time + Duration::from_secs(timestamp) > minute_ago
                });
            }

            info!("Production metrics collection completed");
        }
    }

    /// Get memory usage (simplified)
    fn get_memory_usage(&self) -> f64 {
        // In a real implementation, you'd use system APIs to get actual memory usage
        // For now, return a placeholder
        42.0 // MB
    }

    /// Get CPU usage (simplified)
    fn get_cpu_usage(&self) -> f64 {
        // In a real implementation, you'd use system APIs to get actual CPU usage
        // For now, return a placeholder
        15.0 // Percent
    }

    /// Get disk usage (simplified)
    fn get_disk_usage(&self) -> f64 {
        // In a real implementation, you'd use system APIs to get actual disk usage
        // For now, return a placeholder
        60.0 // Percent
    }
}

impl Default for ProductionMetricsCollector {
    fn default() -> Self {
        Self::new(60, 10000) // Collect every minute, keep 10k samples
    }
}

/// Metrics exporter for different formats
pub struct MetricsExporter;

impl MetricsExporter {
    /// Export metrics as JSON
    pub fn export_json(metrics: &AggregatedMetrics) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(metrics)
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(metrics: &AggregatedMetrics) -> String {
        let mut output = String::new();

        // Request metrics
        output.push_str(&format!(
            "# HELP terraphim_requests_total Total number of requests\n\
            # TYPE terraphim_requests_total counter\n\
            terraphim_requests_total {}\n\n",
            metrics.total_requests
        ));

        output.push_str(&format!(
            "# HELP terraphim_requests_successful_total Total successful requests\n\
            # TYPE terraphim_requests_successful_total counter\n\
            terraphim_requests_successful_total {}\n\n",
            metrics.successful_requests
        ));

        output.push_str(&format!(
            "# HELP terraphim_requests_failed_total Total failed requests\n\
            # TYPE terraphim_requests_failed_total counter\n\
            terraphim_requests_failed_total {}\n\n",
            metrics.failed_requests
        ));

        // Response time metrics
        output.push_str(&format!(
            "# HELP terraphim_response_time_ms Average response time in milliseconds\n\
            # TYPE terraphim_response_time_ms gauge\n\
            terraphim_response_time_ms {:.2}\n\n",
            metrics.avg_response_time_ms
        ));

        output.push_str(&format!(
            "# HELP terraphim_response_time_p95_ms 95th percentile response time in milliseconds\n\
            # TYPE terraphim_response_time_p95_ms gauge\n\
            terraphim_response_time_p95_ms {}\n\n",
            metrics.p95_response_time_ms
        ));

        output.push_str(&format!(
            "# HELP terraphim_response_time_p99_ms 99th percentile response time in milliseconds\n\
            # TYPE terraphim_response_time_p99_ms gauge\n\
            terraphim_response_time_p99_ms {}\n\n",
            metrics.p99_response_time_ms
        ));

        // Token metrics
        output.push_str(&format!(
            "# HELP terraphim_tokens_total Total tokens processed\n\
            # TYPE terraphim_tokens_total counter\n\
            terraphim_tokens_total {}\n\n",
            metrics.total_tokens
        ));

        // Rate metrics
        output.push_str(&format!(
            "# HELP terraphim_requests_per_second Requests per second\n\
            # TYPE terraphim_requests_per_second gauge\n\
            terraphim_requests_per_second {:.2}\n\n",
            metrics.requests_per_second
        ));

        output.push_str(&format!(
            "# HELP terraphim_error_rate Error rate percentage\n\
            # TYPE terraphim_error_rate gauge\n\
            terraphim_error_rate {:.2}\n\n",
            metrics.error_rate
        ));

        // Provider metrics
        for (provider_name, provider_stats) in &metrics.provider_metrics {
            output.push_str(&format!(
                "# HELP terraphim_provider_requests_total Total requests for provider {}\n\
                # TYPE terraphim_provider_requests_total counter\n\
                terraphim_provider_requests_total{{provider=\"{}\"}} {}\n\n",
                provider_name, provider_name, provider_stats.total_requests
            ));

            output.push_str(&format!(
                "# HELP terraphim_provider_response_time_ms Average response time for provider {}\n\
                # TYPE terraphim_provider_response_time_ms gauge\n\
                terraphim_provider_response_time_ms{{provider=\"{}\"}} {:.2}\n\n",
                provider_name, provider_name, provider_stats.avg_response_time_ms
            ));

            output.push_str(&format!(
                "# HELP terraphim_provider_error_rate Error rate for provider {}\n\
                # TYPE terraphim_provider_error_rate gauge\n\
                terraphim_provider_error_rate{{provider=\"{}\"}} {:.2}\n\n",
                provider_name, provider_name, provider_stats.error_rate
            ));
        }

        // Session metrics
        output.push_str(&format!(
            "# HELP terraphim_active_sessions Current number of active sessions\n\
            # TYPE terraphim_active_sessions gauge\n\
            terraphim_active_sessions {}\n\n",
            metrics.session_metrics.active_sessions
        ));

        output.push_str(&format!(
            "# HELP terraphim_session_cache_hit_rate Session cache hit rate percentage\n\
            # TYPE terraphim_session_cache_hit_rate gauge\n\
            terraphim_session_cache_hit_rate {:.2}\n\n",
            metrics.session_metrics.cache_hit_rate
        ));

        // System health
        output.push_str(&format!(
            "# HELP terraphim_uptime_seconds System uptime in seconds\n\
            # TYPE terraphim_uptime_seconds counter\n\
            terraphim_uptime_seconds {}\n\n",
            metrics.system_health.uptime_seconds
        ));

        output.push_str(&format!(
            "# HELP terraphim_memory_usage_mb Memory usage in MB\n\
            # TYPE terraphim_memory_usage_mb gauge\n\
            terraphim_memory_usage_mb {:.2}\n\n",
            metrics.system_health.memory_usage_mb
        ));

        output.push_str(&format!(
            "# HELP terraphim_cpu_usage_percent CPU usage percentage\n\
            # TYPE terraphim_cpu_usage_percent gauge\n\
            terraphim_cpu_usage_percent {:.2}\n\n",
            metrics.system_health.cpu_usage_percent
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = ProductionMetricsCollector::new(30, 5000);
        assert_eq!(collector.collection_interval, Duration::from_secs(30));
        assert_eq!(collector.max_history_size, 5000);
    }

    #[test]
    fn test_metrics_collector_default() {
        let collector = ProductionMetricsCollector::default();
        assert_eq!(collector.collection_interval, Duration::from_secs(60));
        assert_eq!(collector.max_history_size, 10000);
    }

    #[test]
    fn test_request_recording() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        let context = RequestContext::new();
        let provider_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_response_time(Duration::from_millis(100))
        .with_tokens(50)
        .success();

        collector.record_request(&context, &provider_metrics);

        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.total_requests, 1);
        assert_eq!(aggregated.successful_requests, 1);
        assert_eq!(aggregated.failed_requests, 0);
        assert_eq!(aggregated.total_tokens, 50);
        assert_eq!(aggregated.avg_response_time_ms, 100.0);
        assert_eq!(aggregated.error_rate, 0.0);
    }

    #[test]
    fn test_provider_stats_aggregation() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        let context = RequestContext::new();
        let provider_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_response_time(Duration::from_millis(100))
        .with_tokens(50)
        .success();

        // Record multiple requests
        for _ in 0..10 {
            collector.record_request(&context, &provider_metrics);
        }

        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.total_requests, 10);
        assert_eq!(aggregated.successful_requests, 10);

        let provider_stats = aggregated.provider_metrics.get("test_provider").unwrap();
        assert_eq!(provider_stats.total_requests, 10);
        assert_eq!(provider_stats.successful_requests, 10);
        assert_eq!(provider_stats.error_rate, 0.0);
        assert_eq!(provider_stats.total_tokens, 500); // 10 * 50
        assert!(provider_stats.is_healthy);
    }

    #[test]
    fn test_error_rate_calculation() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        let context = RequestContext::new();

        // Record successful requests
        let success_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_tokens(50)
        .success();

        for _ in 0..8 {
            collector.record_request(&context, &success_metrics);
        }

        // Record failed requests
        let error_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_tokens(50)
        .error("timeout".to_string());

        for _ in 0..2 {
            collector.record_request(&context, &error_metrics);
        }

        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.total_requests, 10);
        assert_eq!(aggregated.successful_requests, 8);
        assert_eq!(aggregated.failed_requests, 2);
        assert_eq!(aggregated.error_rate, 20.0);
    }

    #[test]
    fn test_routing_metrics_recording() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        let routing_metrics = RoutingMetrics::new(
            "req_456".to_string(),
            "Thinking".to_string(),
            "deepseek".to_string(),
            "deepseek-chat".to_string(),
        )
        .with_decision_time(Duration::from_millis(50));

        collector.record_routing(&routing_metrics);

        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.routing_metrics.total_routing_decisions, 1);
        assert_eq!(aggregated.routing_metrics.avg_decision_time_ms, 50.0);
        assert_eq!(aggregated.routing_metrics.fallback_used, 0);
    }

    #[test]
    fn test_session_metrics_recording() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        // Record session operations
        collector.record_session_created();
        collector.record_cache_hit();
        collector.record_cache_miss();
        collector.record_cache_hit();
        collector.record_session_expired(30);

        collector.update_active_sessions(5, 100);

        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.session_metrics.active_sessions, 5);
        assert_eq!(aggregated.session_metrics.max_sessions, 100);
        assert_eq!(aggregated.session_metrics.cache_hit_rate, 66.67);
        assert_eq!(aggregated.session_metrics.total_sessions_created, 1);
        assert_eq!(aggregated.session_metrics.total_sessions_expired, 1);
        assert_eq!(
            aggregated.session_metrics.avg_session_duration_minutes,
            30.0
        );
    }

    #[test]
    fn test_health_status_assessment() {
        let collector = ProductionMetricsCollector::new(60, 1000);

        // Test healthy status
        let context = RequestContext::new();
        let success_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_response_time(Duration::from_millis(100))
        .success();

        collector.record_request(&context, &success_metrics);
        let aggregated = collector.get_aggregated_metrics();
        assert_eq!(aggregated.system_health.status, HealthStatus::Healthy);

        // Test degraded status (high error rate)
        let error_metrics = ProviderMetrics::new(
            "test_provider".to_string(),
            "test_model".to_string(),
            "https://api.test.com".to_string(),
            "req_123".to_string(),
        )
        .with_tokens(50)
        .error("timeout".to_string());

        for _ in 0..6 {
            collector.record_request(&context, &error_metrics);
        }

        let degraded = collector.get_aggregated_metrics();
        assert_eq!(degraded.system_health.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_json_export() {
        let metrics = AggregatedMetrics {
            timestamp: "2026-01-13T12:00:00Z".to_string(),
            total_requests: 100,
            successful_requests: 95,
            failed_requests: 5,
            avg_response_time_ms: 250.5,
            p95_response_time_ms: 800,
            p99_response_time_ms: 1200,
            total_tokens: 5000,
            requests_per_second: 1.67,
            error_rate: 5.0,
            provider_metrics: HashMap::new(),
            routing_metrics: RoutingStats {
                total_routing_decisions: 100,
                fallback_used: 5,
                avg_decision_time_ms: 15.2,
                scenario_distribution: HashMap::new(),
                provider_distribution: HashMap::new(),
            },
            session_metrics: SessionStats {
                active_sessions: 25,
                max_sessions: 100,
                cache_hit_rate: 85.5,
                avg_session_duration_minutes: 12.3,
                total_sessions_created: 100,
                total_sessions_expired: 75,
            },
            system_health: SystemHealth {
                status: HealthStatus::Healthy,
                uptime_seconds: 3600,
                memory_usage_mb: 256.7,
                cpu_usage_percent: 15.3,
                disk_usage_percent: 45.2,
                last_health_check: 1234567890,
                health_issues: vec![],
            },
        };

        let json = MetricsExporter::export_json(&metrics).unwrap();
        assert!(json.contains("\"total_requests\": 100"));
        assert!(json.contains("\"error_rate\": 5.0"));
        assert!(json.contains("\"status\": \"Healthy\""));
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = AggregatedMetrics {
            timestamp: "2026-01-13T12:00:00Z".to_string(),
            total_requests: 100,
            successful_requests: 95,
            failed_requests: 5,
            avg_response_time_ms: 250.5,
            p95_response_time_ms: 800,
            p99_response_time_ms: 1200,
            total_tokens: 5000,
            requests_per_second: 1.67,
            error_rate: 5.0,
            provider_metrics: HashMap::new(),
            routing_metrics: RoutingStats {
                total_routing_decisions: 100,
                fallback_used: 5,
                avg_decision_time_ms: 15.2,
                scenario_distribution: HashMap::new(),
                provider_distribution: HashMap::new(),
            },
            session_metrics: SessionStats {
                active_sessions: 25,
                max_sessions: 100,
                cache_hit_rate: 85.5,
                avg_session_duration_minutes: 12.3,
                total_sessions_created: 100,
                total_sessions_expired: 75,
            },
            system_health: SystemHealth {
                status: HealthStatus::Healthy,
                uptime_seconds: 3600,
                memory_usage_mb: 256.7,
                cpu_usage_percent: 15.3,
                disk_usage_percent: 45.2,
                last_health_check: 1234567890,
                health_issues: vec![],
            },
        };

        let prometheus = MetricsExporter::export_prometheus(&metrics);
        assert!(prometheus.contains("terraphim_requests_total 100"));
        assert!(prometheus.contains("terraphim_response_time_ms 250.50"));
        assert!(prometheus.contains("terraphim_error_rate 5.00"));
        assert!(prometheus.contains("terraphim_active_sessions 25"));
    }
}
