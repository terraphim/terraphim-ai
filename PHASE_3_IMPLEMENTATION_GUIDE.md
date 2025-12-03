# Phase 3 Implementation Guide: Foundation for Reusable Components

## Overview

Phase 3 establishes the core abstractions and infrastructure needed for all subsequent reusable components. This phase focuses on creating the foundation patterns that will ensure consistent behavior, performance, and testability across the entire component ecosystem.

## Week 1: Core Abstractions

### Day 1-2: ReusableComponent Trait

**File**: `crates/terraphim_desktop_gpui/src/components/reusable.rs`

```rust
use gpui::*;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use anyhow::Result;

/// Universal identifier for components
pub type ComponentId = String;

/// All reusable components must implement this trait
pub trait ReusableComponent: Send + Sync {
    type Config: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>;
    type State: Clone + Send + Sync;
    type Event: Send + Sync;

    /// Initialize component with configuration
    fn new(config: Self::Config) -> Self where Self: Sized;

    /// Get component identifier
    fn component_id(&self) -> &ComponentId;

    /// Get current state
    fn state(&self) -> &Self::State;

    /// Handle component events
    fn handle_event(&mut self, event: Self::Event) -> Result<(), ComponentError>;

    /// Render component (GPUI integration)
    fn render(&self, cx: &mut Context<Self>) -> impl IntoElement;

    /// Performance metrics
    fn metrics(&self) -> ComponentMetrics;

    /// Component lifecycle hooks
    fn on_mount(&mut self, cx: &mut Context<Self>) -> Result<(), ComponentError> {
        let _ = cx;
        Ok(())
    }

    fn on_unmount(&mut self, cx: &mut Context<Self>) -> Result<(), ComponentError> {
        let _ = cx;
        Ok(())
    }

    fn on_config_change(&mut self, config: Self::Config, cx: &mut Context<Self>) -> Result<(), ComponentError>;
}

/// Component error types
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("State error: {0}")]
    State(String),
    #[error("Event handling error: {0}")]
    Event(String),
    #[error("Performance error: {0}")]
    Performance(String),
    #[error("Service error: {0}")]
    Service(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Component performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_id: ComponentId,
    pub response_time_p50: Duration,
    pub response_time_p95: Duration,
    pub response_time_p99: Duration,
    pub throughput: f64,          // operations per second
    pub error_rate: f64,          // errors per operation
    pub cache_hit_rate: f64,      // cache hits per operation
    pub memory_usage: usize,      // bytes
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        Self {
            component_id: "unknown".to_string(),
            response_time_p50: Duration::ZERO,
            response_time_p95: Duration::ZERO,
            response_time_p99: Duration::ZERO,
            throughput: 0.0,
            error_rate: 0.0,
            cache_hit_rate: 0.0,
            memory_usage: 0,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Performance tracking for components
pub struct PerformanceTracker {
    metrics: dashmap::DashMap<ComponentId, ComponentMetrics>,
    samples: dashmap::DashMap<ComponentId, Vec<Duration>>,
    max_samples: usize,
}

impl PerformanceTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            metrics: dashmap::DashMap::new(),
            samples: dashmap::DashMap::new(),
            max_samples,
        }
    }

    pub fn record_operation(&self, component_id: &str, duration: Duration) {
        let mut samples = self.samples.entry(component_id.to_string()).or_insert_with(Vec::new);
        samples.push(duration);

        // Keep only the most recent samples
        if samples.len() > self.max_samples {
            samples.remove(0);
        }

        // Update metrics every 10 samples or if we have enough data
        if samples.len() % 10 == 0 || samples.len() >= 100 {
            self.update_metrics(component_id);
        }
    }

    fn update_metrics(&self, component_id: &str) {
        if let Some(samples) = self.samples.get(component_id) {
            if samples.is_empty() {
                return;
            }

            let mut sorted_samples = samples.clone();
            sorted_samples.sort();

            let p50_idx = (sorted_samples.len() as f64 * 0.5) as usize;
            let p95_idx = (sorted_samples.len() as f64 * 0.95) as usize;
            let p99_idx = (sorted_samples.len() as f64 * 0.99) as usize;

            let metrics = ComponentMetrics {
                component_id: component_id.to_string(),
                response_time_p50: sorted_samples[p50_idx.min(sorted_samples.len() - 1)],
                response_time_p95: sorted_samples[p95_idx.min(sorted_samples.len() - 1)],
                response_time_p99: sorted_samples[p99_idx.min(sorted_samples.len() - 1)],
                throughput: 1.0 / sorted_samples.iter().sum::<Duration>().as_secs_f64() * sorted_samples.len() as f64,
                ..Default::default()
            };

            self.metrics.insert(component_id.to_string(), metrics);
        }
    }

    pub fn get_metrics(&self, component_id: &str) -> Option<ComponentMetrics> {
        self.metrics.get(component_id).map(|m| m.clone())
    }
}
```

