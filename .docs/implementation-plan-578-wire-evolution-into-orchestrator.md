# Implementation Plan: Wire terraphim_agent_evolution into ADF Orchestrator (#578)

**Status**: Draft
**Research Doc**: `.docs/research-578-wire-evolution-into-orchestrator.md`
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-15
**Estimated Effort**: 2-3 days

## Overview

### Summary
Wire the `terraphim_agent_evolution` crate into `terraphim_orchestrator` so that agent outputs feed into the evolution system and evolution snapshots are available in handoff context. Create project-level skill and agent definitions that leverage Claude Code and opencode's project-specific configuration.

### Approach
Add evolution as an optional subsystem that supplements (not replaces) the existing SharedLearningStore. Each agent gets its own `AgentEvolutionSystem` instance, keyed by `AgentDefinition.name`. Output events become `MemoryItem`s. Agent exits create snapshots. Handoff context carries the snapshot key. A project-level skill provides evolution-aware instructions for both tools.

### Scope

**In Scope (Top 5):**
1. Add `terraphim_agent_evolution` dependency to orchestrator (feature-gated)
2. Wire output events -> evolution memory in `drain_output_events()`
3. Wire agent exit -> evolution snapshot in `poll_agent_exits()`
4. Add `evolution_snapshot_key` to `HandoffContext`
5. Wire evolution memory context into `spawn_agent()` prompt

**Out of Scope:**
- Workflow pattern execution (PromptChaining, Routing, etc.) -- follow-up
- Real LLM adapters -- v1 uses MockLlmAdapter
- Evolution-driven config mutation -- research only
- Cross-agent evolution sharing -- requires permission model
- Semantic memory concept extraction -- requires NLP pipeline

**Avoid At All Cost (from 5/25 analysis):**
- Wiring EvolutionWorkflowManager.execute_task() into orchestrator dispatch -- this is the workflow execution engine, not needed for tracking
- Adding LLM calls within the orchestrator process -- all LLM interaction stays in spawned CLI tools
- Replacing SharedLearningStore -- the trust-based lesson system serves a different purpose
- Adding a new HTTP/RPC service for evolution -- over-engineering for v1
- Auto-mutating AgentDefinition based on evolution data -- safety hazard without review

## Architecture

### Component Diagram

```
                    terraphim_orchestrator
                    ┌─────────────────────────────────────────────────┐
                    │  AgentOrchestrator                              │
                    │  ┌─────────────┐  ┌──────────────────────────┐ │
                    │  │ Shared      │  │ EvolutionManager         │ │
                    │  │ Learning    │  │ ┌──────────────────────┐ │ │
                    │  │ Store       │  │ │ HashMap<AgentId,     │ │ │
                    │  │ (existing)  │  │ │   AgentEvolutionSys> │ │ │
                    │  └─────────────┘  │ └──────────────────────┘ │ │
                    │                   │ + record_output()        │ │
                    │                   │ + snapshot_on_exit()     │ │
                    │                   │ + render_context()       │ │
                    │                   └──────────────────────────┘ │
                    │                                                │
                    │  spawn_agent() ──── render_context() ──────┐  │
                    │  drain_output() ── record_output() ───────┐│  │
                    │  poll_exits() ──── snapshot_on_exit() ───┐││  │
                    │                                          │││  │
                    │  ┌──────────────┐                        │││  │
                    │  │ HandoffBuffer│◄── snapshot_key ───────┘││  │
                    │  └──────────────┘                         ││  │
                    └───────────────────────────────────────────┼┼──┘
                                                              ││
  terraphim_agent_evolution                                   ││
  ┌────────────────────────────────────────────┐              ││
  │ AgentEvolutionSystem                       │              ││
  │  ├─ MemoryEvolution (add_memory)           │◄─────────────┘│
  │  ├─ TasksEvolution (add/complete_task)     │◄──────────────┘
  │  └─ LessonsEvolution (add_lesson)          │
  │                                            │
  │ Persists via terraphim_persistence         │
  │ Keys: agent_{name}/memory/current          │
  │        agent_{name}/tasks/current          │
  │        agent_{name}/lessons/current        │
  └────────────────────────────────────────────┘
```

### Data Flow

