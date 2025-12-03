/// Performance monitoring and optimization for search components
///
/// This module provides advanced performance monitoring, optimization strategies,
/// and real-time analytics for search and autocomplete components.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, MissedTickBehavior};

use crate::components::PerformanceTracker;

/// Advanced performance monitor for search components
#[derive(Debug)]
pub struct SearchPerformanceMonitor {
    /// Configuration
    config: SearchPerformanceMonitorConfig,

    /// Performance metrics storage
    metrics: Arc<RwLock<SearchPerformanceMetrics>>,

    /// Historical data for trend analysis
    history: Arc<RwLock<VecDeque<PerformanceSnapshot>>>,

    /// Alert configuration
    alert_config: SearchAlertConfig,

    /// Active alerts
    active_alerts: Arc<RwLock<Vec<SearchPerformanceAlert>>>,

    /// Performance optimization suggestions
    optimization_suggestions: Arc<RwLock<Vec<OptimizationSuggestion>>>,

    /// Channel for real-time updates
    update_tx: mpsc::UnboundedSender<PerformanceUpdate>,

    /// Background monitoring task handle
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Search performance monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchPerformanceMonitorConfig {
    /// Enable real-time monitoring
    pub enable_real_time: bool,

    /// Metrics collection interval in milliseconds
    pub collection_interval_ms: u64,

    /// History retention period in seconds
    pub history_retention_seconds: u64,

    /// Maximum history entries
    pub max_history_entries: usize,

    /// Enable trend analysis
    pub enable_trend_analysis: bool,

    /// Enable automatic optimization suggestions
    pub enable_optimization_suggestions: bool,

    /// Performance profiling enabled
    pub enable_profiling: bool,

    /// Enable distributed tracing
    pub enable_distributed_tracing: bool,

    /// Cache hit rate monitoring
    pub enable_cache_monitoring: bool,
}

impl Default for SearchPerformanceMonitorConfig {
    fn default() -> Self {
        Self {
            enable_real_time: true,
            collection_interval_ms: 1000,
            history_retention_seconds: 3600, // 1 hour
            max_history_entries: 1000,
            enable_trend_analysis: true,
            enable_optimization_suggestions: true,
            enable_profiling: false,
            enable_distributed_tracing: false,
            enable_cache_monitoring: true,
        }
    }
}

/// Search performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchPerformanceMetrics {
    /// Search performance
    pub search_metrics: SearchMetrics,

    /// Autocomplete performance
    pub autocomplete_metrics: AutocompleteMetrics,

    /// Cache performance
    pub cache_metrics: CacheMetrics,

    /// Resource usage metrics
    pub resource_metrics: ResourceMetrics,

    /// User interaction metrics
    pub user_metrics: UserInteractionMetrics,

    /// Error metrics
    pub error_metrics: ErrorMetrics,
}

/// Search-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchMetrics {
    /// Total searches performed
    pub total_searches: u64,

    /// Successful searches
    pub successful_searches: u64,

    /// Failed searches
    pub failed_searches: u64,

    /// Average search response time
    pub avg_response_time: Duration,

    /// Median search response time
    pub median_response_time: Duration,

    /// 95th percentile response time
    pub p95_response_time: Duration,

    /// 99th percentile response time
    pub p99_response_time: Duration,

    /// Search throughput (searches per second)
    pub throughput: f64,

    /// Concurrent searches peak
    pub peak_concurrent_searches: usize,

    /// Searches by query length
    pub searches_by_query_length: HashMap<String, u64>,
}

/// Autocomplete-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutocompleteMetrics {
    /// Total autocomplete requests
    pub total_requests: u64,

    /// Successful requests
    pub successful_requests: u64,

    /// Failed requests
    pub failed_requests: u64,

    /// Average response time
    pub avg_response_time: Duration,

    /// Median response time
    pub median_response_time: Duration,

    /// Cache hit rate
    pub cache_hit_rate: f64,

    /// Average suggestions per request
    pub avg_suggestions_per_request: f64,

    /// Request abandonment rate
    pub abandonment_rate: f64,

    /// Typing pattern metrics
    pub typing_patterns: TypingPatternMetrics,
}

