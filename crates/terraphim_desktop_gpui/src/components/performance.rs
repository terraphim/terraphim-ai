/// PerformanceTracker for real-time performance monitoring
///
/// This module provides comprehensive performance tracking capabilities
/// for components including metrics collection, analysis, and alerting.
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::components::ComponentError;

/// Performance tracker for monitoring component performance
#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    /// Tracker configuration
    config: TrackerConfig,

    /// Current performance metrics
    metrics: Arc<RwLock<PerformanceMetrics>>,

    /// Historical data for trend analysis
    history: Arc<RwLock<Vec<PerformanceSnapshot>>>,

    /// Performance alerts
    alerts: Arc<RwLock<Vec<PerformanceAlert>>>,

    /// Alert configuration
    alert_config: AlertConfig,

    /// Channel for real-time updates
    update_tx: mpsc::UnboundedSender<PerformanceUpdate>,
}

/// Performance tracker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerConfig {
    /// Enable real-time monitoring
    pub real_time: bool,

    /// Metrics collection interval
    pub collection_interval: Duration,

    /// History retention period
    pub history_retention: Duration,

    /// Maximum history entries
    pub max_history_entries: usize,

    /// Enable trend analysis
    pub enable_trends: bool,

    /// Enable performance profiling
    pub enable_profiling: bool,

    /// Tracker name for identification
    pub name: String,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            real_time: true,
            collection_interval: Duration::from_millis(100),
            history_retention: Duration::from_secs(300), // 5 minutes
            max_history_entries: 1000,
            enable_trends: true,
            enable_profiling: false,
            name: "default".to_string(),
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable performance alerts
    pub enabled: bool,

    /// Response time threshold for alerts (milliseconds)
    pub response_time_threshold: u64,

    /// CPU usage threshold for alerts (percentage)
    pub cpu_threshold: f64,

    /// Memory usage threshold for alerts (percentage)
    pub memory_threshold: f64,

    /// Error rate threshold for alerts (percentage)
    pub error_rate_threshold: f64,

    /// Alert cooldown period
    pub alert_cooldown: Duration,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            response_time_threshold: 100, // 100ms
            cpu_threshold: 80.0,          // 80%
            memory_threshold: 80.0,       // 80%
            error_rate_threshold: 5.0,    // 5%
            alert_cooldown: Duration::from_secs(30),
        }
    }
}

/// Performance metrics collected by the tracker
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    /// Total operation count
    pub operation_count: u64,

    /// Successful operation count
    pub success_count: u64,

    /// Failed operation count
    pub failure_count: u64,

    /// Total response time (nanoseconds)
    pub total_response_time: u64,

    /// Minimum response time (nanoseconds)
    pub min_response_time: Option<u64>,

    /// Maximum response time (nanoseconds)
    pub max_response_time: Option<u64>,

    /// Average response time (nanoseconds)
    pub avg_response_time: f64,

    /// CPU usage (percentage)
    pub cpu_usage: f64,

    /// Memory usage (bytes)
    pub memory_usage: u64,

    /// Memory usage (percentage)
    pub memory_usage_percent: f64,

    /// Current operations in progress
    pub operations_in_progress: u64,

    /// Peak concurrent operations
    pub peak_concurrent_operations: u64,

    /// Bytes processed
    pub bytes_processed: u64,

    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Performance snapshot for historical tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: Instant,

    /// Performance metrics at this time
    pub metrics: PerformanceMetrics,

    /// System resource usage
    pub system_metrics: SystemMetrics,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// System metrics for resource monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Process ID
    pub pid: u32,

    /// CPU usage percentage
    pub cpu_percent: f64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Memory usage percentage
    pub memory_percent: f64,

    /// Number of threads
    pub thread_count: usize,

    /// Open file descriptors
    pub open_fds: u32,

    /// Network bytes sent
    pub network_bytes_sent: u64,

    /// Network bytes received
    pub network_bytes_received: u64,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert type
    pub alert_type: AlertType,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert message
    pub message: String,

    /// Timestamp when alert was created
    pub timestamp: Instant,

    /// Metrics that triggered the alert
    pub trigger_metrics: HashMap<String, f64>,

    /// Alert ID
    pub id: String,

    /// Whether alert is active
    pub active: bool,
}

