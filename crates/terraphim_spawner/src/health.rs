//! Health checking for spawned agents with circuit breaker and history tracking.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

use terraphim_types::capability::ProcessId;

/// Health status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Agent is healthy
    Healthy,
    /// Agent is degraded (responding but slow or partially failing)
    Degraded,
    /// Agent is unhealthy
    Unhealthy,
    /// Health check timed out
    Timeout,
    /// Agent has been terminated
    Terminated,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Timeout => write!(f, "timeout"),
            HealthStatus::Terminated => write!(f, "terminated"),
        }
    }
}

// --------------- Circuit Breaker ---------------

/// Circuit breaker state machine.
///
/// Prevents routing to a failing provider by tracking consecutive failures:
/// - **Closed**: Normal operation. Failures increment a counter.
/// - **Open**: Too many failures. All requests are rejected until cooldown expires.
/// - **HalfOpen**: Cooldown expired. One probe request is allowed through.
///   Success -> Closed. Failure -> Open (reset cooldown).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation -- requests pass through.
    Closed,
    /// Circuit tripped -- requests are rejected.
    Open,
    /// Cooldown expired -- one probe request allowed.
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// Configuration for the circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u32,
    /// How long the circuit stays open before transitioning to half-open.
    pub cooldown: Duration,
    /// Number of consecutive successes in half-open required to close.
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown: Duration::from_secs(30),
            success_threshold: 1,
        }
    }
}

