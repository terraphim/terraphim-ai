# Research Document: Wire terraphim_agent_evolution into ADF Orchestrator (#578)

**Status**: Draft
**Author**: opencode (GLM-5.1)
**Date**: 2026-05-15
**Issue**: terraphim/terraphim-ai#578

## Executive Summary

The `terraphim_agent_evolution` crate (8,872 lines, 5 AI workflow patterns, versioned memory/tasks/lessons tracking) is completely unwired from the orchestrator. Two other crates declare it as a dependency but never use it. This research maps the exact integration points, identifies that the orchestrator's existing learning store and handoff buffer are the natural extension points, and proposes a project-level skill + agent configuration approach that leverages Claude Code and opencode's project-specific capabilities.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Closes a 8,872-line dead-code island; unlocks agent self-improvement loop |
| Leverages strengths? | Yes | Orchestrator already has learning store, handoff buffer, and exit classification -- evolution extends these naturally |
| Meets real need? | Yes | ADF agents currently have zero memory across runs; evolution gives them persistent learning |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
The `terraphim_agent_evolution` crate implements versioned memory, task tracking, and lessons-learned management for AI agents. It also implements 5 workflow patterns (prompt chaining, routing, parallelisation, orchestrator-workers, evaluator-optimizer) with LLM adapters. Despite being rich and well-tested, it is completely isolated from the rest of the codebase. The orchestrator manages agent lifecycle (spawn -> run -> output -> exit) but has no mechanism to feed outputs into an evolution system or carry evolution state between agent runs.

### Impact
- Agents start every run with zero memory of past successes/failures
- The existing `SharedLearningStore` provides simple text-based lessons but no structured memory hierarchy (short-term, long-term, episodic, semantic)
- No agent can improve its own behaviour based on accumulated experience
- 8,872 lines of tested code sit unused

### Success Criteria
1. Agent outputs feed into the evolution system as MemoryItems
2. Evolution snapshots are created on agent exit
3. Evolution state is available in handoff context for inter-agent transfers
4. Project-level skill/agent configuration enables evolution without global changes
5. All existing tests pass

## Current State Analysis

### Existing Implementation

#### terraphim_agent_evolution (the integration target)

**Key types and their roles:**

| Type | Purpose | Integration Relevance |
|------|---------|----------------------|
| `AgentEvolutionSystem` | Top-level coordinator (memory + tasks + lessons) | Primary integration point |
| `EvolutionWorkflowManager` | Workflow execution + evolution tracking | Orchestrator will create per-agent |
| `MemoryEvolution` | Versioned short/long/working/episodic/semantic memory | Replaces flat text lessons for deep memory |
| `TasksEvolution` | Versioned task lifecycle tracking | Tracks agent task progress |
| `LessonsEvolution` | Categorised lessons with validation/evidence | Enriches existing SharedLearningStore |
| `MemoryItem` | Core memory unit (type, content, importance, tags) | Agent output events become these |
| `EvolutionIndex` | Versioned snapshot index (persistence key) | Stored alongside handoff context |
| `AgentSnapshot` | Point-in-time state of all three subsystems | Created on agent exit |
| `LlmAdapter` trait | Abstraction for LLM calls (mock + real) | Mock for v1, real for workflow patterns |
| `WorkflowFactory` | Auto-selects pattern from task analysis | Future: orchestrator uses for task routing |

**Data flow within evolution:**
```
EvolutionWorkflowManager::execute_task(prompt)
  -> analyze_task() -> TaskAnalysis
  -> WorkflowFactory::create_for_task() -> Arc<dyn WorkflowPattern>
  -> workflow.execute(WorkflowInput) -> WorkflowOutput
  -> update_evolution_state():
       tasks.add_task() + tasks.complete_task()
       memory.add_memory() [per execution step + result]
       memory.episodic_memory.push(Episode)
       lessons.add_lesson() [performance + process + domain]
```

**Current consumers:** NONE. Zero `use` statements outside the crate itself.

#### terraphim_orchestrator (the integration host)

**Architecture:**
- 57 source files, ~11,000 lines in `lib.rs` alone
- Main loop: `run()` -> `select!` on events -> `reconcile_tick()` (20 steps)
- Agent lifecycle: `spawn_agent()` -> background process -> `poll_agent_exits()` -> `handle_agent_exit()`
- Already has: `SharedLearningStore`, `HandoffBuffer`, `ExitClassifier`, `AgentRunRecord`

**Integration-relevant code locations:**