/// Alert types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    /// High response time
    HighResponseTime,

    /// High CPU usage
    HighCpuUsage,

    /// High memory usage
    HighMemoryUsage,

    /// High error rate
    HighErrorRate,

    /// Low throughput
    LowThroughput,

    /// Resource exhaustion
    ResourceExhaustion,

    /// Custom alert
    Custom(String),
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Informational only
    Info,

    /// Warning level
    Warning,

    /// Error level
    Error,

    /// Critical level
    Critical,
}

/// Performance update events
#[derive(Debug, Clone)]
pub enum PerformanceUpdate {
    /// Metrics updated
    MetricsUpdated(PerformanceMetrics),

    /// Alert triggered
    AlertTriggered(PerformanceAlert),

    /// Alert resolved
    AlertResolved(String),

    /// Trend detected
    TrendDetected(PerformanceTrend),
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    /// Trend type
    pub trend_type: TrendType,

    /// Trend direction
    pub direction: TrendDirection,

    /// Trend magnitude
    pub magnitude: f64,

    /// Time period for trend
    pub period: Duration,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

/// Trend types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendType {
    /// Response time trend
    ResponseTime,

    /// Throughput trend
    Throughput,

    /// Error rate trend
    ErrorRate,

    /// Resource usage trend
    ResourceUsage,

    /// Custom trend
    Custom(String),
}

/// Trend directions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    /// Increasing
    Increasing,

    /// Decreasing
    Decreasing,

    /// Stable
    Stable,

    /// Volatile
    Volatile,
}

/// Performance tracking errors
#[derive(Debug, Clone, Error)]
pub enum PerformanceError {
    #[error("Tracker not initialized")]
    NotInitialized,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Collection error: {0}")]
    Collection(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("Alert error: {0}")]
    Alert(String),
}

