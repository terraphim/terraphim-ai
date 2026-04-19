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

    #[error("pre-check configuration error for agent '{agent}': {reason}")]
    PreCheckConfig { agent: String, reason: String },

    #[error("flow '{flow_name}' failed: {reason}")]
    FlowFailed { flow_name: String, reason: String },

    #[error("flow '{flow_name}' gate '{step_name}' rejected: {condition}")]
    FlowGateRejected {
        flow_name: String,
        step_name: String,
        condition: String,
    },

    #[error("flow template error: {0}")]
    FlowTemplateError(String),

    #[error("duplicate project id '{0}' (project ids must be unique across base + included configs)")]
    DuplicateProjectId(String),

    #[error(
        "agent '{agent}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownAgentProject { agent: String, project: String },

    #[error(
        "flow '{flow}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownFlowProject { flow: String, project: String },

    #[error(
        "banned LLM provider '{provider}' in {field} for agent '{agent}' (allowed: claude-code, opencode-go, kimi-for-coding, minimax-coding-plan, zai-coding-plan)"
    )]
    BannedProvider {
        agent: String,
        provider: String,
        field: String,
    },

    #[error(
        "mixed project mode: projects are defined but {kind} '{name}' has no project set; every agent and flow must declare a project"
    )]
    MixedProjectMode {
        kind: &'static str,
        name: String,
    },

    #[error("include glob '{pattern}' is invalid: {reason}")]
    InvalidIncludeGlob { pattern: String, reason: String },
}
