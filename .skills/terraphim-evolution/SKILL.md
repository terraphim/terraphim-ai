---
name: terraphim-evolution
description: |
  Query and analyse agent evolution data from the ADF orchestrator.
  Use when the user asks about "agent evolution", "agent memory",
  "evolution snapshot", "agent learning history", or "agent performance over time".
  Triggers: "evolution", "agent memory", "agent learning", "snapshot".
license: Apache-2.0
---

# Terraphim Agent Evolution Skill

## Overview

The ADF orchestrator tracks agent evolution via `terraphim_agent_evolution`.
Each agent accumulates versioned memory, task history, and lessons across runs.
This is supplementary to the existing SharedLearningStore -- evolution adds
structured memory hierarchy (short-term, long-term, episodic, semantic) while
SharedLearningStore provides trust-based lesson injection (L0-L3).

## Architecture

```
AgentOrchestrator
  +-- evolution_manager: EvolutionManager
       +-- systems: HashMap<AgentId, AgentEvolutionSystem>
            +-- memory: MemoryEvolution (short/long/working/episodic/semantic)
            +-- tasks: TasksEvolution (pending/in_progress/completed/blocked)
            +-- lessons: LessonsEvolution (technical/process/domain/failure/success)
```

## Configuration

Enable in `orchestrator.toml`:

```toml
[evolution]
enabled = true
max_memory_tokens = 1500
max_snapshots_per_agent = 100
consolidation_interval_ticks = 200

[[agents]]
name = "security-sentinel"
evolution_enabled = true
```

Requires `evolution` Cargo feature: `cargo build --features evolution`.

## Data Flow

1. **Agent spawn**: Evolution context injected into prompt (recent memories, long-term summary, episodes)
2. **Agent output**: stdout/stderr events recorded as `MemoryItem` entries
3. **Agent exit**: Evolution snapshot created, key stored in `HandoffContext`
4. **Periodic**: Memory consolidation promotes high-importance short-term to long-term

## Key Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/evolution.rs` | EvolutionManager wrapper |
| `crates/terraphim_agent_evolution/src/evolution.rs` | Core AgentEvolutionSystem |
| `crates/terraphim_agent_evolution/src/memory.rs` | Memory tracking |
| `crates/terraphim_agent_evolution/src/tasks.rs` | Task lifecycle |
| `crates/terraphim_agent_evolution/src/lessons.rs` | Lessons management |
| `crates/terraphim_agent_evolution/src/integration.rs` | Workflow-to-evolution bridge |

## Querying Evolution State

Use the evolution crate's viewer module for analysis:

```rust
use terraphim_agent_evolution::viewer::MemoryEvolutionViewer;

let viewer = MemoryEvolutionViewer::new("agent-name".to_string());
let timeline = viewer.get_timeline(&system, start, end).await?;
let insights = viewer.get_insights(&system, TimePeriod::LastWeek).await?;
```

## Integration Points

- `HandoffContext.evolution_snapshot_key` -- carries snapshot reference between agents
- `EvolutionManager::render_context()` -- generates Markdown context for prompts
- `EvolutionManager::snapshot_on_exit()` -- persists state on agent completion
- `reconcile_tick()` consolidation step -- periodic memory promotion