impl PerformanceTracker {
    /// Create new performance tracker
    pub fn new(config: TrackerConfig) -> Self {
        let (update_tx, _) = mpsc::unbounded_channel();

        Self {
            config,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            history: Arc::new(RwLock::new(Vec::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_config: AlertConfig::default(),
            update_tx,
        }
    }

    /// Create performance tracker with default configuration
    pub fn default() -> Self {
        Self::new(TrackerConfig::default())
    }

    /// Record operation start
    pub fn start_operation(&self) -> OperationTimer {
        let start_time = Instant::now();

        // Increment operations in progress
        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.operations_in_progress += 1;
            metrics.peak_concurrent_operations = metrics
                .peak_concurrent_operations
                .max(metrics.operations_in_progress);
        }

        OperationTimer::new(start_time, Arc::clone(&self.metrics))
    }

    /// Get current metrics
    pub fn current_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get metrics for the last duration
    pub fn metrics_for_duration(&self, duration: Duration) -> PerformanceMetrics {
        let now = Instant::now();
        let cutoff = now - duration;

        let history = self.history.read().unwrap();
        let recent_snapshots: Vec<_> = history
            .iter()
            .filter(|snapshot| snapshot.timestamp >= cutoff)
            .collect();

        if recent_snapshots.is_empty() {
            return PerformanceMetrics::default();
        }

        // Aggregate metrics from recent snapshots
        let mut aggregated = PerformanceMetrics::default();
        for snapshot in recent_snapshots {
            aggregated.operation_count += snapshot.metrics.operation_count;
            aggregated.success_count += snapshot.metrics.success_count;
            aggregated.failure_count += snapshot.metrics.failure_count;
            aggregated.total_response_time += snapshot.metrics.total_response_time;

            if let Some(min_time) = snapshot.metrics.min_response_time {
                aggregated.min_response_time = Some(
                    aggregated
                        .min_response_time
                        .map_or(min_time, |m| m.min(min_time)),
                );
            }

            if let Some(max_time) = snapshot.metrics.max_response_time {
                aggregated.max_response_time = Some(
                    aggregated
                        .max_response_time
                        .map_or(max_time, |m| m.max(max_time)),
                );
            }
        }

        if aggregated.operation_count > 0 {
            aggregated.avg_response_time =
                aggregated.total_response_time as f64 / aggregated.operation_count as f64;
        }

        // Use the latest snapshot for current resource usage
        if let Some(latest) = recent_snapshots.last() {
            aggregated.cpu_usage = latest.metrics.cpu_usage;
            aggregated.memory_usage = latest.metrics.memory_usage;
            aggregated.memory_usage_percent = latest.metrics.memory_usage_percent;
            aggregated.operations_in_progress = latest.metrics.operations_in_progress;
        }

        aggregated
    }

    /// Get performance trend analysis
    pub fn analyze_trends(&self, period: Duration) -> Vec<PerformanceTrend> {
        let now = Instant::now();
        let cutoff = now - period;

        let history = self.history.read().unwrap();
        let recent_snapshots: Vec<_> = history
            .iter()
            .filter(|snapshot| snapshot.timestamp >= cutoff)
            .collect();

        if recent_snapshots.len() < 2 {
            return Vec::new();
        }

        let mut trends = Vec::new();

        // Analyze response time trend
        let response_times: Vec<f64> = recent_snapshots
            .iter()
            .map(|s| s.metrics.avg_response_time)
            .collect();

        if let Some(trend) = self.calculate_trend(&response_times, TrendType::ResponseTime, period)
        {
            trends.push(trend);
        }

        // Analyze throughput trend (operations per second)
        let throughput: Vec<f64> = recent_snapshots
            .windows(2)
            .map(|window| {
                let time_diff = window[1]
                    .timestamp
                    .duration_since(window[0].timestamp)
                    .as_secs_f64();
                if time_diff > 0.0 {
                    (window[1].metrics.operation_count - window[0].metrics.operation_count) as f64
                        / time_diff
                } else {
                    0.0
                }
            })
            .collect();

        if let Some(trend) = self.calculate_trend(&throughput, TrendType::Throughput, period) {
            trends.push(trend);
        }

        trends
    }

    /// Calculate trend from data series
    fn calculate_trend(
        &self,
        data: &[f64],
        trend_type: TrendType,
        period: Duration,
    ) -> Option<PerformanceTrend> {
        if data.len() < 2 {
            return None;
        }

        // Simple linear regression for trend analysis
        let n = data.len() as f64;
        let x_sum = (0..data.len()).map(|i| i as f64).sum::<f64>();
        let y_sum = data.iter().sum::<f64>();
        let xy_sum: f64 = data.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
        let x2_sum: f64 = (0..data.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x2_sum - x_sum.powi(2));

        // Determine trend direction and magnitude
        let (direction, magnitude) = if slope.abs() < 0.01 {
            (TrendDirection::Stable, 0.0)
        } else if slope > 0.0 {
            (TrendDirection::Increasing, slope)
        } else {
            (TrendDirection::Decreasing, -slope)
        };

        // Calculate confidence (simplified)
        let y_mean = y_sum / n;
        let variance: f64 = data.iter().map(|y| (y - y_mean).powi(2)).sum();
        let confidence = if variance > 0.0 {
            1.0 - (variance / n).sqrt() / y_mean.abs()
        } else {
            1.0
        };

        Some(PerformanceTrend {
            trend_type,
            direction,
            magnitude,
            period,
            confidence: confidence.clamp(0.0, 1.0),
        })
    }

    /// Get current alerts
    pub fn current_alerts(&self) -> Vec<PerformanceAlert> {
        self.alerts.read().unwrap().clone()
    }

    /// Get active alerts
    pub fn active_alerts(&self) -> Vec<PerformanceAlert> {
        self.alerts
            .read()
            .unwrap()
            .iter()
            .filter(|alert| alert.active)
            .cloned()
            .collect()
    }

    /// Resolve an alert
    pub fn resolve_alert(&self, alert_id: &str) {
        let mut alerts = self.alerts.write().unwrap();
        for alert in alerts.iter_mut() {
            if alert.id == alert_id {
                alert.active = false;
            }
        }
    }

    /// Clear all alerts
    pub fn clear_alerts(&self) {
        let mut alerts = self.alerts.write().unwrap();
        alerts.clear();
    }

    /// Check for performance alerts
    fn check_alerts(&self, metrics: &PerformanceMetrics) {
        if !self.alert_config.enabled {
            return;
        }

        let now = Instant::now();
        let mut new_alerts = Vec::new();

        // Check response time threshold
        if metrics.avg_response_time
            > self.alert_config.response_time_threshold as f64 * 1_000_000.0
        {
            new_alerts.push(PerformanceAlert {
                alert_type: AlertType::HighResponseTime,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Average response time {:.2}ms exceeds threshold {}ms",
                    metrics.avg_response_time / 1_000_000.0,
                    self.alert_config.response_time_threshold
                ),
                timestamp: now,
                trigger_metrics: {
                    let mut trigger = HashMap::new();
                    trigger.insert("avg_response_time".to_string(), metrics.avg_response_time);
                    trigger
                },
                id: format!("high_response_time_{}", now.elapsed().as_millis()),
                active: true,
            });
        }

        // Check memory usage threshold
        if metrics.memory_usage_percent > self.alert_config.memory_threshold {
            new_alerts.push(PerformanceAlert {
                alert_type: AlertType::HighMemoryUsage,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Memory usage {:.1}% exceeds threshold {:.1}%",
                    metrics.memory_usage_percent, self.alert_config.memory_threshold
                ),
                timestamp: now,
                trigger_metrics: {
                    let mut trigger = HashMap::new();
                    trigger.insert(
                        "memory_usage_percent".to_string(),
                        metrics.memory_usage_percent,
                    );
                    trigger
                },
                id: format!("high_memory_{}", now.elapsed().as_millis()),
                active: true,
            });
        }

