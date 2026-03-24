use terraphim_router::RoutingError;
use terraphim_spawner::SpawnerError;

/// Errors that can occur during orchestrator operation.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("agent spawn failed for '{agent}': {reason}")]
    SpawnFailed { agent: String, reason: String },

    #[error("agent '{0}' not found")]
    AgentNotFound(String),

    #[error("scheduler error: {0}")]
    SchedulerError(String),

    #[error("compound review failed: {0}")]
    CompoundReviewFailed(String),

    #[error(
        "invalid agent name '{0}': must contain only alphanumeric, dash, or underscore characters"
    )]
    InvalidAgentName(String),

    #[error("handoff failed from '{from}' to '{to}': {reason}")]
    HandoffFailed {
        from: String,
        to: String,
        reason: String,
    },

    #[error(transparent)]
    Spawner(#[from] SpawnerError),

    #[error(transparent)]
    Routing(#[from] RoutingError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
