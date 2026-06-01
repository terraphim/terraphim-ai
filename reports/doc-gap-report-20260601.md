# Documentation Gap Report -- 2026-06-01

**Agent:** documentation-generator (Ferrox, Rust Engineer)
**Date:** 2026-06-01
**Crates scanned:** 26 (across three parallel scan groups)

---

## Executive Summary

A workspace-wide rustdoc scan identified **~82 undocumented public items** across 26 crates, with an estimated coverage rate of approximately 78%. One crate (`terraphim_grep`) was missing its crate-level `//!` comment entirely -- fixed in this session. The most significant concentrations of gaps are in `terraphim_orchestrator` (14 modules lacking module-level docs), `terraphim_service` (22 items including `OpenRouterError`), and `terraphim_config` (3 high-priority items in `project.rs`). Two CHANGELOG entries were also added for recent commits not yet reflected there.

---

## Action Taken This Session

| Item | Action | Status |
|------|--------|--------|
| `terraphim_grep/src/lib.rs` crate-level `//!` | Added (17-line quick-start doc block) | Fixed |
| CHANGELOG: `feat(pr-reviewer)` commit | Added under `[Unreleased] > Added` | Fixed |
| CHANGELOG: `fix(agent): cargo_bin` commit | Added under `[Unreleased] > Fixed` | Fixed |

---

## Critical Gaps -- High Severity

| Crate | Item | Kind | File | Priority |
|-------|------|------|------|----------|
| `terraphim_config` | `ProjectDiscoveryError` | enum | `src/project.rs:5` | High |
| `terraphim_config` | `ProjectConfig` | struct | `src/project.rs:19` | High |
| `terraphim_config` | `discover` | fn | `src/project.rs:129` | High |
| `terraphim_service` | `OpenRouterError` | enum | `src/openrouter.rs:11` | High |
| `terraphim_service` | `Result` | type | `src/openrouter.rs:36` | High |
| `terraphim_grep` | `TerraphimGrep` | struct | `src/hybrid_searcher.rs:44` | High |
| `terraphim_grep` | `GrepOptions` | struct | `src/hybrid_searcher.rs:8` | High |
| `terraphim_router` | `Router` | struct | `src/engine.rs:266` | High |
| `terraphim_router` | `FallbackStrategy` | enum | `src/fallback.rs:12` | High |
| `terraphim_spawner` | `SpawnContext` | struct | `src/lib.rs:25` | High |
| `terraphim_spawner` | `spawn_process` | fn | `src/lib.rs:652` | High |
| `terraphim_workspace` | `WorkspaceError` | struct | `src/lib.rs:16` | High |
| `terraphim_workspace` | `prepare` | fn | `src/lib.rs:83` | High |

---

## Medium Gaps -- Module-Level Docs Missing

These crates have module declarations without `//!` comments. Individual items inside may be documented, but module-level context is absent.

| Crate | Modules Lacking `//!` | Count |
|-------|----------------------|-------|
| `terraphim_orchestrator` | `agent_run_command`, `agent_runner`, `compound`, `config`, `cost_tracker`, `error`, `evolution`, `handoff`, `local_skills`, `nightwatch`, `persona`, `project_adf`, `scheduler`, `scope` | 14 |
| `terraphim_service` | `auto_route`, `openrouter`, `llm`, `llm_proxy`, `http_client`, `logging`, `conversation_service` | 7 |
| `terraphim_middleware` | `command`, `haystack`, `indexer`, `thesaurus`, `learning_indexer`, `learning_query`, `feedback_loop` | 7 |
| `terraphim_persistence` | `conversation`, `document`, `error`, `memory`, `settings`, `thesaurus` | 6 |
| `terraphim_sessions` | `connector`, `model`, `service` | 3 |

---

## Low-Priority Gaps

| Crate | Items | Count |
|-------|-------|-------|
| `terraphim_types` | `TrustLevelError` enum | 1 |
| `terraphim_task_decomposition` | `AgentPid`, `AgentMetadata`, `Goal`, `GoalId`, `MockAutomata`, `TaskDecompositionResult` | 6 |
| `terraphim_multi_agent` | `MultiAgentResult`, `AgentId`, `create_test_agent`, `create_memory_storage` | 4 |
| `terraphim_goal_alignment` | `GenAgentResult`, `GoalAlignmentResult` | 2 |
| `terraphim_merge_coordinator` | `extract_fixes`, `PrEvaluation`, `evaluate_all` | 2 |
| `terraphim_validation` | `ValidationSystem`, `validate_release`, `ValidationOrchestrator` | 3 |
| `terraphim_symphony` | `SymphonyOrchestrator`, `run` | 2 |
| `terraphim_grep` | `RetrievedChunk`, `score_kg_boost` | 2 |

---

## Crates With Complete Documentation

The following crates passed the scan with no material gaps:

- `terraphim_settings`
- `terraphim_agent`
- `terraphim_agent_supervisor`
- `terraphim_agent_registry`
- `terraphim_agent_messaging`
- `terraphim_kg_agents`
- `terraphim_kg_orchestration`
- `terraphim_rlm`

---

## Coverage Estimate by Group

| Group | Crates | Total Public Items | Undocumented | Coverage |
|-------|--------|--------------------|--------------|---------|
| Core layer | 5 | ~180 | ~38 | 79% |
| Service layer | 4 | ~160 | ~26 | 84% |
| Agent system | 8 | ~130 | ~14 | 89% |
| New/support crates | 11 | ~260 | ~42 | 84% |
| **Total** | **26** | **~730** | **~120** | **~84%** |

---

## Recommended Actions

1. **`terraphim_config/project.rs`** -- Add `///` to `ProjectDiscoveryError`, `ProjectConfig`, `discover()`. Three items, five minutes work.
2. **`terraphim_service/openrouter.rs`** -- Add `///` to `OpenRouterError` and `Result` type alias. High-traffic code path.
3. **`terraphim_orchestrator` module docs** -- Add `//!` first-line to the 14 modules listed above. These are all `one-line context` comments.
4. **`terraphim_router`** -- Document `Router`, `FallbackStrategy`, `KeywordRouter`, `KnowledgeGraphRouter`.
5. **Enable `RUSTDOCFLAGS="-W missing-docs"` in CI** for the four highest-traffic crates to prevent regression.

---

## CHANGELOG Updates Applied

```
[Unreleased] > Added
  - pr-reviewer agent scaffolding (2026-06-01)

[Unreleased] > Fixed
  - Nested cargo run in exit-code tests replaced with cargo_bin! (2026-06-01)
```