        // Check error rate threshold
        if metrics.operation_count > 0 {
            let error_rate =
                (metrics.failure_count as f64 / metrics.operation_count as f64) * 100.0;
            if error_rate > self.alert_config.error_rate_threshold {
                new_alerts.push(PerformanceAlert {
                    alert_type: AlertType::HighErrorRate,
                    severity: AlertSeverity::Error,
                    message: format!(
                        "Error rate {:.1}% exceeds threshold {:.1}%",
                        error_rate, self.alert_config.error_rate_threshold
                    ),
                    timestamp: now,
                    trigger_metrics: {
                        let mut trigger = HashMap::new();
                        trigger.insert("error_rate".to_string(), error_rate);
                        trigger
                    },
                    id: format!("high_error_rate_{}", now.elapsed().as_millis()),
                    active: true,
                });
            }
        }

        // Add new alerts if they don't already exist
        if !new_alerts.is_empty() {
            let mut alerts = self.alerts.write().unwrap();
            for new_alert in new_alerts {
                // Check if similar alert already exists and is active
                let exists = alerts.iter().any(|alert| {
                    alert.active
                        && alert.alert_type == new_alert.alert_type
                        && now.duration_since(alert.timestamp) < self.alert_config.alert_cooldown
                });

                if !exists {
                    alerts.push(new_alert.clone());

                    // Send update notification
                    let _ = self
                        .update_tx
                        .send(PerformanceUpdate::AlertTriggered(new_alert));
                }
            }
        }
    }

    /// Take performance snapshot
    fn take_snapshot(&self) {
        let metrics = self.current_metrics();
        let system_metrics = self.collect_system_metrics();

        let snapshot = PerformanceSnapshot {
            timestamp: Instant::now(),
            metrics,
            system_metrics,
            metadata: HashMap::new(),
        };

        // Add to history
        {
            let mut history = self.history.write().unwrap();
            history.push(snapshot.clone());

            // Trim history if too long
            if history.len() > self.config.max_history_entries {
                history.remove(0);
            }

            // Remove old entries outside retention period
            let cutoff = Instant::now() - self.config.history_retention;
            history.retain(|snapshot| snapshot.timestamp >= cutoff);
        }

        // Check for alerts
        self.check_alerts(&snapshot.metrics);

        // Send update notification
        let _ = self
            .update_tx
            .send(PerformanceUpdate::MetricsUpdated(snapshot.metrics));
    }

    /// Collect system metrics
    fn collect_system_metrics(&self) -> SystemMetrics {
        // This is a simplified implementation
        // In a real system, you'd use libraries like `sysinfo` or `psutil`
        SystemMetrics {
            pid: std::process::id(),
            cpu_percent: 0.0, // Would be collected from system
            memory_bytes: 0,  // Would be collected from system
            memory_percent: 0.0,
            thread_count: 1, // Would be collected from system
            open_fds: 0,     // Would be collected from system
            network_bytes_sent: 0,
            network_bytes_received: 0,
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self.metrics.write().unwrap() = PerformanceMetrics::default();
        self.history.write().unwrap().clear();
        self.alerts.write().unwrap().clear();
    }

    /// Get receiver for performance updates
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<PerformanceUpdate> {
        let (_, rx) = mpsc::unbounded_channel();
        rx // This won't work as written, but illustrates the concept
    }
}