### Day 3-4: Service Abstraction Layer

**File**: `crates/terraphim_desktop_gpui/src/services/abstract.rs`

```rust
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;

/// Universal service identifier
pub type ServiceId = String;

/// Generic service interface for dependency injection
#[async_trait]
pub trait ServiceInterface: Send + Sync + 'static {
    type Request: Send + Sync + Serialize + for<'de> Deserialize<'de>;
    type Response: Send + Sync + Serialize + for<'de> Deserialize<'de>;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Execute service request
    async fn execute(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;

    /// Service health check
    async fn health_check(&self) -> Result<(), Self::Error>;

    /// Service capabilities
    fn capabilities(&self) -> ServiceCapabilities;

    /// Service metadata
    fn metadata(&self) -> ServiceMetadata;
}

/// Service capabilities descriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    pub supports_caching: bool,
    pub supports_streaming: bool,
    pub supports_batch: bool,
    pub supports_cancellation: bool,
    pub max_concurrent_requests: Option<usize>,
    pub rate_limit: Option<RateLimit>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

/// Service metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<ServiceId>,
}

/// Type-erased service wrapper
pub trait AnyService: Send + Sync + 'static {
    fn service_id(&self) -> &ServiceId;
    fn metadata(&self) -> ServiceMetadata;
    async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Service registry for dependency injection
pub struct ServiceRegistry {
    services: dashmap::DashMap<ServiceId, Arc<dyn AnyService>>,
    factories: HashMap<ServiceId, ServiceFactory>,
    metrics: ServiceMetrics,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: dashmap::DashMap::new(),
            factories: HashMap::new(),
            metrics: ServiceMetrics::new(),
        }
    }

    /// Register a service instance
    pub fn register<T>(&self, service: Arc<T>) -> Result<()>
    where
        T: ServiceInterface + AnyService + 'static,
    {
        let service_id = service.service_id().clone();
        self.services.insert(service_id, service);
        Ok(())
    }

    /// Register a service factory for lazy initialization
    pub fn register_factory<T>(&mut self, factory: impl Fn() -> Arc<T> + Send + Sync + 'static)
    where
        T: ServiceInterface + AnyService + 'static,
    {
        let service_id = T::default_metadata().name;
        self.factories.insert(
            service_id,
            ServiceFactory {
                create: Box::new(move || {
                    let service = factory();
                    let service_id = service.service_id().clone();
                    (service_id, service as Arc<dyn AnyService>)
                }),
            },
        );
    }

    /// Get a service by ID
    pub fn get<T>(&self, service_id: &str) -> Result<Arc<T>>
    where
        T: ServiceInterface + AnyService + 'static,
    {
        // Try to get existing service
        if let Some(service) = self.services.get(service_id) {
            if let Some(typed_service) = service.as_any().downcast_ref::<T>() {
                return Ok(Arc::clone(typed_service));
            }
        }

        // Try to create from factory
        if let Some(factory) = self.factories.get(service_id) {
            let (id, service) = (factory.create)();
            self.services.insert(id, Arc::clone(&service));

            if let Some(typed_service) = service.as_any().downcast_ref::<T>() {
                return Ok(Arc::clone(typed_service));
            }
        }

        Err(anyhow::anyhow!("Service not found: {}", service_id))
    }

    /// Get all services
    pub fn list_services(&self) -> Vec<ServiceMetadata> {
        self.services
            .iter()
            .map(|s| s.value().metadata())
            .collect()
    }

    /// Health check all services
    pub async fn health_check_all(&self) -> HashMap<ServiceId, Result<(), String>> {
        let mut results = HashMap::new();

        for entry in self.services.iter() {
            let service_id = entry.key().clone();
            let service = entry.value().clone();

            match service.health_check().await {
                Ok(_) => {
                    results.insert(service_id, Ok(()));
                }
                Err(e) => {
                    results.insert(service_id, Err(e.to_string()));
                }
            }
        }

        results
    }
}

/// Service factory for lazy initialization
struct ServiceFactory {
    create: Box<dyn Fn() -> (ServiceId, Arc<dyn AnyService>) + Send + Sync>,
}

/// Service-wide metrics
#[derive(Debug, Default)]
pub struct ServiceMetrics {
    pub total_requests: std::sync::atomic::AtomicU64,
    pub successful_requests: std::sync::atomic::AtomicU64,
    pub failed_requests: std::sync::atomic::AtomicU64,
    pub total_response_time: std::sync::atomic::AtomicU64, // in microseconds
}

impl ServiceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, duration: Duration, success: bool) {
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_response_time.fetch_add(
            duration.as_micros() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        if success {
            self.successful_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }

        let successful = self.successful_requests.load(std::sync::atomic::Ordering::Relaxed);
        successful as f64 / total as f64
    }

    pub fn get_average_response_time(&self) -> Duration {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total == 0 {
            return Duration::ZERO;
        }

        let total_time = self.total_response_time.load(std::sync::atomic::Ordering::Relaxed);
        Duration::from_micros(total_time / total)
    }
}
```

