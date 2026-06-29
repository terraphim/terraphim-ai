//! Global concurrency controller with fairness.
//!
//! Enforces global agent limits, per-mode quotas, and per-project caps, and
//! ensures fairness between time-driven and issue-driven modes.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Concurrency controller with fairness policies.
#[derive(Clone)]
pub struct ConcurrencyController {
    /// Global semaphore for total agent limit.
    global: Arc<Semaphore>,
    /// Per-mode quotas.
    quotas: ModeQuotas,
    /// Optional per-project caps (project id -> max concurrent agents).
    /// Missing entries mean "no per-project cap".
    project_caps: Arc<HashMap<String, ProjectCaps>>,
    /// Currently running agents by mode + per-project.
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

/// Per-project concurrency caps.
#[derive(Debug, Clone, Copy)]
pub struct ProjectCaps {
    /// Maximum concurrent agents (time + issue + mention) for this project.
    pub max_concurrent_agents: usize,
    /// Maximum concurrent mention-driven agents for this project.
    /// None means "no per-project mention cap" (fall back to global).
    pub max_concurrent_mention_agents: Option<usize>,
}

/// Currently running agent counts.
#[derive(Debug, Default)]
struct RunningCounts {
    time_driven: usize,
    issue_driven: usize,
    mention_driven: usize,
    per_project: HashMap<String, ProjectRunning>,
}

#[derive(Debug, Default, Clone, Copy)]
struct ProjectRunning {
    total: usize,
    mention: usize,
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
    project: String,
    running: Arc<Mutex<RunningCounts>>,
}

#[derive(Debug, Clone, Copy)]
enum AgentMode {
    TimeDriven,
    IssueDriven,
    MentionDriven,
}

impl Drop for AgentPermit {
    fn drop(&mut self) {
        let mode = self.mode;
        let project = std::mem::take(&mut self.project);
        let running = self.running.clone();
        tokio::spawn(async move {
            let mut counts = running.lock().await;
            match mode {
                AgentMode::TimeDriven => counts.time_driven = counts.time_driven.saturating_sub(1),
                AgentMode::IssueDriven => {
                    counts.issue_driven = counts.issue_driven.saturating_sub(1)
                }
                AgentMode::MentionDriven => {
                    counts.mention_driven = counts.mention_driven.saturating_sub(1)
                }
            }
            if let Some(proj) = counts.per_project.get_mut(&project) {
                proj.total = proj.total.saturating_sub(1);
                if let AgentMode::MentionDriven = mode {
                    proj.mention = proj.mention.saturating_sub(1);
                }
                if proj.total == 0 && proj.mention == 0 {
                    counts.per_project.remove(&project);
                }
            }
        });
    }
}

impl ConcurrencyController {
    /// Create a new concurrency controller with no per-project caps.
    pub fn new(global_max: usize, quotas: ModeQuotas, fairness: FairnessPolicy) -> Self {
        Self::with_project_caps(global_max, quotas, fairness, HashMap::new())
    }

    /// Create a new concurrency controller with explicit per-project caps.
    pub fn with_project_caps(
        global_max: usize,
        quotas: ModeQuotas,
        fairness: FairnessPolicy,
        project_caps: HashMap<String, ProjectCaps>,
    ) -> Self {
        Self {
            global: Arc::new(Semaphore::new(global_max)),
            quotas,
            project_caps: Arc::new(project_caps),
            running: Arc::new(Mutex::new(RunningCounts::default())),
            fairness,
        }
    }

    /// Try to acquire a slot for a time-driven agent in the given project.
    pub async fn acquire_time_driven(&self, project: &str) -> Option<AgentPermit> {
        self.acquire(AgentMode::TimeDriven, project).await
    }

    /// Try to acquire a slot for an issue-driven agent in the given project.
    pub async fn acquire_issue_driven(&self, project: &str) -> Option<AgentPermit> {
        self.acquire(AgentMode::IssueDriven, project).await
    }

    /// Try to acquire a slot for a mention-driven agent in the given project.
    pub async fn acquire_mention_driven(&self, project: &str) -> Option<AgentPermit> {
        self.acquire(AgentMode::MentionDriven, project).await
    }