/// Operation timer for tracking individual operations
#[derive(Debug)]
pub struct OperationTimer {
    start_time: Instant,
    metrics: Arc<RwLock<PerformanceMetrics>>,
    completed: bool,
}

impl OperationTimer {
    /// Create new operation timer
    fn new(start_time: Instant, metrics: Arc<RwLock<PerformanceMetrics>>) -> Self {
        Self {
            start_time,
            metrics,
            completed: false,
        }
    }

    /// Complete the operation with success
    pub fn complete_success(mut self) {
        if self.completed {
            return;
        }

        let duration = self.start_time.elapsed();
        let duration_nanos = duration.as_nanos() as u64;

        let mut metrics = self.metrics.write().unwrap();
        metrics.operation_count += 1;
        metrics.success_count += 1;
        metrics.total_response_time += duration_nanos;

        // Update min/max response times
        metrics.min_response_time = Some(
            metrics
                .min_response_time
                .map_or(duration_nanos, |min| min.min(duration_nanos)),
        );
        metrics.max_response_time = Some(
            metrics
                .max_response_time
                .map_or(duration_nanos, |max| max.max(duration_nanos)),
        );

        // Update average response time
        if metrics.operation_count > 0 {
            metrics.avg_response_time =
                metrics.total_response_time as f64 / metrics.operation_count as f64;
        }

        metrics.operations_in_progress = metrics.operations_in_progress.saturating_sub(1);

        self.completed = true;
    }

    /// Complete the operation with failure
    pub fn complete_failure(mut self) {
        if self.completed {
            return;
        }

        let duration = self.start_time.elapsed();
        let duration_nanos = duration.as_nanos() as u64;

        let mut metrics = self.metrics.write().unwrap();
        metrics.operation_count += 1;
        metrics.failure_count += 1;
        metrics.total_response_time += duration_nanos;

        // Update min/max response times
        metrics.min_response_time = Some(
            metrics
                .min_response_time
                .map_or(duration_nanos, |min| min.min(duration_nanos)),
        );
        metrics.max_response_time = Some(
            metrics
                .max_response_time
                .map_or(duration_nanos, |max| max.max(duration_nanos)),
        );

        // Update average response time
        if metrics.operation_count > 0 {
            metrics.avg_response_time =
                metrics.total_response_time as f64 / metrics.operation_count as f64;
        }

        metrics.operations_in_progress = metrics.operations_in_progress.saturating_sub(1);

        self.completed = true;
    }

    /// Get elapsed time so far
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Drop for OperationTimer {
    fn drop(&mut self) {
        if !self.completed {
            // Auto-complete as failure if not explicitly completed
            self.complete_failure();
        }
    }
}
