# API Reference Snippets

**Generated:** 2026-05-06

## terraphim_agent

### Core Types

```rust
/// Unique identity for an agent within the system.
pub struct AgentIdentity {
    pub agent_name: String,
    pub gitea_login: Option<String>,
}
```

### Key Functions

```rust
/// Creates a new agent identity with the given name.
pub fn new(agent_name: impl Into<String>) -> Self;

/// Returns the resolved Gitea login, falling back to agent name.
pub fn resolved_gitea_login(&self) -> &str;
```

## terraphim_orchestrator

### PR Dispatch

```rust
/// Request to dispatch an agent for PR review.
pub struct ReviewPrRequest {
    pub pr_number: u64,
    pub repo_owner: String,
    pub repo_name: String,
}
```

### Project Control

```rust
/// Determines whether a project should pause processing.
pub enum ShouldPause {
    Yes { reason: String },
    No,
}

/// Tracks consecutive failures for circuit-breaker logic.
pub struct ProjectFailureCounter {
    pub count: u32,
    pub last_failure: Option<Instant>,
}
```

## terraphim_types

### Medical Types

```rust
/// Classification of nodes in the medical knowledge graph.
pub enum MedicalNodeType {
    Disease,
    Symptom,
    Treatment,
    Gene,
    Drug,
}

/// Types of relationships between medical nodes.
pub enum MedicalEdgeType {
    Treats,
    Causes,
    Indicates,
    Contraindicated,
}
```

## terraphim_service

### OpenRouter Integration

```rust
/// Errors from the OpenRouter service.
pub enum OpenRouterError {
    ApiError(String),
    RateLimited,
    InvalidResponse,
}

/// Service for routing LLM requests through OpenRouter.
pub struct OpenRouterService {
    // ...
}
```

---

*This is a partial reference. Full documentation requires addressing 3,010 documentation gaps across the workspace.*
