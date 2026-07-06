# Implementation Plan: bollard 0.20.2 -> 0.21.0

**Status**: Approved
**Research Doc**: `docs/plans/research-pr909-bollard-021.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 15 minutes

## Overview

### Summary
Merge the Dependabot `bollard` minor update and verify the Docker executor still compiles with the `docker-backend` feature.

### Scope
**In scope:**
- Merge PR #909.
- Feature-gated builds for `terraphim_rlm` and `terraphim_validation`.

**Out of scope:**
- Live Docker integration tests.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.lock` | `bollard` 0.20.2 -> 0.21.0 |
| `crates/terraphim_rlm/Cargo.toml` | `bollard = { version = "0.20", optional = true }` -> `bollard = { version = "0.21", optional = true }` |
| `crates/terraphim_validation/Cargo.toml` | `bollard = { version = "0.20", optional = true }` -> `bollard = { version = "0.21", optional = true }` |

## Test Strategy

| Check | Command |
|-------|---------|
| RLM docker backend build | `cargo build -p terraphim_rlm --features docker-backend` |
| Validation docker feature build | `cargo build -p terraphim_validation --features docker` |

## Implementation Steps

### Step 1: Merge
`gh pr merge 909 --repo terraphim/terraphim-ai --squash`

### Step 2: Verify feature builds
```bash
cargo build -p terraphim_rlm --features docker-backend
cargo build -p terraphim_validation --features docker
```

### Step 3: Mirror
Push GitHub `main` to Gitea `main`.

## Rollback Plan

Revert merge if feature builds fail; reopen PR with failure output.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `bollard` | 0.20.2 | 0.21.0 | Docker API client update |
