use terraphim_router::RoutingError;
use terraphim_spawner::SpawnerError;

/// Render every accepted `model` / `fallback_model` form for the
/// `BannedProvider` operator guidance.
///
/// Derived from the gate's source constants -- [`config::ALLOWED_PROVIDER_PREFIXES`]
/// plus the Anthropic accepted forms ([`config::ANTHROPIC_BARE_PROVIDERS`],
/// valid both bare and as `anthropic/...`) and the claude-code CLI bare
/// models ([`config::CLAUDE_CLI_BARE_MODELS`]) -- so the rendered guidance
/// can never drift from what the gate actually accepts. Every listed token
/// is an accepted value for `model` / `fallback_model`.
fn allowed_provider_guidance() -> String {
    let mut forms: Vec<&str> = Vec::with_capacity(
        crate::config::ALLOWED_PROVIDER_PREFIXES.len()
            + crate::config::ANTHROPIC_BARE_PROVIDERS.len()
            + crate::config::CLAUDE_CLI_BARE_MODELS.len(),
    );
    forms.extend_from_slice(crate::config::ALLOWED_PROVIDER_PREFIXES);
    forms.extend_from_slice(crate::config::ANTHROPIC_BARE_PROVIDERS);
    forms.extend_from_slice(crate::config::CLAUDE_CLI_BARE_MODELS);
    forms.join(", ")
}

/// Errors that can occur during orchestrator operation.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("agent spawn failed for '{agent}': {reason}")]
    SpawnFailed { agent: String, reason: String },

    #[error("agent worktree creation failed for '{agent}' in '{repo}': {reason}")]
    WorktreeCreationFailed {
        agent: String,
        repo: String,
        reason: String,
    },

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

    #[error(
        "duplicate project id '{0}' (project ids must be unique across base + included configs)"
    )]
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
        "banned LLM provider '{provider}' in {field} for agent '{agent}' (allowed: {})",
        allowed_provider_guidance()
    )]
    // The guidance is rendered from the gate's source constants
    // (`ALLOWED_PROVIDER_PREFIXES` + `ANTHROPIC_BARE_PROVIDERS` +
    // `CLAUDE_CLI_BARE_MODELS`) at format time, so it cannot drift from the
    // allow-list; `provider_gate_tests::banned_provider_error_guidance_lists_every_allowed_prefix`
    // fails the build if an accepted form ever goes missing from the message.
    BannedProvider {
        agent: String,
        provider: String,
        field: String,
    },

    #[error(
        "mixed project mode: projects are defined but {kind} '{name}' has no project set; every agent and flow must declare a project"
    )]
    MixedProjectMode { kind: &'static str, name: String },

    #[error("include glob '{pattern}' is invalid: {reason}")]
    InvalidIncludeGlob { pattern: String, reason: String },

    #[error("agent '{agent}' {field} value {value}s is outside allowed range [{min}s, {max}s]")]
    AgentFieldOutOfRange {
        agent: String,
        field: String,
        value: u64,
        min: u64,
        max: u64,
    },

    #[error("nightwatch probe_ttl_secs {value}s is below minimum {min}s (rate-limit protection)")]
    ProbeTtlTooShort { value: u64, min: u64 },

    /// Issue #3293: the PR gate dispatch contract is fail-closed. When the
    /// orchestrator cannot assemble the authoritative evidence pack for a
    /// canonical PR gate agent, the dispatch is rejected — no
    /// degraded evidence substitution, no spawn.
    #[error(
        "PR gate evidence unavailable for {project}#{pr_number} (agent={agent}, head={head_sha}): {reason}"
    )]
    PrGateEvidenceUnavailable {
        project: String,
        agent: String,
        pr_number: u64,
        head_sha: String,
        reason: String,
    },
}