```
Agent spawns:
  1. [existing] render_lessons_section() -> SharedLearningStore lessons
  2. [NEW] evolution_manager.render_context(agent_name) -> structured memory
  3. Both injected into prompt

Agent runs (output events):
  1. [existing] drain_output_events() -> nightwatch + telemetry
  2. [NEW] evolution_manager.record_output(agent_name, event) -> MemoryItem

Agent exits:
  1. [existing] ExitClassifier -> AgentRunRecord -> learning outcome
  2. [NEW] evolution_manager.snapshot_on_exit(agent_name) -> EvolutionIndex
  3. [NEW] HandoffContext.evolution_snapshot_key = Some(snapshot_key)

Handoff (inter-agent transfer):
  1. [existing] HandoffBuffer.insert(context)
  2. [NEW] Receiver loads evolution state via snapshot_key

Periodic maintenance:
  1. [existing] archive_stale() for SharedLearningStore
  2. [NEW] evolution_manager.consolidate_memories() for all agents
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Feature-gate behind `evolution` Cargo feature | Default-off; existing configs unaffected | Always-on (risky for existing fleet) |
| Supplement SharedLearningStore, don't replace | Trust levels, verify_pattern, applicable_agents are unique to SharedLearningStore | Replace (loses trust model) |
| Wrap evolution in `EvolutionManager` struct | Single integration surface; orchestrator doesn't import evolution types directly | Direct integration (couples orchestrator to evolution internals) |
| MockLlmAdapter for v1 | Issue #578 scopes to mock adapters | Real adapters (requires LLM provider config) |
| Project-level skill in `.skills/` | Both Claude Code and opencode discover this directory | Global skill (requires user-level install) |

### Simplicity Check

**What if this could be easy?**

The simplest design: add a `HashMap<String, AgentEvolutionSystem>` to the orchestrator, call `add_memory()` on output, call `save_snapshot()` on exit. That is essentially what we are doing. The `EvolutionManager` wrapper is one struct with three methods -- it exists only to keep evolution imports out of the main `lib.rs` which is already 11,000 lines.

**Senior Engineer Test**: Not overcomplicated. No new services, no new processes, no new protocols. Just a HashMap and three function calls at existing extension points.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request (only #578 acceptance criteria)
- [x] No abstractions "in case we need them later" (EvolutionManager is minimal)
- [x] No flexibility "just in case" (fixed MemoryItem types from output events)
- [x] No error handling for scenarios that cannot occur (evolution failures are logged, not fatal)
- [x] No premature optimization (no batching, no caching beyond what evolution crate provides)

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/evolution.rs` | EvolutionManager wrapper (record_output, snapshot_on_exit, render_context) |
| `crates/terraphim_orchestrator/tests/evolution_integration.rs` | Integration tests for evolution lifecycle |
| `.skills/terraphim-evolution/SKILL.md` | Project-level skill for evolution queries |
| `.claude/agents/evolution-manager.md` | Project-level agent for evolution tasks |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/Cargo.toml` | Add `terraphim_agent_evolution` dependency with `evolution` feature |
| `crates/terraphim_orchestrator/src/config.rs` | Add `EvolutionConfig` struct, `evolution` field to `OrchestratorConfig`, `evolution_enabled` to `AgentDefinition` |
| `crates/terraphim_orchestrator/src/lib.rs` | Add `mod evolution;`, `evolution_manager` field to `AgentOrchestrator`, wire into reconcile_tick |
| `crates/terraphim_orchestrator/src/handoff.rs` | Add `evolution_snapshot_key: Option<String>` to `HandoffContext` |
| `AGENTS.md` | Add evolution configuration section |

### Deleted Files
None.

## API Design

### Public Types

```rust
// crates/terraphim_orchestrator/src/evolution.rs

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EvolutionConfig {
    pub enabled: bool,
    pub max_memory_tokens: usize,
    pub max_snapshots_per_agent: usize,
    pub consolidation_interval_ticks: u64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_memory_tokens: 1500,
            max_snapshots_per_agent: 100,
            consolidation_interval_ticks: 200,
        }
    }
}

pub struct EvolutionManager {
    systems: HashMap<AgentId, AgentEvolutionSystem>,
    config: EvolutionConfig,
}