### Day 5: Configuration System

**File**: `crates/terraphim_desktop_gpui/src/config/component.rs`

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use anyhow::Result;

/// Universal component identifier
pub type ComponentId = String;

/// Standardized configuration for all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub component_id: ComponentId,
    pub version: String,
    pub theme: ThemeConfig,
    pub performance: PerformanceConfig,
    pub features: FeatureFlags,
    pub integrations: IntegrationConfig,
    pub custom: HashMap<String, serde_json::Value>,
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    pub primary_color: String,
    pub secondary_color: String,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub custom_css: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    Auto,
}

/// Performance optimization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub cache_size: Option<usize>,
    pub debounce_ms: u64,
    pub batch_size: usize,
    pub timeout_ms: u64,
    pub enable_metrics: bool,
    pub enable_profiling: bool,
    pub max_memory_mb: Option<usize>,
    pub gc_strategy: GarbageCollectionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GarbageCollectionStrategy {
    Immediate,
    Scheduled,
    Threshold,
    Manual,
}

/// Feature flags for conditional functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_animations: bool,
    pub enable_keyboard_shortcuts: bool,
    pub enable_accessibility: bool,
    pub enable_debug_mode: bool,
    pub enable_telemetry: bool,
    pub custom: HashMap<String, bool>,
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub services: HashMap<String, ServiceIntegration>,
    pub events: EventIntegrationConfig,
    pub storage: StorageIntegrationConfig,
}

/// Service-specific integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceIntegration {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub retry_policy: RetryPolicy,
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub exponential_backoff: bool,
}

/// Event integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIntegrationConfig {
    pub enable_bus: bool,
    pub buffer_size: usize,
    pub persistence: Option<EventPersistenceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPersistenceConfig {
    pub enabled: bool,
    pub max_events: usize,
    pub retention_days: u32,
}

