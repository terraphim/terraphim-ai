# Research Document: Fix CI Main Branch Failures After Merge

**Status**: Draft
**Author**: opencode
**Date**: 2026-06-14

## Executive Summary

CI on `main` has been broken since June 1st (commit `fbb5a85f9`, "scope duplicate GitHub CI workflows to manual-only"). Our merge from `task/2668-terraphim-lsp-foundation` didn't introduce new failures -- it inherited the pre-existing state. Three issues need fixing: (1) `terraphim_lsp` crate from the merge needs a `Cargo.toml` or exclusion from the workspace, (2) `rch exec` fails to resolve workspace members after directory deletions, (3) Gitea CI (primary) needs to work before GitHub (public mirror) matters.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | Release quality directly impacts user trust |
| Leverages strengths? | Yes | We own both CI runners and the `rch` infrastructure |
| Meets real need? | Yes | Release pipeline blocked; PR #2706 can't validate |

**Proceed**: Yes (3/3 YES)

## Problem Statement

### Description
After merging `task/2668-terraphim-lsp-foundation` into `main`, the `ci-main.yml` workflow fails at `Rust Format and Lint`, `Security Scan`, and `WASM Build` jobs. The root cause is pre-existing: `cargo metadata` fails because workspace member `crates/terraphim_agent` has no `Cargo.toml` on the runner. Our merge added `crates/terraphim_lsp` which may face the same issue.

### Impact
- Release tag `v1.20.4` has no green CI
- PR #2706 can't merge with confidence
- No automated CI on push -- all runs are manual `workflow_dispatch`
- Gitea CI (primary repo) is unvalidated

### Success Criteria
1. `ci-main.yml` workflow passes all jobs on `main`
2. `cargo check --workspace` succeeds without orphan crate errors
3. `terraphim_lsp` crate either compiles or is excluded from workspace
4. Gitea-side CI (`act_runner`) validates the same code

## Current State Analysis

### CI workflow architecture

| Workflow | Trigger | Status |
|----------|---------|--------|
| `ci-main.yml` | `workflow_dispatch` **only** | Manual-only, failing since June 1 |
| `ci-pr.yml` | `workflow_dispatch` **only** | Manual-only, has `needs` bug (missing jobs) |
| `ci-native.yml` | `workflow_dispatch` **only** | Legacy, manual-only |
| `release-comprehensive.yml` | `push: tags: v*` | **Only automated trigger** -- only runs on tag push |
| `sentrux-quality-gate.yml` | `workflow_dispatch` **only** | Manual-only |

**Key finding**: No workflow triggers on push to `main`. The `ci-main.yml` trigger was changed to `workflow_dispatch` only by commit `fbb5a85f9`. The release workflow `release-comprehensive.yml` is the only automated pipeline, and it only triggers on tags.

### Workspace crate analysis

**Members** (`Cargo.toml:4`):
```toml
members = ["crates/*", "terraphim_server", "terraphim_firecracker", "terraphim_ai_nodejs"]
```

**Excluded** (21 entries): `terraphim_agent`, `terraphim_agent_evolution`, `terraphim_orchestrator`, `terraphim_service`, etc.

**`terraphim_lsp` status**: The merge commit `5c48dabec` added `crates/terraphim_lsp/` with Cargo.toml and 10 source files. It is NOT in the `exclude` list. The `crates/*` glob WILL match it. If the directory exists on the runner, cargo will try to build it.

### Merge conflict resolution artefacts

The merge `9812a75c2..5c48dabec` changed 67 files (+30,666/-597). Key conflicts:
1. **modify/delete** conflicts on `terraphim_agent`, `terraphim_orchestrator`, `terraphim_spawner`, `terraphim_lsp` -- our branch modified them, main deleted them
2. **content** conflicts on `Cargo.toml`, `Cargo.lock`, `terraphim_rlm/Cargo.toml`, `terraphim_rlm/src/rlm.rs`
3. We resolved using `--theirs` for workspace-level files and kept our crate-level changes

Result: workspace Cargo.toml = main's version (no terraphim_lsp member), but terraphim_lsp directory exists with Cargo.toml (from our branch). This creates a mismatch where `crates/*` matches it but the workspace resolution doesn't include it properly.

### Gitea as primary, GitHub as public mirror

From `AGENTS.md`:
- **origin** (GitHub) -- push first, public
- **gitea** (Gitea) -- mirror, internal
- **No automated sync** -- push is manual: `git push origin main && git push gitea main`
- **Gitea CI** uses `act_runner` (self-hosted runner), not GitHub Actions
- **Gitea workflows** only create issues/comments, not build/test

### `rch exec` mechanism

`rch` is NOT a remote file sync tool. It's a local process queue that gates concurrent cargo builds (max 6 slots). The issue on the CI runner is that the runner's working directory doesn't have the same files as the repo -- it has stale directories from previous builds.

### Code locations

