# Implementation Plan: PR #1788 Slice 4 Agent Registry

**Status**: Draft
**Research Doc**: `.docs/research-pr-1788-slice4-agent-registry.md`
**Author**: OpenCode
**Date**: 2026-05-31
**Estimated Effort**: 1 focused implementation session for lookup registry; 1 additional session for lifecycle key migration if approved separately

## Overview

### Summary

Implement a project-scoped, read-only `AgentRegistry` for effective ADF agent definitions. The registry will centralise lookups by legacy scope or project id, prevent duplicate agents within a scope, and replace the most direct project-specific `config.agents.iter().find(...)` lookups in PR and push dispatch.

This plan deliberately avoids full runtime lifecycle key migration in the first implementation, because `active_agents` and related maps have broad impact across restart, timeout, stop, logging, and mention flows.

### Approach

Use a minimal typed index over the already merged `OrchestratorConfig`:

- `AgentScope`: `Legacy` or `Project(String)`.
- `AgentKey`: `(AgentScope, name)`.
- `RegisteredAgent`: definition plus source metadata.
- `AgentRegistry`: `BTreeMap<AgentKey, RegisteredAgent>` plus per-scope name index.

Wire it into `AgentOrchestrator::new` and use it for direct PR/push lookup paths. Keep mention resolver and direct-dispatch validation unchanged until follow-up steps, unless tests show the migration is trivial.

### Scope

**In Scope:**

- Add `agent_registry.rs` module.
- Export or crate-expose registry types intentionally.
- Build one `AgentRegistry` during `AgentOrchestrator::new`.
- Add duplicate `(scope, agent name)` detection during registry construction.
- Replace `dispatch_pr_reviewer_for_pr`, `dispatch_build_runner_for_pr`, and push build-runner direct lookups with registry methods.
- Add unit/integration tests for legacy lookup, project lookup, duplicate detection, and project boundary isolation.
- Document that active runtime state remains bare-name keyed until a follow-up migration.

**Out of Scope:**

