//! Policy-planning vocabulary for the Terraphim platform runtime.

use crate::{PolicyError, Route, TrustTier, Workspace};

/// Outcome of a policy decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDecision {
    /// Chosen execution route.
    pub route: Route,
    /// Assigned trust tier.
    pub trust: TrustTier,
    /// Human-readable rationale for the decision.
    pub rationale: String,
}

/// Plans how a command should be routed and trusted within a workspace.
///
/// `terraphim_gitea_runner` keeps its own `PolicyPlanner` until it is re-homed
/// here in a follow-up; do NOT modify `gitea_runner` in this task.
#[async_trait::async_trait]
pub trait PolicyPlanner: Send + Sync {
    /// Decide how `command` should run in `workspace`.
    async fn decide(
        &self,
        command: &str,
        workspace: &Workspace,
    ) -> Result<PolicyDecision, PolicyError>;
}