/// Storage integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageIntegrationConfig {
    pub backend: StorageBackend,
    pub connection_string: Option<String>,
    pub pool_size: Option<usize>,
    pub encryption: Option<EncryptionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Memory,
    File,
    Sqlite,
    Postgresql,
    Redis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub key_derivation: KeyDerivation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyDerivation {
    PBKDF2,
    Argon2,
    Scrypt,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            component_id: "default".to_string(),
            version: "1.0.0".to_string(),
            theme: ThemeConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeatureFlags::default(),
            integrations: IntegrationConfig::default(),
            custom: HashMap::new(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Auto,
            primary_color: "#007acc".to_string(),
            secondary_color: "#6c757d".to_string(),
            font_family: None,
            font_size: None,
            custom_css: None,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            cache_size: Some(1000),
            debounce_ms: 100,
            batch_size: 10,
            timeout_ms: 5000,
            enable_metrics: true,
            enable_profiling: false,
            max_memory_mb: Some(512),
            gc_strategy: GarbageCollectionStrategy::Threshold,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_animations: true,
            enable_keyboard_shortcuts: true,
            enable_accessibility: true,
            enable_debug_mode: false,
            enable_telemetry: false,
            custom: HashMap::new(),
        }
    }
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            services: HashMap::new(),
            events: EventIntegrationConfig::default(),
            storage: StorageIntegrationConfig::default(),
        }
    }
}

impl Default for EventIntegrationConfig {
    fn default() -> Self {
        Self {
            enable_bus: true,
            buffer_size: 1000,
            persistence: Some(EventPersistenceConfig {
                enabled: false,
                max_events: 10000,
                retention_days: 30,
            }),
        }
    }
}

impl Default for StorageIntegrationConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Memory,
            connection_string: None,
            pool_size: Some(10),
            encryption: None,
        }
    }
}

/// Configuration loader and validator
pub struct ConfigManager {
    configs: HashMap<ComponentId, ComponentConfig>,
    schema_validator: Option<jsonschema::JSONSchema>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            schema_validator: None,
        }
    }

    pub fn load_config(&mut self, config: ComponentConfig) -> Result<()> {
        // Validate configuration
        self.validate_config(&config)?;

        // Store configuration
        self.configs.insert(config.component_id.clone(), config);
        Ok(())
    }

    pub fn get_config(&self, component_id: &str) -> Option<&ComponentConfig> {
        self.configs.get(component_id)
    }

    pub fn update_config(&mut self, component_id: &str, updates: ComponentConfig) -> Result<()> {
        self.validate_config(&updates)?;
        self.configs.insert(component_id.to_string(), updates);
        Ok(())
    }

    fn validate_config(&self, config: &ComponentConfig) -> Result<()> {
        // Basic validation
        if config.component_id.is_empty() {
            return Err(anyhow::anyhow!("Component ID cannot be empty"));
        }

        if config.performance.debounce_ms == 0 {
            return Err(anyhow::anyhow!("Debounce delay must be greater than 0"));
        }

        if config.performance.batch_size == 0 {
            return Err(anyhow::anyhow!("Batch size must be greater than 0"));
        }

        // Schema validation if available
        if let Some(validator) = &self.schema_validator {
            let json_value = serde_json::to_value(config)?;
            validator.validate(&json_value)
                .map_err(|e| anyhow::anyhow!("Configuration schema validation failed: {}", e))?;
        }

        Ok(())
    }

    pub fn load_from_file(&mut self, file_path: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;
        let config: ComponentConfig = serde_json::from_str(&content)?;
        self.load_config(config)
    }

    pub fn save_to_file(&self, component_id: &str, file_path: &str) -> Result<()> {
        if let Some(config) = self.get_config(component_id) {
            let content = serde_json::to_string_pretty(config)?;
            std::fs::write(file_path, content)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Configuration not found for component: {}", component_id))
        }
    }
}
```

### Day 6-7: Performance Monitoring Framework

**File**: `crates/terraphim_desktop_gpui/src/monitoring/performance.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Performance alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    pub component_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    ResponseTime,
    ErrorRate,
    MemoryUsage,
    CacheHitRate,
    Throughput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Performance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub response_time_p95: Duration,
    pub error_rate: f64,
    pub memory_usage_mb: usize,
    pub cache_hit_rate: f64,
    pub throughput_min: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            response_time_p95: Duration::from_millis(100),
            error_rate: 0.01, // 1%
            memory_usage_mb: 512,
            cache_hit_rate: 0.8, // 80%
            throughput_min: 100.0,
        }
    }
}

