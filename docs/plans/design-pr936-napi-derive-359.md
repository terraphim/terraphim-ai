# Implementation Plan: napi-derive 3.5.7 -> 3.5.9

**Status**: Approved
**Research Doc**: `docs/plans/research-pr936-napi-derive-359.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 5 minutes (merge-only)

## Overview

### Summary
Merge the cleaned Dependabot lockfile patch for `napi-derive`.

### Scope
**In scope:**
- Merge PR #936.
- Build `terraphim_ai_nodejs` on main.

**Out of scope:**
- Bumping `napi` crate.
- Changing Node.js binding source code.

**Avoid At All Cost:**
- Reintroducing unrelated `Cargo.lock` drift.

## Architecture

No architecture changes. The derive macro is a compile-time dependency of the Node bindings crate.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.lock` | `napi-derive` 3.5.7 -> 3.5.9; `napi-derive-backend` 5.0.5 -> 5.1.1 |

## Test Strategy

### Build Verification
| Check | Command |
|-------|---------|
| Node bindings build | `cargo build -p terraphim_ai_nodejs` |
| JS smoke test | `cd terraphim_ai_nodejs && node test_knowledge_graph.js` if available |

## Implementation Steps

### Step 1: Merge
**Action:** `gh pr merge 936 --repo terraphim/terraphim-ai --squash`

### Step 2: Mirror to Gitea
**Action:** Push GitHub `main` to Gitea `main`.

### Step 3: Build smoke test
**Action:** `cargo build -p terraphim_ai_nodejs`
**Verification:** No macro errors; binary/shared object produced.

## Rollback Plan

If build fails:
1. Revert merge commit.
2. Push revert to both remotes.
3. Reopen PR #936 with build log.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `napi-derive` | 3.5.7 | 3.5.9 | Patch backend update |
| `napi-derive-backend` | 5.0.5 | 5.1.1 | Transitive backend update |
