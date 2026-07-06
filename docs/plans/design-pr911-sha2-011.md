# Implementation Plan: sha2 0.10.9 -> 0.11.0

**Status**: Approved
**Research Doc**: `docs/plans/research-pr911-sha2-011.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 15 minutes

## Overview

### Summary
Merge the Dependabot `sha2` major update and verify the cryptographic codepaths still compile and pass tests.

### Scope
**In scope:**
- Merge PR #911.
- Build workspace.
- Run orchestrator webhook tests.

**Out of scope:**
- Refactoring HMAC usage.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.lock` | `sha2` 0.10.9 -> 0.11.0 |
| `crates/terraphim_update/Cargo.toml` | `sha2 = "0.10"` -> `sha2 = "0.11"` |
| `crates/terraphim_validation/Cargo.toml` | `sha2 = "0.10"` -> `sha2 = "0.11"` |

## Test Strategy

| Check | Command |
|-------|---------|
| Workspace build | `cargo build --workspace` |
| Webhook tests | `cargo test -p terraphim_orchestrator webhook` |
| Workspace clippy | `cargo clippy --workspace --all-targets -- -D warnings` |

## Implementation Steps

### Step 1: Merge
`gh pr merge 911 --repo terraphim/terraphim-ai --squash`

### Step 2: Verify builds
`cargo build --workspace`

### Step 3: Verify HMAC path
`cargo test -p terraphim_orchestrator webhook`

### Step 4: Mirror
Push GitHub `main` to Gitea `main`.

## Rollback Plan

Revert merge if workspace build or webhook tests fail; reopen PR with failure output.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `sha2` | 0.10.9 | 0.11.0 | Cryptographic dependency update |
