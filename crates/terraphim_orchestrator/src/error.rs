use terraphim_router::RoutingError;
use terraphim_spawner::SpawnerError;

/// Errors that can occur during orchestrator operation.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    /// A configuration value is invalid or missing.
    #[error("configuration error: {0}")]
    Config(String),

    /// An agent process failed to start.
    #[error("agent spawn failed for '{agent}': {reason}")]
    SpawnFailed {
        /// Name of the agent that failed to spawn.
        agent: String,
        /// Human-readable description of the failure.
        reason: String,
    },

    /// A git worktree could not be created for an agent.
    #[error("agent worktree creation failed for '{agent}' in '{repo}': {reason}")]
    WorktreeCreationFailed {
        /// Name of the agent whose worktree creation failed.
        agent: String,
        /// Path or URL of the repository in which the worktree was being created.
        repo: String,
        /// Human-readable description of the failure.
        reason: String,
    },

    /// An operation referenced an agent that does not exist in the registry.
    #[error("agent '{0}' not found")]
    AgentNotFound(String),

    /// The internal task scheduler encountered an error.
    #[error("scheduler error: {0}")]
    SchedulerError(String),

    /// A compound (multi-reviewer) review pipeline failed.
    #[error("compound review failed: {0}")]
    CompoundReviewFailed(String),

    /// The supplied agent name contains characters that are not allowed.
    #[error(
        "invalid agent name '{0}': must contain only alphanumeric, dash, or underscore characters"
    )]
    InvalidAgentName(String),

    /// An agent handoff (work transfer between agents) could not be completed.
    #[error("handoff failed from '{from}' to '{to}': {reason}")]
    HandoffFailed {
        /// Name of the agent that is handing off work.
        from: String,
        /// Name of the agent that should receive the work.
        to: String,
        /// Human-readable description of why the handoff failed.
        reason: String,
    },

    /// A [`SpawnerError`] was returned by the process-spawning layer.
    #[error(transparent)]
    Spawner(#[from] SpawnerError),

    /// A [`RoutingError`] was returned by the routing layer.
    #[error(transparent)]
    Routing(#[from] RoutingError),

    /// An I/O error occurred at the operating-system level.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An agent's pre-execution check was misconfigured.
    #[error("pre-check configuration error for agent '{agent}': {reason}")]
    PreCheckConfig {
        /// Name of the agent with the invalid pre-check configuration.
        agent: String,
        /// Human-readable description of the configuration problem.
        reason: String,
    },

    /// A named flow failed during execution.
    #[error("flow '{flow_name}' failed: {reason}")]
    FlowFailed {
        /// Name of the flow that failed.
        flow_name: String,
        /// Human-readable description of the failure.
        reason: String,
    },

    /// A gate step within a flow rejected execution.
    #[error("flow '{flow_name}' gate '{step_name}' rejected: {condition}")]
    FlowGateRejected {
        /// Name of the flow containing the rejected gate.
        flow_name: String,
        /// Name of the gate step that rejected execution.
        step_name: String,
        /// The condition expression that caused the rejection.
        condition: String,
    },

    /// A flow template could not be parsed or instantiated.
    #[error("flow template error: {0}")]
    FlowTemplateError(String),

    /// Two projects share the same identifier, which must be unique.
    #[error(
        "duplicate project id '{0}' (project ids must be unique across base + included configs)"
    )]
    DuplicateProjectId(String),

    /// An agent configuration references a project id that does not exist.
    #[error(
        "agent '{agent}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownAgentProject {
        /// Name of the agent with the invalid project reference.
        agent: String,
        /// The project id that could not be resolved.
        project: String,
    },

    /// A flow configuration references a project id that does not exist.
    #[error(
        "flow '{flow}' references unknown project '{project}' (must match a Project.id in projects list)"
    )]
    UnknownFlowProject {
        /// Name of the flow with the invalid project reference.
        flow: String,
        /// The project id that could not be resolved.
        project: String,
    },

    /// An agent or flow specifies an LLM provider that is not on the allow-list.
    #[error(
        "banned LLM provider '{provider}' in {field} for agent '{agent}' (allowed: claude-code, opencode-go, kimi-for-coding, minimax-coding-plan, zai-coding-plan)"
    )]
    BannedProvider {
        /// Name of the agent whose configuration contains the banned provider.
        agent: String,
        /// The banned provider identifier that was supplied.
        provider: String,
        /// The configuration field in which the banned provider appeared.
        field: String,
    },

    /// Projects are declared but an agent or flow has no project assigned.
    #[error(
        "mixed project mode: projects are defined but {kind} '{name}' has no project set; every agent and flow must declare a project"
    )]
    MixedProjectMode {
        /// Whether the offending item is an `"agent"` or a `"flow"`.
        kind: &'static str,
        /// Name of the agent or flow that is missing a project assignment.
        name: String,
    },

    /// A glob pattern used in an `include` directive is syntactically invalid.
    #[error("include glob '{pattern}' is invalid: {reason}")]
    InvalidIncludeGlob {
        /// The glob pattern string that failed to parse.
        pattern: String,
        /// Human-readable description of the parse error.
        reason: String,
    },

    /// A numeric field on an agent config is outside its permitted range.
    #[error("agent '{agent}' {field} value {value}s is outside allowed range [{min}s, {max}s]")]
    AgentFieldOutOfRange {
        /// Name of the agent with the out-of-range field.
        agent: String,
        /// Name of the field that is out of range.
        field: String,
        /// The value that was supplied (in seconds).
        value: u64,
        /// The minimum permitted value (in seconds).
        min: u64,
        /// The maximum permitted value (in seconds).
        max: u64,
    },

    /// The `nightwatch` probe TTL is shorter than the rate-limit protection floor.
    #[error("nightwatch probe_ttl_secs {value}s is below minimum {min}s (rate-limit protection)")]
    ProbeTtlTooShort {
        /// The probe TTL value that was supplied (in seconds).
        value: u64,
        /// The minimum allowed probe TTL (in seconds).
        min: u64,
    },
}
