# Documentation Gap Report -- 2026-05-07 (v4 Full Workspace Scan)

**Generated:** 2026-05-07
**Agent:** Ferrox (documentation-generator)
**Branch:** task/446-anthropic-probe-circuit-breaker-fix
**Method:** Scan all public items (`pub fn`, `pub async fn`, `pub struct`, `pub enum`, `pub trait`, `pub type`) lacking `///` doc comments across all workspace crates.

---

## Summary

| Metric | Value |
|--------|-------|
| Crates scanned | 55 |
| Prior reports scanned | 9 crates only |
| Total public items | ~5,700 |
| Total missing documentation | ~2,450 |
| Crates at 100% gap rate | 8 |
| Crates with >60% gap rate | 16 |

---

## Per-Crate Breakdown (full workspace)

| Crate | Total Pub | Missing | % Missing | Priority |
|-------|-----------|---------|-----------|----------|
| haystack_atlassian | 29 | 29 | 100% | High |
| haystack_core | 1 | 1 | 100% | Medium |
| haystack_discourse | 9 | 9 | 100% | Medium |
| terraphim_automata_py | 3 | 3 | 100% | Low |
| terraphim_ccusage | 11 | 11 | 100% | Medium |
| terraphim_kg_linter | 15 | 15 | 100% | Medium |
| terraphim_negative_contribution | 8 | 8 | 100% | Low |
| terraphim_rolegraph_py | 7 | 7 | 100% | Low |
| terraphim_codebase_eval | 13 | 11 | 84% | Medium |
| terraphim-markdown-parser | 21 | 17 | 80% | Medium |
| terraphim_kg_agents | 38 | 29 | 76% | Medium |
| terraphim_agent_application | 89 | 67 | 75% | Medium |
| terraphim-session-analyzer | 146 | 106 | 72% | High |
| haystack_grepapp | 19 | 13 | 68% | Medium |
| haystack_jmap | 9 | 6 | 66% | Medium |
| terraphim_usage | 75 | 49 | 65% | High |
| terraphim_persistence | 26 | 16 | 61% | High |
| terraphim_multi_agent | 333 | 191 | 57% | High |
| terraphim_task_decomposition | 85 | 49 | 57% | Medium |
| terraphim_middleware | 48 | 27 | 56% | High |
| terraphim_onepassword_cli | 11 | 6 | 54% | Low |
| terraphim_settings | 11 | 6 | 54% | Medium |
| terraphim_agent | 727 | 371 | 51% | Critical |
| terraphim_agent_registry | 106 | 54 | 50% | High |
| terraphim_config | 42 | 21 | 50% | High |
| terraphim_dsm | 12 | 6 | 50% | Low |
| terraphim_agent_evolution | 223 | 104 | 46% | High |
| terraphim_types | 318 | 148 | 46% | High |
| terraphim_automata | 125 | 55 | 44% | High |
| terraphim_agent_supervisor | 61 | 25 | 40% | Medium |
| terraphim_tracker | 57 | 23 | 40% | Medium |
| terraphim_atomic_client | 63 | 25 | 39% | Medium |
| terraphim_goal_alignment | 104 | 40 | 38% | Medium |
| terraphim_orchestrator | 611 | 236 | 38% | Critical |
| terraphim_sessions | 101 | 38 | 37% | Medium |
| terraphim_agent_messaging | 99 | 36 | 36% | Medium |
| terraphim_kg_orchestration | 63 | 21 | 33% | Medium |
| terraphim_test_utils | 12 | 4 | 33% | Low |
| terraphim_service | 231 | 75 | 32% | High |
| terraphim_validation | 355 | 116 | 32% | High |
| terraphim_github_runner | 110 | 35 | 31% | Medium |
| terraphim_build_args | 82 | 25 | 30% | Low |
| terraphim_router | 103 | 27 | 26% | Medium |
| terraphim_hooks | 32 | 8 | 25% | Low |
| terraphim_symphony | 126 | 32 | 25% | Medium |
| terraphim_tinyclaw | 238 | 56 | 23% | Medium |
| terraphim_update | 90 | 21 | 23% | Low |
| terraphim_spawner | 90 | 20 | 22% | Medium |
| terraphim_github_runner_server | 10 | 2 | 20% | Low |
| terraphim_rolegraph | 105 | 21 | 20% | Medium |
| terraphim_rlm | 257 | 50 | 19% | Medium |
| terraphim_workspace | 23 | 4 | 17% | Low |
| terraphim_file_search | 8 | 1 | 12% | Low |
| terraphim_cli | 19 | 2 | 10% | Low |
| terraphim_mcp_server | 34 | 3 | 8% | Low |

---

## Critical Crates (>200 public items, high gap rate)

