//! Provider health monitoring with circuit breaker pattern
//!
//! This module provides health monitoring for LLM providers with automatic
//! circuit breaker functionality to fail over when providers are unhealthy.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::config::Provider;

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed and requests flow normally
    Closed,
    /// Circuit is open and requests are automatically failed
    Open,
    /// Circuit is half-open and testing requests to see if provider has recovered
    HalfOpen,
}

/// Provider health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Provider is healthy and responding normally
    Healthy,
    /// Provider is degraded but still responding
    Degraded,
    /// Provider is unhealthy and not responding
    Unhealthy,
    /// Provider status is unknown
    Unknown,
}

/// Health check result for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub provider_name: String,
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub timestamp_ms: u64,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes before closing circuit
    pub success_threshold: u32,
    /// How long to wait before transitioning from Open to HalfOpen
    pub recovery_timeout_ms: u64,
    /// How often to perform health checks
    pub health_check_interval_ms: u64,
    /// Response time threshold in milliseconds for considering provider degraded
    pub response_time_threshold_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            recovery_timeout_ms: 30000,       // 30 seconds
            health_check_interval_ms: 10000,  // 10 seconds
            response_time_threshold_ms: 5000, // 5 seconds
        }
    }
}

/// Circuit breaker for a single provider
#[derive(Debug)]
pub struct CircuitBreaker {
    provider_name: String,
    config: CircuitBreakerConfig,
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure_time: Option<Instant>,
    last_success_time: Option<Instant>,
    last_health_check: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(provider_name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            provider_name,
            config,
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_time: None,
            last_success_time: None,
            last_health_check: None,
        }
    }

    /// Check if a request should be allowed through the circuit breaker
    pub fn allow_request(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout has passed
                if let Some(last_failure) = self.last_failure_time {
                    let elapsed = last_failure.elapsed();
                    if elapsed >= Duration::from_millis(self.config.recovery_timeout_ms) {
                        return true; // Allow to transition to HalfOpen
                    }
                }
                false
            }
            CircuitState::HalfOpen => true, // Allow some requests to test recovery
        }
    }

    /// Record a successful request
    pub fn record_success(&mut self, response_time_ms: u64) {
        self.consecutive_successes += 1;
        self.consecutive_failures = 0;
        self.last_success_time = Some(Instant::now());

        match self.state {
            CircuitState::Open => {
                // Transition to HalfOpen on first success after recovery timeout
                self.state = CircuitState::HalfOpen;
                info!(
                    provider = %self.provider_name,
                    "Circuit breaker transitioning from Open to HalfOpen"
                );
            }
            CircuitState::HalfOpen => {
                // Transition to Closed if we've had enough consecutive successes
                if self.consecutive_successes >= self.config.success_threshold {
                    self.state = CircuitState::Closed;
                    info!(
                        provider = %self.provider_name,
                        consecutive_successes = self.consecutive_successes,
                        "Circuit breaker transitioning from HalfOpen to Closed"
                    );
                }
            }
            CircuitState::Closed => {
                // Already closed, just update metrics
                debug!(
                    provider = %self.provider_name,
                    response_time_ms = response_time_ms,
                    "Recorded successful request"
                );
            }
        }
    }

    /// Record a failed request
    pub fn record_failure(&mut self, error_message: String) {
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                // Check if we should open the circuit
                if self.consecutive_failures >= self.config.failure_threshold {
                    self.state = CircuitState::Open;
                    warn!(
                        provider = %self.provider_name,
                        consecutive_failures = self.consecutive_failures,
                        error = %error_message,
                        "Circuit breaker opening due to consecutive failures"
                    );
                } else {
                    debug!(
                        provider = %self.provider_name,
                        consecutive_failures = self.consecutive_failures,
                        error = %error_message,
                        "Recorded failed request"
                    );
                }
            }
            CircuitState::Open => {
                // Already open, just update metrics
                debug!(
                    provider = %self.provider_name,
                    error = %error_message,
                    "Recorded failure while circuit is open"
                );
            }
        }
    }

    /// Get current circuit breaker state
    pub fn state(&self) -> &CircuitState {
        &self.state
    }

    /// Get provider health status based on circuit breaker state
    pub fn health_status(&self) -> HealthStatus {
        match self.state {
            CircuitState::Closed => HealthStatus::Healthy,
            CircuitState::HalfOpen => HealthStatus::Degraded,
            CircuitState::Open => HealthStatus::Unhealthy,
        }
    }

    /// Check if health check should be performed
    pub fn should_health_check(&self) -> bool {
        if let Some(last_check) = self.last_health_check {
            last_check.elapsed() >= Duration::from_millis(self.config.health_check_interval_ms)
        } else {
            true
        }
    }

    /// Update last health check time
    pub fn update_health_check_time(&mut self) {
        self.last_health_check = Some(Instant::now());
    }

    /// Get circuit breaker statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        CircuitBreakerStats {
            provider_name: self.provider_name.clone(),
            state: self.state.clone(),
            consecutive_failures: self.consecutive_failures,
            consecutive_successes: self.consecutive_successes,
            last_failure_time_ms: self.last_failure_time.map(|_| now), // Use current time since Instant can't convert to SystemTime
            last_success_time_ms: self.last_success_time.map(|_| now), // Use current time since Instant can't convert to SystemTime
            health_status: self.health_status(),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    pub provider_name: String,
    pub state: CircuitState,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_failure_time_ms: Option<u64>,
    pub last_success_time_ms: Option<u64>,
    pub health_status: HealthStatus,
}