| Component | Location |
|-----------|----------|
| Workspace config | `Cargo.toml` (root) |
| ci-main.yml | `.github/workflows/ci-main.yml` |
| ci-pr.yml | `.github/workflows/ci-pr.yml` |
| release-comprehensive.yml | `.github/workflows/release-comprehensive.yml` |
| terraphim_lsp crate | `crates/terraphim_lsp/` (new from merge) |
| rch daemon | `/home/alex/.cache/rch/rch.sock` |
| rch workers | `/home/alex/.config/rch/workers.toml` |
| CI scripts | `scripts/ci-*.sh` |

## Constraints

### Vital Few (Max 3)

| Constraint | Why Vital | Evidence |
|------------|-----------|----------|
| Must not break existing workspace builds | `terraphim_agent` exclusion must remain; `terraphim_lsp` must be added or fixed | Cargo.toml members/exclude lists |
| Must work on both Gitea and GitHub | Dual-remote architecture; Gitea is primary, GitHub is public | AGENTS.md REMOTE_SYNC_PROTOCOL |
| Gitea CI (`act_runner`) is the authoritative CI | GitHub Actions is secondary; Gitea runs on self-hosted runner with real build environment | `.gitea/workflows/` directory |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Re-enabling push triggers on ci-main.yml | Commit `fbb5a85f9` deliberately made them manual-only; ADF agents handle orchestration |
| Fixing ci-pr.yml `needs` bug with missing rust-clippy/rust-compile jobs | Separate PR concern; not related to our merge |
| Fixing `cargo audit --no-default-features` flag | Pre-existing bug in workflow YAML, not our code change |
| WASM build failure | Same directory-sync issue as format/lint; will be fixed by root cause fix |
| Automating Gitea↔GitHub sync | Out of scope; ADF agents handle this |
| Normalising runner label casing (`x64` vs `X64`) | Cosmetic; no functional impact observed |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `terraphim_lsp` has compilation errors on fresh build | Medium | High | Build it locally first before pushing fix |
| Gitea CI uses different Rust version from GitHub | Low | Medium | Verify toolchain compatibility |
| `rch` queue contention from concurrent ADF agents | Low | Low | Already designed for 6 concurrent slots |

### Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `crates/terraphim_lsp` compiles on main with current deps | It compiled on the feature branch | Minor -- just needs Cargo.toml fix | No |
| Gitea `act_runner` is functional | ADF agents use it daily | None -- we'd notice immediately | No |
| `terraphim_agent` directory is genuinely deleted on the runner | CI error log confirms `No such file or directory` | Low | Yes (CI log) |

## Research Findings

### Key Insights

1. **The CI was already broken before our merge.** The `Rust Format and Lint` job has been failing since June 1st due to `cargo metadata` failing on the deleted `terraphim_agent` directory. Our merge added `terraphim_lsp` which faces the same class of issue.

2. **No automated CI on push.** All CI workflows are `workflow_dispatch` only. The only automated trigger is `release-comprehensive.yml` on tag push. This means every CI run is manual -- ADF agents or humans trigger them explicitly.

3. **The root cause is workspace member resolution.** The `ci-main.yml` runs `rch exec -- cargo fmt --all -- --check`. The `--all` flag tells cargo to process all workspace members. When a directory exists in `crates/` but has no `Cargo.toml`, or when a workspace member directory is missing but still referenced, `cargo metadata` fails.

4. **Gitea is the authoritative CI.** GitHub Actions is the public mirror. Fixes should be verified on Gitea first.

5. **The `terraphim_lsp` crate needs explicit handling.** Either: (a) add it to the workspace `exclude` list if we don't want it compiled, or (b) ensure `Cargo.toml` exists and it compiles. Since the LSP is a deliverable of this epic, option (b) is correct.

### Technical Spikes Needed

| Spike | Purpose | Effort |
|-------|---------|--------|
| Verify `terraphim_lsp` compiles on main | Confirm no hidden dependency issues | 5 min |
| Check Gitea CI runner status | Verify `act_runner` is healthy and has correct workspace | 5 min |

## Recommendations

### Proceed/No-Proceed
**Proceed.** Fix is low-risk: add `terraphim_lsp` to workspace members and ensure its `Cargo.toml` is present.

### Proposed fix (3 actions)

1. **Fix workspace `Cargo.toml`**: Add `crates/terraphim_lsp` to `members` list explicitly (or ensure `crates/*` resolves it properly by verifying the Cargo.toml exists)

2. **Verify `terraphim_lsp` compiles**: Run `cargo check -p terraphim_lsp` on main branch locally

3. **Clean up stale directories on CI runner**: The `terraphim_agent` directory (and others) have no `Cargo.toml` because they were deleted. The workspace correctly excludes them, so `cargo --all` should skip them. The CI failure (`No such file or directory`) suggests `cargo metadata` is looking for a Cargo.toml that was expected but not found -- this could be a cargo caching issue.

4. **Gitea CI**: Push to Gitea main and verify `act_runner` processes it correctly. The Gitea workflows may have different configuration.

### Files to modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `crates/terraphim_lsp` to members list |
| `crates/terraphim_lsp/Cargo.toml` | Verify it exists (should be present from merge) |
