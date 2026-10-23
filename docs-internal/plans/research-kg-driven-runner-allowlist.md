# Research: KG-Driven Dynamic Command Allowlist for Gitea Runner

**Status**: Draft
**Canonical Path**: `docs/plans/research-kg-driven-runner-allowlist.md`
**Change Slug**: `kg-driven-runner-allowlist`
**Author**: opencode session
**Date**: 2026-06-20
**Reviewers**: TBD

## Executive Summary

The Gitea runner's command allowlist is a hardcoded `const ALLOWLIST: &[&str]` in `policy.rs`. Two competing security PRs (#2740 removing `docker`, #2694 removing `sh`/`bash`) conflict because they both edit the same static array. The allowlist should be data-driven via a taxonomy markdown file, following the same `directive::` pattern already used by the ADF orchestrator's KG routing. This eliminates code-change PRs for allowlist edits and enables per-project command policies.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Stops the allowlist arms race; aligns runner config with KG-driven architecture |
| Leverages strengths? | Yes | Reuses the existing taxonomy `directive::` parsing pattern already proven in ADF routing |
| Meets real need? | Yes | Two conflicting PRs in the same week prove the static approach doesn't scale |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

`DeterministicPlanner` in `crates/terraphim_gitea_runner/src/policy.rs:182` hardcodes `ALLOWLIST` as a `const`. Every addition or removal requires a Rust code change, PR, review, merge, rebuild, and redeploy. Two PRs this week (#2740 and #2694) made opposing changes to the same const, producing a merge conflict that blocks the release.

### Impact

- **Release blocked**: PRs #2694 and #2783 and #2696 cannot rebase cleanly because they all conflict with the allowlist change in #2740.
- **Security decisions buried in code**: Whether `docker` or `sh` is allowed is a policy decision, not an implementation detail. It should be visible and editable as data.
- **No per-project policy**: All repos share the same allowlist. A repo that needs `python` (e.g. `digital-twins` SDK tests) either gets it globally or not at all.

### Success Criteria

1. The allowlist is defined in a taxonomy markdown file, not in Rust source.
2. Editing the allowlist requires no Rust changes — only a data file edit + runner restart.
3. Per-project overrides are supported (repo-level allowlist extends or restricts the global baseline).
4. The runner fails closed (deny-by-default) if the taxonomy file is missing or unparseable.
5. Existing tests pass without modification to their assertions about routing behaviour.

## Current State Analysis

### Existing Implementation

The `DeterministicPlanner` (policy.rs:57-92) implements the `PolicyPlanner` trait. It:
1. Extracts the program name from each workflow step command (after stripping env prefixes).
2. Checks membership in `const ALLOWLIST` (27 hardcoded entries).
3. Routes `cargo build/check/clippy/doc` to `rch exec` when `rch` is available.
4. Everything else on the allowlist runs on the host.

The planner is constructed via `DeterministicPlanner::detect()` in the binary entry point (`bin/terraphim-gitea-runner.rs:150`), then injected into `Poller::new()` as `Arc<dyn PolicyPlanner>`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Hardcoded allowlist | `policy.rs:182-186` | `const ALLOWLIST: &[&str]` — the problem |
| RCH cargo subcmds | `policy.rs:194` | `const RCH_CARGO_SUBCMDS` — also data, also static |
| Planner construction | `bin/terraphim-gitea-runner.rs:150` | `DeterministicPlanner::detect()` |
| RunnerConfig | `config.rs:8-40` | Has `active_repos` but no allowlist config |
| Poller (consumer) | `poller.rs:17-26` | Holds `Arc<P: PolicyPlanner>` |
| Taxonomy dir (ADF) | `docs/taxonomy/routing_scenarios/adf/` | Existing KG routing files (model routing) |

### Data Flow

```
Workflow YAML → workflow_payload::decode → ParsedWorkflow
  → DeterministicPlanner::compile(workflow)
    → for each step: route(command) → check ALLOWLIST → rewrite for rch if needed
  → ExecutionPlan { workflow, routes, trust_level }
  → task_worker executes each step per its route
```

### Integration Points

- **`PolicyPlanner` trait** (policy.rs:45-48): The seam. Any new planner that implements this trait drops into the existing pipeline without changing the poller or task worker.
- **Taxonomy directive pattern**: The ADF orchestrator already parses `route::`, `action::`, `trigger::`, `synonyms::` directives from markdown files in `docs/taxonomy/routing_scenarios/`. The runner can reuse this parsing pattern for `allow::`, `deny::`, `route_to::` directives.

## Constraints

### Technical Constraints

- **No KG crate in workspace**: The KG router (`kg_router.rs`) lives in `terraphim-agents-2301`, not in `terraphim-ai`. The runner must parse taxonomy files with its own lightweight parser, not import the orchestrator's KG router.
- **Runner is a standalone binary**: It runs as a systemd service on bigbox. It cannot query the orchestrator at runtime (different process, different repo). It must load its config at startup.
- **`PolicyPlanner` is async**: `compile()` returns `Result<ExecutionPlan>`. The taxonomy parser must be synchronous (loaded once at startup) so no async file I/O is needed.
- **No new workspace dependencies**: The runner crate has minimal deps. The parser should use `std::fs` and string splitting, matching how the ADF orchestrator parses taxonomy files.

### Business Constraints

- **Release-critical**: This blocks merging #2694, #2783, #2696. The fix must be small and low-risk.
- **Backward compatible**: Existing `.gitea/workflows/*.yml` files and `BUILD.md` files must continue to work without modification.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Startup latency (taxonomy parse) | < 50 ms | N/A (const lookup) |
| Allowlist edit → effect | Restart runner (seconds) | Rebuild + redeploy (minutes) |
| Memory overhead | < 10 KB for parsed rules | 0 (const in binary) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|---------|
| Allowlist is data, not code | Eliminates the PR conflict class entirely | #2740 vs #2694 conflict |
| Fails closed on missing file | Security: deny-by-default when config absent | CWE-78 command injection risk |
| Per-project overrides | Different repos need different tools | digital-twins needs python; terraphim-ai doesn't |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|---------------|
| Hot-reload (watch taxonomy file for changes) | Not needed for M1; runner restart is acceptable |
| Full KG Aho-Corasick matching for commands | Overkill; exact string match on program name is sufficient |
| Integrating with orchestrator's KgRouter | Different binary, different repo; coupling would be fragile |
| Sandboxed execution policy (Firecracker routing) | M2 scope; runner already has `CommandRoute::Firecracker` enum variant reserved |
| OIDC/JWT-based per-step auth | Out of scope; the allowlist is the policy gate |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `PolicyPlanner` trait | New planner implements it; no trait change needed | Low |
| `RunnerConfig` | Add `taxonomy_dir: Option<PathBuf>` field | Low |
| `DeterministicPlanner` | Either extend it or create `TaxonomyPlanner` | Medium — naming decision |

### External Dependencies

No new external dependencies. The parser uses only `std::fs` and string operations.

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Taxonomy file malformed at runtime | Low | High (runner rejects all commands) | Validate at startup; fall back to hardcoded baseline if parse fails |
| Permissive taxonomy accidentally allows dangerous commands | Medium | High (security regression) | Start with a deny-by-default baseline; require explicit `allow::` for every command |
| Taxonomy file not found on deployed runner | Medium | High (runner won't start) | Embed a default taxonomy in the binary as `include_str!` fallback |

### Open Questions

1. Should the taxonomy file live in the runner crate (`crates/terraphim_gitea_runner/default_policy.md`) or in the shared taxonomy dir (`docs/taxonomy/runner/`)? — **Recommendation: both** — embedded default in the crate, override path in `docs/taxonomy/runner/`.
2. Should `RCH_CARGO_SUBCMDS` also be data-driven? — **Yes**, but defer to a follow-up; the `route_to:: rch` directive can handle it.
3. Should the planner hot-reload on file change? — **No** for M1. systemd `Restart=on-failure` + `systemctl restart` is sufficient.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| The runner has filesystem access to the taxonomy directory at startup | Runner runs on bigbox where the repo is checked out | High — runner can't load policy | Yes — `RUNNER_CHECKOUT_DIR` points to repo root |
| Exact program-name matching is sufficient (no glob/regex needed) | Current `ALLOWLIST.contains(&prog)` is exact match | Low — if regex needed later, add then | Yes |
| The `PolicyPlanner` trait doesn't need to change | The trait takes `ParsedWorkflow` and returns `ExecutionPlan` | Low — trait is stable | Yes |

## Research Findings

### Key Insights

1. **The `PolicyPlanner` trait is the perfect seam.** It's already `Send + Sync`, takes `Arc<dyn PolicyPlanner>` in the poller, and has a clean `compile()` method. A new `TaxonomyPlanner` can implement it without touching the poller or task worker.

2. **The ADF taxonomy format is proven and simple.** Each directive is a line starting with `keyword:: value`. The runner only needs three new directives:
   - `allow:: cargo, make, bun, git, ...` — add to allowlist
   - `deny:: docker, curl, wget, ...` — explicit deny (overrides allow)
   - `route_to:: rch, cargo, build check clippy doc` — route certain program+subcommands to rch

3. **Per-project overrides need a second file or a section marker.** The simplest approach: a `docs/taxonomy/runner/<repo>.md` file per repo that extends the baseline `docs/taxonomy/runner/default.md`.

4. **The binary should embed a safe default.** Using `include_str!("default_policy.md")`, the binary always has a baseline even if the repo checkout is missing. The external taxonomy file overrides the embedded default.

### Relevant Prior Art

- **ADF KG routing** (`docs/taxonomy/routing_scenarios/adf/*.md`): The existing `route::`/`action::`/`trigger::` pattern. Our `allow::`/`deny::` directives follow the same format.
- **GitHub Actions `jobs.<id>.steps[*].run`**: The runner executes these. The allowlist gates which programs can appear in `run:`.
- **sudoers file**: The closest analog — a data file that defines what commands a user is allowed to run. Our taxonomy file plays the same role for the runner.

## Recommendations

### Proceed/No-Proceed

**Proceed.** The change is small (one new struct, one parser function, one taxonomy file), eliminates a class of merge conflicts, and aligns the runner with the KG-driven architecture used everywhere else in the system.

### Scope Recommendations

1. **Create `TaxonomyPlanner`** that implements `PolicyPlanner`, loading from a taxonomy file at construction time.
2. **Keep `DeterministicPlanner`** as the embedded-default fallback (its `ALLOWLIST` becomes the `include_str!` baseline).
3. **Add `taxonomy_dir: Option<PathBuf>` to `RunnerConfig`**, set via `RUNNER_TAXONOMY_DIR` env var.
4. **Defer per-project overrides** to a follow-up. M1 loads one global taxonomy file.

### Risk Mitigation Recommendations

- Write the taxonomy file first, run it through the parser in a unit test, then wire it into the planner.
- The parser should fail loud at startup (log error + deny all) rather than silently falling back to a permissive default.

## Next Steps

If approved:
1. Proceed to Phase 2 (Design): create `docs/plans/design-kg-driven-runner-allowlist.md`
2. Design the taxonomy file format, the parser, and the `TaxonomyPlanner` struct
3. Specify test strategy (unit tests for parser, integration test for planner)

## Appendix

### Current ALLOWLIST (the problem)

```rust
const ALLOWLIST: &[&str] = &[
    "cargo", "make", "bun", "bunx", "npm", "yarn", "pnpm", "rch", "sccache", "echo", "mkdir",
    "git", "ls", "cat", "cd", "cp", "mv", "rm", "chmod", "sh", "bash", "test", "export", "source",
    "true", "set", "rustup",
];
```

### Proposed taxonomy format (draft)

```markdown
# Runner Command Policy

## Baseline Allowlist
allow:: cargo, make, bun, bunx, npm, yarn, pnpm, rch, sccache
allow:: echo, mkdir, git, ls, cat, cd, cp, mv, rm, chmod
allow:: sh, bash, test, export, source, true, set, rustup

## Explicit Deny (security — overrides allow)
deny:: docker, curl, wget, nc, ncat, python, python3, perl, ruby

## RCH Routing
route_to:: rch, cargo, build check clippy doc
```