/// Circuit breaker that tracks failure/success patterns.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_state_change: Instant,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    /// Create a new circuit breaker in the closed state.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_state_change: Instant::now(),
            config,
        }
    }

    /// Get current circuit state, checking for cooldown expiry.
    pub fn state(&self) -> CircuitState {
        if self.state == CircuitState::Open
            && self.last_state_change.elapsed() >= self.config.cooldown
        {
            // Cooldown expired -> logically half-open
            CircuitState::HalfOpen
        } else {
            self.state
        }
    }

    /// Check whether a request should be allowed through.
    pub fn should_allow(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Allow probe
        }
    }

    /// Record a successful health check / request.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;

        match self.state() {
            CircuitState::HalfOpen => {
                self.consecutive_successes += 1;
                if self.consecutive_successes >= self.config.success_threshold {
                    self.transition(CircuitState::Closed);
                    self.consecutive_successes = 0;
                    log::info!("Circuit breaker closed after successful probe");
                }
            }
            CircuitState::Closed => {
                // Already closed, nothing to do
            }
            CircuitState::Open => {
                // Shouldn't happen (requests blocked), but handle gracefully
            }
        }
    }

    /// Record a failed health check / request.
    pub fn record_failure(&mut self) {
        self.consecutive_successes = 0;
        self.consecutive_failures += 1;

        match self.state() {
            CircuitState::Closed => {
                if self.consecutive_failures >= self.config.failure_threshold {
                    self.transition(CircuitState::Open);
                    log::warn!(
                        "Circuit breaker opened after {} consecutive failures",
                        self.consecutive_failures
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Probe failed -> back to open with fresh cooldown
                self.transition(CircuitState::Open);
                log::warn!("Circuit breaker re-opened after failed probe");
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    /// Force-reset to closed (e.g., after manual intervention).
    pub fn reset(&mut self) {
        self.transition(CircuitState::Closed);
        self.consecutive_failures = 0;
        self.consecutive_successes = 0;
    }

    fn transition(&mut self, new_state: CircuitState) {
        self.state = new_state;
        self.last_state_change = Instant::now();
    }
}

// --------------- Health History ---------------

/// A single health check record.
#[derive(Debug, Clone)]
pub struct HealthRecord {
    /// When the check occurred.
    pub timestamp: Instant,
    /// Result of the check.
    pub status: HealthStatus,
}

/// Sliding-window health history for trend analysis.
#[derive(Debug)]
pub struct HealthHistory {
    records: VecDeque<HealthRecord>,
    /// Maximum records to retain.
    max_records: usize,
    /// Window duration for degradation analysis.
    window: Duration,
}

impl HealthHistory {
    /// Create a new health history with a sliding window.
    pub fn new(max_records: usize, window: Duration) -> Self {
        Self {
            records: VecDeque::with_capacity(max_records),
            max_records,
            window,
        }
    }

    /// Record a health check result.
    pub fn record(&mut self, status: HealthStatus) {
        let now = Instant::now();

        // Evict old records
        self.evict_stale(now);

        // Evict oldest if at capacity
        if self.records.len() >= self.max_records {
            self.records.pop_front();
        }

        self.records.push_back(HealthRecord {
            timestamp: now,
            status,
        });
    }

    /// Calculate the success rate within the sliding window (0.0 to 1.0).
    pub fn success_rate(&self) -> f64 {
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);

        let window_records: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .collect();

        if window_records.is_empty() {
            return 1.0; // No data = assume healthy
        }

        let healthy_count = window_records
            .iter()
            .filter(|r| r.status == HealthStatus::Healthy)
            .count();

        healthy_count as f64 / window_records.len() as f64
    }

    /// Determine if the provider is degraded based on success rate threshold.
    ///
    /// Returns `Degraded` if success rate is below the threshold but not zero,
    /// `Unhealthy` if success rate is zero, `Healthy` otherwise.
    pub fn assess(&self, degradation_threshold: f64) -> HealthStatus {
        let rate = self.success_rate();
        if rate >= degradation_threshold {
            HealthStatus::Healthy
        } else if rate > 0.0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }

    /// Number of records currently stored.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the history is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    fn evict_stale(&mut self, now: Instant) {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        while let Some(front) = self.records.front() {
            if front.timestamp < cutoff {
                self.records.pop_front();
            } else {
                break;
            }
        }
    }
}

// --------------- Health Checker ---------------

/// Health checker for agent processes
#[derive(Debug, Clone)]
pub struct HealthChecker {
    process_id: ProcessId,
    interval: Duration,
    healthy: Arc<AtomicBool>,
    status: Arc<std::sync::Mutex<HealthStatus>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(process_id: ProcessId, interval: Duration) -> Self {
        let checker = Self {
            process_id,
            interval,
            healthy: Arc::new(AtomicBool::new(true)),
            status: Arc::new(std::sync::Mutex::new(HealthStatus::Healthy)),
        };

        // Start health check loop
        checker.start_check_loop();

        checker
    }

    /// Check if the agent is healthy
    pub async fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Relaxed)
    }

    /// Get the current health status
    pub fn status(&self) -> HealthStatus {
        *self.status.lock().unwrap()
    }

    /// Mark the agent as unhealthy
    pub fn mark_unhealthy(&self) {
        self.healthy.store(false, Ordering::Relaxed);
        *self.status.lock().unwrap() = HealthStatus::Unhealthy;
    }

    /// Mark the agent as degraded
    pub fn mark_degraded(&self) {
        // Degraded is still "alive" but not fully healthy
        self.healthy.store(true, Ordering::Relaxed);
        *self.status.lock().unwrap() = HealthStatus::Degraded;
    }

    /// Mark the agent as terminated
    pub fn mark_terminated(&self) {
        self.healthy.store(false, Ordering::Relaxed);
        *self.status.lock().unwrap() = HealthStatus::Terminated;
    }

    /// Start the health check loop
    fn start_check_loop(&self) {
        let healthy = Arc::clone(&self.healthy);
        let status = Arc::clone(&self.status);
        let interval_duration = self.interval;
        let process_id = self.process_id;

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);

            loop {
                ticker.tick().await;

                // Check the healthy flag
                if !healthy.load(Ordering::Relaxed) {
                    let current = *status.lock().unwrap();
                    log::debug!(
                        "Process {} health check: {} (exiting loop)",
                        process_id,
                        current
                    );
                    break;
                }

                log::debug!("Process {} health check: healthy", process_id);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
        assert_eq!(format!("{}", HealthStatus::Timeout), "timeout");
        assert_eq!(format!("{}", HealthStatus::Terminated), "terminated");
    }

    #[tokio::test]
    async fn test_health_checker() {
        let checker = HealthChecker::new(ProcessId::new(), Duration::from_secs(1));

        // Initially healthy
        assert!(checker.is_healthy().await);
        assert_eq!(checker.status(), HealthStatus::Healthy);

        // Mark as unhealthy
        checker.mark_unhealthy();
        assert!(!checker.is_healthy().await);
        assert_eq!(checker.status(), HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_checker_degraded() {
        let checker = HealthChecker::new(ProcessId::new(), Duration::from_secs(1));

        checker.mark_degraded();
        // Degraded is still "alive" for routing purposes
        assert!(checker.is_healthy().await);
        assert_eq!(checker.status(), HealthStatus::Degraded);
    }

    // --------------- Circuit Breaker Tests ---------------

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.should_allow());
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            cooldown: Duration::from_secs(30),
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure(); // 3rd failure -> opens
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.should_allow());
    }

    #[test]
    fn test_circuit_breaker_success_resets_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // Resets consecutive failures
        cb.record_failure();
        cb.record_failure();
        // Only 2 consecutive failures, not 3
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_after_cooldown() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown: Duration::from_millis(1), // Very short cooldown for testing
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure(); // Opens immediately
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.should_allow());
    }

    #[test]
    fn test_circuit_breaker_half_open_success_closes() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown: Duration::from_millis(1),
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure(); // -> Open
        std::thread::sleep(Duration::from_millis(5)); // -> HalfOpen
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success(); // -> Closed
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure_reopens() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            cooldown: Duration::from_millis(1),
            success_threshold: 1,
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure(); // -> Open
        std::thread::sleep(Duration::from_millis(5)); // -> HalfOpen

        cb.record_failure(); // Probe failed -> Open again
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let mut cb = CircuitBreaker::new(config);

        cb.record_failure(); // -> Open
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.should_allow());
    }

    // --------------- Health History Tests ---------------

    #[test]
    fn test_health_history_empty() {
        let history = HealthHistory::new(100, Duration::from_secs(60));
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert_eq!(history.success_rate(), 1.0); // No data = assume healthy
    }

    #[test]
    fn test_health_history_all_healthy() {
        let mut history = HealthHistory::new(100, Duration::from_secs(60));

        for _ in 0..10 {
            history.record(HealthStatus::Healthy);
        }

        assert_eq!(history.success_rate(), 1.0);
        assert_eq!(history.assess(0.8), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_history_mixed() {
        let mut history = HealthHistory::new(100, Duration::from_secs(60));

        // 7 healthy, 3 unhealthy = 70% success rate
        for _ in 0..7 {
            history.record(HealthStatus::Healthy);
        }
        for _ in 0..3 {
            history.record(HealthStatus::Unhealthy);
        }

        let rate = history.success_rate();
        assert!((rate - 0.7).abs() < 0.01);
        // With 80% threshold, 70% is degraded
        assert_eq!(history.assess(0.8), HealthStatus::Degraded);
    }

    #[test]
    fn test_health_history_all_unhealthy() {
        let mut history = HealthHistory::new(100, Duration::from_secs(60));

        for _ in 0..5 {
            history.record(HealthStatus::Unhealthy);
        }

        assert_eq!(history.success_rate(), 0.0);
        assert_eq!(history.assess(0.5), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_history_max_records() {
        let mut history = HealthHistory::new(5, Duration::from_secs(600));

        for _ in 0..10 {
            history.record(HealthStatus::Healthy);
        }

        // Only 5 records retained
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "closed");
        assert_eq!(format!("{}", CircuitState::Open), "open");
        assert_eq!(format!("{}", CircuitState::HalfOpen), "half_open");
    }
}
