# Documentation Gap Report

Generated: 2026-04-28 02:50 UTC
Workspace version: 1.16.37

## Summary

| Metric | Value |
|--------|-------|
| Total crates scanned | 45 |
| Total undocumented public items | 916 |
| Crates with >10 gaps | 22 |
| Crates with >50 gaps | 4 |

## Undocumented Public Items by Crate

| Crate | Count | Severity |
|-------|-------|----------|
| terraphim_agent | 161 | Critical |
| terraphim_multi_agent | 84 | High |
| terraphim_orchestrator | 83 | High |
| terraphim_validation | 67 | High |
| terraphim_service | 49 | Medium |
| terraphim_types | 48 | Medium |
| terraphim_usage | 41 | Medium |
| terraphim_automata | 33 | Medium |
| terraphim-session-analyzer | 27 | Medium |
| haystack_atlassian | 25 | Medium |
| terraphim_middleware | 22 | Medium |
| terraphim_agent_messaging | 17 | Low |
| terraphim_build_args | 16 | Low |
| terraphim_agent_registry | 16 | Low |
| terraphim-markdown-parser | 16 | Low |
| terraphim_tinyclaw | 14 | Low |
| terraphim_kg_linter | 14 | Low |
| terraphim_persistence | 14 | Low |
| terraphim_task_decomposition | 13 | Low |
| terraphim_atomic_client | 12 | Low |
| terraphim_symphony | 11 | Low |
| terraphim_ccusage | 11 | Low |
| terraphim_config | 10 | Low |
| terraphim_rlm | 10 | Low |
| terraphim_sessions | 9 | Low |
| haystack_discourse | 8 | Low |
| terraphim_negative_contribution | 8 | Low |
| terraphim_update | 7 | Low |
| terraphim_rolegraph | 7 | Low |
| terraphim_agent_application | 7 | Low |
| terraphim_agent_evolution | 7 | Low |
| terraphim_goal_alignment | 7 | Low |
| terraphim_router | 7 | Low |
| terraphim_agent_supervisor | 6 | Low |
| terraphim_spawner | 5 | Low |
| terraphim_github_runner_server | 4 | Low |
| terraphim_github_runner | 4 | Low |
| terraphim_kg_orchestration | 3 | Low |
| terraphim_file_search | 3 | Low |
| terraphim_settings | 3 | Low |
| terraphim_kg_agents | 2 | Low |
| terraphim_mcp_server | 2 | Low |
| haystack_core | 1 | Low |
| terraphim_cli | 1 | Low |
| terraphim_workspace | 1 | Low |

## Representative Examples

### Critical Gaps (>100 items)

#### terraphim_agent

- `crates/terraphim_agent/src/listener.rs:10` -- pub struct `AgentIdentity`
- `crates/terraphim_agent/src/listener.rs:19` -- pub fn `new`
- `crates/terraphim_agent/src/listener.rs:27` -- pub fn `resolved_gitea_login`
- `crates/terraphim_agent/src/listener.rs:31` -- pub fn `accepted_target_names`
- `crates/terraphim_agent/src/listener.rs:41` -- pub enum `NotificationRuleKind`


## Recommendations

1. **Immediate (Critical crates)**: Add `#![warn(missing_docs)]` to `terraphim_agent`, `terraphim_orchestrator`, `terraphim_multi_agent` and document top-level module entries
2. **Short-term**: Target crates with >50 gaps for focused documentation sprints
3. **Process**: Require doc comments on all new `pub` items in PR checklist
4. **Automation**: Add `cargo doc --no-deps` with `-D warnings` to CI gate

## CHANGELOG Status

- Last updated: 2026-04-28 (this report)
- Previous version: 1.14.0 (2026-03-22)
- Commits since: ~726
- Version bump: 1.14.0 -> 1.16.37