/// Comprehensive performance tracking system
pub struct PerformanceTracker {
    component_metrics: Arc<RwLock<HashMap<String, ComponentMetrics>>>,
    global_metrics: Arc<RwLock<GlobalMetrics>>,
    thresholds: PerformanceThresholds,
    alerts: Arc<RwLock<Vec<PerformanceAlert>>>,
    alert_handlers: Vec<Box<dyn AlertHandler>>,
}

impl PerformanceTracker {
    pub fn new(thresholds: PerformanceThresholds) -> Self {
        Self {
            component_metrics: Arc::new(RwLock::new(HashMap::new())),
            global_metrics: Arc::new(RwLock::new(GlobalMetrics::new())),
            thresholds,
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_handlers: Vec::new(),
        }
    }

    /// Record operation performance
    pub async fn record_operation(&self, component_id: &str, duration: Duration, success: bool) {
        let start = Instant::now();

        // Update component metrics
        {
            let mut metrics = self.component_metrics.write().await;
            let component_metrics = metrics.entry(component_id.to_string())
                .or_insert_with(|| ComponentMetrics::new(component_id.to_string()));

            component_metrics.record_operation(duration, success);
        }

        // Update global metrics
        {
            let mut global = self.global_metrics.write().await;
            global.record_operation(duration, success);
        }

        // Check for alerts (only if operation took longer than expected)
        if start.elapsed() > Duration::from_millis(1) {
            self.check_alerts(component_id).await;
        }
    }

    /// Record custom metrics
    pub async fn record_metric(&self, component_id: &str, metric_name: &str, value: f64) {
        let mut metrics = self.component_metrics.write().await;
        let component_metrics = metrics.entry(component_id.to_string())
            .or_insert_with(|| ComponentMetrics::new(component_id.to_string()));

        component_metrics.record_custom_metric(metric_name, value);
    }

    /// Get metrics for a component
    pub async fn get_component_metrics(&self, component_id: &str) -> Option<ComponentMetrics> {
        let metrics = self.component_metrics.read().await;
        metrics.get(component_id).cloned()
    }