### terraphim_agent (727 total, 371 missing, 51%)

Top undocumented items in `listener.rs`:
- `AgentIdentity`, `NotificationRuleKind`, `NotificationRule`, `DelegationPolicy`
- `GiteaConnection`, `ListenerConfig`, `for_identity`, `validate`, `load_from_path`
- `run_forever`, `run_once`

### terraphim_orchestrator (611 total, 236 missing, 38%)

Top undocumented items:
- `config.rs`: `PreCheckStrategy`, `QuickwitConfig`
- `mention.rs`: `load_or_now`
- `scheduler.rs`: `take_event_rx`
- `provider_budget.rs`: `is_empty`
- All `lib.rs` `pub mod` declarations (32+ entries)

### terraphim_validation (355 total, 116 missing, 32%)

High volume, medium priority -- many small validation helpers.

### terraphim_multi_agent (333 total, 191 missing, 57%)

Large surface, high gap rate. Entry-point types for agent coordination undocumented.

### terraphim_types (318 total, 148 missing, 46%)

Shared type definitions used across all crates. Documenting this crate delivers the highest cross-crate leverage.

---

## New Gaps Since v3

The v3 report scanned 9 crates; this v4 scan covers all 55. New crates brought into scope:

| Previously Unseen Crate | Missing Items |
|-------------------------|---------------|
| terraphim-session-analyzer | 106 |
| terraphim_multi_agent | 191 |
| terraphim_agent_evolution | 104 |
| terraphim_validation | 116 |
| terraphim_tinyclaw | 56 |
| terraphim_rlm | 50 |
| terraphim_usage | 49 |
| terraphim_task_decomposition | 49 |
| haystack_atlassian | 29 |
| terraphim_kg_agents | 29 |

---

## API Reference Snippets (new this run)

```rust
/// Returns `true` when the error string indicates a local environment or
/// configuration problem rather than a genuine provider health failure.
///
/// Errors matching this predicate must not advance the circuit-breaker failure
/// counter because they reflect local setup (missing CLI tool, routing config,
/// C1 allow-list) -- not transient API unavailability.
fn is_environment_error(error: &str) -> bool { /* ... */ }

/// Aggregated health state for all configured LLM providers.
///
/// Maintains per-provider circuit-breaker state.  Updated on every probe cycle
/// via [`ProviderHealthMap::update_from_results`].
pub struct ProviderHealthMap { /* ... */ }

/// Notification rule controlling which Gitea events an agent listener accepts.
///
/// Matched against incoming webhook payloads before dispatching to the agent
/// handler.  See [`ListenerConfig::validate`] for validation semantics.
pub struct NotificationRule { /* ... */ }

/// Unified type definitions shared across all terraphim workspace crates.
///
/// Import via `terraphim_types::prelude::*` to bring common types into scope.
pub mod prelude { /* ... */ }
```

---

## Recommendations

### Priority 1 -- Critical (address within current sprint)

1. **`terraphim_types`**: Document `prelude`, `Document`, `SearchQuery`, `RelevanceFunction` -- highest leverage as these appear in every crate.
2. **`terraphim_agent` listener.rs**: Document `ListenerConfig`, `NotificationRule`, `DelegationPolicy` entry points (public API contract).
3. **`terraphim_orchestrator` config.rs**: Document `PreCheckStrategy`, `OrchestratorConfig`, `QuickwitConfig`.

### Priority 2 -- High (address within next two sprints)

4. **`terraphim_multi_agent`**: Module-level `//!` docs on at least the top-level types.
5. **`terraphim_agent_evolution`**: Document trait definitions and public constructors.
6. **`haystack_atlassian`**: Currently 100% undocumented -- add crate-level `//!` and document `AtlassianHaystack::new`.

### Priority 3 -- Ongoing

7. CI gate: add `cargo doc --no-deps 2>&1 | grep "^warning.*missing documentation"` as a soft-fail step.
8. Enforce `#![warn(missing_docs)]` in `terraphim_types` and `terraphim_orchestrator` as a start.

---

## Comparison to Prior Reports

| Date | Run | Crates | Items Missing |
|------|-----|--------|---------------|
| 2026-04-29 | morning | 9 (struct/enum/trait only) | 564 |
| 2026-05-07 | v1 | 9 (struct/enum/trait only) | 307 (-45%) |
| 2026-05-07 | v2 | 9 (+ fn/async fn) | 395 |
| 2026-05-07 | v3 | 9 (+ fn/async fn, afternoon) | 395 (unchanged) |
| 2026-05-07 | v4 | **55** (full workspace) | ~2,450 |

The step from 395 to ~2,450 reflects expanded scope (55 crates vs 9), not regression.

Theme-ID: doc-gap
