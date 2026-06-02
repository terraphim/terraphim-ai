# Polyrepo topology (Gitea #1910)

terraphim-ai has been split from a single Cargo workspace into seven repositories
with a strictly acyclic dependency direction. Every cross-repo dependency is
consumed from the private Gitea cargo registry; there are no upward or cyclic edges.

## Layer map

```
terraphim-core
  -> terraphim-config-persistence
       -> terraphim-service
            -> terraphim-agents
            -> terraphim-kg-agents
                 -> terraphim-clients
                      -> terraphim-ai (server-monorepo)
```

Dependencies flow strictly left-to-right (downward). A crate may depend only on
crates in its own repo or in a repo to its left.

## Repositories

| Repo | Layer | Crates |
|---|---|---|
| `terraphim-core` | foundational | terraphim_types, terraphim_automata, terraphim_rolegraph, terraphim-markdown-parser, terraphim_test_utils |
| `terraphim-config-persistence` | config/storage | terraphim_config, terraphim_persistence, terraphim_settings, terraphim_atomic_client, terraphim_onepassword_cli |
| `terraphim-service` | service/middleware | terraphim_service, terraphim_middleware, terraphim_router, terraphim_usage, terraphim_ccusage, terraphim_file_search, haystack_core, haystack_jmap, haystack_grepapp, terraphim-session-analyzer, terraphim_spawner |
| `terraphim-agents` | agent system | terraphim_multi_agent, terraphim_orchestrator, terraphim_agent_evolution, terraphim_agent_messaging, terraphim_agent_registry, terraphim_agent_supervisor, terraphim_goal_alignment, terraphim_kg_orchestration, terraphim_task_decomposition, terraphim_tracker |
| `terraphim-kg-agents` | KG agents | terraphim_kg_agents, terraphim_kg_linter, terraphim_codebase_eval |
| `terraphim-clients` | clients/integrations | terraphim-cli, terraphim_agent, terraphim_mcp_server, terraphim_sessions, terraphim_lsp, terraphim_grep, terraphim_hooks, terraphim_update, terraphim_command_runtime, terraphim_negative_contribution |
| `terraphim-ai` (this repo) | server-monorepo | terraphim_server, terraphim_validation, terraphim-firecracker, terraphim_ai_nodejs + tooling (terraphim_dsm, terraphim_merge_coordinator, terraphim_workspace) + experimental (terraphim_rlm, terraphim_tinyclaw, ...) |

## Registry

All non-leaf repos publish their crates to the Gitea cargo registry under the
`terraphim` org:

```toml
# .cargo/config.toml
[registry]
global-credential-providers = ["cargo:token"]

[registries.terraphim]
index = "sparse+https://git.terraphim.cloud/api/packages/terraphim/cargo/"
```

The `terraphim` org is public-read (`auth-required: false`), so builds resolve
dependencies without a token. Publishing requires `CARGO_REGISTRIES_TERRAPHIM_TOKEN`
with a `Bearer ` prefix.

Dependency form across repos:

```toml
terraphim_types = { version = "1.20", registry = "terraphim" }
```

Within a repo, crates use path deps that also carry `version` + `registry` so the
same manifest works for local builds (path wins) and publishing (registry wins).

## Versioning

Lockstep-first: foundational + service tier stay at the 1.20.x baseline through the
split. Leaf client repos may take independent semver after one clean release cycle.

## Follow-ups

- Per-repo CI (build + clippy + test + sentrux + public-api on frozen crates).
- CODEOWNERS per repo.
- Promote `terraphim_mcp_server` to its own repo only if its tag frequency exceeds
  the rest of the clients repo by ~3x over two cycles.
