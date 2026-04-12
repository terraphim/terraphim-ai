# Implementation Plan: PR #790 Cleanup - Split into 4 Atomic PRs

**Status**: Draft
**Research Doc**: `.docs/research-pr790-cleanup.md`
**Author**: AI Design Agent
**Date**: 2026-04-12
**Estimated Effort**: 2-3 hours

## Overview

### Summary
Split PR #790 (4 commits, 20 files, +3,620/-183 lines) into 4 clean, atomic PRs with verified independence.

### Approach
Fresh branches from `origin/main` with file-level cherry-picks from the existing branch, squashed into single commits per concern.

### Scope

**In Scope:**
- Split existing work into 4 PRs
- Each PR compiles independently
- Each PR has its own tests passing

**Out of Scope:**
- New functionality
- Refactoring of existing changes
- Changing the content of any change

**Avoid At All Cost:**
- Re-interleaving concerns during the split
- Creating PRs that cannot compile on their own
- Modifying the actual code changes (only reorganize)

## Architecture

### Dependency Graph
```
PR-1 (tracker+symphony) ─┐
PR-2 (update)            ─┼─> PR-3 (events+listener) ─> PR-4 (misc)
                          │
              (independent, either first)
```

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| 7 separate PRs | Too granular, increases overhead | Review fatigue |
| Keep as single PR | Status quo problem | Blocks all fixes |
| Interactive rebase | Commits are interleaved | Very high conflict risk |
| Cherry-pick whole commits | Each commit has mixed concerns | Does not solve the problem |

### Simplicity Check
The simplest approach: extract files by concern into new branches. No rebasing, no cherry-picking of whole commits. Just `git checkout <commit> -- <paths>` to pick exact files.

## File Changes

### PR-1: fix(tracker): restore Gitea paging, claim verification, and symphony adapter

**New branch**: `fix/tracker-paging-claim-symphony` from `origin/main`

| File | Source Commit | Change Type |
|------|---------------|-------------|
| `crates/terraphim_tracker/src/gitea.rs` | `7aa18ec4` | Modified (+884/-81) |
| `crates/terraphim_tracker/tests/gitea_create_issue_test.rs` | `3efe03c8` | Modified (+2/-0) |
| `crates/terraphim_symphony/bin/symphony.rs` | `7aa18ec4` | Modified (+114/-14) |

### PR-2: fix(update): GNU/MUSL fallback for autoupdate and release pipeline

**New branch**: `fix/update-gnu-musl-fallback` from `origin/main`

| File | Source Commit | Change Type |
|------|---------------|-------------|
| `crates/terraphim_update/Cargo.toml` | `3efe03c8` | Modified (+1/-1) |
| `crates/terraphim_update/src/lib.rs` | `21dc532c` | Modified (+109/-58) |
| `.github/workflows/release-comprehensive.yml` | `21dc532c` | Modified (+25/-3) |

### PR-3: feat(agent): add event-driven listener with durable retries

**New branch**: `feat/events-listener` from `origin/main` (after PR-1 merged)

| File | Source Commit | Change Type |
|------|---------------|-------------|
| `crates/terraphim_orchestrator/src/control_plane/events.rs` | `1b22da3c` | New (697 lines) |
| `crates/terraphim_orchestrator/src/control_plane/mod.rs` | `3efe03c8` | Modified (+5/-0) |
| `crates/terraphim_orchestrator/src/lib.rs` | `3efe03c8` | Modified (+4/-0) |
| `crates/terraphim_orchestrator/src/dual_mode.rs` | `3efe03c8` | Modified (+2/-0) |
| `crates/terraphim_orchestrator/src/output_poster.rs` | `3efe03c8` | Modified (+7/-0) |
| `crates/terraphim_agent/src/listener.rs` | `7aa18ec4` + `21dc532c` | New (1560 lines) |
| `crates/terraphim_agent/src/main.rs` | `3efe03c8` | Modified (+47/-0) |
| `crates/terraphim_agent/Cargo.toml` | `3efe03c8` | Modified (+3/-0) |
| `Cargo.toml` | `3efe03c8` | Modified (+1/-1) |
| `Cargo.lock` | `3efe03c8` | Modified (auto-generated) |

