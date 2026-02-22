//! Metrics and tracing for the router
//!
//! This module provides instrumentation for routing decisions,
//! agent spawn times, and health check failures.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use terraphim_types::capability::{ProcessId, Provider};

/// Metrics collector for routing operations
#[derive(Debug, Default)]
pub struct RouterMetrics {
    /// Total routing requests
    routing_requests: AtomicU64,
    /// Successful routings
    routing_success: AtomicU64,
    /// Failed routings
    routing_failures: AtomicU64,
    /// Agent spawn attempts
    spawn_attempts: AtomicU64,
    /// Successful spawns
    spawn_success: AtomicU64,
    /// Failed spawns
    spawn_failures: AtomicU64,
    /// Health check failures
    health_failures: AtomicU64,
}

impl RouterMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a routing request
    pub fn record_routing_request(&self,
        provider: &Provider,
        duration_ms: u64,
    ) {
        self.routing_requests.fetch_add(1, Ordering::Relaxed);
        self.routing_success.fetch_add(1, Ordering::Relaxed);

        log::info!(
            target: "terraphim_router::metrics",
            "routing_request{{provider=\"{}\", duration_ms={}}}",
            provider.id,
            duration_ms
        );
    }

    /// Record a routing failure
    pub fn record_routing_failure(&self,
        reason: &str,
    ) {
        self.routing_failures.fetch_add(1, Ordering::Relaxed);

        log::warn!(
            target: "terraphim_router::metrics",
            "routing_failure{{reason=\"{}\"}}",
            reason
        );
    }

    /// Record an agent spawn attempt
    pub fn record_spawn_attempt(&self,
        provider: &Provider,
    ) {
        self.spawn_attempts.fetch_add(1, Ordering::Relaxed);

        log::info!(
            target: "terraphim_router::metrics",
            "spawn_attempt{{provider=\"{}\"}}",
            provider.id
        );
    }

    /// Record a successful spawn
    pub fn record_spawn_success(&self,
        process_id: ProcessId,
        duration_ms: u64,
    ) {
        self.spawn_success.fetch_add(1, Ordering::Relaxed);

        log::info!(
            target: "terraphim_router::metrics",
            "spawn_success{{process_id=\"{}\", duration_ms={}}}",
            process_id,
            duration_ms
        );
    }

    /// Record a spawn failure
    pub fn record_spawn_failure(&self,
        provider: &Provider,
        error: &str,
    ) {
        self.spawn_failures.fetch_add(1, Ordering::Relaxed);

        log::error!(
            target: "terraphim_router::metrics",
            "spawn_failure{{provider=\"{}\", error=\"{}\"}}",
            provider.id,
            error
        );
    }

    /// Record a health check failure
    pub fn record_health_failure(&self,
        process_id: ProcessId,
    ) {
        self.health_failures.fetch_add(1, Ordering::Relaxed);

        log::warn!(
            target: "terraphim_router::metrics",
            "health_failure{{process_id=\"{}\"}}",
            process_id
        );
    }

    /// Get total routing requests
    pub fn routing_requests(&self) -> u64 {
        self.routing_requests.load(Ordering::Relaxed)
    }

    /// Get routing success count
    pub fn routing_success(&self) -> u64 {
        self.routing_success.load(Ordering::Relaxed)
    }

    /// Get routing failure count
    pub fn routing_failures(&self) -> u64 {
        self.routing_failures.load(Ordering::Relaxed)
    }

    /// Get spawn attempt count
    pub fn spawn_attempts(&self) -> u64 {
        self.spawn_attempts.load(Ordering::Relaxed)
    }

    /// Get spawn success count
    pub fn spawn_success(&self) -> u64 {
        self.spawn_success.load(Ordering::Relaxed)
    }

    /// Get spawn failure count
    pub fn spawn_failures(&self) -> u64 {
        self.spawn_failures.load(Ordering::Relaxed)
    }

    /// Get health failure count
    pub fn health_failures(&self) -> u64 {
        self.health_failures.load(Ordering::Relaxed)
    }

    /// Print metrics summary
    pub fn print_summary(&self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        writeln!(f, "Router Metrics Summary:")?;
        writeln!(f, "  Routing Requests: {}", self.routing_requests())?;
        writeln!(f, "  Routing Success: {}", self.routing_success())?;
        writeln!(f, "  Routing Failures: {}", self.routing_failures())?;
        writeln!(f, "  Spawn Attempts: {}", self.spawn_attempts())?;
        writeln!(f, "  Spawn Success: {}", self.spawn_success())?;
        writeln!(f, "  Spawn Failures: {}", self.spawn_failures())?;
        writeln!(f, "  Health Failures: {}", self.health_failures())?;
        Ok(())
    }
}

impl std::fmt::Display for RouterMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.print_summary(f)
    }
}

/// Timer for measuring operation durations
#[derive(Debug)]
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::start()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = RouterMetrics::new();

        // Record some events
        let provider = Provider::new(
            "test",
            "Test",
            terraphim_types::capability::ProviderType::Llm {
                model_id: "test".to_string(),
                api_endpoint: "https://test.com".to_string(),
            },
            vec![],
        );

        metrics.record_routing_request(&provider, 100);
        metrics.record_spawn_attempt(&provider);
        metrics.record_spawn_success(ProcessId::new(), 500);
        metrics.record_health_failure(ProcessId::new());

        // Verify counts
        assert_eq!(metrics.routing_requests(), 1);
        assert_eq!(metrics.spawn_attempts(), 1);
        assert_eq!(metrics.spawn_success(), 1);
        assert_eq!(metrics.health_failures(), 1);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10);
    }
}
