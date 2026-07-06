# Implementation Plan: axum 0.7.9 -> 0.8.9 in terraphim_validation

**Status**: Approved
**Research Doc**: `docs/plans/research-pr923-axum-089.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 15 minutes (merge + verification)

## Overview

### Summary
Merge the Dependabot PR that upgrades `terraphim_validation` to `axum 0.8`, unifying the workspace axum graph.

### Scope
**In scope:**
- Merge PR #923.
- Build and test `terraphim_validation`.
- Build the workspace to ensure no downstream breakage.

**Out of scope:**
- Refactoring handlers.
- Adopting new `axum 0.8` conveniences.

**Avoid At All Cost:**
- Merging without running `terraphim_validation` tests.
- Leaving duplicate `axum` versions in the lockfile.

## Architecture

No architecture changes. The crate remains a validation/test helper; only the framework minor version changes.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_validation/Cargo.toml` | `axum = "0.7"` -> `axum = "0.8"` |
| `Cargo.lock` | Removes `axum 0.7.9`, `axum-core 0.4.5`, `matchit 0.7.3`; updates all dependents to single `axum 0.8.9` graph |

## Test Strategy

### Build/Clippy/Test
| Check | Command |
|-------|---------|
| Validation build | `cargo build -p terraphim_validation` |
| Validation tests | `cargo test -p terraphim_validation` |
| Validation clippy | `cargo clippy -p terraphim_validation --all-targets -- -D warnings` |
| Workspace build | `cargo build --workspace` |

## Implementation Steps

### Step 1: Merge
**Action:** `gh pr merge 923 --repo terraphim/terraphim-ai --squash`

### Step 2: Verify validation crate
**Action:** `cargo build -p terraphim_validation && cargo test -p terraphim_validation && cargo clippy -p terraphim_validation --all-targets -- -D warnings`

### Step 3: Verify workspace
**Action:** `cargo build --workspace`

### Step 4: Mirror to Gitea
**Action:** Push GitHub `main` to Gitea `main`.

## Rollback Plan

If any verification fails:
1. Revert merge commit.
2. Push revert to both remotes.
3. Reopen PR #923 with the failing command and output.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `axum` | 0.7.9 | 0.8.9 | Unify workspace on axum 0.8 |
| `axum-core` | 0.4.5 | removed | Consolidated into 0.5 |
| `matchit` | 0.7.3 | removed | Consolidated into 0.8 |