pub struct EvolutionOutput {
    pub agent_id: AgentId,
    pub content: String,
    pub event_type: MemoryItemType,
    pub importance: ImportanceLevel,
}
```

### Public Functions

```rust
impl EvolutionManager {
    pub fn new(config: EvolutionConfig) -> Self;
    pub fn ensure_agent(&mut self, agent_id: &str);
    pub fn record_output(&mut self, output: EvolutionOutput) -> Result<(), EvolutionError>;
    pub fn record_task_start(&mut self, agent_id: &str, task_content: &str) -> Result<TaskId, EvolutionError>;
    pub fn record_task_complete(&mut self, agent_id: &str, task_id: &TaskId, result: &str) -> Result<(), EvolutionError>;
    pub fn record_lesson(&mut self, agent_id: &str, title: &str, context: &str, insight: &str, category: LessonCategory) -> Result<(), EvolutionError>;
    pub fn snapshot_on_exit(&mut self, agent_id: &str) -> Result<Option<String>, EvolutionError>;
    pub fn render_context(&self, agent_id: &str) -> Result<String, EvolutionError>;
    pub fn consolidate_all(&mut self) -> Result<usize, EvolutionError>;
    pub fn is_enabled(&self) -> bool;
}
```

### Error Handling

Evolution errors are **never fatal** to the orchestrator. All evolution calls are wrapped in `if let Ok(...)` or logged via `warn!()`. If evolution fails, the orchestrator continues operating exactly as before.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_evolution_config_default_disabled` | `evolution.rs` | Default config has `enabled: false` |
| `test_evolution_manager_new` | `evolution.rs` | Create manager, verify empty |
| `test_ensure_agent_creates_system` | `evolution.rs` | Agent gets its own evolution system |
| `test_record_output_adds_memory` | `evolution.rs` | Output becomes MemoryItem |
| `test_record_task_lifecycle` | `evolution.rs` | Start -> complete creates task |
| `test_snapshot_creates_key` | `evolution.rs` | Snapshot returns storage key |
| `test_render_context_returns_string` | `evolution.rs` | Context is non-empty after recording |
| `test_render_context_truncates` | `evolution.rs` | Respects max_memory_tokens |
| `test_consolidate_promotes_memories` | `evolution.rs` | High-importance items promoted |
| `test_disabled_manager_is_noop` | `evolution.rs` | All methods are no-ops when disabled |
| `test_handoff_backward_compat` | `handoff.rs` | Old JSON deserialises without snapshot_key |
| `test_handoff_with_snapshot_key` | `handoff.rs` | New JSON round-trips |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_evolution_lifecycle` | `tests/evolution_integration.rs` | spawn -> output -> exit -> snapshot exists -> context renderable |
| `test_evolution_persistence` | `tests/evolution_integration.rs` | Create snapshot, reload, verify state matches |
| `test_evolution_with_handoff` | `tests/evolution_integration.rs` | Create handoff with snapshot_key, receiver loads evolution |
| `test_multiple_agents_independent` | `tests/evolution_integration.rs` | Two agents have separate evolution state |
| `test_existing_tests_still_pass` | existing | Regression check |

## Implementation Steps

### Step 1: Add dependency and config structs
**Files:** `Cargo.toml`, `src/config.rs`
**Description:** Add `terraphim_agent_evolution` as optional dep behind `evolution` feature. Add `EvolutionConfig` and `evolution_enabled` field.
**Tests:** Config parsing with/without evolution section
**Estimated:** 1 hour

**Cargo.toml changes:**
```toml
[features]
evolution = ["dep:terraphim_agent_evolution"]

[dependencies]
terraphim_agent_evolution = { path = "../terraphim_agent_evolution", optional = true }
```

**config.rs changes:**
```rust
// Add to OrchestratorConfig
#[serde(default)]
pub evolution: EvolutionConfig,

// Add to AgentDefinition
#[serde(default)]
pub evolution_enabled: bool,
```

### Step 2: Create EvolutionManager
**Files:** `src/evolution.rs` (new)
**Description:** Wrapper struct with HashMap<String, AgentEvolutionSystem>. All methods are no-ops when config.enabled is false.
**Tests:** All unit tests from test strategy
**Dependencies:** Step 1
**Estimated:** 3 hours

### Step 3: Extend HandoffContext
**Files:** `src/handoff.rs`
**Description:** Add `evolution_snapshot_key: Option<String>` with backward-compatible serde.
**Tests:** Backward compat tests, round-trip tests
**Dependencies:** None (parallel with Step 2)
**Estimated:** 30 min

### Step 4: Wire into reconcile_tick
**Files:** `src/lib.rs`
**Description:**
1. Add `mod evolution;` and `#[cfg(feature = "evolution")]` guard
2. Add `evolution_manager: Option<EvolutionManager>` field to AgentOrchestrator
3. Initialise in `new()` from config
4. In `drain_output_events()`: call `record_output()` for each event (when enabled)
5. In `poll_agent_exits()`: call `record_task_complete()` and `snapshot_on_exit()`
6. In `reconcile_tick()`: add periodic `consolidate_all()` step
7. In `spawn_agent()`: call `ensure_agent()`, `record_task_start()`, `render_context()` -> inject into prompt
**Tests:** Integration tests
**Dependencies:** Steps 2, 3
**Estimated:** 4 hours

### Step 5: Create project-level skill
**Files:** `.skills/terraphim-evolution/SKILL.md` (new)
**Description:** Skill that provides evolution query capabilities. Available to both Claude Code and opencode via project-level `.skills/` directory.
**Dependencies:** Steps 1-4
**Estimated:** 1 hour

