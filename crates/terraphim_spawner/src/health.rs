//! Health checking for spawned agents

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use terraphim_types::capability::ProcessId;

/// Health status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Agent is healthy
    Healthy,
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
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Timeout => write!(f, "timeout"),
            HealthStatus::Terminated => write!(f, "terminated"),
        }
    }
}

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

                // In a real implementation, this would check if the process is still running
                // For now, we just verify the healthy flag hasn't been manually set to false
                if !healthy.load(Ordering::Relaxed) {
                    log::debug!("Process {} health check: unhealthy", process_id);
                    break;
                }

                // Update last check time (simplified)
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
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
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
}
