# Implementation Plan: Gitea PR Merge Execution

**Status**: Draft
**Research Doc**: `.docs/research-gitea-pr-merge-plan.md`
**Author**: AI Agent
**Date**: 2026-04-05
**Estimated Effort**: 2-3 hours

## Overview

### Summary
Merge 8 active PRs and close 5 dead PRs from Gitea in a sequenced, conflict-minimizing order with build verification at each step.

### Approach
Rebase-and-merge strategy: for each PR, rebase the branch onto current `main`, resolve conflicts, verify build, merge via Gitea API, push to both remotes.

### Scope

**In Scope:**
- Close 5 superseded/duplicate PRs
- Merge 8 active PRs in dependency order
- Build verification after each merge
- Push to gitea and origin remotes

**Out of Scope:**
- Cleaning agent-generated noise from branches (post-merge cleanup)
- Running full test suite after each merge
- Reviewing PR content for correctness (assumed pre-reviewed)

**Avoid At All Cost:**
- Force-pushing to `main`
- Merging without build verification
- Merging #239 after other Cargo.toml-touching PRs (cascading conflicts)

## Architecture

### Merge Flow
```
main (current)
  |
  Step 0: Close dead PRs (#157, #184, #345, #349, #327)
  |
  Step 1: #239 (workspace deps) --- CRITICAL PATH
  |         resolve 12 Cargo.toml conflicts
  |
  Step 2: #283 (CI fix)
  |         trivial rebase, 1 file
  |
  Step 3: #156 (offline TUI)
  |         rebase, resolve main.rs
  |
  Step 4: #245 (registry consolidation)
  |         rebase after #239
  |
  Step 5: #250 (ADF warp-drive-theme)
  |         resolve Cargo.lock
  |
  Step 6: #154 (MeSH benchmarks)
  |         rebase after #239
  |
  Step 7: #185 (SharedLearning + automata)
  |         rebase after #239 + #250
  |
  Step 8: #353 (CVE security fix)
  |         rebase after all above
  |
  Final: push to origin + gitea
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Rebase-then-merge (not squash) | Preserves commit history for traceability | Squash merge loses per-commit context |
| Merge #239 first | Touches 48 Cargo.toml files; prevents cascading conflicts | Merging others first creates 48-file rebase nightmare |
| Close #327 instead of merging | #250 contains the actual feature code; #327 is 43-commit integration noise | Merging #327 would pull in ADRs but also merge-commits and noise |
| Push to gitea per-step, origin at end | Gitea is interim work remote; origin is release quality | Pushing to origin per-step risks incomplete state |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Cherry-pick specific commits from #353 | Too surgical for a security fix branch | Missing context, harder to verify |
| Merge all in one integration branch | Loses per-PR traceability | Can't bisect to find which PR broke something |
| Run full test suite per merge | Adds 30+ min per step | Time waste; tests pass on individual PR branches |

## File Changes

### Step 0: Close Dead PRs (no file changes)

| PR # | Action | Gitea-robot Command |
|------|--------|---------------------|
| #157 | Close + comment "Already merged into main" | `gitea-robot_close_issue` or `gitea-robot_merge_pull` |
| #184 | Close + comment "Superseded by PR #185 which includes all changes plus SharedLearning" | Close via API |
| #345 | Close + comment "Superseded by PR #353 which includes these changes plus security fix" | Close via API |
| #349 | Close + comment "Superseded by PR #353 which includes these changes plus security fix" | Close via API |
| #327 | Close + comment "Superseded by PR #250 which contains the actual feature code" | Close via API |

### Step 1: PR #239 - Workspace Dependency Consolidation

**Files:** ~50 Cargo.toml files + Cargo.lock
**Conflict Resolution:**
```bash
git checkout main
git checkout -b merge/239-workspace-deps gitea/feat/fff-kg-boosted-file-search
git rebase main
# Resolve 12 Cargo.toml conflicts: accept theirs (workspace = true conversions)
# Regenerate Cargo.lock:
cargo build --workspace
git add -A && git rebase --continue
```

**Verification:**
```bash
cargo build --workspace
cargo clippy --workspace
```

### Step 2: PR #283 - CI Checkout Clean

**Files:** `.github/workflows/ci-native.yml` (2 lines)
**Conflict Resolution:** Simple rebase, unlikely to conflict.
```bash
git checkout -b merge/283-ci-fix gitea/task/ci-cleanup-checkout
git rebase main
```

### Step 3: PR #156 - Offline TUI Default

**Files:** 5 files in `crates/terraphim_agent/` + `docs/plans/`
**Conflict Resolution:** `main.rs` may conflict due to prior changes. Accept incoming changes and verify build.
```bash
git checkout -b merge/156-offline-tui gitea/task/153-offline-default-tui
git rebase main
# Resolve main.rs conflict
cargo build --workspace
```

### Step 4: PR #245 - Registry Consolidation

**Files:** 14 files in `crates/terraphim_multi_agent/`
**Conflict Resolution:** Should be clean after #239 merges workspace deps.
```bash
git checkout -b merge/245-registry gitea/feat/consolidate-registry-and-deps
git rebase main
```

### Step 5: PR #250 - ADF Warp-Drive Theme

**Files:** 13 files in `crates/terraphim_orchestrator/` + `terraphim_tracker/`
**Conflict Resolution:** Cargo.lock conflict -- regenerate with `cargo build`.
```bash
git checkout -b merge/250-adf gitea/feature/warp-drive-theme
git rebase main
# Cargo.lock: regenerate
cargo build --workspace
git add Cargo.lock && git rebase --continue
```

### Step 6: PR #154 - MeSH Benchmarks

**Files:** 9 files, mostly new (additive)
**Conflict Resolution:** `terraphim_tracker/src/gitea.rs` may conflict with #250 changes. Resolve by accepting both additions.
```bash
git checkout -b merge/154-mesh gitea/task/phase3-4-mesh-benchmark-scalability-v2
git rebase main
```

### Step 7: PR #185 - SharedLearning + Automata

**Files:** 49 files across multiple crates
**Conflict Resolution:** Most complex rebase. Shares base with #184 (closed). Will conflict with #239's Cargo.toml changes and #250's orchestrator changes. May need manual resolution.
```bash
git checkout -b merge/185-shared-learning gitea/task/141-improve-id-generation
git rebase main
# Expect conflicts in:
# - Cargo.toml files (workspace deps from #239)
# - terraphim_orchestrator/ (ADF additions from #250)
# - terraphim_automata/ (if changed by #239)
# Resolve each, then:
cargo build --workspace
```

### Step 8: PR #353 - CVE Security Fix

**Files:** 16 files (Cargo.toml, deny.toml, tinyclaw changes)
**Conflict Resolution:** After all above merges, should be a clean rebase. Agent noise files can be excluded during rebase if desired.
```bash
git checkout -b merge/353-cve gitea/task/341-remediation-rustls-webpki-security
git rebase main
# May want to drop commits that add noise files (.opencode/reviews/, curl, .md)
# Use git rebase -i to drop noise commits (but no -i in CI, do manually)
```

## Test Strategy

### Build Verification (After Each Merge)
| Check | Command | Expected |
|-------|---------|----------|
| Build | `cargo build --workspace` | Success |
| Lint | `cargo clippy --workspace 2>&1 | grep error` | No errors |
| Format | `cargo fmt --check` | Clean (or no new issues) |

### Final Verification (After All Merges)
| Check | Command | Purpose |
|-------|---------|---------|
| Full build | `cargo build --workspace` | Verify no cascading issues |
| Clippy | `cargo clippy --workspace` | Verify no new warnings |
| Git log | `git log --oneline -30` | Verify clean history |

## Implementation Steps

### Step 0: Close Dead PRs
**Action:** Close 5 PRs via Gitea API
**Time:** 5 minutes
**Dependencies:** None

### Step 1: Merge PR #239 (Workspace Deps)
**Action:** Rebase, resolve 12 Cargo.toml conflicts, build verify, merge
**Time:** 20 minutes
**Dependencies:** Step 0

### Step 2: Merge PR #283 (CI Fix)
**Action:** Rebase (trivial), build verify, merge
**Time:** 5 minutes
**Dependencies:** Step 1

### Step 3: Merge PR #156 (Offline TUI)
**Action:** Rebase, resolve main.rs conflict, build verify, merge
**Time:** 10 minutes
**Dependencies:** Step 1

### Step 4: Merge PR #245 (Registry Consolidation)
**Action:** Rebase (clean after #239), build verify, merge
**Time:** 10 minutes
**Dependencies:** Step 1

### Step 5: Merge PR #250 (ADF Warp-Drive)
**Action:** Rebase, resolve Cargo.lock, build verify, merge
**Time:** 15 minutes
**Dependencies:** Step 1

### Step 6: Merge PR #154 (MeSH Benchmarks)
**Action:** Rebase, resolve gitea.rs conflict, build verify, merge
**Time:** 10 minutes
**Dependencies:** Step 1, Step 5

### Step 7: Merge PR #185 (SharedLearning + Automata)
**Action:** Rebase (complex, 49 files), resolve multiple conflicts, build verify, merge
**Time:** 30 minutes
**Dependencies:** Step 1, Step 5

### Step 8: Merge PR #353 (CVE Security Fix)
**Action:** Rebase (should be clean), build verify, merge
**Time:** 10 minutes
**Dependencies:** All previous steps

### Step 9: Final Push
**Action:** Push main to gitea and origin remotes
**Time:** 5 minutes
**Dependencies:** Step 8

## Rollback Plan

If any merge breaks the build:
1. `git reset --hard HEAD~1` to undo the merge commit
2. Investigate the conflict, fix, and retry
3. If unrecoverable, reset to the known-good commit before the failing step

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify ADR docs from #327 aren't lost by closing it | Pending | Human |
| Decide whether to strip agent noise from #353 before merge | Pending | Human |
| Confirm #156's local-merged status matches gitea remote | Pending | Human |

## Approval

- [ ] Research document reviewed and approved
- [ ] Merge order approved
- [ ] Human approval to proceed with merges