### PR-4: fix(misc): validation latency, benchmark baseline, KG docs

**New branch**: `fix/misc-validation-benchmark-docs` from `origin/main`

| File | Source Commit | Change Type |
|------|---------------|-------------|
| `crates/terraphim_hooks/src/validation.rs` | `7aa18ec4` | Modified (+6/-3) |
| `crates/terraphim_validation/src/bin/performance_benchmark.rs` | `21dc532c` | Modified (+80/-6) |
| `.github/workflows/performance-benchmarking.yml` | `21dc532c` | Modified (+43/-2) |
| `crates/terraphim_agent/docs/src/kg/test_ranking_kg.md` | `1b22da3c` | New (13 lines) |

## Implementation Steps

### Step 1: Create PR-1 Branch (Tracker + Symphony)
**Files**: 3 files from 2 commits
**Description**: Extract tracker paging, claim verification, and symphony adapter
**Tests**: `cargo test -p terraphim_tracker` + `cargo check -p terraphim_symphony`
**Estimated**: 20 minutes

```bash
git fetch origin
git checkout -b fix/tracker-paging-claim-symphony origin/main

# Extract tracker changes (final version from commit 1, but only tracker files)
git checkout 7aa18ec4 -- crates/terraphim_tracker/src/gitea.rs
git checkout 7aa18ec4 -- crates/terraphim_symphony/bin/symphony.rs
git checkout 3efe03c8 -- crates/terraphim_tracker/tests/gitea_create_issue_test.rs

# Regenerate Cargo.lock if needed
cargo check -p terraphim_tracker
cargo check -p terraphim_symphony

# Verify tests pass
cargo test -p terraphim_tracker

# Commit and push
git add -A
git commit -m "fix(tracker): restore Gitea paging, claim verification, and symphony adapter

- Add proper pagination for Gitea issue fetching (50 per page)
- Add claim_issue() with idempotency, conflict detection, and verification
- Add ClaimResult/ClaimStrategy types for structured claim outcomes
- Add LinearTrackerAdapter in symphony for tracker API compatibility
- Support gitea-robot CLI with REST API fallback for claims

Refs #791"
```

### Step 2: Create PR-2 Branch (Update)
**Files**: 3 files from 2 commits
**Description**: Extract GNU/MUSL fallback for autoupdate
**Tests**: `cargo test -p terraphim_update`
**Estimated**: 15 minutes

```bash
git checkout -b fix/update-gnu-musl-fallback origin/main

git checkout 3efe03c8 -- crates/terraphim_update/Cargo.toml
git checkout 21dc532c -- crates/terraphim_update/src/lib.rs
git checkout 21dc532c -- .github/workflows/release-comprehensive.yml

cargo check -p terraphim_update
cargo test -p terraphim_update

git add -A
git commit -m "fix(update): GNU/MUSL fallback for autoupdate and release pipeline

- Add get_target_triples_with_fallback() for Linux dual-target support
- Try GNU first, fall back to MUSL if GNU binary not available
- Prioritize signed archives (.tar.gz) over raw binaries
- Skip Rust cache for x86_64-unknown-linux-gnu to prevent stale artifacts
- Add MUSL SHA256 fallback in Homebrew formula generation

Refs #791"
```

### Step 3: Create PR-4 Branch (Misc)
**Files**: 4 files from 3 commits
**Description**: Extract standalone fixes
**Tests**: `cargo test -p terraphim_hooks` + `cargo check -p terraphim_validation`
**Estimated**: 15 minutes

```bash
git checkout -b fix/misc-validation-benchmark-docs origin/main

git checkout 7aa18ec4 -- crates/terraphim_hooks/src/validation.rs
git checkout 21dc532c -- crates/terraphim_validation/src/bin/performance_benchmark.rs
git checkout 21dc532c -- .github/workflows/performance-benchmarking.yml
git checkout 1b22da3c -- crates/terraphim_agent/docs/src/kg/test_ranking_kg.md

cargo check -p terraphim_hooks
cargo check -p terraphim_validation
cargo test -p terraphim_hooks

git add -A
git commit -m "fix(misc): validation latency threshold, benchmark baseline schema, KG docs

- Increase validation test latency threshold to 5ms for CI stability
- Add cache warmup before validation benchmark
- Fix benchmark baseline JSON schema for performance-benchmarking.yml
- Add test ranking knowledge graph documentation"
```