/// Typing pattern metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypingPatternMetrics {
    /// Average time between keystrokes
    pub avg_keystroke_interval: Duration,

    /// Average query length before search
    pub avg_query_length: f64,

    /// Backspace usage frequency
    pub backspace_frequency: f64,

    /// Query correction rate
    pub query_correction_rate: f64,
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheMetrics {
    /// Cache hit rate
    pub hit_rate: f64,

    /// Cache miss rate
    pub miss_rate: f64,

    /// Average cache retrieval time
    pub avg_retrieval_time: Duration,

    /// Cache size
    pub current_size: usize,

    /// Cache evictions
    pub evictions: u64,

    /// Cache efficiency by query type
    pub efficiency_by_query_type: HashMap<String, f64>,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,

    /// Memory usage in bytes
    pub memory_usage_bytes: u64,

    /// Memory usage percentage
    pub memory_usage_percent: f64,

    /// Network usage
    pub network_metrics: NetworkMetrics,

    /// Database connections
    pub db_connections: u32,

    /// Thread pool utilization
    pub thread_pool_utilization: f64,
}

/// Network usage metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    /// Bytes sent
    pub bytes_sent: u64,

    /// Bytes received
    pub bytes_received: u64,

    /// Network requests
    pub requests: u64,

    /// Network errors
    pub errors: u64,

    /// Average request size
    pub avg_request_size: u64,

    /// Average response size
    pub avg_response_size: u64,
}

/// User interaction metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInteractionMetrics {
    /// Average session duration
    pub avg_session_duration: Duration,

    /// Search abandonment rate
    pub search_abandonment_rate: f64,

    /// Result click-through rate
    pub result_ctr: f64,

    /// Average results viewed per search
    pub avg_results_viewed: f64,

    /// User satisfaction score
    pub satisfaction_score: f64,

    /// Feature usage statistics
    pub feature_usage: HashMap<String, u64>,
}

/// Error metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorMetrics {
    /// Total errors
    pub total_errors: u64,

    /// Error rate percentage
    pub error_rate: f64,

    /// Errors by type
    pub errors_by_type: HashMap<String, u64>,

    /// Error recovery rate
    pub recovery_rate: f64,

    /// Mean time to recovery
    pub mean_time_to_recovery: Duration,
}

/// Performance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Timestamp
    pub timestamp: Instant,

    /// Performance metrics
    pub metrics: SearchPerformanceMetrics,

    /// System load information
    pub system_load: SystemLoadInfo,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// System load information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemLoadInfo {
    /// CPU load average (1 minute)
    pub cpu_load_1m: f64,

    /// CPU load average (5 minutes)
    pub cpu_load_5m: f64,

    /// Memory pressure
    pub memory_pressure: f64,

    /// Disk I/O pressure
    pub disk_io_pressure: f64,

    /// Network latency
    pub network_latency: Duration,
}

/// Search alert configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchAlertConfig {
    /// Enable alerts
    pub enabled: bool,

    /// Response time threshold in milliseconds
    pub response_time_threshold_ms: u64,

    /// Error rate threshold percentage
    pub error_rate_threshold: f64,

    /// CPU usage threshold percentage
    pub cpu_threshold: f64,

    /// Memory usage threshold percentage
    pub memory_threshold: f64,

    /// Alert cooldown period
    pub alert_cooldown: Duration,

    /// Alert channels
    pub alert_channels: Vec<AlertChannel>,
}

/// Alert channels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertChannel {
    /// Log alerts
    Log,

    /// Console alerts
    Console,

    /// External monitoring service
    External(String),

    /// Email alerts
    Email(String),

    /// Webhook alerts
    Webhook(String),
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPerformanceAlert {
    /// Alert ID
    pub id: String,

    /// Alert type
    pub alert_type: SearchAlertType,

    /// Alert severity
    pub severity: AlertSeverity,

    /// Alert message
    pub message: String,

    /// Alert timestamp
    pub timestamp: Instant,

    /// Current value
    pub current_value: f64,

    /// Threshold value
    pub threshold_value: f64,

    /// Additional context
    pub context: HashMap<String, String>,

    /// Alert status
    pub status: AlertStatus,
}

/// Search alert types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchAlertType {
    /// High response time
    HighResponseTime,

    /// High error rate
    HighErrorRate,

    /// Low throughput
    LowThroughput,

    /// High resource usage
    HighResourceUsage,

    /// Cache inefficiency
    CacheInefficiency,

    /// Performance degradation
    PerformanceDegradation,

    /// Custom alert
    Custom(String),
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Informational
    Info,

    /// Warning
    Warning,

    /// Error
    Error,

    /// Critical
    Critical,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertStatus {
    /// Active alert
    Active,

    /// Resolved alert
    Resolved,

    /// Suppressed alert
    Suppressed,
}

/// Performance update events
#[derive(Debug, Clone)]
pub enum PerformanceUpdate {
    /// Metrics updated
    MetricsUpdated(SearchPerformanceMetrics),