**Skill content:**
```markdown
---
name: terraphim-evolution
description: |
  Query and analyse agent evolution data from the ADF orchestrator.
  Use when the user asks about "agent evolution", "agent memory",
  "evolution snapshot", "agent learning history", or "agent performance over time".
  Triggers: "evolution", "agent memory", "agent learning", "snapshot".
---

# Terraphim Agent Evolution Skill

## Overview
The ADF orchestrator tracks agent evolution via terraphim_agent_evolution.
Each agent accumulates memory, task history, and lessons across runs.

## Key Concepts
- **MemoryItem**: A memory entry with type, content, importance, tags
- **AgentSnapshot**: Point-in-time state of memory + tasks + lessons
- **EvolutionIndex**: Versioned snapshot reference (persistence key)

## CLI Queries
```bash
# Query evolution state for an agent (if orchestrator exposes CLI)
# Future: adf-ctl evolution status <agent_name>
# Future: adf-ctl evolution snapshot <agent_name> --at <timestamp>
```

## Integration Points
- Agent outputs become MemoryItems (type: ExecutionResult, WorkflowEvent)
- Agent exits create snapshots (EvolutionIndex)
- Handoff context carries snapshot keys
- Spawn injects evolution memory context into prompts
```

### Step 6: Create project-level agent
**Files:** `.claude/agents/evolution-manager.md` (new)
**Description:** Claude Code specialist agent for evolution-related tasks.
**Dependencies:** Steps 1-4
**Estimated:** 30 min

**Agent definition:**
```markdown
---
name: evolution-manager
description: |
  Specialist agent for querying and analysing agent evolution data.
  Use when the user asks about agent performance trends, memory consolidation,
  lesson extraction, or evolution system configuration.
  <example>query evolution state for security-sentinel</example>
  <example>analyse agent learning trends over the past week</example>
  <example>show evolution snapshot for product-developer</example>
model: inherit
color: purple
---

You are a specialist in the Terraphim Agent Evolution system.

## Your Domain
- terraphim_agent_evolution crate API
- Agent memory hierarchy (short-term, long-term, working, episodic, semantic)
- Task lifecycle tracking (pending -> in_progress -> completed)
- Lesson categorisation (Technical, Process, Domain, Failure, SuccessPattern)
- Evolution snapshots and versioned state

## Key Files
- crates/terraphim_agent_evolution/src/evolution.rs - Core system
- crates/terraphim_agent_evolution/src/memory.rs - Memory tracking
- crates/terraphim_agent_evolution/src/tasks.rs - Task tracking
- crates/terraphim_agent_evolution/src/lessons.rs - Lessons management
- crates/terraphim_agent_evolution/src/integration.rs - Workflow integration
- crates/terraphim_orchestrator/src/evolution.rs - Orchestrator wiring

## Configuration
Evolution is configured in orchestrator.toml:
```toml
[evolution]
enabled = true
max_memory_tokens = 1500
max_snapshots_per_agent = 100
consolidation_interval_ticks = 200
```

Per-agent opt-in in agent definitions:
```toml
[[agents]]
name = "security-sentinel"
evolution_enabled = true
```
```

### Step 7: Update project documentation
**Files:** `AGENTS.md`
**Description:** Add evolution configuration section to agent instructions.
**Dependencies:** Steps 5, 6
**Estimated:** 30 min

### Step 8: Write integration tests
**Files:** `tests/evolution_integration.rs` (new)
**Description:** Full lifecycle tests: create manager -> record output -> snapshot -> reload -> verify.
**Dependencies:** Steps 2-4
**Estimated:** 2 hours

## Rollback Plan

1. If evolution causes issues: set `evolution.enabled = false` in config (all evolution code becomes no-op)
2. If compilation fails: remove `evolution` feature from Cargo.toml (all code behind `#[cfg(feature = "evolution")]`)
3. No database schema changes -- evolution uses the existing `terraphim_persistence` key-value store

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `terraphim_agent_evolution` | workspace (path) | Core integration target |

No new external crates. All transitive dependencies are already in the workspace.

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| `record_output()` | < 1ms | In-memory HashMap lookup + push |
| `snapshot_on_exit()` | < 100ms | Disk write (versioned persistence) |
| `render_context()` | < 50ms | Memory traversal + Markdown formatting |
| `consolidate_all()` | < 500ms | Runs every 200 ticks (~5 min), not on critical path |

### Memory Overhead
- Per-agent: ~100KB for in-memory evolution state (typical: 50-200 memory items)
- 10 agents: ~1MB total
- Snapshots persisted to disk, not held in memory

## Open Items

| Item | Status | Resolution |
|------|--------|------------|
| Verify Persistable + DeviceStorage compatibility | Pending | Spike in Step 1 |
| Should `consolidate_all()` run on a separate tick or within reconcile_tick? | Pending | Within reconcile_tick (simpler) |
| Should disabled agents (evolution_enabled=false) skip ensure_agent()? | Pending | Yes, for memory efficiency |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
