# Implementation Plan: napi 3.9.4 -> 3.10.3

**Status**: Approved
**Research Doc**: `docs/plans/research-pr935-napi-3103.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 5 minutes (merge-only)

## Overview

### Summary
Merge the cleaned Dependabot lockfile minor update for `napi`.

### Scope
**In scope:**
- Merge PR #935.
- Build `terraphim_ai_nodejs` and run JS smoke tests.

**Out of scope:**
- Source code changes to Node bindings.
- Bumping `napi-derive` separately (handled by PR #936).

**Avoid At All Cost:**
- Reintroducing unrelated `Cargo.lock` drift.

## Architecture

No architecture changes. `napi` is the runtime Node-API binding dependency of `terraphim_ai_nodejs`.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.lock` | `napi` 3.9.4 -> 3.10.3 |

## Test Strategy

### Build Verification
| Check | Command |
|-------|---------|
| Node bindings build | `cargo build -p terraphim_ai_nodejs` |
| JS smoke test | `cd terraphim_ai_nodejs && node test_knowledge_graph.js` |

## Implementation Steps

### Step 1: Merge
**Action:** `gh pr merge 935 --repo terraphim/terraphim-ai --squash`

### Step 2: Mirror to Gitea
**Action:** Push GitHub `main` to Gitea `main`.

### Step 3: Smoke test
**Action:** Build the Node bindings and run a JS smoke test.
**Verification:** No build errors; smoke test passes.

## Rollback Plan

If build or smoke test fails:
1. Revert merge commit.
2. Push revert to both remotes.
3. Reopen PR #935 with failure details.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `napi` | 3.9.4 | 3.10.3 | Upstream correctness fixes for JS error handling |