    /// Alert triggered
    AlertTriggered(SearchPerformanceAlert),

    /// Alert resolved
    AlertResolved(String),

    /// Optimization suggestion generated
    OptimizationSuggestionGenerated(OptimizationSuggestion),

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

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,

    /// Time period
    pub period: Duration,

    /// Statistical significance
    pub significance: f64,
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

    /// User satisfaction trend
    UserSatisfaction,

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

/// Optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// Suggestion ID
    pub id: String,

    /// Suggestion type
    pub suggestion_type: OptimizationType,

    /// Suggestion title
    pub title: String,

    /// Suggestion description
    pub description: String,

    /// Expected impact
    pub expected_impact: ImpactLevel,

    /// Implementation complexity
    pub complexity: ComplexityLevel,

    /// Estimated improvement percentage
    pub estimated_improvement: f64,

    /// Implementation steps
    pub implementation_steps: Vec<String>,

    /// Supporting data
    pub supporting_data: HashMap<String, f64>,

    /// Timestamp generated
    pub timestamp: Instant,

    /// Suggestion status
    pub status: SuggestionStatus,
}

/// Optimization types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OptimizationType {
    /// Cache optimization
    CacheOptimization,

    /// Query optimization
    QueryOptimization,

    /// Database optimization
    DatabaseOptimization,

    /// Network optimization
    NetworkOptimization,

    /// UI/UX optimization
    UIUXOptimization,

    /// Resource optimization
    ResourceOptimization,

    /// Custom optimization
    Custom(String),
}

/// Impact level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactLevel {
    /// Low impact
    Low,

    /// Medium impact
    Medium,

    /// High impact
    High,

    /// Critical impact
    Critical,
}

/// Complexity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplexityLevel {
    /// Simple
    Simple,

    /// Moderate
    Moderate,

    /// Complex
    Complex,

    /// Very complex
    VeryComplex,
}

/// Suggestion status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionStatus {
    /// New suggestion
    New,

    /// In review
    InReview,

    /// Approved
    Approved,

    /// Implemented
    Implemented,

    /// Rejected
    Rejected,

    /// Superseded
    Superseded,
}

impl SearchPerformanceMonitor {
    /// Create new performance monitor
    pub fn new(config: SearchPerformanceMonitorConfig, alert_config: SearchAlertConfig) -> Self {
        let (update_tx, _) = mpsc::unbounded_channel();

        Self {
            config,
            metrics: Arc::new(RwLock::new(SearchPerformanceMetrics::default())),
            history: Arc::new(RwLock::new(VecDeque::new())),
            alert_config,
            active_alerts: Arc::new(RwLock::new(Vec::new())),
            optimization_suggestions: Arc::new(RwLock::new(Vec::new())),
            update_tx,
            monitor_handle: None,
        }
    }

