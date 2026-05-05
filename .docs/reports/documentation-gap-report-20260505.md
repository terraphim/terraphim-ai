# Documentation Gap Report

**Generated**: 2026-05-05 05:43 CEST
**Agent**: documentation-generator (Ferrox)
**Issue**: #1183

## Summary

| Metric | Count |
|--------|-------|
| Crates scanned | 10 |
| Total missing doc comments | 306 |
| Most affected crate | terraphim_agent (126) |
| Least affected crate | terraphim_config, terraphim_service (0) |

## Per-Crate Breakdown

| Crate | Missing Items | Severity |
|-------|--------------|----------|
| terraphim_agent | 126 | High |
| terraphim_orchestrator | 87 | High |
| terraphim_middleware | 35 | Medium |
| terraphim_router | 17 | Medium |
| terraphim_persistence | 16 | Medium |
| terraphim_types | 14 | Low |
| terraphim_rolegraph | 10 | Low |
| haystack_core | 1 | Low |
| terraphim_config | 0 | Clean |
| terraphim_service | 0 | Clean |

## High-Impact Targets

### terraphim_agent (126 items)
**Priority**: Critical -- this is the primary user-facing crate.

Key undocumented items:
- `OutputFormat` enum and all robot output structs (`SourcesOutput`, `SessionListOutput`, etc.)
- `ApiClient` and all DTOs (`SearchResponse`, `ConfigResponse`, `ChatMessage`, etc.)
- `ReplHandler` constructors (`new_offline`, `new_server`)
- `TuiService` public interface
- All command enums (`ReplCommand`, `RobotSubcommand`, `RoleSubcommand`, etc.)
- `ForgivingParser` and `AliasRegistry`
- `BudgetEngine` and `BudgetedResults`
- `SharedLearningStore` and scoring functions

### terraphim_orchestrator (87 items)
**Priority**: High -- core ADF infrastructure.

Key undocumented items:
- All submodule re-exports (35+ `pub mod` declarations)
- `GiteaConnection`, `ListenerConfig`, `DelegationPolicy`
- `FlowExecutor`, `FlowDefinition`, `FlowStepDef`
- `RoutingDecisionEngine`, `DispatchContext`, `RouteCandidate`
- `TelemetryStore` and `TelemetrySummary`
- `Nightwatch` alert system
- `MentionChain` and `OutputPoster`

### terraphim_middleware (35 items)
**Priority**: Medium.

Key undocumented items:
- All haystack indexer re-exports (`AiAssistantHaystackIndexer`, `ClickUpHaystackIndexer`, etc.)
- `RipgrepCommand` and `Message` enum
- `Error` enum and `Result` type alias

## API Reference Snippets

### terraphim_agent::robot

```rust
/// Output format for robot mode machine-readable responses.
pub enum OutputFormat {
    Json,
    // ...
}

/// Token budget engine for controlling robot output size.
pub struct BudgetEngine {
    // ...
}

/// Applies token budget constraints to search results.
pub fn apply(&self, results: &[SearchResultItem]) -> Result<BudgetedResults, BudgetError>;
```

### terraphim_orchestrator::flow

```rust
/// Executor for multi-step agent workflows.
pub struct FlowExecutor {
    // ...
}

/// Definition of a workflow with typed steps.
pub struct FlowDefinition {
    // ...
}
```

## CHANGELOG Update

Updated [Unreleased] section with:
- Streaming output log drain (Refs #1219)
- Agent output persistence to per-run log files
- GITEA_URL injection from project config
- ADF fleet DEGRADED alert fix (#1233)
- CI pipeline degradation fixes
- Service-dependent test ignore markers

## Recommendations

1. **Priority 1**: Document `terraphim_agent` public API -- this is the primary CLI/SDK interface
2. **Priority 2**: Document `terraphim_orchestrator` module structure and key types
3. **Priority 3**: Add module-level docs (`//!`) to all `lib.rs` files in affected crates
4. **Enable `#![warn(missing_docs)]`** in CI to prevent regression

Theme-ID: doc-gap