- TLA/spec files and model checking docs (issue #1924).
- Generated `.terraphim/learnings/*.md` artefacts.
- Hot-reloadable registry.
- Source-file attribution for every agent definition.
- Full `active_agents`/restart/log key migration.
- Rewriting `mention::resolve_mention` in this first implementation unless explicitly approved.

**Avoid At All Cost:**

- Claiming the registry fully solves project-scoped runtime identity while `active_agents` remains bare-keyed.
- Creating a second config loading path inside the registry.
- Bundling TLA/spec work into this runtime implementation.
- Replacing mention/persona resolution without targeted tests.
- Adding speculative metadata fields that are not populated by current config loading.

## Architecture

### Component Diagram

```text
OrchestratorConfig (merged + validated)
        |
        v
AgentRegistry::from_config(&config)
        |
        +--> by_key: BTreeMap<AgentKey, RegisteredAgent>
        |
        +--> by_project: BTreeMap<AgentScope, BTreeSet<String>>
        |
        v
AgentOrchestrator.agent_registry
        |
        +--> PR reviewer lookup
        +--> PR build-runner lookup
        +--> Push build-runner lookup

Existing for now:
        +--> mention::resolve_mention(config.agents)
        +--> DirectDispatchAgentIndex::from_agents(config.agents)
        +--> active_agents: HashMap<String, ManagedAgent>
```

### Data Flow

```text
Startup:
  OrchestratorConfig::load_and_validate
    -> AgentOrchestrator::new(config)
    -> AgentRegistry::from_config(&config)
    -> store registry on orchestrator

PR dispatch:
  ReviewPr webhook
    -> project derived from repo
    -> agents_on_pr_open_for_project(project)
    -> lookup_project(project, entry.name)
    -> spawn selected AgentDefinition

Push dispatch:
  Push webhook
    -> project derived from repo
    -> lookup_project(project, "build-runner")
    -> spawn selected AgentDefinition
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Read-only registry over merged config | Keeps `OrchestratorConfig` as the source of truth | Registry loading TOML/project sources directly. |
| `AgentKey` includes `AgentScope` | Prevents same-name cross-project lookup confusion | Bare-name-only registry. |
| Use `BTreeMap`/`BTreeSet` | Deterministic iteration and stable tests | `HashMap` with arbitrary order. |
| Duplicate detection in `from_config` | Makes invariant local to index construction | Silent overwrite, or relying only on config validation. |
| Lookup-only first PR | Low-risk value while preserving runtime behaviour | Full lifecycle key migration in same PR. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full lifecycle identity migration now | Too many call sites and runtime semantics | Regressions in restarts, timeout cleanup, stop, logs, and mention dedup. |
| Registry owns config merge/load | Duplicates existing responsibilities | Two sources of truth. |
| Public mutable registry | No current caller needs mutation | Accidental divergence from config. |
| Source path attribution | Current effective config does not preserve exact per-agent source file | Speculative fields and misleading metadata. |
| Registry-backed mention resolver now | Mention semantics include project hints, persona resolution, and chain context | Behaviour drift without enough focused tests. |

### Simplicity Check

The simplest useful design is a typed read-only map built once at startup, with explicit lookup methods and tests. It does not need traits, dynamic updates, source metadata, or a service abstraction.

**Senior Engineer Test**: A senior engineer would likely accept a small typed index, but would reject calling it complete while `active_agents` remains keyed by bare name. This plan makes that boundary explicit.

**Nothing Speculative Checklist:**

- [x] No features the user did not request.
- [x] No abstractions for future hot reload.
- [x] No unused source attribution fields beyond a single explicit `ConfigMerged` marker.
- [x] No lifecycle migration hidden inside lookup changes.
- [x] No TLA/spec or generated artefact bundling.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/agent_registry.rs` | Read-only project-scoped registry and tests for registry internals. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Declare module, optionally re-export types, store `agent_registry`, build in `new`, replace direct PR/push lookups. |
| `crates/terraphim_orchestrator/tests/multi_project_tests.rs` | Add integration tests for registry lookup across merged multi-project configs. |

### Deleted Files

None.

## API Design

### Public or Crate-Visible Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentScope {
    Legacy,
    Project(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentKey {
    pub scope: AgentScope,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSource {
    ConfigMerged,
}

#[derive(Debug, Clone)]
pub struct RegisteredAgent {
    pub key: AgentKey,
    pub definition: AgentDefinition,
    pub source: AgentSource,
}

#[derive(Debug, Clone, Default)]
pub struct AgentRegistry {
    by_key: BTreeMap<AgentKey, RegisteredAgent>,
    by_project: BTreeMap<AgentScope, BTreeSet<String>>,
}
```

### Functions and Methods

```rust
impl AgentScope {
    pub fn from_project(project: Option<&str>) -> Self;
    pub fn label(&self) -> &str;
}

impl AgentKey {
    pub fn new(scope: AgentScope, name: impl Into<String>) -> Self;
    pub fn project(project: impl Into<String>, name: impl Into<String>) -> Self;
    pub fn legacy(name: impl Into<String>) -> Self;
}

impl RegisteredAgent {
    pub fn project_id(&self) -> Option<&str>;
    pub fn event_only(&self) -> bool;
}

impl AgentRegistry {
    pub fn from_config(config: &OrchestratorConfig) -> Result<Self, OrchestratorError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, key: &AgentKey) -> Option<&RegisteredAgent>;
    pub fn lookup_project(&self, project: &str, name: &str) -> Option<&RegisteredAgent>;
    pub fn lookup_legacy(&self, name: &str) -> Option<&RegisteredAgent>;
    pub fn lookup(&self, project: Option<&str>, name: &str) -> Option<&RegisteredAgent>;
    pub fn names_for_scope(&self, scope: &AgentScope) -> Vec<&str>;
}
```

### Error Handling

Use existing `OrchestratorError::Config(String)` for duplicate registry keys in the first implementation:

```rust
return Err(OrchestratorError::Config(format!(
    "duplicate agent '{}' in project '{}'",
    agent.name,
    scope.label()
)));
```

A dedicated `DuplicateAgent` variant is not necessary unless callers need structured matching later.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `registry_builds_legacy_agents` | `agent_registry.rs` | Legacy `project = None` agents can be looked up by name. |
| `registry_builds_project_agents` | `agent_registry.rs` | Project-scoped agents can be looked up by `(project, name)`. |
| `registry_allows_same_name_across_projects` | `agent_registry.rs` | Same name in `alpha` and `beta` maps to distinct definitions. |
| `registry_rejects_duplicate_agent_in_same_scope` | `agent_registry.rs` | Duplicate `(scope, name)` returns `OrchestratorError::Config`. |
| `names_for_scope_returns_sorted_names` | `agent_registry.rs` | Deterministic names list for direct dispatch/parser construction. |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `registry_indexes_merged_project_agents` | `tests/multi_project_tests.rs` | Registry indexes agents loaded from merged multi-project config fragments. |
| `registry_lookup_does_not_cross_project_boundaries` | `tests/multi_project_tests.rs` | `alpha/build-runner` and `beta/build-runner` resolve to different tasks. |
| `review_pr_uses_project_scoped_registry_lookup` | `src/lib.rs` tests or existing PR dispatch tests | Missing build-runner in one project does not accidentally use another project's build-runner. |

### Regression Tests to Avoid in This PR

Do not add TLA validation tests in this runtime slice. TLA/spec validation belongs to issue #1924.

## Implementation Steps

### Step 1: Add Registry Module

**Files:** `crates/terraphim_orchestrator/src/agent_registry.rs`

**Description:** Define `AgentScope`, `AgentKey`, `AgentSource`, `RegisteredAgent`, and `AgentRegistry` with read-only lookup methods.

**Tests:** Add pure unit tests for legacy lookup, project lookup, duplicate rejection, and sorted names.

**Estimated:** 1.5 hours.

### Step 2: Wire Registry into Orchestrator Construction

**Files:** `crates/terraphim_orchestrator/src/lib.rs`

**Description:** Add `pub mod agent_registry;`, expose types if needed, add `agent_registry: AgentRegistry` to `AgentOrchestrator`, and build it in `AgentOrchestrator::new` after config-derived components are initialised.

**Tests:** Existing orchestrator construction tests should continue passing; duplicate registry tests should fail through `AgentOrchestrator::new` if added.

**Dependencies:** Step 1.

**Estimated:** 45 minutes.

### Step 3: Migrate Direct PR/Push Lookups

**Files:** `crates/terraphim_orchestrator/src/lib.rs`

**Description:** Replace direct scans in these functions:

- `dispatch_pr_reviewer_for_pr`
- `dispatch_build_runner_for_pr`
- push build-runner dispatch helper around current line 3158

Use:

```rust
let def = match self.agent_registry.lookup_project(project.as_str(), agent_name) {
    Some(agent) => agent.definition.clone(),
    None => { /* existing skip warning */ }
};
```

**Tests:** Existing PR dispatch tests plus a targeted test where another project has the requested agent name but the current project does not.

**Dependencies:** Step 2.

**Estimated:** 1 hour.

### Step 4: Add Multi-Project Integration Tests

**Files:** `crates/terraphim_orchestrator/tests/multi_project_tests.rs`

**Description:** Add tests using merged multi-project ADF configs with same agent names and different tasks.

**Tests:** New integration tests as listed above.

**Dependencies:** Step 3.

**Estimated:** 1 hour.

### Step 5: Verification

**Files:** Changed Rust files.

**Description:** Run focused and crate-level verification.

**Commands:**

```bash
cargo test -p terraphim_orchestrator agent_registry
cargo test -p terraphim_orchestrator --test multi_project_tests
cargo test -p terraphim_orchestrator --lib
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo fmt --all -- --check
ubs --diff --only=rust .
```

**Dependencies:** Steps 1-4.

**Estimated:** 1 hour.

## Rollback Plan

If implementation creates regressions:

1. Revert the registry wiring commit.
2. Restore direct `config.agents.iter().find(...)` lookups in PR/push paths.
3. Keep the research/design docs and file a narrower follow-up issue if the module itself is sound but wiring is problematic.

No data migration is required; the registry is derived from config at runtime.

## Migration

No persisted data or database schema changes.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Registry build | O(n log n) at startup | Covered by construction tests; no benchmark needed for small config. |
| Lookup | O(log n) deterministic | Unit tests and code review. |
| Memory | One clone of each `AgentDefinition` | Acceptable; current dispatch code already clones definitions. |

### Benchmarks to Add

None for first implementation. Agent counts are small enough that correctness matters more than micro-performance.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide whether `AgentRegistry` types should be public exports or crate-private | Pending | Human maintainer |
| Decide whether lifecycle key migration should be the next PR | Pending | Human maintainer |
| Decide whether duplicate `(scope, name)` should also be enforced in `OrchestratorConfig::validate` | Pending | Human maintainer |

## Follow-Up Design: Scoped Runtime Lifecycle Keys

This is intentionally not part of the first registry PR, but it is the next correctness boundary.

Potential follow-up changes:

- Change `active_agents: HashMap<String, ManagedAgent>` to `HashMap<AgentKey, ManagedAgent>` or introduce `RuntimeAgentKey`.
- Store `ManagedAgent.key` alongside `definition`.
- Update `should_skip_dispatch`, restart counters, cooldowns, timeout paths, output logs, and stop APIs.
- Preserve legacy CLI/API inputs by resolving bare names to one active key only when unambiguous.

This follow-up should have its own research/design or an appended Phase 2.5 specification interview because it affects external behaviour.

## Approval

- [ ] Technical review complete.
- [ ] Test strategy approved.
- [ ] Public vs crate-private registry visibility decided.
- [ ] Lifecycle-key follow-up explicitly accepted or deferred.
- [ ] Human approval received before implementation.
