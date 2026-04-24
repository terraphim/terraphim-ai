# Implementation Plan: Wire Learning System into ADF Orchestrator

**Status**: Draft
**Research Doc**: `.docs/research-learning-adf-integration-2026-04-23.md`
**Date**: 2026-04-23
**Estimated Effort**: 4-6 hours (single PR, ~350-450 LOC)

## Overview

### Summary

Wire `terraphim_orchestrator::learning::SharedLearningStore` into `AgentOrchestrator` so that agents receive prior learnings in their prompt at spawn time and exit outcomes flow back as learning validation evidence.

### Approach

Use the existing `learning.rs` module (already in the orchestrator crate) with its `SharedLearningStore` and `LearningPersistence` trait. Add a single `Option<SharedLearningStore>` field to `AgentOrchestrator`. Inject learnings at the existing skill-chain injection point in `spawn_agent()`. Record outcomes at the existing exit-classification point in `poll_agent_exits()`.

### Scope

**In Scope:**
- Instantiate `SharedLearningStore` in `AgentOrchestrator` (behind config toggle)
- Inject learning context into agent prompts at spawn
- Record exit outcomes as learning validation evidence
- Periodic stale learning archival during reconciliation tick
- Config fields: `learning_enabled`, `learning_min_trust`, `learning_max_tokens`, `learning_max_entries`
- Unit tests for all new code

**Out of Scope:**
- `terraphim_agent_evolution` wiring (#578 full scope)
- Cross-agent injection via `LearningInjector`
- Nightly failure clustering (#279)
- JSONL import pipeline
- Gitea wiki sync from orchestrator

**Avoid At All Cost:**
- Adding `terraphim_agent_evolution` as a dependency of orchestrator (two competing learning systems)
- Importing `terraphim_agent::shared_learning` from orchestrator (circular dep risk)
- Feature-gating the wiring (learning.rs is always compiled, no new feature flag needed)
- Building a new learning system -- reuse what exists

## Architecture

### Component Diagram

```
AgentOrchestrator
  +-- Option<SharedLearningStore>
        |-- LearningPersistence (trait)
        |     +-- InMemoryLearningPersistence (tests)
        |     +-- DeviceStorageLearningPersistence (production)
        |
        +-- spawn_agent() hook:
        |     store.query_relevant(agent_name)
        |     -> render as "## Prior Lessons" section
        |     -> append to composed_task
        |
        +-- poll_agent_exits() hook:
        |     match exit_class {
        |       Success => store.record_effective(id)
        |       Failure => store.record_applied(id)  // applied but not effective
        |       _ => {} // skip
        |     }
        |
        +-- reconcile_tick() hook:
              every N ticks: store.archive_stale(30)
```

### Data Flow

```
Spawn Path:
  OrchestratorConfig.learning_enabled = true
      -> SharedLearningStore::new(DeviceStorageLearningPersistence)
      -> spawn_agent(def):
           store.query_relevant(def.name)
           -> Vec<Learning> (top 10, min_trust L1)
           -> render_lessons_section(learnings, max_tokens=1500)
           -> append to composed_task

Exit Path:
  poll_agent_exits():
      classify exit -> AgentRunRecord { exit_class, ... }
      if learning_store.is_some():
          for each learning that was injected:
              match exit_class {
                  Success | EmptySuccess => record_effective(id)
                  Failure | Timeout | RateLimit => record_applied(id)
                  _ => {}
              }

Tick Path:
  reconcile_tick():
      if tick_count % 100 == 0:
          store.archive_stale(30)  // archive L0 learnings older than 30 days
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use `learning.rs` not `terraphim_agent_evolution` | No new deps, types designed for orchestrator, already in crate | `LessonsEvolution` wiring -- heavier, adds crate dep |
| Inline prompt injection not file-based | Follows skill chain pattern, no filesystem dependency | `write_context_file()` to `/tmp/` -- extra I/O, cleanup burden |
| `Option<SharedLearningStore>` not bare | Allows graceful degradation when DeviceStorage init fails or feature disabled | Bare field -- requires always-on DeviceStorage |
| Track injected learning IDs per agent | Needed to record_effective on the right learnings at exit | No tracking -- can't close the feedback loop |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| `LessonsEvolution` dependency | Not in vital few; `learning.rs` covers same ground | Two competing systems, confusion, maintenance burden |
| File-based context (`/tmp/adf-context-*.md`) | Over-engineering for inline use case | Filesystem cleanup, race conditions, debugging complexity |
| JSONL import in orchestrator loop | Speculative -- no flow-state-parser.py pipeline exists | Complex parsing, error handling for uncertain benefit |
| Gitea wiki sync from orchestrator | Already works from CLI; orchestrator doesn't need to duplicate | API token management, rate limiting, failure modes |

### Simplicity Check

> What if this could be easy?

The simplest design: add one field, call two methods at two existing hook points. No new crates, no new traits, no new files. The `learning.rs` module already provides everything needed.

**Senior Engineer Test**: This is 3 changes -- a field, a prompt append, an exit callback. Not overcomplicated.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/config.rs` | Add `LearningConfig` struct and field to `OrchestratorConfig` |
| `crates/terraphim_orchestrator/src/lib.rs` | Add `learning_store` field to `AgentOrchestrator`; inject at spawn; record at exit; archive on tick |
| `crates/terraphim_orchestrator/src/learning.rs` | Add `render_lessons_section()` helper method |

### New Files

None.

### Deleted Files

None.

## API Design

### Config Types (in `config.rs`)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LearningConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_trust")]
    pub min_trust: String, // "L0"|"L1"|"L2"|"L3"
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize, // default 1500
    #[serde(default = "default_max_entries")]
    pub max_entries: usize, // default 10
    #[serde(default = "default_archive_days")]
    pub archive_days: u32, // default 30
    #[serde(default = "default_consolidation_ticks")]
    pub consolidation_ticks: u64, // default 100
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: false, // opt-in
            min_trust: "L1".to_string(),
            max_tokens: 1500,
            max_entries: 10,
            archive_days: 30,
            consolidation_ticks: 100,
        }
    }
}
```

### Orchestrator Field (in `lib.rs`)

```rust
pub struct AgentOrchestrator {
    // ... existing fields ...
    learning_store: Option<learning::SharedLearningStore>,
    learning_config: LearningConfig,
    injected_learning_ids: HashMap<String, Vec<String>>, // agent_name -> [learning_id]
}
```

### Helper Methods

```rust
impl AgentOrchestrator {
    fn init_learning_store(&mut self) {
        // Called from new() or from_config_file()
        // If learning_config.enabled, create SharedLearningStore with DeviceStorage
        // On failure: log warning, leave as None (graceful degradation)
    }