/// Provider health monitor with circuit breakers
#[derive(Debug)]
pub struct ProviderHealthMonitor {
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    config: CircuitBreakerConfig,
}

impl ProviderHealthMonitor {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Add a provider to monitor
    pub async fn add_provider(&self, provider: &Provider) {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = CircuitBreaker::new(provider.name.clone(), self.config.clone());
        breakers.insert(provider.name.clone(), breaker);

        info!(
            provider = %provider.name,
            "Added provider to health monitor"
        );
    }

    /// Check if a request should be allowed for a provider
    pub async fn allow_request(&self, provider_name: &str) -> bool {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(provider_name) {
            breaker.allow_request()
        } else {
            // Unknown provider, allow request but log warning
            warn!(
                provider = %provider_name,
                "Provider not found in health monitor, allowing request"
            );
            true
        }
    }

    /// Record a successful request for a provider
    pub async fn record_success(&self, provider_name: &str, response_time_ms: u64) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(provider_name) {
            breaker.record_success(response_time_ms);
        } else {
            warn!(
                provider = %provider_name,
                response_time_ms = response_time_ms,
                "Attempted to record success for unknown provider"
            );
        }
    }

    /// Record a failed request for a provider
    pub async fn record_failure(&self, provider_name: &str, error_message: String) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(provider_name) {
            breaker.record_failure(error_message);
        } else {
            warn!(
                provider = %provider_name,
                "Attempted to record failure for unknown provider"
            );
        }
    }

    /// Get health status for all providers
    pub async fn get_all_health_status(&self) -> HashMap<String, HealthCheckResult> {
        let breakers = self.circuit_breakers.read().await;
        let mut results = HashMap::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        for (provider_name, breaker) in breakers.iter() {
            let stats = breaker.stats();
            let result = HealthCheckResult {
                provider_name: provider_name.clone(),
                status: stats.health_status,
                response_time_ms: 0, // Not available from circuit breaker alone
                error_message: None,
                timestamp_ms: now,
                consecutive_failures: stats.consecutive_failures,
                consecutive_successes: stats.consecutive_successes,
            };
            results.insert(provider_name.clone(), result);
        }

        results
    }

    /// Get health status for a specific provider
    pub async fn get_provider_health(&self, provider_name: &str) -> Option<HealthCheckResult> {
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(provider_name) {
            let stats = breaker.stats();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            Some(HealthCheckResult {
                provider_name: provider_name.to_string(),
                status: stats.health_status,
                response_time_ms: 0,
                error_message: None,
                timestamp_ms: now,
                consecutive_failures: stats.consecutive_failures,
                consecutive_successes: stats.consecutive_successes,
            })
        } else {
            None
        }
    }

    /// Get circuit breaker statistics for all providers
    pub async fn get_circuit_breaker_stats(&self) -> Vec<CircuitBreakerStats> {
        let breakers = self.circuit_breakers.read().await;
        breakers.values().map(|breaker| breaker.stats()).collect()
    }

    /// Get healthy providers (circuit is closed)
    pub async fn get_healthy_providers(&self) -> Vec<String> {
        let breakers = self.circuit_breakers.read().await;
        breakers
            .iter()
            .filter(|(_, breaker)| breaker.state() == &CircuitState::Closed)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get unhealthy providers (circuit is open)
    pub async fn get_unhealthy_providers(&self) -> Vec<String> {
        let breakers = self.circuit_breakers.read().await;
        breakers
            .iter()
            .filter(|(_, breaker)| breaker.state() == &CircuitState::Open)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Get degraded providers (circuit is half-open)
    pub async fn get_degraded_providers(&self) -> Vec<String> {
        let breakers = self.circuit_breakers.read().await;
        breakers
            .iter()
            .filter(|(_, breaker)| breaker.state() == &CircuitState::HalfOpen)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Reset circuit breaker for a provider (force to closed state)
    pub async fn reset_provider(&self, provider_name: &str) -> Result<(), String> {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(provider_name) {
            breaker.state = CircuitState::Closed;
            breaker.consecutive_failures = 0;
            breaker.consecutive_successes = 0;
            breaker.last_failure_time = None;
            breaker.last_success_time = None;

            info!(
                provider = %provider_name,
                "Reset circuit breaker to closed state"
            );
            Ok(())
        } else {
            Err(format!(
                "Provider {} not found in health monitor",
                provider_name
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Provider;

    #[test]
    fn test_circuit_breaker_new() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test-provider".to_string(), config);

        assert_eq!(breaker.state(), &CircuitState::Closed);
        assert_eq!(breaker.consecutive_failures, 0);
        assert_eq!(breaker.consecutive_successes, 0);
        assert_eq!(breaker.health_status(), HealthStatus::Healthy);
    }

    #[test]
    fn test_circuit_breaker_allow_request() {
        let config = CircuitBreakerConfig::default();
        let mut breaker = CircuitBreaker::new("test-provider".to_string(), config);

        // Should allow request when closed
        assert!(breaker.allow_request());

        // Simulate failures to open circuit
        for _ in 0..5 {
            breaker.record_failure("test error".to_string());
        }

        // Should not allow request when open
        assert!(!breaker.allow_request());

        // Should allow request after recovery timeout
        breaker.last_failure_time = Some(Instant::now() - Duration::from_millis(31000));
        assert!(breaker.allow_request());
    }

    #[test]
    fn test_circuit_breaker_success_recording() {
        let config = CircuitBreakerConfig::default();
        let mut breaker = CircuitBreaker::new("test-provider".to_string(), config);

        // Record success
        breaker.record_success(100);
        assert_eq!(breaker.consecutive_successes, 1);
        assert_eq!(breaker.consecutive_failures, 0);
        assert_eq!(breaker.state(), &CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_failure_recording() {
        let config = CircuitBreakerConfig::default();
        let mut breaker = CircuitBreaker::new("test-provider".to_string(), config);

        // Record failures to open circuit
        for i in 1..=5 {
            breaker.record_failure(format!("error {}", i));
            assert_eq!(breaker.consecutive_failures, i);
            assert_eq!(breaker.consecutive_successes, 0);
        }

        // Should be open after 5 failures
        assert_eq!(breaker.state(), &CircuitState::Open);
        assert_eq!(breaker.health_status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig::default();
        let mut breaker = CircuitBreaker::new("test-provider".to_string(), config);

        // Open circuit with failures
        for _ in 0..5 {
            breaker.record_failure("test error".to_string());
        }
        assert_eq!(breaker.state(), &CircuitState::Open);

        // Simulate recovery timeout passing
        breaker.last_failure_time = Some(Instant::now() - Duration::from_millis(31000));

        // First success should transition to half-open
        breaker.record_success(100);
        assert_eq!(breaker.state(), &CircuitState::HalfOpen);
        assert_eq!(breaker.health_status(), HealthStatus::Degraded);

        // More successes should close circuit
        for _ in 0..3 {
            breaker.record_success(100);
        }
        assert_eq!(breaker.state(), &CircuitState::Closed);
        assert_eq!(breaker.health_status(), HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_provider_health_monitor_add_provider() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config);

        let provider = Provider {
            name: "test-provider".to_string(),
            api_base_url: "http://localhost:8000".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        };

        monitor.add_provider(&provider).await;

        let health = monitor.get_provider_health("test-provider").await;
        assert!(health.is_some());
        assert_eq!(health.unwrap().status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_provider_health_monitor_record_failure() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config);

        let provider = Provider {
            name: "test-provider".to_string(),
            api_base_url: "http://localhost:8000".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        };

        monitor.add_provider(&provider).await;

        // Record failures to open circuit
        for _ in 0..5 {
            monitor
                .record_failure("test-provider", "test error".to_string())
                .await;
        }

        let health = monitor.get_provider_health("test-provider").await;
        assert!(health.is_some());
        assert_eq!(health.unwrap().status, HealthStatus::Unhealthy);

        // Should not allow requests
        let allowed = monitor.allow_request("test-provider").await;
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_provider_health_monitor_healthy_providers() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config);

        let provider1 = Provider {
            name: "healthy-provider".to_string(),
            api_base_url: "http://localhost:8000".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        };

        let provider2 = Provider {
            name: "unhealthy-provider".to_string(),
            api_base_url: "http://localhost:8001".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        };

        monitor.add_provider(&provider1).await;
        monitor.add_provider(&provider2).await;

        // Make one provider unhealthy
        for _ in 0..5 {
            monitor
                .record_failure("unhealthy-provider", "test error".to_string())
                .await;
        }

        let healthy = monitor.get_healthy_providers().await;
        let unhealthy = monitor.get_unhealthy_providers().await;

        assert_eq!(healthy.len(), 1);
        assert_eq!(unhealthy.len(), 1);
        assert!(healthy.contains(&"healthy-provider".to_string()));
        assert!(unhealthy.contains(&"unhealthy-provider".to_string()));
    }

    #[tokio::test]
    async fn test_provider_health_monitor_reset() {
        let config = CircuitBreakerConfig::default();
        let monitor = ProviderHealthMonitor::new(config);

        let provider = Provider {
            name: "test-provider".to_string(),
            api_base_url: "http://localhost:8000".to_string(),
            api_key: "test-key".to_string(),
            models: vec!["test-model".to_string()],
            transformers: vec![],
        };

        monitor.add_provider(&provider).await;

        // Make provider unhealthy
        for _ in 0..5 {
            monitor
                .record_failure("test-provider", "test error".to_string())
                .await;
        }

        // Reset provider
        let result = monitor.reset_provider("test-provider").await;
        assert!(result.is_ok());

        let health = monitor.get_provider_health("test-provider").await;
        assert!(health.is_some());
        assert_eq!(health.unwrap().status, HealthStatus::Healthy);

        // Should allow requests again
        let allowed = monitor.allow_request("test-provider").await;
        assert!(allowed);
    }
}
