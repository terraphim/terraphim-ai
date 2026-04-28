# Documentation Gap Report

**Generated:** 2026-04-28
**Agent:** documentation-generator (Ferrox)
**Tool:** `cargo check -Wmissing_docs`

## Executive Summary

Documentation coverage across the Terraphim workspace is below acceptable thresholds. Key public crates have extensive gaps in module-level, struct-level, and function-level documentation. This report quantifies the gaps and recommends priority targets for remediation.

## Missing Documentation by Crate

| Crate | Missing Items | Priority | Notes |
|-------|--------------|----------|-------|
| terraphim_orchestrator | 431 | **Critical** | New crate (v1.8.0); core ADF framework; zero module docs |
| terraphim_server | 139 | **High** | Public API surface (Axum handlers, middleware) |
| terraphim_agent | 102 | **High** | CLI entry point and REPL commands |
| terraphim_service | 114 | **High** | Business logic layer |
| terraphim_types | 79 | **High** | Core type definitions; affects all downstream crates |
| terraphim_config | 39 | **Medium** | Configuration structs |
| terraphim_persistence | 30 | **Medium** | Storage abstractions |
| terraphim_rolegraph | 22 | **Medium** | Role graph types |

**Total identified gaps: ~956 items**

## Priority Remediation Targets

### 1. terraphim_orchestrator (431 gaps)
**Rationale:** This is a newly introduced crate (v1.8.0) forming the core of the Autonomous Development Flow framework. It has zero module-level documentation and minimal item-level docs.

**Key files requiring attention:**
- `src/lib.rs` -- missing module-level documentation
- `src/agents/` -- agent templates undocumented
- `src/workflows/` -- workflow definitions undocumented
- `src/dispatch/` -- PR/push dispatch logic undocumented

### 2. terraphim_server (139 gaps)
**Rationale:** Public HTTP API surface. Undocumented endpoints and error types reduce API discoverability.

**Key files requiring attention:**
- `src/api.rs` -- Axum handlers and request/response types
- `src/error.rs` -- Error variants and conversion functions
- `src/workflows/mod.rs` -- Workflow module documentation

### 3. terraphim_agent (102 gaps)
**Rationale:** CLI entry point. REPL commands and robot subcommand lack documentation.

**Key files requiring attention:**
- `src/repl/commands.rs` -- REPL command variants and arguments
- `src/repl/handler.rs` -- Command handlers
- `src/robot/` -- Self-documentation API and budget types
- `src/main.rs` -- CLI argument structs

### 4. terraphim_service (114 gaps)
**Rationale:** Business logic layer bridging API and persistence.

### 5. terraphim_types (79 gaps)
**Rationale:** Core types used across the entire workspace. Missing docs propagate to all dependent crates.

**Key files requiring attention:**
- `src/lib.rs` -- Core types (Entity, Role, Profile, etc.)
- `src/review.rs` -- Review-related structs

## API Reference Snippets

### terraphim_agent::robot::Self-Documentation API

```rust
/// Exposes runtime self-documentation for agent introspection.
pub async fn robot_query(
    query: String,
    role: Option<String>,
) -> Result<ResponseMeta, AgentError>;
```

### terraphim_server::api

```rust
/// Main Axum router composing all API routes.
pub fn api_router() -> Router<AppState>;

/// Health check endpoint.
/// Returns 200 OK when the server is operational.
async fn health_check(State(state): State<AppState>) -> impl IntoResponse;
```

### terraphim_types::core

```rust
/// Unique identifier for a Terraphim entity.
pub type Id = String;

/// Core entity representing a node in the knowledge graph.
pub struct Entity {
    pub id: Id,
    pub label: String,
    pub properties: HashMap<String, Value>,
}
```

## Recommendations

1. **Immediate:** Add `#![warn(missing_docs)]` to all crate roots to prevent regression
2. **Short-term:** Target terraphim_orchestrator first (431 gaps) -- it is the newest crate with the most gaps
3. **Medium-term:** Document terraphim_server public API endpoints and terraphim_types core structs
4. **Long-term:** Enforce `#![deny(missing_docs)]` on release builds after gap closure

## Verification

Run the following to reproduce this report:

```bash
RUSTFLAGS="-Wmissing_docs" cargo check -p terraphim_orchestrator 2>&1 | grep "missing documentation" | wc -l
RUSTFLAGS="-Wmissing_docs" cargo check -p terraphim_server 2>&1 | grep "missing documentation" | wc -l
RUSTFLAGS="-Wmissing_docs" cargo check -p terraphim_agent 2>&1 | grep "missing documentation" | wc -l
RUSTFLAGS="-Wmissing_docs" cargo check -p terraphim_types 2>&1 | grep "missing documentation" | wc -l
```

---

Theme-ID: doc-gap