    fn render_lessons_section(&self, agent_name: &str) -> (String, Vec<String>) {
        // Returns (markdown_section, injected_learning_ids)
        // Queries store, renders top N learnings truncated to max_tokens
        // Returns empty string if store is None or no learnings found
    }

    fn record_learning_outcome(&self, agent_name: &str, exit_class: &ExitClass) {
        // Looks up injected_learning_ids for agent
        // Calls record_effective or record_applied based on exit_class
    }
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_learning_config_default_disabled` | `config.rs` | Learning disabled by default |
| `test_render_lessons_empty_store` | `lib.rs` | No crash when store is empty |
| `test_render_lessons_respects_max_tokens` | `lib.rs` | Truncation works |
| `test_render_lessons_respects_trust_level` | `lib.rs` | L0 learnings filtered |
| `test_record_outcome_success` | `lib.rs` | record_effective called on success |
| `test_record_outcome_failure` | `lib.rs` | record_applied called on failure |
| `test_archive_stale_on_tick` | `lib.rs` | Archive fires at correct interval |
| `test_learning_degradation_on_init_failure` | `lib.rs` | Store=None when DeviceStorage fails |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_spawn_with_learning_injects_section` | `tests/` | End-to-end: learning appears in prompt |
| `test_exit_feeds_back_to_store` | `tests/` | Exit class updates learning trust |

All tests use `InMemoryLearningPersistence` -- no mocks.

## Implementation Steps

### Step 1: Config types
**Files:** `crates/terraphim_orchestrator/src/config.rs`
**Description:** Add `LearningConfig` struct with serde defaults. Add `learning: LearningConfig` field to `OrchestratorConfig`.
**Tests:** Default is disabled; TOML parsing round-trip.
**Estimated:** 30 min

### Step 2: Orchestrator field and init
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Add `learning_store: Option<SharedLearningStore>`, `learning_config: LearningConfig`, `injected_learning_ids: HashMap<String, Vec<String>>` to `AgentOrchestrator`. Add init in `new()` -- if config enabled, try `DeviceStorageLearningPersistence`, fallback to None on error.
**Tests:** Init succeeds with enabled config; degrades to None on failure.
**Dependencies:** Step 1
**Estimated:** 45 min

### Step 3: Spawn-time injection
**Files:** `crates/terraphim_orchestrator/src/lib.rs`, `crates/terraphim_orchestrator/src/learning.rs`
**Description:** Add `render_lessons_section()` to `SharedLearningStore` (or as method on orchestrator). In `spawn_agent()`, after skill chain injection (line ~1552), call `render_lessons_section()`, append to `composed_task`. Store injected IDs in `injected_learning_ids`.
**Tests:** Section appears in prompt; respects max_tokens; respects trust level.
**Dependencies:** Step 2
**Estimated:** 60 min

### Step 4: Exit-time feedback
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** In `poll_agent_exits()`, after `AgentRunRecord` construction (line ~4758), call `record_learning_outcome()` which matches on `exit_class` and calls `record_effective`/`record_applied` for each injected learning ID.
**Tests:** Success exits record effective; failure exits record applied; missing agent skips.
**Dependencies:** Step 3
**Estimated:** 45 min

### Step 5: Tick-time archival
**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** In `reconcile_tick()`, if `tick_count % consolidation_ticks == 0` and store is Some, call `archive_stale(archive_days)`.
**Tests:** Archive fires at interval; no-op when disabled.
**Dependencies:** Step 4
**Estimated:** 15 min

### Step 6: Full test pass + clippy
**Description:** Run `cargo test -p terraphim_orchestrator`, `cargo clippy -p terraphim_orchestrator -- -D warnings`.
**Estimated:** 30 min

## Rollback Plan

`LearningConfig.enabled` defaults to `false`. If issues arise:
1. Set `learning.enabled = false` in orchestrator.toml (or remove the section)
2. All learning code is guarded behind `self.learning_store.is_some()` checks

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Learning query at spawn | < 1ms | In-memory cache in DeviceStorageLearningPersistence |
| Token overhead | < 1500 tokens | Configurable, default 1500 |
| Exit feedback | < 1ms | Single in-memory update + async persist |

## Related Gitea Issues

- **Closes**: #578 (wire evolution into orchestrator -- using learning.rs instead)
- **Closes**: #668 (cross-run lesson injection -- uses query_relevant + record_effective)
- **Updates**: #278 (AgentRunRecord -- already done, this adds learning feedback consumer)
- **Updates**: #180 (SharedLearning store -- Phase 1 done, now wired to orchestrator)
- **References**: #179 (Epic: ADF shared learnings)