| Component | Location | Purpose |
|-----------|----------|---------|
| `AgentOrchestrator` struct | `lib.rs:~200` | Main engine with all subsystems |
| `SharedLearningStore` | `learning.rs:648` | Simple text-based lesson injection |
| `HandoffContext` | `handoff.rs:11-32` | Inter-agent state transfer |
| `HandoffBuffer` | `handoff.rs:141-225` | In-memory HashMap with TTL |
| `AgentDefinition` | `config.rs:588-667` | Per-agent config (no evolution fields) |
| `LearningConfig` | `config.rs:304-351` | Learning system settings |
| `poll_agent_exits()` | `lib.rs:6158` | Agent completion handling |
| `handle_agent_exit()` | `lib.rs:6921` | Per-layer restart logic |
| `drain_output_events()` | `lib.rs:7094` | Output stream processing |
| `render_lessons_section()` | `lib.rs:1625` | Lesson injection into prompts |
| `spawn_agent()` lesson injection | `lib.rs:2017-2030` | Where lessons enter prompts |
| `reconcile_tick()` learning archive | `lib.rs:5674-5690` | Periodic lesson consolidation |

### Data Flow (Current)

```
Agent spawns:
  1. render_lessons_section(agent_name) -> Markdown text
  2. inject into prompt before spawning

Agent runs:
  1. drain_output_events() -> feeds nightwatch + telemetry
  2. stdout/stderr parsed for token counts

Agent exits:
  1. ExitClassifier -> ExitClass (Success/Timeout/RateLimit/...)
  2. AgentRunRecord built
  3. Learning outcome: record_effective() or record_applied()
  4. handle_agent_exit() -> restart logic per layer
  5. Output posted to Gitea (if configured)

Learning archive (periodic):
  1. archive_stale() removes old lessons
```

### Integration Points

| Point | Current Behaviour | Evolution Extension |
|-------|-------------------|---------------------|
| `spawn_agent()` | Renders flat text lessons from SharedLearningStore | Also load evolution memory items, render structured context |
| `drain_output_events()` | Parses telemetry only | Also feed output as MemoryItems to evolution |
| `poll_agent_exits()` | Classifies exit, records learning outcome | Also create evolution snapshot |
| `handle_agent_exit()` | Per-layer restart | No change needed (restart logic unchanged) |
| `HandoffContext` | Shallow state transfer | Add `evolution_snapshot_key` field |
| `reconcile_tick()` | Periodic learning archive | Add periodic memory consolidation |
| `AgentDefinition` | Static config | Add `evolution_enabled: bool` flag |

## Constraints

### Technical Constraints
- **No in-process LLM**: The orchestrator delegates all LLM interaction to CLI tools spawned as child processes. The evolution crate's `LlmAdapter` trait and workflow patterns are designed for in-process LLM calls. For v1, we use `MockLlmAdapter` for workflow execution tracking only (no actual LLM calls from orchestrator). Real LLM adapters deferred to follow-up work.
- **Persistence model**: The evolution crate uses `terraphim_persistence::Persistable` trait (versioned key-value store via DeviceStorage). The orchestrator already depends on `terraphim_persistence` via SharedLearningStore, so this is compatible.
- **Agent identity**: Evolution keys are `agent_{id}/...`. Agent names in the orchestrator come from `AgentDefinition.name`, which is unique per config. We use this as the AgentId.
- **Synchronous boundary**: `SharedLearningStore` wraps async persistence behind `block_in_place`. Evolution integration should follow the same pattern to avoid polluting the async runtime.

