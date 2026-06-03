use terraphim_router::RoutingError;
use terraphim_spawner::SpawnerError;

/// Errors that can occur during orchestrator operation.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    /// A configuration value is invalid or missing.
    #[error("configuration error: {0}")]
    Config(String),

    /// An agent failed to spawn.
    #[error("agent spawn failed for '{agent}': {reason}")]
    SpawnFailed {
        /// Name of the agent that failed to spawn.
        agent: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// Creating a git worktree for an agent failed.
    #[error("agent worktree creation failed for '{agent}' in '{repo}': {reason}")]
    WorktreeCreationFailed {
        /// Name of the agent whose worktree could not be created.
        agent: String,
        /// Repository path where the worktree should have been created.
        repo: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// The requested agent does not exist in the registry.
    #[error("agent '{0}' not found")]
    AgentNotFound(String),

    /// The scheduler encountered an error.
    #[error("scheduler error: {0}")]
    SchedulerError(String),

    /// A compound (multi-agent) review operation failed.
    #[error("compound review failed: {0}")]
    CompoundReviewFailed(String),

    /// The provided agent name contains invalid characters.
    #[error(
        "invalid agent name '{0}': must contain only alphanumeric, dash, or underscore characters"
    )]
    InvalidAgentName(String),

    /// An agent handoff (delegation of control) failed.
    #[error("handoff failed from '{from}' to '{to}': {reason}")]
    HandoffFailed {
        /// Agent that initiated the handoff.
        from: String,
        /// Agent that was the intended recipient.
        to: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// A spawner-level error propagated up from the spawner crate.
    #[error(transparent)]
    Spawner(#[from] SpawnerError),

    /// A routing-level error propagated up from the router crate.
    #[error(transparent)]
    Routing(#[from] RoutingError),

    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Pre-flight configuration validation failed for an agent.
    #[error("pre-check configuration error for agent '{agent}': {reason}")]
    PreCheckConfig {
        /// Name of the agent whose pre-check failed.
        agent: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// A flow step or the entire flow failed.
    #[error("flow '{flow_name}' failed: {reason}")]
    FlowFailed {
        /// Name of the flow that failed.
        flow_name: String,
        /// Human-readable reason for the failure.
        reason: String,
    },

    /// A flow gate step rejected the PR or payload.
    #[error("flow '{flow_name}' gate '{step_name}' rejected: {condition}")]
    FlowGateRejected {
        /// Name of the flow containing the gate.
        flow_name: String,
        /// Name of the gate step that rejected.
        step_name: String,
        /// The condition or rule that caused the rejection.
        condition: String,
    },

    /// A flow template could not be expanded or rendered.
    #[error("flow template error: {0}")]
    FlowTemplateError(String),

    /// Two projects share the same identifier, which must be unique.
    #[error(
        "duplicate project id '{0}' (project ids must be unique across base + included configs)"
    )]
    DuplicateProjectId(String),

    /// An agent references a project that is not declared in the projects list.
    #[error(
        "agent '{agent}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownAgentProject {
        /// Name of the agent with the unknown project reference.
        agent: String,
        /// The project identifier that could not be resolved.
        project: String,
    },

    /// A flow references a project that is not declared in the projects list.
    #[error(
        "flow '{flow}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownFlowProject {
        /// Name of the flow with the unknown project reference.
        flow: String,
        /// The project identifier that could not be resolved.
        project: String,
    },

    /// An agent or flow specifies a disallowed LLM provider.
    #[error(
        "banned LLM provider '{provider}' in {field} for agent '{agent}' (allowed: claude-code, opencode-go, kimi-for-coding, minimax-coding-plan, zai-coding-plan)"
    )]
    BannedProvider {
        /// Name of the agent or flow with the banned provider.
        agent: String,
        /// The banned provider identifier.
        provider: String,
        /// The config field (e.g. `model`) that contains the banned provider.
        field: String,
    },

    /// Some agents or flows lack a project assignment in mixed-mode configs.
    #[error(
        "mixed project mode: projects are defined but {kind} '{name}' has no project set; every agent and flow must declare a project"
    )]
    MixedProjectMode {
        /// Whether the offending item is an `"agent"` or a `"flow"`.
        kind: &'static str,
        /// Name of the agent or flow missing a project assignment.
        name: String,
    },

    /// An include glob pattern is syntactically invalid.
    #[error("include glob '{pattern}' is invalid: {reason}")]
    InvalidIncludeGlob {
        /// The glob pattern that could not be parsed.
        pattern: String,
        /// Human-readable reason the pattern is invalid.
        reason: String,
    },

    /// A numeric agent field is outside the allowed range.
    #[error("agent '{agent}' {field} value {value}s is outside allowed range [{min}s, {max}s]")]
    AgentFieldOutOfRange {
        /// Name of the agent with the out-of-range field.
        agent: String,
        /// Name of the field that is out of range.
        field: String,
        /// The provided value (in seconds).
        value: u64,
        /// Minimum allowed value (in seconds).
        min: u64,
        /// Maximum allowed value (in seconds).
        max: u64,
    },

    /// The nightwatch probe TTL is too short for rate-limit protection.
    #[error("nightwatch probe_ttl_secs {value}s is below minimum {min}s (rate-limit protection)")]
    ProbeTtlTooShort {
        /// The provided TTL value (in seconds).
        value: u64,
        /// The minimum acceptable TTL (in seconds).
        min: u64,
    },
}
