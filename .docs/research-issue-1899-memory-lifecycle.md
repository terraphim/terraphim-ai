# Research Document: Unified Memory Lifecycle CLI for terraphim-agent

**Status**: Draft
**Author**: OpenCode Agent (disciplined-research)
**Date**: 2026-07-01
**Related Issue**: terraphim/terraphim-ai#1899
**Prior Research**: `~/cto-executive-system/research/terraphim-ai-memory-lifecycle-research.md` (2026-05-30)

## Executive Summary

Issue #1899 proposes consolidating the eight-stage agentic memory lifecycle (capture, distill, scope, provenance, retrieve, apply, validate, retire) behind a single `terraphim-agent memory` CLI namespace. Six of eight stages already exist as working primitives scattered across crates and CLI subcommands. This research updates the 2026-05-30 findings with current codebase state as of 2026-07-01 and confirms the feasibility of consolidation without new crates or schema changes.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | ADF agents repeat mistakes; second-run signal invisible; Q2 marketing lever blocked |
| Leverages strengths? | Yes | We already own all six working primitives (Aho-Corasick, learn, sessions, hooks, persistence) |
| Meets real need? | Yes | 42 ADF agents running; no metric that memory accelerates iteration; memco owns the narrative |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
The existing agent memory primitives (`learn`, `sessions`, `terraphim_agent_evolution`, `terraphim_hooks`, `terraphim_persistence`) are scattered and undiscoverable. `terraphim-agent --help` shows 22 commands but no coherent memory story. The Validate and Retire lifecycle stages have no programmatic API. There is no formal reliability rubric. ADF agents repeat mistakes.

### Impact
- Internal: agents cannot check whether a memory item is faithful, scoped, actionable, current, or risky.
- External: Terraphim has no memory narrative to compete with memco.ai field guide positioning.
- Q2: Memory consolidation is a lead-measure blocker for the marketing act.

### Success Criteria
1. `terraphim-agent memory --help` lists all ten lifecycle subcommands.
2. `MemoryEvolution`/`LessonsEvolution` are wired from library-only into CLI.
3. Memory Reliability Rubric (6 dimensions, markdown readout, < 10 min).
4. `MEMORY_POLICY.md` at repo root.
5. `second-run-signal.json` per ADF task.

## Current State Analysis (updated 2026-07-01)

### Existing Implementation

| Lifecycle stage | Existing primitive | Status |
|---|---|---|
| 1. Capture | `terraphim-agent learn capture`, `learn hook` (PostToolUse hook) | Working |
| 2. Distill | `learn compile`, `learn export-kg`, KG markdown entries | Working, manual trigger |
| 3. Scope | Role-based KGs, project KGs under `kg/projects/<slug>/` | Working |
| 4. Provenance | `terraphim-agent sessions` (Claude Code, Cursor connectors) | Working |
| 5. Retrieve | Aho-Corasick via `terraphim_automata`, `terraphim-agent search` | Working |
| 6. Apply | `terraphim_hooks` PreToolUse text replacement | Working |
| 7. Validate | `/evolve` skill (manual), judge pipeline (CJE-calibrated) | Partial |
| 8. Retire | `learned-rules.md` graduation/demotion (manual) | Partial |

### Current CLI Surface (verified 2026-07-01)

```
terraphim-agent learn
  capture      Capture a failed command as a learning
  list         List recent learnings
  query        Query learnings by pattern
  correct      Add correction to an existing learning
  correction   Record a user correction
  hook         Process hook input (stdin JSON)
  install-hook Install hook for AI agent
  procedure    Manage captured procedures
  compile      Compile corrections into thesaurus
  export-kg    Export corrections as KG markdown artefacts

terraphim-agent sessions
  sources      Detect available session sources
  list         List cached sessions
  search       Search sessions by query string
  stats        Show session statistics
```

No `memory` top-level command exists. The two CLI trees above are the raw material for the consolidation.

### Code Locations (verified 2026-07-01)