### Step 4: Create PR-3 Branch (Events + Listener) - AFTER PR-1 MERGED
**Files**: 10 files from 3 commits
**Description**: Extract events module and agent listener (depends on PR-1)
**Tests**: `cargo test -p terraphim_orchestrator` + `cargo check -p terraphim_agent`
**Dependencies**: Step 1 (PR-1 merged)
**Estimated**: 30 minutes

```bash
# Wait for PR-1 to be merged, then:
git fetch origin
git checkout -b feat/events-listener origin/main

# Events module
git checkout 1b22da3c -- crates/terraphim_orchestrator/src/control_plane/events.rs
git checkout 3efe03c8 -- crates/terraphim_orchestrator/src/control_plane/mod.rs
git checkout 3efe03c8 -- crates/terraphim_orchestrator/src/lib.rs
git checkout 3efe03c8 -- crates/terraphim_orchestrator/src/dual_mode.rs
git checkout 3efe03c8 -- crates/terraphim_orchestrator/src/output_poster.rs

# Agent listener (use final version from last commit that touched it)
git checkout 21dc532c -- crates/terraphim_agent/src/listener.rs
git checkout 3efe03c8 -- crates/terraphim_agent/src/main.rs
git checkout 3efe03c8 -- crates/terraphim_agent/Cargo.toml

# Workspace deps (may need manual resolution after PR-1 and PR-2 merge)
git checkout 3efe03c8 -- Cargo.toml

cargo check -p terraphim_orchestrator
cargo check -p terraphim_agent
cargo test -p terraphim_orchestrator

git add -A
git commit -m "feat(agent): add event-driven listener with durable retries

- Add control_plane::events module with NormalizedAgentEvent types
- Add event normalization from Gitea webhook/poll sources
- Add terraphim_agent::listener for Gitea event-driven issue processing
- Implement durable retry logic for transient claim failures
- Add deduplication via event_id tracking
- Wire listener into agent main with ADF command parsing

Refs #523"
```

### Step 5: Clean Up
**Description**: Close old PR #790 and clean up old branch
**Estimated**: 5 minutes

```bash
# After all 4 PRs are merged:
gh pr close 790 --comment "Split into PR-XXX, PR-XXX, PR-XXX, PR-XXX"
git branch -D fix/autoupdate-gnu-musl-fallback
git push origin --delete fix/autoupdate-gnu-musl-fallback
```

## Rollback Plan

If any PR fails CI:
1. Fix the specific PR in isolation
2. If dependency ordering is wrong, re-order by merging independent PRs first
3. If Cargo.lock conflicts occur, regenerate with `cargo generate-lockfile`

## Test Strategy

### Per-PR Verification

| PR | Compile Check | Test Command | Expected |
|-----|---------------|--------------|----------|
| PR-1 | `cargo check -p terraphim_tracker -p terraphim_symphony` | `cargo test -p terraphim_tracker` | All pass |
| PR-2 | `cargo check -p terraphim_update` | `cargo test -p terraphim_update` | 28 tests pass |
| PR-3 | `cargo check -p terraphim_orchestrator -p terraphim_agent` | `cargo test -p terraphim_orchestrator` | All pass |
| PR-4 | `cargo check -p terraphim_hooks -p terraphim_validation` | `cargo test -p terraphim_hooks` | All pass |

### Final Integration Test
After all PRs merged: `cargo build --workspace && cargo test --workspace`

## Performance Considerations

No performance changes expected. The split is purely organizational.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify PR-3 compiles after PR-1 merge | Pending | Implementer |
| Resolve Cargo.lock conflicts if PR-1 and PR-2 change overlapping deps | Pending | Implementer |
| Create Gitea issues for each PR | Pending | Alex |

## Approval

- [ ] Research document reviewed
- [ ] Decomposition approved
- [ ] 4 PR structure approved
- [ ] Dependency ordering approved
- [ ] Human approval received
