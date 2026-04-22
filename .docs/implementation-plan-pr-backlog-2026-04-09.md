# Implementation Plan: PR Backlog Prioritization & Merge Strategy

**Status**: Draft
**Research Doc**: `.docs/research-pr-backlog-2026-04-09.md`
**Author**: AI Analysis
**Date**: 2026-04-09
**Estimated Effort**: 3-4 hours (execution time, not including verification)

## Overview

### Summary
Execute a 5-phase strategy to clear the 17-open-PR backlog by consolidating duplicates, merging security/compliance fixes first, then proceeding to infrastructure (without Tauri) and feature work in dependency order. **Tauri is being moved to terraphim-ai-desktop repository.**

### Approach
1. Reconcile Gitea PR state with actual branch state
2. Close duplicate PRs to prevent merge conflicts
3. Execute merge in priority order (security → compliance → bugfixes → features)
4. Tauri-related PRs (#491) to be closed - Tauri moves to terraphim-ai-desktop

### Scope

**In Scope:**
- 17 open PRs across GitHub and Gitea
- Duplicate identification and closure
- Merge sequence execution (5 phases, Tauri removed)
- CI verification

**Out of Scope:**
- Tauri desktop application (moving to terraphim-ai-desktop)
- Investigating why duplicates were created
- Refactoring ADF swarm architecture
- Adding new features not in backlog

**Avoid At All Cost:**
- Merging duplicates before closing originals (would cause conflicts)
- Skipping security PRs for "faster" feature work
- Ignoring CI failures to "get things done"
- Merging Tauri-related PRs here (belongs in terraphim-ai-desktop)

## Architecture

### Merge Flow Diagram
```
                    ┌─────────────────────────────────────────┐
                    │          PHASE 1: CLEANUP               │
                    │  Close duplicates (#496, #503, #776)  │
                    │  Close #491 (Tauri → desktop repo)    │
                    └─────────────────┬───────────────────────┘
                                      ▼
                    ┌─────────────────────────────────────────┐
                    │      PHASE 2: SECURITY (Priority)      │
                    │  #486 RUSTSEC + Ollama binding        │
                    │  #412 Compliance verification         │
                    └─────────────────┬───────────────────────┘
                                      ▼
                    ┌─────────────────────────────────────────┐
                    │      PHASE 3: COMPLIANCE               │
                    │  #493 Consolidate license fixes        │
                    └─────────────────┬───────────────────────┘
                                      ▼
                    ┌─────────────────────────────────────────┐
                    │      PHASE 4: BUGFIXES                 │
                    │  #475, #477, #508, #512               │
                    └─────────────────┬───────────────────────┘
                                      ▼
                    ┌─────────────────────────────────────────┐
                    │      PHASE 5: FEATURES                 │
                    │  #520, #405, #519                     │
                    └─────────────────────────────────────────┘
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Security first | RUSTSEC is active CVE | Features first (rejected - security debt) |
| Close duplicates before merge | Prevent merge conflicts | Merge all, resolve conflicts (rejected - wasteful) |
| Tauri removed from repo | Moving to terraphim-ai-desktop | Keep here (rejected - bloat, separate concerns) |
| Consolidate license PRs | 3 PRs same fix | Merge all 3 (rejected - conflicts) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Merge all PRs simultaneously | Would cause massive conflicts | Unreleasable state |
| Ignore duplicate PRs | Merge conflicts inevitable | Wasted CI cycles |
| Skip CI verification | Security regressions possible | Production vulnerabilities |
| Merge GitHub #776 separately | Duplicate of Gitea #520 | Confusing state |

### Simplicity Check

**What if this could be easy?**
- Just close obvious duplicates first
- Run `cargo audit` and `cargo deny` to verify security
- Use tea CLI one-liners for merges
- If CI passes, merge; if not, debug

**Answer**: The plan IS simple. The execution is just methodical.

## File Changes

This plan does not modify source code. It coordinates GitHub/Gitea PR state.

### Git Operations

| Action | Branch | Purpose |
|--------|--------|---------|
| Reconcile state | main | Verify PR vs branch state |
| Push cleanup | main | Sync reconciled state |

### External State Changes (GitHub/Gitea)

| PR | Action | Command |
|----|--------|---------|
| GitHub #776 | Close (duplicate) | `gh pr close 776` |
| GitHub #775 | Close (already merged) | `gh pr close 775` |
| Gitea #491 | Close (Tauri moved) | `tea pulls close 491` |
| Gitea #496 | Close (duplicate) | `tea pulls close 496` |
| Gitea #503 | Close (duplicate) | `tea pulls close 503` |

## API Design

### tea/Gitea CLI Commands

```bash
# Close duplicate PRs
tea pulls close {496,503} --repo terraphim/terraphim-ai

# Merge PRs
tea pulls merge {ID} --repo terraphim/terraphim-ai

# Check PR status
tea pulls list --repo terraphim/terraphim-ai --state open
```

### gh/GitHub CLI Commands

```bash
# Close duplicate PRs
gh pr close 776 --repo terraphim/terraphim-ai
gh pr close 775 --repo terraphim/terraphim-ai

# Check PR status
gh pr list --repo terraphim/terraphim-ai --state open
```

## Test Strategy

### Pre-Merge Verification

| Test | Command | Pass Criteria |
|------|---------|---------------|
| Cargo audit | `cargo audit` | 0 vulnerabilities |
| Cargo deny | `cargo deny check licenses` | 0 failures |
| Clippy | `cargo clippy --workspace` | 0 warnings |
| Format | `cargo fmt -- --check` | 0 failures |
| Build | `cargo build --workspace` | Compiles |

### Post-Merge Verification

| Test | Purpose |
|------|---------|
| CI pipeline | All GitHub Actions pass |
| gitea-robot ready | Next PR becomes "ready" |

## Implementation Steps

### Step 1: Reconcile PR State
**Purpose**: Verify which PRs are actually open vs already merged
**Command**: `git log --oneline --all | head -50`
**Estimated**: 15 minutes

### Step 2: Close GitHub Duplicates
**Files**: GitHub remote
**Description**: Close GitHub PRs that duplicate Gitea work
**Commands**:
```bash
gh pr close 776 --repo terraphim/terraphim-ai --comment "Duplicate of Gitea #520"
gh pr close 775 --repo terraphim/terraphim-ai --comment "Already merged in main"
```
**Estimated**: 5 minutes

### Step 3: Close Gitea Duplicates
**Files**: Gitea remote
**Description**: Close Gitea PRs that are duplicates
**Commands**:
```bash
tea pulls close 496 --repo terraphim/terraphim-ai --comment "Duplicate of #493"
tea pulls close 503 --repo terraphim/terraphim-ai --comment "Duplicate of #493"
```
**Estimated**: 5 minutes

### Step 4: Merge Security PRs
**Files**: Gitea remote
**Description**: Merge RUSTSEC and compliance PRs
**Commands**:
```bash
# Verify first
cargo audit
cargo deny check licenses

# Then merge
tea pulls merge 486 --repo terraphim/terraphim-ai
tea pulls merge 412 --repo terraphim/terraphim-ai
```
**Estimated**: 30 minutes (including verification)

### Step 5: Merge Compliance PR
**Files**: Gitea remote
**Description**: Merge consolidated license fix
**Commands**:
```bash
tea pulls merge 493 --repo terraphim/terraphim-ai
```
**Estimated**: 10 minutes

### Step 6: Merge Bugfix PRs
**Files**: Gitea remote
**Description**: Merge low-risk bugfixes
**Commands**:
```bash
tea pulls merge 475 --repo terraphim/terraphim-ai
tea pulls merge 477 --repo terraphim/terraphim-ai
tea pulls merge 508 --repo terraphim/terraphim-ai  # if still open
tea pulls merge 512 --repo terraphim/terraphim-ai  # if still open
```
**Estimated**: 20 minutes

### Step 7: Close Tauri PR
**Files**: Gitea remote
**Description**: Tauri being moved to terraphim-ai-desktop
**Commands**:
```bash
tea pulls close 491 --repo terraphim/terraphim-ai --comment "Tauri moving to terraphim-ai-desktop repository"
```
**Estimated**: 5 minutes

### Step 8: Merge Feature PRs
**Files**: Gitea remote
**Description**: Merge final wave of features
**Commands**:
```bash
tea pulls merge 520 --repo terraphim/terraphim-ai  # ValidationService
tea pulls merge 405 --repo terraphim/terraphim-ai  # Phase 7 (verify phase completion first)
tea pulls merge 519 --repo terraphim/terraphim-ai  # Token tracking
```
**Estimated**: 45 minutes

### Step 9: Final Verification
**Purpose**: Ensure main is clean and pushable
**Commands**:
```bash
git pull --rebase gitea main
cargo build --workspace
cargo test --workspace
git push gitea main
```
**Estimated**: 30 minutes

## Rollback Plan

If issues discovered during merge:
1. Stop immediately - do not continue to next PR
2. Revert last merge: `git revert HEAD && git push gitea`
3. Debug issue before resuming
4. Use `git stash` if needed to isolate work

Feature flag: N/A (PR-level, not code-level)

## Migration

N/A - this is a PR coordination plan, not a code migration

## Dependencies

### New Dependencies
None - uses existing tea/gh CLI tools

### Dependency Updates
None

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to clear backlog | < 1 week | From now to all PRs closed/merged |
| CI pass rate | 100% | All merged PRs pass CI |
| Merge conflicts | 0 | Confirmed by closing duplicates first |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify #508, #512 actual state | Pending | Git history check |
| Confirm #516 blocks #520 | Pending | Issue investigation |
| Verify #405 phase completion | Pending | Review phase criteria |
| Tauri moved to terraphim-ai-desktop | Confirmed | terraphim-ai-desktop repo |

## Approval

- [ ] Research document reviewed and approved
- [ ] Implementation plan reviewed and approved
- [ ] Human approval received
- [ ] Execution can begin

## Summary

| Phase | PRs | Action | Est. Time |
|-------|-----|--------|-----------|
| 1 | Duplicates + Tauri | Close 5 PRs | 20 min |
| 2 | Security (#486, #412) | Merge | 30 min |
| 3 | Compliance (#493) | Merge | 10 min |
| 4 | Bugfixes (#475, #477, #508, #512) | Merge | 20 min |
| 5 | Features (#520, #405, #519) | Merge | 45 min |
| **Total** | | | **~2.5 hours** |
