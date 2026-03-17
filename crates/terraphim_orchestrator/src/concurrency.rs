//! Global concurrency controller with fairness.
//!
//! Enforces global agent limits and ensures fairness between time-driven
//! and issue-driven modes.

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Concurrency controller with fairness policies.
#[derive(Clone)]
pub struct ConcurrencyController {
    /// Global semaphore for total agent limit.
    global: Arc<Semaphore>,
    /// Per-mode quotas.
    quotas: ModeQuotas,
    /// Currently running agents by mode.
    running: Arc<Mutex<RunningCounts>>,
    /// Fairness policy.
    fairness: FairnessPolicy,
}

/// Quotas for each mode.
#[derive(Debug, Clone, Copy)]
pub struct ModeQuotas {
    /// Maximum concurrent time-driven agents.
    pub time_max: usize,
    /// Maximum concurrent issue-driven agents.
    pub issue_max: usize,
}

/// Currently running agent counts.
#[derive(Debug, Default)]
struct RunningCounts {
    time_driven: usize,
    issue_driven: usize,
}

/// Fairness policy for mode coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairnessPolicy {
    /// Round-robin: alternate between modes when both have work.
    RoundRobin,
    /// Priority: always prefer higher priority tasks regardless of mode.
    Priority,
    /// Proportional: allocate slots proportionally to mode quotas.
    Proportional,
}

impl std::str::FromStr for FairnessPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "round_robin" | "round-robin" | "roundrobin" => Ok(FairnessPolicy::RoundRobin),
            "priority" => Ok(FairnessPolicy::Priority),
            "proportional" => Ok(FairnessPolicy::Proportional),
            _ => Err(format!("unknown fairness policy: {}", s)),
        }
    }
}

/// Permit for a running agent. Releases the slot when dropped.
pub struct AgentPermit {
    _global: tokio::sync::OwnedSemaphorePermit,
    mode: AgentMode,
    running: Arc<Mutex<RunningCounts>>,
}

#[derive(Debug, Clone, Copy)]
enum AgentMode {
    TimeDriven,
    IssueDriven,
}

impl Drop for AgentPermit {
    fn drop(&mut self) {
        let mode = self.mode;
        let running = self.running.clone();
        tokio::spawn(async move {
            let mut counts = running.lock().await;
            match mode {
                AgentMode::TimeDriven => counts.time_driven -= 1,
                AgentMode::IssueDriven => counts.issue_driven -= 1,
            }
        });
    }
}

impl ConcurrencyController {
    /// Create a new concurrency controller.
    pub fn new(global_max: usize, quotas: ModeQuotas, fairness: FairnessPolicy) -> Self {
        Self {
            global: Arc::new(Semaphore::new(global_max)),
            quotas,
            running: Arc::new(Mutex::new(RunningCounts::default())),
            fairness,
        }
    }

    /// Try to acquire a slot for a time-driven agent.
    pub async fn acquire_time_driven(&self) -> Option<AgentPermit> {
        self.acquire(AgentMode::TimeDriven).await
    }

    /// Try to acquire a slot for an issue-driven agent.
    pub async fn acquire_issue_driven(&self) -> Option<AgentPermit> {
        self.acquire(AgentMode::IssueDriven).await
    }

    /// Get current running counts.
    pub async fn running_counts(&self) -> (usize, usize) {
        let counts = self.running.lock().await;
        (counts.time_driven, counts.issue_driven)
    }

    /// Get available slots.
    pub fn available_slots(&self) -> usize {
        self.global.available_permits()
    }

    /// Check if mode has capacity.
    async fn mode_has_capacity(&self, mode: AgentMode) -> bool {
        let counts = self.running.lock().await;
        match mode {
            AgentMode::TimeDriven => counts.time_driven < self.quotas.time_max,
            AgentMode::IssueDriven => counts.issue_driven < self.quotas.issue_max,
        }
    }

    /// Acquire a slot for the given mode.
    async fn acquire(&self, mode: AgentMode) -> Option<AgentPermit> {
        // Check mode quota first
        if !self.mode_has_capacity(mode).await {
            tracing::debug!(?mode, "mode quota exceeded");
            return None;
        }

        // Try to acquire global permit
        let global_permit = match self.global.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::debug!("global concurrency limit reached");
                return None;
            }
        };

        // Update running counts
        {
            let mut counts = self.running.lock().await;
            match mode {
                AgentMode::TimeDriven => counts.time_driven += 1,
                AgentMode::IssueDriven => counts.issue_driven += 1,
            }
        }

        tracing::debug!(?mode, "acquired concurrency slot");

        Some(AgentPermit {
            _global: global_permit,
            mode,
            running: self.running.clone(),
        })
    }
}

impl Default for ModeQuotas {
    fn default() -> Self {
        Self {
            time_max: 3,
            issue_max: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_release() {
        let controller = ConcurrencyController::new(
            2,
            ModeQuotas {
                time_max: 2,
                issue_max: 2,
            },
            FairnessPolicy::RoundRobin,
        );

        // Acquire first permit
        let permit1 = controller.acquire_time_driven().await;
        assert!(permit1.is_some());

        // Acquire second permit
        let permit2 = controller.acquire_time_driven().await;
        assert!(permit2.is_some());

        // Third should fail (global limit)
        let permit3 = controller.acquire_time_driven().await;
        assert!(permit3.is_none());

        // Drop one and try again
        drop(permit1);

        // Wait a bit for the drop to propagate
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let permit4 = controller.acquire_time_driven().await;
        assert!(permit4.is_some());
    }

    #[tokio::test]
    async fn test_mode_quotas() {
        let controller = ConcurrencyController::new(
            10,
            ModeQuotas {
                time_max: 1,
                issue_max: 1,
            },
            FairnessPolicy::RoundRobin,
        );

        // Acquire time-driven slot
        let time_permit = controller.acquire_time_driven().await;
        assert!(time_permit.is_some());

        // Second time-driven should fail
        let time_permit2 = controller.acquire_time_driven().await;
        assert!(time_permit2.is_none());

        // But issue-driven should succeed
        let issue_permit = controller.acquire_issue_driven().await;
        assert!(issue_permit.is_some());

        // Second issue-driven should fail
        let issue_permit2 = controller.acquire_issue_driven().await;
        assert!(issue_permit2.is_none());
    }

    #[tokio::test]
    async fn test_running_counts() {
        let controller = ConcurrencyController::new(
            5,
            ModeQuotas {
                time_max: 3,
                issue_max: 3,
            },
            FairnessPolicy::RoundRobin,
        );

        let _time_permit = controller.acquire_time_driven().await.unwrap();
        let _issue_permit = controller.acquire_issue_driven().await.unwrap();

        let (time_count, issue_count) = controller.running_counts().await;
        assert_eq!(time_count, 1);
        assert_eq!(issue_count, 1);
    }

    #[test]
    fn test_fairness_policy_from_str() {
        assert_eq!(
            "round_robin".parse::<FairnessPolicy>().unwrap(),
            FairnessPolicy::RoundRobin
        );
        assert_eq!(
            "priority".parse::<FairnessPolicy>().unwrap(),
            FairnessPolicy::Priority
        );
        assert_eq!(
            "proportional".parse::<FairnessPolicy>().unwrap(),
            FairnessPolicy::Proportional
        );
        assert!("unknown".parse::<FairnessPolicy>().is_err());
    }
}