    /// Get all component metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, ComponentMetrics> {
        let metrics = self.component_metrics.read().await;
        metrics.clone()
    }

    /// Get global metrics
    pub async fn get_global_metrics(&self) -> GlobalMetrics {
        let global = self.global_metrics.read().await;
        global.clone()
    }

    /// Check for performance alerts
    async fn check_alerts(&self, component_id: &str) {
        let metrics = self.component_metrics.read().await;
        if let Some(component_metrics) = metrics.get(component_id) {
            let mut new_alerts = Vec::new();

            // Check response time
            if component_metrics.response_time_p95() > self.thresholds.response_time_p95 {
                new_alerts.push(PerformanceAlert {
                    component_id: component_id.to_string(),
                    alert_type: AlertType::ResponseTime,
                    severity: AlertSeverity::Warning,
                    message: format!(
                        "Response time P95 ({:?}) exceeds threshold ({:?})",
                        component_metrics.response_time_p95(),
                        self.thresholds.response_time_p95
                    ),
                    timestamp: chrono::Utc::now(),
                    metrics: HashMap::from([(
                        "response_time_p95_ms".to_string(),
                        component_metrics.response_time_p95().as_millis() as f64
                    )]),
                });
            }

            // Check error rate
            if component_metrics.error_rate() > self.thresholds.error_rate {
                new_alerts.push(PerformanceAlert {
                    component_id: component_id.to_string(),
                    alert_type: AlertType::ErrorRate,
                    severity: AlertSeverity::Error,
                    message: format!(
                        "Error rate ({:.2%}) exceeds threshold ({:.2%})",
                        component_metrics.error_rate(),
                        self.thresholds.error_rate
                    ),
                    timestamp: chrono::Utc::now(),
                    metrics: HashMap::from([(
                        "error_rate".to_string(),
                        component_metrics.error_rate()
                    )]),
                });
            }

            // Check cache hit rate
            if component_metrics.cache_hit_rate() < self.thresholds.cache_hit_rate {
                new_alerts.push(PerformanceAlert {
                    component_id: component_id.to_string(),
                    alert_type: AlertType::CacheHitRate,
                    severity: AlertSeverity::Warning,
                    message: format!(
                        "Cache hit rate ({:.2%}) below threshold ({:.2%})",
                        component_metrics.cache_hit_rate(),
                        self.thresholds.cache_hit_rate
                    ),
                    timestamp: chrono::Utc::now(),
                    metrics: HashMap::from([(
                        "cache_hit_rate".to_string(),
                        component_metrics.cache_hit_rate()
                    )]),
                });
            }

            // Store new alerts
            if !new_alerts.is_empty() {
                let mut alerts = self.alerts.write().await;
                alerts.extend(new_alerts.clone());

                // Trigger alert handlers
                for handler in &self.alert_handlers {
                    for alert in &new_alerts {
                        handler.handle_alert(alert).await;
                    }
                }
            }
        }
    }

    /// Get recent alerts
    pub async fn get_alerts(&self, since: Option<chrono::DateTime<chrono::Utc>>) -> Vec<PerformanceAlert> {
        let alerts = self.alerts.read().await;
        alerts.iter()
            .filter(|alert| {
                if let Some(since_time) = since {
                    alert.timestamp > since_time
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Add alert handler
    pub fn add_alert_handler(&mut self, handler: Box<dyn AlertHandler>) {
        self.alert_handlers.push(handler);
    }

    /// Clear old metrics and alerts
    pub async fn cleanup(&self, retention_period: Duration) {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(retention_period).unwrap();

        // Clean up alerts
        {
            let mut alerts = self.alerts.write().await;
            alerts.retain(|alert| alert.timestamp > cutoff);
        }

        // Clean up old metric samples (if any component supports it)
        // This would need to be implemented in ComponentMetrics
    }
}

/// Alert handler trait
#[async_trait::async_trait]
pub trait AlertHandler: Send + Sync {
    async fn handle_alert(&self, alert: &PerformanceAlert);
}

/// Component metrics with detailed tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    pub component_id: String,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub memory_usage: usize,
    pub custom_metrics: HashMap<String, f64>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ComponentMetrics {
    pub fn new(component_id: String) -> Self {
        Self {
            component_id,
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            total_response_time: Duration::ZERO,
            min_response_time: Duration::MAX,
            max_response_time: Duration::ZERO,
            cache_hits: 0,
            cache_misses: 0,
            memory_usage: 0,
            custom_metrics: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn record_operation(&mut self, duration: Duration, success: bool) {
        self.total_operations += 1;
        self.total_response_time += duration;

        if duration < self.min_response_time {
            self.min_response_time = duration;
        }
        if duration > self.max_response_time {
            self.max_response_time = duration;
        }

        if success {
            self.successful_operations += 1;
        } else {
            self.failed_operations += 1;
        }

        self.last_updated = chrono::Utc::now();
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.last_updated = chrono::Utc::now();
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
        self.last_updated = chrono::Utc::now();
    }

    pub fn record_custom_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics.insert(name.to_string(), value);
        self.last_updated = chrono::Utc::now();
    }

    pub fn average_response_time(&self) -> Duration {
        if self.total_operations == 0 {
            Duration::ZERO
        } else {
            self.total_response_time / self.total_operations as u32
        }
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.failed_operations as f64 / self.total_operations as f64
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total_cache_operations = self.cache_hits + self.cache_misses;
        if total_cache_operations == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total_cache_operations as f64
        }
    }

    /// Approximate P95 based on max response time
    pub fn response_time_p95(&self) -> Duration {
        self.max_response_time
    }
}

/// Global system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    pub total_operations: u64,
    pub total_response_time: Duration,
    pub active_components: usize,
    pub system_memory_usage: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl GlobalMetrics {
    pub fn new() -> Self {
        Self {
            total_operations: 0,
            total_response_time: Duration::ZERO,
            active_components: 0,
            system_memory_usage: 0,
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn record_operation(&mut self, duration: Duration, success: bool) {
        let _ = success; // Currently unused, but could track global success rate
        self.total_operations += 1;
        self.total_response_time += duration;
        self.last_updated = chrono::Utc::now();
    }

    pub fn average_response_time(&self) -> Duration {
        if self.total_operations == 0 {
            Duration::ZERO
        } else {
            self.total_response_time / self.total_operations as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_tracking() {
        let tracker = PerformanceTracker::new(PerformanceThresholds::default());

        // Record some operations
        tracker.record_operation("test_component", Duration::from_millis(10), true).await;
        tracker.record_operation("test_component", Duration::from_millis(20), true).await;
        tracker.record_operation("test_component", Duration::from_millis(30), false).await;

        // Check metrics
        let metrics = tracker.get_component_metrics("test_component").await.unwrap();
        assert_eq!(metrics.total_operations, 3);
        assert_eq!(metrics.successful_operations, 2);
        assert_eq!(metrics.failed_operations, 1);
        assert_eq!(metrics.error_rate(), 1.0 / 3.0);
        assert_eq!(metrics.average_response_time(), Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_alert_generation() {
        let thresholds = PerformanceThresholds {
            response_time_p95: Duration::from_millis(5), // Very low threshold
            ..Default::default()
        };

        let tracker = PerformanceTracker::new(thresholds);

        // Record slow operation that should trigger alert
        tracker.record_operation("slow_component", Duration::from_millis(100), true).await;

        // Check for alerts
        let alerts = tracker.get_alerts(None).await;
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].component_id, "slow_component");
        assert!(matches!(alerts[0].alert_type, AlertType::ResponseTime));
    }
}
```

## Week 1 Testing Plan

**File**: `tests/foundation_week1.rs`

```rust
use terraphim_desktop_gpui::{
    components::{ReusableComponent, ComponentError, PerformanceTracker},
    services::{ServiceInterface, ServiceRegistry},
    config::{ComponentConfig, ConfigManager},
};
use std::time::Duration;
use tokio::sync::mpsc;

#[cfg(test)]
mod tests {
    use super::*;

    // Test component implementation
    struct TestComponent {
        id: String,
        config: ComponentConfig,
        state: TestState,
        metrics: ComponentMetrics,
    }

    #[derive(Debug, Clone)]
    struct TestState {
        value: i32,
    }

    #[derive(Debug)]
    enum TestEvent {
        Increment,
        Decrement,
        SetValue(i32),
    }

    impl ReusableComponent for TestComponent {
        type Config = ComponentConfig;
        type State = TestState;
        type Event = TestEvent;

        fn new(config: Self::Config) -> Self {
            Self {
                id: config.component_id.clone(),
                config,
                state: TestState { value: 0 },
                metrics: ComponentMetrics::default(),
            }
        }

        fn component_id(&self) -> &String {
            &self.id
        }

        fn state(&self) -> &Self::State {
            &self.state
        }

        fn handle_event(&mut self, event: Self::Event) -> Result<(), ComponentError> {
            match event {
                TestEvent::Increment => {
                    self.state.value += 1;
                }
                TestEvent::Decrement => {
                    self.state.value -= 1;
                }
                TestEvent::SetValue(v) => {
                    self.state.value = v;
                }
            }
            Ok(())
        }

        fn render(&self, _cx: &mut Context<Self>) -> impl IntoElement {
            // Mock rendering
            gpui::div().child(format!("Value: {}", self.state.value))
        }

        fn metrics(&self) -> ComponentMetrics {
            self.metrics.clone()
        }

        fn on_config_change(&mut self, config: Self::Config, _cx: &mut Context<Self>) -> Result<(), ComponentError> {
            self.config = config;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_reusable_component_lifecycle() {
        let config = ComponentConfig::default();
        let mut component = TestComponent::new(config);

        // Test initial state
        assert_eq!(component.state().value, 0);

        // Test event handling
        component.handle_event(TestEvent::Increment).unwrap();
        assert_eq!(component.state().value, 1);

        component.handle_event(TestEvent::SetValue(42)).unwrap();
        assert_eq!(component.state().value, 42);
    }

    #[tokio::test]
    async fn test_service_registry() {
        let registry = ServiceRegistry::new();

        // Test empty registry
        assert!(registry.get::<TestService>("nonexistent").is_err());

        // Test service registration
        let service = Arc::new(TestService::new());
        registry.register(service.clone()).unwrap();

        // Test service retrieval
        let retrieved: Arc<TestService> = registry.get("test_service").unwrap();
        assert_eq!(retrieved.name(), service.name());
    }

    #[tokio::test]
    async fn test_performance_tracking() {
        let tracker = PerformanceTracker::new(100);

        // Record some operations
        tracker.record_operation("test", Duration::from_millis(10));
        tracker.record_operation("test", Duration::from_millis(20));
        tracker.record_operation("test", Duration::from_millis(30));

        // Check metrics
        let metrics = tracker.get_metrics("test").unwrap();
        assert_eq!(metrics.response_time_p50, Duration::from_millis(20));
        assert!(metrics.response_time_p95 >= Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_config_management() {
        let mut manager = ConfigManager::new();

        // Test config loading
        let config = ComponentConfig {
            component_id: "test".to_string(),
            ..Default::default()
        };

        assert!(manager.load_config(config.clone()).is_ok());

        // Test config retrieval
        let retrieved = manager.get_config("test").unwrap();
        assert_eq!(retrieved.component_id, config.component_id);

        // Test config validation
        let invalid_config = ComponentConfig {
            component_id: "".to_string(), // Empty ID should fail validation
            ..Default::default()
        };

        assert!(manager.load_config(invalid_config).is_err());
    }

    // Mock service for testing
    struct TestService {
        name: String,
    }

    impl TestService {
        fn new() -> Self {
            Self {
                name: "test_service".to_string(),
            }
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    // Implement required traits for TestService
    #[async_trait::async_trait]
    impl ServiceInterface for TestService {
        type Request = String;
        type Response = String;
        type Error = anyhow::Error;

        async fn execute(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
            Ok(format!("Processed: {}", request))
        }

        async fn health_check(&self) -> Result<(), Self::Error> {
            Ok(())
        }

        fn capabilities(&self) -> terraphim_desktop_gpui::services::ServiceCapabilities {
            terraphim_desktop_gpui::services::ServiceCapabilities {
                supports_caching: false,
                supports_streaming: false,
                supports_batch: true,
                supports_cancellation: false,
                max_concurrent_requests: Some(10),
                rate_limit: None,
            }
        }

        fn metadata(&self) -> terraphim_desktop_gpui::services::ServiceMetadata {
            terraphim_desktop_gpui::services::ServiceMetadata {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                description: "Test service for unit testing".to_string(),
                tags: vec!["test".to_string()],
                dependencies: vec![],
            }
        }
    }

    impl terraphim_desktop_gpui::services::AnyService for TestService {
        fn service_id(&self) -> &str {
            &self.name
        }

        fn metadata(&self) -> terraphim_desktop_gpui::services::ServiceMetadata {
            terraphim_desktop_gpui::services::ServiceMetadata {
                name: self.name.clone(),
                version: "1.0.0".to_string(),
                description: "Test service for unit testing".to_string(),
                tags: vec!["test".to_string()],
                dependencies: vec![],
            }
        }

        async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}
```

## Success Metrics for Week 1

1. **Code Quality**:
   - All new code passes `cargo clippy` with zero warnings
   - Test coverage >95% for all new modules
   - Documentation for all public APIs

2. **Performance**:
   - Component trait implementation overhead <1ms
   - Service registry lookup <100μs
   - Configuration validation <10ms
   - Performance tracking overhead <1% of operation time

3. **Functionality**:
   - All unit tests pass
   - Integration tests demonstrate component reusability
   - Service dependency injection works correctly
   - Performance tracking produces accurate metrics

## Deliverables

1. **Core Abstractions**:
   - `ReusableComponent` trait with complete lifecycle management
   - `ServiceInterface` trait for dependency injection
   - `ComponentConfig` system for configuration-driven behavior

2. **Infrastructure**:
   - `ServiceRegistry` for dependency management
   - `PerformanceTracker` for metrics collection
   - `ConfigManager` for configuration validation

3. **Testing Framework**:
   - Unit tests for all core abstractions
   - Integration tests demonstrating reusability
   - Performance benchmarks for infrastructure components

4. **Documentation**:
   - API documentation with examples
   - Architecture decision records
   - Implementation guidelines for future components

This foundation provides the necessary building blocks for all subsequent reusable components, ensuring consistent behavior, performance monitoring, and testability across the entire system.