    /// Try to acquire a generic slot without mode-specific quota checks.
    ///
    /// Only enforces the global semaphore and per-project caps. Used when the
    /// caller does not know the agent mode (e.g. `spawn_agent` is shared across
    /// cron, mention, and issue-driven paths).
    pub async fn acquire_any(&self, project: &str) -> Option<AgentPermit> {
        if !self
            .project_has_capacity(AgentMode::TimeDriven, project)
            .await
        {
            return None;
        }
        let global_permit = match self.global.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::debug!("global concurrency limit reached");
                return None;
            }
        };
        {
            let mut counts = self.running.lock().await;
            counts.time_driven += 1;
            let entry = counts.per_project.entry(project.to_string()).or_default();
            entry.total += 1;
        }
        tracing::debug!(project, "acquired generic concurrency slot");
        Some(AgentPermit {
            _global: global_permit,
            mode: AgentMode::TimeDriven,
            project: project.to_string(),
            running: self.running.clone(),
        })
    }

    /// Get current running counts (time_driven, issue_driven).
    pub async fn running_counts(&self) -> (usize, usize) {
        let counts = self.running.lock().await;
        (counts.time_driven, counts.issue_driven)
    }

    /// Get the running count for a specific project.
    pub async fn project_running_count(&self, project: &str) -> usize {
        let counts = self.running.lock().await;
        counts
            .per_project
            .get(project)
            .map(|p| p.total)
            .unwrap_or(0)
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
            // Mention-driven currently has no global mode quota; per-project
            // mention cap is checked separately in `project_has_capacity`.
            AgentMode::MentionDriven => true,
        }
    }

    /// Check if project has capacity for this mode.
    async fn project_has_capacity(&self, mode: AgentMode, project: &str) -> bool {
        let Some(caps) = self.project_caps.get(project) else {
            return true;
        };
        let counts = self.running.lock().await;
        let running = counts.per_project.get(project).copied().unwrap_or_default();

        if running.total >= caps.max_concurrent_agents {
            tracing::debug!(
                project,
                total = running.total,
                cap = caps.max_concurrent_agents,
                "per-project cap reached"
            );
            return false;
        }
        if matches!(mode, AgentMode::MentionDriven) {
            if let Some(mention_cap) = caps.max_concurrent_mention_agents {
                if running.mention >= mention_cap {
                    tracing::debug!(
                        project,
                        mention = running.mention,
                        cap = mention_cap,
                        "per-project mention cap reached"
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Get the active fairness policy.
    pub fn fairness_policy(&self) -> FairnessPolicy {
        self.fairness
    }

    /// Acquire a slot for the given mode in the given project.
    async fn acquire(&self, mode: AgentMode, project: &str) -> Option<AgentPermit> {
        // Check mode quota first
        if !self.mode_has_capacity(mode).await {
            tracing::debug!(?mode, "mode quota exceeded");
            return None;
        }

        // Check per-project cap
        if !self.project_has_capacity(mode, project).await {
            return None;
        }

        // Apply fairness policy: under Proportional, check whether the mode
        // is consuming more than its fair share of global capacity.
        if self.fairness == FairnessPolicy::Proportional {
            let counts = self.running.lock().await;
            let total = counts.time_driven + counts.issue_driven;
            let global_cap = self.global.available_permits() + total;
            if global_cap > 0 {
                // Proportional fairness only applies to time/issue modes with
                // quotas; mention-driven has no mode quota and is exempt.
                let mode_count = match mode {
                    AgentMode::TimeDriven => counts.time_driven,
                    AgentMode::IssueDriven => counts.issue_driven,
                    AgentMode::MentionDriven => 0,
                };
                let mode_quota = match mode {
                    AgentMode::TimeDriven => self.quotas.time_max,
                    AgentMode::IssueDriven => self.quotas.issue_max,
                    AgentMode::MentionDriven => usize::MAX,
                };
                let total_quota = self.quotas.time_max + self.quotas.issue_max;
                // Fair share = global_cap * (mode_quota / total_quota)
                if !matches!(mode, AgentMode::MentionDriven) {
                    let fair_share = (global_cap * mode_quota) / total_quota.max(1);
                    if mode_count >= fair_share && fair_share > 0 {
                        tracing::debug!(
                            ?mode,
                            mode_count,
                            fair_share,
                            "proportional fairness limit"
                        );
                        return None;
                    }
                }
            }
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
                AgentMode::MentionDriven => counts.mention_driven += 1,
            }
            let entry = counts.per_project.entry(project.to_string()).or_default();
            entry.total += 1;
            if matches!(mode, AgentMode::MentionDriven) {
                entry.mention += 1;
            }
        }

        tracing::debug!(?mode, project, "acquired concurrency slot");

        Some(AgentPermit {
            _global: global_permit,
            mode,
            project: project.to_string(),
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

    const TEST_PROJECT: &str = "__global__";

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
        let permit1 = controller.acquire_time_driven(TEST_PROJECT).await;
        assert!(permit1.is_some());

        // Acquire second permit
        let permit2 = controller.acquire_time_driven(TEST_PROJECT).await;
        assert!(permit2.is_some());

        // Third should fail (global limit)
        let permit3 = controller.acquire_time_driven(TEST_PROJECT).await;
        assert!(permit3.is_none());

        // Drop one and try again
        drop(permit1);

        // Wait a bit for the drop to propagate
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let permit4 = controller.acquire_time_driven(TEST_PROJECT).await;
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
        let time_permit = controller.acquire_time_driven(TEST_PROJECT).await;
        assert!(time_permit.is_some());

        // Second time-driven should fail
        let time_permit2 = controller.acquire_time_driven(TEST_PROJECT).await;
        assert!(time_permit2.is_none());

        // But issue-driven should succeed
        let issue_permit = controller.acquire_issue_driven(TEST_PROJECT).await;
        assert!(issue_permit.is_some());

        // Second issue-driven should fail
        let issue_permit2 = controller.acquire_issue_driven(TEST_PROJECT).await;
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

        let _time_permit = controller.acquire_time_driven(TEST_PROJECT).await.unwrap();
        let _issue_permit = controller.acquire_issue_driven(TEST_PROJECT).await.unwrap();

        let (time_count, issue_count) = controller.running_counts().await;
        assert_eq!(time_count, 1);
        assert_eq!(issue_count, 1);
    }

    #[tokio::test]
    async fn test_per_project_cap_saturates_independently() {
        let mut caps = HashMap::new();
        caps.insert(
            "alpha".to_string(),
            ProjectCaps {
                max_concurrent_agents: 1,
                max_concurrent_mention_agents: None,
            },
        );
        caps.insert(
            "beta".to_string(),
            ProjectCaps {
                max_concurrent_agents: 2,
                max_concurrent_mention_agents: None,
            },
        );
        let controller = ConcurrencyController::with_project_caps(
            10,
            ModeQuotas {
                time_max: 5,
                issue_max: 5,
            },
            FairnessPolicy::RoundRobin,
            caps,
        );

        // alpha: cap 1
        let a1 = controller.acquire_time_driven("alpha").await;
        assert!(a1.is_some());
        let a2 = controller.acquire_time_driven("alpha").await;
        assert!(
            a2.is_none(),
            "alpha should be saturated after reaching its cap of 1"
        );

        // beta: cap 2 — independent of alpha
        let b1 = controller.acquire_time_driven("beta").await;
        let b2 = controller.acquire_time_driven("beta").await;
        assert!(b1.is_some());
        assert!(b2.is_some());
        let b3 = controller.acquire_time_driven("beta").await;
        assert!(
            b3.is_none(),
            "beta should be saturated after reaching its cap of 2"
        );

        // Release alpha — permit slot returns to alpha independently.
        drop(a1);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let a3 = controller.acquire_time_driven("alpha").await;
        assert!(a3.is_some(), "alpha should have capacity after drop");
    }

    #[tokio::test]
    async fn test_per_project_mention_cap() {
        let mut caps = HashMap::new();
        caps.insert(
            "alpha".to_string(),
            ProjectCaps {
                max_concurrent_agents: 5,
                max_concurrent_mention_agents: Some(1),
            },
        );
        let controller = ConcurrencyController::with_project_caps(
            10,
            ModeQuotas {
                time_max: 5,
                issue_max: 5,
            },
            FairnessPolicy::RoundRobin,
            caps,
        );

        let m1 = controller.acquire_mention_driven("alpha").await;
        assert!(m1.is_some());
        let m2 = controller.acquire_mention_driven("alpha").await;
        assert!(
            m2.is_none(),
            "mention cap of 1 should block second mention acquire"
        );
        // Other modes still have capacity under the total cap of 5.
        let t1 = controller.acquire_time_driven("alpha").await;
        assert!(t1.is_some());
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