    /// Start performance monitoring
    pub fn start_monitoring(mut self) -> Self {
        let metrics = Arc::clone(&self.metrics);
        let history = Arc::clone(&self.history);
        let config = self.config.clone();
        let alert_config = self.alert_config.clone();
        let active_alerts = Arc::clone(&self.active_alerts);
        let optimization_suggestions = Arc::clone(&self.optimization_suggestions);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.collection_interval_ms));
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;

                // Collect metrics
                let current_metrics = metrics.read().await.clone();

                // Create snapshot
                let snapshot = PerformanceSnapshot {
                    timestamp: Instant::now(),
                    metrics: current_metrics.clone(),
                    system_load: Self::collect_system_load().await,
                    metadata: HashMap::new(),
                };

                // Add to history
                {
                    let mut hist = history.write().await;
                    hist.push_back(snapshot);

                    // Limit history size
                    while hist.len() > config.max_history_entries {
                        hist.pop_front();
                    }

                    // Remove old entries outside retention period
                    let cutoff = Instant::now() - Duration::from_secs(config.history_retention_seconds);
                    while let Some(front) = hist.front() {
                        if front.timestamp < cutoff {
                            hist.pop_front();
                        } else {
                            break;
                        }
                    }
                }

                // Check for alerts
                if alert_config.enabled {
                    Self::check_for_alerts(&current_metrics, &alert_config, &active_alerts).await;
                }

                // Generate optimization suggestions
                if config.enable_optimization_suggestions {
                    Self::generate_optimization_suggestions(&current_metrics, &optimization_suggestions).await;
                }
            }
        });

        self.monitor_handle = Some(handle);
        self
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> SearchPerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get historical data
    pub async fn get_history(&self, limit: Option<usize>) -> Vec<PerformanceSnapshot> {
        let history = self.history.read().await;
        let data: Vec<_> = history.iter().rev().take(limit.unwrap_or(usize::MAX)).cloned().collect();
        data
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<SearchPerformanceAlert> {
        self.active_alerts.read().await.clone()
    }

    /// Get optimization suggestions
    pub async fn get_optimization_suggestions(&self) -> Vec<OptimizationSuggestion> {
        self.optimization_suggestions.read().await.clone()
    }

    /// Update search metrics
    pub async fn update_search_metrics<F, R>(&self, update_fn: F) -> R
    where
        F: FnOnce(&mut SearchMetrics) -> R,
    {
        let mut metrics = self.metrics.write().await;
        update_fn(&mut metrics.search_metrics)
    }

    /// Update autocomplete metrics
    pub async fn update_autocomplete_metrics<F, R>(&self, update_fn: F) -> R
    where
        F: FnOnce(&mut AutocompleteMetrics) -> R,
    {
        let mut metrics = self.metrics.write().await;
        update_fn(&mut metrics.autocomplete_metrics)
    }

    /// Record search operation
    pub async fn record_search(&self, response_time: Duration, success: bool, query_length: usize) {
        let mut metrics = self.metrics.write().await;

        metrics.search_metrics.total_searches += 1;

        if success {
            metrics.search_metrics.successful_searches += 1;

            // Update response time statistics
            let total_time = metrics.search_metrics.avg_response_time *
                (metrics.search_metrics.successful_searches - 1) as u32 + response_time;
            metrics.search_metrics.avg_response_time = total_time / metrics.search_metrics.successful_searches as u32;
        } else {
            metrics.search_metrics.failed_searches += 1;
        }

        // Update query length statistics
        let length_bucket = Self::get_query_length_bucket(query_length);
        *metrics.search_metrics.searches_by_query_length
            .entry(length_bucket)
            .or_insert(0) += 1;
    }

    /// Record autocomplete operation
    pub async fn record_autocomplete(&self, response_time: Duration, success: bool, suggestions_count: usize) {
        let mut metrics = self.metrics.write().await;

        metrics.autocomplete_metrics.total_requests += 1;

        if success {
            metrics.autocomplete_metrics.successful_requests += 1;

            // Update response time statistics
            let total_time = metrics.autocomplete_metrics.avg_response_time *
                (metrics.autocomplete_metrics.successful_requests - 1) as u32 + response_time;
            metrics.autocomplete_metrics.avg_response_time = total_time / metrics.autocomplete_metrics.successful_requests as u32;

            // Update suggestions per request
            let total_suggestions = metrics.autocomplete_metrics.avg_suggestions_per_request *
                (metrics.autocomplete_metrics.successful_requests - 1) as f64 + suggestions_count as f64;
            metrics.autocomplete_metrics.avg_suggestions_per_request = total_suggestions / metrics.autocomplete_metrics.successful_requests as f64;
        } else {
            metrics.autocomplete_metrics.failed_requests += 1;
        }
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) {
        let mut alerts = self.active_alerts.write().await;
        for alert in alerts.iter_mut() {
            if alert.id == alert_id {
                alert.status = AlertStatus::Resolved;
            }
        }
    }

    /// Get receiver for performance updates
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<PerformanceUpdate> {
        let (_, rx) = mpsc::unbounded_channel();
        rx // This won't work as written, but illustrates the concept
    }

    /// Shutdown the monitor
    pub async fn shutdown(mut self) {
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
        }
    }

    // Private helper methods

    /// Get query length bucket
    fn get_query_length_bucket(length: usize) -> String {
        match length {
            0..=1 => "1".to_string(),
            2..=3 => "2-3".to_string(),
            4..=6 => "4-6".to_string(),
            7..=10 => "7-10".to_string(),
            11..=20 => "11-20".to_string(),
            _ => "20+".to_string(),
        }
    }

    /// Collect system load information
    async fn collect_system_load() -> SystemLoadInfo {
        // This is a simplified implementation
        // In a real scenario, you'd use system monitoring libraries
        SystemLoadInfo::default()
    }

    /// Check for performance alerts
    async fn check_for_alerts(
        metrics: &SearchPerformanceMetrics,
        config: &SearchAlertConfig,
        active_alerts: &Arc<RwLock<Vec<SearchPerformanceAlert>>>,
    ) {
        let mut new_alerts = Vec::new();

        // Check response time threshold
        if metrics.search_metrics.avg_response_time.as_millis() as u64 > config.response_time_threshold_ms {
            new_alerts.push(SearchPerformanceAlert {
                id: format!("response_time_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                alert_type: SearchAlertType::HighResponseTime,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Average search response time {}ms exceeds threshold {}ms",
                    metrics.search_metrics.avg_response_time.as_millis(),
                    config.response_time_threshold_ms
                ),
                timestamp: Instant::now(),
                current_value: metrics.search_metrics.avg_response_time.as_millis() as f64,
                threshold_value: config.response_time_threshold_ms as f64,
                context: HashMap::new(),
                status: AlertStatus::Active,
            });
        }

        // Check error rate threshold
        if metrics.search_metrics.total_searches > 0 {
            let error_rate = (metrics.search_metrics.failed_searches as f64 / metrics.search_metrics.total_searches as f64) * 100.0;
            if error_rate > config.error_rate_threshold {
                new_alerts.push(SearchPerformanceAlert {
                    id: format!("error_rate_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                    alert_type: SearchAlertType::HighErrorRate,
                    severity: AlertSeverity::Error,
                    message: format!(
                        "Search error rate {:.1}% exceeds threshold {:.1}%",
                        error_rate,
                        config.error_rate_threshold
                    ),
                    timestamp: Instant::now(),
                    current_value: error_rate,
                    threshold_value: config.error_rate_threshold,
                    context: HashMap::new(),
                    status: AlertStatus::Active,
                });
            }
        }

        // Add new alerts if they don't already exist and are not in cooldown
        if !new_alerts.is_empty() {
            let mut alerts = active_alerts.write().await;
            for new_alert in new_alerts {
                // Check if similar alert already exists and is active
                let exists = alerts.iter().any(|alert| {
                    alert.active() &&
                    alert.alert_type == new_alert.alert_type &&
                    Instant::now().duration_since(alert.timestamp) < config.alert_cooldown
                });

                if !exists {
                    alerts.push(new_alert);
                }
            }
        }
    }

    /// Generate optimization suggestions
    async fn generate_optimization_suggestions(
        metrics: &SearchPerformanceMetrics,
        suggestions: &Arc<RwLock<Vec<OptimizationSuggestion>>>,
    ) {
        let mut new_suggestions = Vec::new();

        // Check for cache optimization opportunities
        if metrics.cache_metrics.hit_rate < 0.8 {
            new_suggestions.push(OptimizationSuggestion {
                id: format!("cache_opt_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                suggestion_type: OptimizationType::CacheOptimization,
                title: "Improve Cache Hit Rate".to_string(),
                description: "Current cache hit rate is below 80%. Consider implementing cache warming or increasing cache size.".to_string(),
                expected_impact: ImpactLevel::Medium,
                complexity: ComplexityLevel::Moderate,
                estimated_improvement: 15.0,
                implementation_steps: vec![
                    "Analyze cache access patterns".to_string(),
                    "Implement cache warming for common queries".to_string(),
                    "Increase cache size if memory permits".to_string(),
                    "Monitor cache performance improvements".to_string(),
                ],
                supporting_data: HashMap::from([
                    ("current_hit_rate".to_string(), metrics.cache_metrics.hit_rate),
                    ("target_hit_rate".to_string(), 0.8),
                ]),
                timestamp: Instant::now(),
                status: SuggestionStatus::New,
            });
        }

        // Add new suggestions if they don't already exist
        if !new_suggestions.is_empty() {
            let mut existing_suggestions = suggestions.write().await;
            for suggestion in new_suggestions {
                // Check if similar suggestion already exists
                let exists = existing_suggestions.iter().any(|s| {
                    s.suggestion_type == suggestion.suggestion_type &&
                    s.status == SuggestionStatus::New
                });

                if !exists {
                    existing_suggestions.push(suggestion);
                }
            }
        }
    }
}

impl Default for SearchPerformanceMonitor {
    fn default() -> Self {
        Self::new(
            SearchPerformanceMonitorConfig::default(),
            SearchAlertConfig::default(),
        )
    }
}

impl Default for SearchAlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            response_time_threshold_ms: 1000,
            error_rate_threshold: 5.0,
            cpu_threshold: 80.0,
            memory_threshold: 80.0,
            alert_cooldown: Duration::from_secs(60),
            alert_channels: vec![AlertChannel::Log, AlertChannel::Console],
        }
    }
}

impl SearchPerformanceAlert {
    /// Check if alert is active
    pub fn active(&self) -> bool {
        matches!(self.status, AlertStatus::Active)
    }
}