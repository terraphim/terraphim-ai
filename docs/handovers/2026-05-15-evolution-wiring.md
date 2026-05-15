# Handover: Evolution Wiring into ADF Orchestrator

**Date:** 2026-05-15 12:23 BST
**Branch:** `main` @ `07b002e97`
**Status:** Clean working tree, both remotes in sync
**Issue:** #578 (CLOSED)
**PR:** #1487 (MERGED, squash)

## Tasks Completed This Session

1. **Full disciplined workflow for #578** -- Research, Design, Implementation, Verification
2. **Wired `terraphim_agent_evolution` crate into `terraphim_orchestrator`** behind `evolution` feature gate
3. **16 new unit tests** -- 11 no-op (without feature), 5 active (with feature)
4. **All 681 existing orchestrator tests pass** with `--features evolution`; 675 without
5. **PR #1487 merged** (squash) into main at `07b002e97`
6. **Issue #578 closed** with all acceptance criteria met
7. **Both remotes synced** -- Gitea (origin) and GitHub (github) identical

## Current Implementation State

### What's Working

- `EvolutionManager` wrapper in `crates/terraphim_orchestrator/src/evolution.rs`
- Feature-gated `evolution` Cargo feature (disabled by default)
- 4 integration points wired into orchestrator:
  - `spawn_agent()` -- evolution context injected into prompt
  - `drain_output_events()` -- output events recorded as memories
  - `poll_agent_exits()` -- lesson + snapshot on agent exit
  - `reconcile_tick()` -- periodic memory consolidation
- `EvolutionConfig` struct with `max_memory_tokens`, `max_snapshots_per_agent`, `consolidation_interval_ticks`
- `evolution_enabled` per-agent opt-in on `AgentDefinition`
- `HandoffContext.evolution_snapshot_key: Option<String>` for inter-agent snapshot transfer
- Project-level skill (`.skills/terraphim-evolution/SKILL.md`)
- Project-level agent (`.claude/agents/evolution-manager.md`)
- `AGENTS.md` updated with evolution configuration section

### What's Blocked / Deferred

- **Real LLM adapters for evolution** -- deferred per #578 scope, uses MockLlmAdapter for v1
- **Integration test with live agents** -- unit tests cover lifecycle; full integration requires running ADF pipeline
- **Evolution persistence** -- snapshots are in-memory only; disk/S3 persistence not yet implemented

## Technical Context

```
Current branch: main @ 07b002e97
Remotes: origin (Gitea) = github (GitHub) -- identical

Key commits:
  07b002e97 feat(orchestrator): wire terraphim_agent_evolution into ADF orchestrator (#1487)
  5b5ef402a docs: add handover for ADF orchestrator investigation and fix
  2d41798cd fix(agent): populate concepts_matched in robot-mode search envelope (#1486)

Modified files (in merge commit):
  43 files changed, 2396 insertions(+), 147 deletions(-)
```

### Key Files

| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/evolution.rs` | EvolutionManager wrapper + 16 tests |
| `crates/terraphim_orchestrator/src/config.rs` | EvolutionConfig, evolution_enabled on AgentDefinition |
| `crates/terraphim_orchestrator/src/handoff.rs` | evolution_snapshot_key on HandoffContext |
| `crates/terraphim_orchestrator/src/lib.rs` | 4 wiring points (spawn, drain, exit, reconcile) |
| `crates/terraphim_orchestrator/src/flow/executor.rs` | evolution_enabled: false for flow-spawned agents |
| `.skills/terraphim-evolution/SKILL.md` | Project-level skill |
| `.claude/agents/evolution-manager.md` | Project-level specialist agent |

### Key Design Decisions

- Evolution **supplements** SharedLearningStore, does not replace it
- `EvolutionManager` wrapper keeps evolution imports out of lib.rs (11,000+ lines)
- `AgentEvolutionSystem` exposes public fields -- use direct field access
- `MemoryItem` has no `new()` -- struct literal with `ulid::Ulid::new().to_string()` for ID
- `LessonCategory` enum differs between feature/no-feature -- cfg-gated blocks handle this
- `save_snapshot()` is async -- called via `block_in_place + block_on` pattern

### Build & Test Commands

```bash
cargo build -p terraphim_orchestrator --features evolution
cargo test -p terraphim_orchestrator --features evolution --lib   # 681 tests
cargo test -p terraphim_orchestrator --lib                       # 675 tests
cargo clippy -p terraphim_orchestrator --features evolution
cargo fmt
```

## Next Steps

1. **Pick next task** from `gtr ready --owner terraphim --repo terraphim-ai`
2. **Evolution persistence** -- implement disk/S3 snapshot storage when needed
3. **Real LLM adapters** for evolution lesson extraction (deferred from #578)
4. **Integration test** with live ADF pipeline when evolution feature is enabled

## Gitea State

- Issue #578: CLOSED
- PR #1487: MERGED (squash), branch deleted
- Open issues: 322
- Open PRs: 30