| Component | Location | Status |
|---|---|---|
| CLI binary entry | `crates/terraphim_agent/` (Cargo.toml, lib.rs currently missing from main -- see #3030) | Recoverable from agents repo |
| Evolution wrapper | `crates/terraphim_orchestrator/src/evolution.rs` | gated behind `evolution` feature |
| Evolution library | `terraphim_agent_evolution` (external crate, from `terraphim-agents` repo) | Provides `MemoryEvolution`, `LessonsEvolution`, `MemoryItem`, `Lesson` |
| Sessions | `crates/terraphim_sessions/src/` | 65-line lib.rs, `SessionService`, `ConnectorRegistry` |
| Persistence | `crates/terraphim_persistence/` | `Persistable` trait |
| Hooks | `crates/terraphim_hooks/` | Production PreToolUse hook |
| Judge pipeline | `~/cto-executive-system/automation/judge/` | Multi-tier, Kimi K2.5 deep tier |

### Key Finding: terraphim_agent_evolution is External

The `terraphim_agent_evolution` crate providing `MemoryEvolution` and `LessonsEvolution` is NOT in this repo. It lives in the separate `terraphim-agents` repository (Gitea #1910 polyrepo extraction). The orchestrator crate wraps it behind a feature flag (`evolution`). The CLI memory facade must either:

A. Call orchestrator's `EvolutionManager` wrapper (already wired), OR
B. Depend directly on `terraphim_agent_evolution` from the registry

**Recommendation**: Option A for the initial implementation (no new crate dependency), with Option B as a future optimisation if the orchestrator wrapper proves too heavy for CLI use.

### Key Finding: main is Non-Buildable

As of 2026-07-01, terraphim-ai `main` is missing critical files (#3030). The running `terraphim-agent` binary (v1.21.0) was built from the `terraphim-agents` repo. The implementation must work against the recovered source from that repo, not the current terraphim-ai main.

## Constraints

### Technical
- **No new crate**: Issue #1899 explictly forbids a 59th crate. CLI facade lives inside existing codepaths.
- **No new persistence schema**: Lifecycle metadata travels in existing `Persistable` types.
- **`evolution` feature gate**: Evolution primitives must work without and with the feature.
- **Bun runtime**: No npm for CLI tooling (TypeScript surfaces use Bun; this is all Rust).
- **main is broken**: Implementation source is `terraphim-agents` repo, not terraphim-ai `main` (#3030).

### Business
- Q2 lead measure: one PR merged per week for Terraphim platform.
- Marketing act required this week.
- 5/5 active project cap.

### Non-Functional

| Requirement | Target | Status |
|---|---|---|
| Capture latency | < 100ms | ~50ms (existing fast path) |
| Retrieval | < 50ms | ~10-30ms (Aho-Corasick) |
| Rubric run | < 10 min | n/a (new) |
| Second-run signal delta | measurable | n/a (new) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|---|---|---|
| No new crate (max reuse) | 58 crates already; new crate = governance cost | #1899 non-goals |
| Must expose via `terraphim-agent memory` CLI | Only CLI is user-visible and agent-accessible | Research finding: evolution is library-only |
| Rubric must reuse judge pipeline | Building a new scorer is 2+ weeks of work | Judge pipeline is CJE-calibrated and working |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|---|---|
| UI/dashboard for memory lifecycle | Not in CLI scope; separate project |
| Cross-organisation memory federation | Research says "non-goal" |
| New persistence backend | Keep `terraphim_persistence` v1.20.x |
| Replacing /evolve skill | Stays as human-in-the-loop weekly review |

## Dependencies

### Internal

| Dependency | Impact | Risk |
|---|---|---|
| `terraphim_agent_evolution` (registry) | Memory/Lessons APIs | Low -- stable external crate |
| `terraphim_sessions` (local) | Provenance source | Low -- working |
| `terraphim_hooks` (local) | Apply stage | Low -- production |
| `crates/terraphim_orchestrator/src/evolution.rs` | Wrapper for EvolutionManager | Low if `evolution` feature is used |
| Judge pipeline (cto-executive-system) | Rubric scoring | Low -- CJE-calibrated |

### External

| Dependency | Version | Risk | Alternative |
|---|---|---|---|
| `terraphim_agent_evolution` | registry from agents repo | Low | Option B: direct dep |
| Gitea API | v1 | Low | -- |
| memco field guide vocabulary | n/a | Low (concepts) | Coin own terms |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Scope creep to become "generic platform initiative" | High | High | Strict scope: CLI facade + rubric + policy. No new crates |
| `terraphim_agent_evolution` API mismatch for CLI | Medium | Medium | Spike: 10-line CLI binding in step 0 |
| main non-buildable (#3030) blocks integration tests | High | Medium | Build from agents repo; verify against that binary |
| Rubric judge cost spike | Low | Low | On-demand, not per-capture |
| Repository divergence between terraphim-ai and terraphim-agents | Medium | High | Coordinate via Gitea issues; PR to both repos |

### Open Questions

1. CLI implementation location: terraphim-ai (recover source from agents repo) or terraphim-agents directly? -- Assume terraphim-ai per #1899; recovery from agents repo for missing files.
2. Should `terraphim-agent memory` load `terraphim_agent_evolution` directly or via orchestrator's `EvolutionManager`? -- Assume direct (simpler CLI dep); orchestrator for ADF context injection.
3. Second-run signal: read from ADF run telemetry or from ADF artefact directory? -- Assume artefact directory (simpler, no telemetry dependency).

### Assumptions

| Assumption | Basis | Risk if Wrong |
|---|---|---|
| `terraphim_agent_evolution` is production-ready | Research doc from 2026-05-30 | Low -- verified in orchestrator tests |
| CLI live in `terraphim_agent`, not a new crate | #1899 non-goals | Standard pattern |
| `terraphim_persistence` schema stable | v1.20.2 in production | Low |
| CTO is single approver for rubric weights | Research doc | Low |

## Research Findings

### Key Insights

1. **We already implement 6 of 8 stages.** `learn` and `sessions` CLI trees cover capture, distill, scope, provenance, retrieve, and apply. Validate and retire are partial.
2. **Evolution library is external.** `terraphim_agent_evolution` lives in `terraphim-agents` repo and is imported via registry. The orchestrator wraps it. CLI must either wrap the wrapper or import directly.
3. **main is broken.** Implementation must target the recovered source from terraphim-agents repo, not current main (#3030).
4. **CLI facade is Net-New Shell.** Ten subcommands route to existing handlers (capture/learn, sessions, search, hooks) plus three new handlers (validate, rubric, second-run). The wiring is well-understood from the existing `learn`/`sessions` pattern.
5. **Second-run signal is measurable today.** ADF task artefacts already contain token counts per run. The delta computation is read-two-files arithmetic.

### Relevant Prior Art
- memco field guide: vocabulary source
- `/evolve` skill: manual weekly validate+retire loop
- `terraphim_agent_evolution`: BTreeMap-keyed versioned memory history
- `learn` CLI: 12 subcommands exposing capture/distill/retrieve

### Technical Spikes

| Spike | Purpose | Estimated Effort |
|---|---|---|
| Wire MemoryEvolution/LessonsEvolution into CLI list/show/export | Confirm CLI shape | 2-3 hours |
| Define second-run metric from ADF artefacts | Confirm measurable | 1 hour |
| Sketch rubric prompt + 6 dimensions for judge pipeline | Confirm judge fit | 2 hours |

## Recommendations

### Proceed/No-Proceed
**Proceed.** Low cost (facade), high value (marketing lever), risks contained (no schema changes, no new crate). Phase 2 design immediately.

### Scope
**In scope:**
- `terraphim-agent memory` CLI namespace with 10 subcommands
- Memory Reliability Rubric (judge-driven, markdown readout)
- `MEMORY_POLICY.md` at terraphim-ai root
- Second-run signal emission from ADF artefact directory

**Out of scope:**
- New persistence backend, cross-org federation, UI/dashboard
- Replacing `/evolve` skill
- Re-implementing judge pipeline

### Risk Mitigation
1. Phase 2 design must include a single umbrella issue with per-stage checklists.
2. Rubric prompt goes through judge calibration once before shipping.
3. Hard 2-week deadline; if it slips, ship rubric and policy only.

## Appendix

### Reference Materials
- memco field guide: https://www.memco.ai/field-guide
- Prior research: `~/cto-executive-system/research/terraphim-ai-memory-lifecycle-research.md`
- Feature request plan: `~/cto-executive-system/plans/terraphim-ai-memory-lifecycle-feature-request.md`
- 3-layer memory architecture: `~/cto-executive-system/docs/memory-architecture-3-layer.md`
- Gitea issue: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1899
