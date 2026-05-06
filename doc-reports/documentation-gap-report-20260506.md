# Documentation Gap Report — $(date +%Y-%m-%d)

## Executive Summary

| Metric | Value |
|--------|-------|
| Total missing-doc warnings | **3,985** |
| Crates affected | **46** |
| Worst offender | `terraphim_agent_evolution` (513 warnings) |

## Warnings by Crate (descending)

| Crate | Warnings | Severity |
|-------|----------|----------|
| terraphim_agent_evolution | 513 | Critical |
| terraphim_orchestrator | 459 | Critical |
| terraphim_validation | 443 | Critical |
| terraphim-firecracker | 341 | Critical |
| terraphim_multi_agent | 292 | High |
| terraphim-session-analyzer | 164 | High |
| terraphim_server | 138 | High |
| terraphim_types | 128 | High |
| terraphim_agent_messaging | 128 | High |
| terraphim_usage | 121 | High |
| terraphim_service | 115 | High |
| terraphim_tinyclaw | 105 | High |
| terraphim_agent | 99 | Medium |
| terraphim_automata | 88 | Medium |
| terraphim_rlm | 75 | Medium |
| terraphim_agent_supervisor | 73 | Medium |
| terraphim_kg_linter | 58 | Medium |
| terraphim-markdown-parser | 52 | Medium |
| terraphim_tracker | 47 | Medium |
| terraphim_agent_registry | 46 | Medium |
| terraphim_middleware | 45 | Medium |
| terraphim_kg_orchestration | 40 | Medium |
| terraphim_config | 38 | Medium |
| terraphim_persistence | 32 | Medium |
| terraphim_spawner | 30 | Medium |
| terraphim_ccusage | 29 | Medium |
| terraphim_router | 29 | Medium |
| terraphim_task_decomposition | 31 | Medium |
| terraphim_rolegraph | 24 | Medium |
| terraphim_ai_nodejs | 22 | Medium |
| terraphim_onepassword_cli | 18 | Medium |
| terraphim_update | 16 | Low |
| terraphim_sessions | 13 | Low |
| terraphim_workspace | 11 | Low |
| terraphim_settings | 8 | Low |
| terraphim_mcp_server | 8 | Low |
| terraphim_negative_contribution | 8 | Low |
| terraphim_file_search | 4 | Low |
| haystack_core | 4 | Low |
| terraphim_kg_agents | 3 | Low |
| terraphim_atomic_client | 5 | Low |
| terraphim_goal_alignment | 25 | Low |

## Common Patterns

1. **Missing module docs** — `//!` header absent in `lib.rs` files
2. **Missing struct/enum docs** — Public types lack `///` summaries
3. **Missing field docs** — Struct fields undocumented
4. **Missing variant docs** — Enum variants undocumented
5. **Missing function docs** — Public `fn` items lack `///`

## Recommendation

Adopt `RUSTDOCFLAGS="-D missing-docs"` in CI once the top 5 crates drop below 50 warnings each. Target the critical crates first:
- `terraphim_agent_evolution`
- `terraphim_orchestrator`
- `terraphim_validation`
- `terraphim-firecracker`
- `terraphim_multi_agent`

