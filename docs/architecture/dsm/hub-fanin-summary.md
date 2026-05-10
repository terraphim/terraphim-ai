# Phase 1 §4 — Hub Fan-In Summary

Captured 2026-05-10 via `cargo tree --workspace -i <crate> --depth 1 --prefix none`.

| Hub crate | Brief said | Actual | Delta | Notes |
|-----------|------------|--------|-------|-------|
| `terraphim_types` | 25 | **39** | +14 | 49 workspace members and 39 depend directly on this. ~80% of the workspace pulls in types. |
| `terraphim_automata` | 18 | **25** | +7 | Includes terraphim_types itself (which depends on automata? — unusual; verify in §15) |
| `terraphim_rolegraph` | 12 | **19** | +7 | Self-listing observed; treat as 18 effective. |
| `terraphim_persistence` | 11 | **19** | +8 | Higher than expected; consistent with persistence-as-SDK pattern |
| `terraphim_config` | 10 | **14** | +4 | Inside acceptable error band but still understated by brief |

## Implications for split

1. **`terraphim_types` is even more central than expected.** Any split must extract this as the foundational repo first; rolling back is impractical once 39 crates depend on a moved version.
2. **`terraphim_persistence` has more reverse-deps than stated** — confirming the cycle break (Phase 1 §7) is high-value: it touches 19 dependent crates' dep graphs.
3. **`terraphim_config` understatement is small** — closest to brief; smallest blast radius for the cycle break PR's config-side changes.
4. **`terraphim_automata` and `terraphim_rolegraph` self-list as their own dependents** — this is a `cargo tree -i` quirk where the search tree includes the search target. Subtract 1 from each if measuring "external dependents only": automata 24, rolegraph 18.

## Top dependents per hub

(Full lists in `reverse-deps/<crate>.txt`. Top categories of consumers:)

- `terraphim_types`: every haystack, every agent crate, every kg crate, all clients (cli/agent/mcp_server/server), middleware, service, router, orchestrator, sessions, all utility crates (file_search, hooks, kg_linter, negative_contribution, usage). Truly foundational.
- `terraphim_automata`: hubs that need text matching — middleware, service, kg_*, agent_registry, kg_linter.
- `terraphim_rolegraph`: knowledge-graph consumers — kg_*, multi_agent, service, middleware.
- `terraphim_persistence`: anything storing state — config, multi_agent, agent_supervisor, agent_evolution, service, sessions, usage.
- `terraphim_config`: orchestrators and clients — service, middleware, mcp_server, agent (TUI), cli, multi_agent, persistence (cycle), server.

## Discrepancy explanation

The brief's numbers were authored by an Explore agent reading Cargo.toml manifests and counting `path = "..."` entries that point intra-workspace. The actual counts include test/dev-deps, transitive 1-step inclusions through cargo's resolver, and the self-listing quirk. Both views are valid; the live `cargo tree` numbers are authoritative for sizing the split.

## Action

Phase 2 §13 topology must use the **higher** numbers as the basis for blast-radius estimates per cluster cut.