### Business Constraints
- **Backward compatibility**: Existing configs without evolution fields must continue to work (evolution disabled by default).
- **Minimal v1 scope**: Issue #578 says "mock LLM adapters for initial wiring". We do NOT wire the workflow patterns into the orchestrator's task routing. We only wire: output -> memory, exit -> snapshot, handoff -> snapshot key.
- **Project-level configuration**: Skills and agents should be configurable at project level, not just globally.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Evolution snapshot creation | < 100ms per agent exit | N/A |
| Memory context rendering | < 50ms per agent spawn | N/A |
| Disk usage per agent | < 1MB per 100 runs | N/A |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Evolution must be opt-in per AgentDefinition | Default-off prevents breaking existing fleet configs | #578 acceptance criteria: backward compatible |
| Output-to-memory must be lossless | Agent output is the primary signal for evolution | AgentRunRecord already captures exit class, model, wall_time |
| Snapshot must be queryable from handoff | Inter-agent evolution transfer is a core requirement | #578: "evolution snapshot available in handoff context" |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Workflow pattern routing (using EvolutionWorkflowManager for task selection) | Follow-up work; v1 only tracks, doesn't route |
| Real LLM adapters | #578 explicitly scopes to mock adapters |
| Evolution-driven config mutation (auto-changing model/provider/persona) | Research phase only; mutation requires careful safety review |
| Semantic memory concept extraction from agent output | Requires NLP pipeline; deferred |
| Multi-agent evolution sharing (one agent learning from another's memory) | Requires cross-agent permission model |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_agent_evolution` | Core crate to integrate | Low -- well-tested, stable API |
| `terraphim_persistence` | Already used by both crates | None -- shared dependency |
| `terraphim_types` | Shared types (Provider, etc.) | None -- already in dep tree |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `tokio` | workspace | None | N/A |
| `serde`/`serde_json` | workspace | None | N/A |
| `chrono` | workspace | None | N/A |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Evolution crate's Persistable trait incompatible with orchestrator's DeviceStorage | Low | High | Both use terraphim_persistence -- verify compatibility in spike |
| Performance overhead of evolution tracking on every agent exit | Low | Medium | Benchmark; evolution disabled by default |
| Evolution memory rendering exceeds prompt token budget | Medium | Medium | Truncate to configurable max_tokens like existing lesson rendering |
| Snapshot key in HandoffContext breaks deserialisation of old handoffs | Low | High | Use `#[serde(default, skip_serializing_if)]` |

### Open Questions
1. Should evolution memory replace or supplement the existing SharedLearningStore? -- Recommendation: supplement, not replace (see below)
2. Should the project-level skill for evolution be Claude Code-only or shared across both tools? -- Recommendation: shared, using `.skills/` directory
3. What is the maximum memory context size that should be injected into agent prompts? -- Recommendation: reuse `LearningConfig.max_tokens` (default 1500)

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Evolution crate's Persistable works with DeviceStorage | Both use terraphim_persistence | Persistence calls fail | No -- needs spike |
| MockLlmAdapter is sufficient for v1 tracking | #578 scope says "mock LLM adapters" | Workflow patterns won't work for real | Yes -- issue says so |
| AgentDefinition.name is unique within a config | Config format doesn't allow duplicates | Evolution state corruption | Yes -- validated by config parsing |
| Existing tests pass after adding evolution dep | Evolution is additive, no breaking changes | Compilation errors from transitive deps | No -- needs verification |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Evolution REPLACES SharedLearningStore | Simplifies code, but loses L0-L3 trust levels, verify_pattern, applicable_agents | Rejected -- SharedLearningStore has rich trust model that evolution doesn't replicate |
| Evolution SUPPLEMENTS SharedLearningStore | Both coexist; evolution adds structured memory, SharedLearningStore keeps trust-based lesson injection | Chosen -- preserves existing behaviour, adds depth incrementally |
| Evolution is a separate daemon/service | Clean isolation but complex deployment | Rejected -- over-engineering for v1, orchestrator already has the agent lifecycle hooks |

## Research Findings

### Key Insights

1. **The evolution crate is an island**: 8,872 lines of tested code with zero external consumers. Two crates declare it as a dependency but never import it. This is the primary motivation for #578.

2. **The orchestrator already has the hooks**: `poll_agent_exits()` (line 6158), `drain_output_events()` (line 7094), and `render_lessons_section()` (line 1625) are the exact integration points needed. No new event loop steps are required -- just extension of existing steps.

3. **SharedLearningStore and evolution lessons serve different purposes**: SharedLearningStore is trust-based with verification patterns (L0->L3 promotion), applicable_agents filtering, and Markdown rendering. Evolution lessons are categorised (Technical/Process/Domain/Failure/SuccessPattern) with evidence tracking and confidence scores. They are complementary, not competing.

4. **Project-level skills/agents in Claude Code and opencode**: Both tools support project-level `.skills/` directories, `.claude/agents/`, and project-level `CLAUDE.md`/`AGENTS.md`. An evolution skill at `.skills/terraphim-evolution/SKILL.md` would be available to both tools without global config changes. An evolution agent at `.claude/agents/evolution-manager.md` would give Claude Code a specialist for evolution tasks.

5. **HandoffContext extension is safe**: The `HandoffContext` struct uses `#[serde(default, skip_serializing_if = "Option::is_none")]` pattern already. Adding an optional `evolution_snapshot_key` field is backward-compatible.

6. **The orchestrator has no in-process LLM layer**: All LLM calls go through spawned CLI processes. The evolution crate's `LlmAdapter` trait and 5 workflow patterns are designed for in-process use. For v1, we track evolution data (memory, tasks, lessons) without using the workflow execution engine.

### Relevant Prior Art
- **SharedLearningStore**: The existing learning system in the orchestrator is the closest prior art. Evolution extends it with structured memory and versioning.
- **terraphim_multi_agent**: Declares evolution as a dependency, has `AgentSnapshot` in its types, but never uses it. This crate would be the secondary consumer after the orchestrator.
- **EvolutionWorkflowManager**: The integration module already handles "execute task -> update evolution state" in one call. The orchestrator would use a simplified version of this pattern.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify Persistable + DeviceStorage compatibility | Ensure evolution persistence works with orchestrator's storage | 1 hour |
| Benchmark evolution snapshot creation | Confirm < 100ms per exit | 30 min |
| Test HandoffContext backward compat | Serialise old format, deserialise with new field | 30 min |

## Recommendations

### Proceed/No-Proceed
**Proceed.** The integration is well-scoped, the evolution crate is stable and tested, and the orchestrator has clear extension points. Risk is low because evolution is opt-in per agent.

### Scope Recommendations
1. **v1 (this issue)**: Wire evolution tracking only -- output -> memory, exit -> snapshot, handoff -> key. Mock LLM adapter. Supplement SharedLearningStore.
2. **v2 (follow-up)**: Wire workflow patterns into task routing. Real LLM adapters. Evolution-driven config mutation proposals.
3. **v3 (future)**: Cross-agent evolution sharing. Semantic memory extraction. Evolution visualisation dashboard.

### Risk Mitigation Recommendations
1. Add `evolution_enabled: bool` to AgentDefinition (default false) for safe rollout
2. Add `EvolutionConfig` section to OrchestratorConfig with max_memory_tokens, max_snapshots_per_agent
3. Feature-gate the entire integration behind `evolution` feature in Cargo.toml
4. Write integration test: agent run -> exit -> snapshot exists -> key in handoff

## Next Steps

1. Create implementation plan (Phase 2 design document)
2. Verify Persistable + DeviceStorage compatibility spike
3. Implement in this order:
   a. Add dependency + config structs
   b. Add evolution fields to AgentOrchestrator
   c. Wire output -> memory in drain_output_events()
   d. Wire exit -> snapshot in poll_agent_exits()
   e. Wire snapshot key into HandoffContext
   f. Wire memory context into spawn_agent()
   g. Create project-level skill (.skills/terraphim-evolution/)
   h. Create project-level agent (.claude/agents/evolution-manager.md)
   i. Update AGENTS.md with evolution configuration
4. Write tests (unit + integration)

## Appendix

### A. Project-Level Skills/Agents Architecture

Both Claude Code and opencode support project-level configuration:

**Claude Code project-level:**
- `.claude/agents/*.md` -- Project-specific agent definitions (currently 2 agents)
- `.claude/settings.local.json` -- Project permissions and hooks
- `.claude/hooks/` -- Project hook scripts (currently 8)
- `skills/` -- Project skills (currently 5)
- `.skills/` -- Project skills (currently 1: terraphim-rlm)
- `CLAUDE.md` -- Project instructions (1205 lines)
- `AGENTS.md` -- Agent instructions (358 lines)

**OpenCode project-level:**
- `.opencode/rules` -- Project rules file
- `.skills/` -- Same skills directory as Claude Code
- `CLAUDE.md` / `AGENTS.md` -- Same instruction files
- `.opencode/swarm.db` -- Swarm coordination

**Proposed additions for evolution:**
- `.skills/terraphim-evolution/SKILL.md` -- Evolution skill for both tools
- `.claude/agents/evolution-manager.md` -- Specialist agent for evolution queries
- `AGENTS.md` update -- Add evolution configuration section

### B. Evolution Crate Public API Summary

The integration requires only a subset of the evolution crate's API:

**Required for v1:**
- `AgentEvolutionSystem::new(agent_id)` -- Create per-agent
- `AgentEvolutionSystem::save_snapshot()` -- On agent exit
- `AgentEvolutionSystem::load_snapshot(timestamp)` -- On handoff receive
- `MemoryEvolution::add_memory(MemoryItem)` -- On output event
- `MemoryEvolution::consolidate_memories()` -- Periodic
- `MemoryEvolution::save_version()` -- With snapshot
- `TasksEvolution::add_task(AgentTask)` -- On agent spawn
- `TasksEvolution::complete_task(task_id, result)` -- On agent exit
- `LessonsEvolution::add_lesson(Lesson)` -- On exit classification
- `LlmAdapterFactory::create_mock()` -- For v1 mock adapter
- `MemoryItem`, `AgentTask`, `Lesson` -- Domain types

**NOT required for v1:**
- `WorkflowFactory`, `WorkflowPattern`, all 5 pattern implementations
- `LlmAdapterFactory::from_config()`, `create_specialized_agent()`
- `EvolutionWorkflowManager::execute_task()` (the workflow execution engine)
- `MemoryEvolutionViewer` (visualisation)
- `Routing`, `PromptChaining`, `Parallelization`, `OrchestratorWorkers`, `EvaluatorOptimizer`
