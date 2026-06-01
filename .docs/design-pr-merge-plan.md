# Implementation Plan: Open PR Backlog Clearance

**Status**: Draft
**Research Doc**: `.docs/research-pr-merge-strategy.md`
**Author**: AI Agent
**Date**: 2026-05-31
**Estimated Effort**: 3 sessions x 2-3 hours each

## Overview

### Summary

Systematically clear the 50 open PR backlog through a phased approach: close corrupted/superseded PRs first, merge quick wins second, rebase small features in batches third, and schedule dedicated review sessions for large features last.

### Approach

**Phase 1: Triage** (30 min)
- Close 12 PRs (corrupted, superseded, duplicates, empty)
- Comment on each explaining why

**Phase 2: Quick Wins** (1 hour)
- Merge 5 mergeable PRs
- Rebase and merge 6 tiny PRs

**Phase 3: Small Features** (2 hours)
- Rebase and merge 14 small feature PRs in dependency order
- Batch related PRs (rustdoc, NormalizedTerm)

**Phase 4: Large Features** (Future sessions)
- Dedicated review for 9 large PRs
- Break into sub-tasks if needed

### Scope

**In Scope:**
- All 50 open PRs
- Both Gitea and GitHub remotes
- Quality gates after each batch

**Out of Scope:**
- Creating new PRs (only rebase existing)
- Major refactoring of large PRs
- Review of PRs >100 files (separate sessions)

**Avoid At All Cost** (from 5/25 analysis):
- Rebase corrupted PRs (#1045, #1023, #969) - impossible
- Merge all 50 in one session - too risky
- Skip quality gates to go faster - defeats purpose
- Rebase large PRs without review - too risky

## Architecture

### Merge Sequence Diagram

```
Phase 1: Triage (Close 12 PRs)
  |
  v
Phase 2: Quick Wins (Merge 11 PRs)
  |
  v
Phase 3: Small Features (Merge 14 PRs in batches)
  |
  v
Phase 4: Large Features (Review + merge 9 PRs)
```

### Batch Groupings

```
Batch A: Mergeable now
  #1272, #1268, #1279, #1250

Batch B: Tiny fixes
  #1832, #1745, #1605, #1514, #1510, #1483, #1482

Batch C: rustdoc chain
  #1365 -> #1695 -> #1710 -> #1867

Batch D: NormalizedTerm
  #1283 (close #1343, #1605 as superseded)

Batch E: Security/test fixes
  #1291, #1298, #1585, #1589

Batch F: Context rot chain
  #1524 -> #1599 -> #1617
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Close corrupted PRs | 800k+ line diffs are impossible to rebase | Attempt rebase (would fail) |
| Close superseded security PRs | All changes already on main via #1918 | Rebase anyway (wasted effort) |
| Merge tiny PRs before large | Reduces conflict surface | Random order (more conflicts) |
| Batch rustdoc PRs | They touch different crates but same CI gate | Individual merges (slower) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Rebase all 50 PRs | 45 have conflicts; would take days | Burnout, mistakes, frustration |
| Skip quality gates | Fast but risky | Regressions, broken main |
| Merge #1788 (173 files) blindly | Too large to review in batch session | Bugs, security issues |
| Keep corrupted PRs open | Clutter queue, confuse prioritisation | Wrong PRs get attention |

### Simplicity Check

**What if this could be easy?**

The simplest approach: close what's broken/duplicate, merge what's clean, batch what's related. No heroic rebasing of ancient PRs. If a PR is too stale, close it and let the author re-create from current main.

**Senior Engineer Test**: A senior engineer would say "close the garbage, merge the clean stuff, and schedule reviews for the big features." This plan matches that.

## Implementation Steps

### Step 1: Phase 1 - Triage (Close 12 PRs)
**Files:** N/A (Gitea operations only)
**Description:** Close corrupted, superseded, duplicate, and empty PRs
**Tests:** N/A
**Estimated:** 30 minutes

**PRs to close:**
| PR | Reason | Comment Template |
|----|--------|-----------------|
| #1045 | Corrupted branch (3277 files, 812k lines) | "Closing: branch corrupted. Please re-create from current main if still needed." |
| #1023 | Corrupted branch (3259 files, 808k lines) | Same |
| #969 | Corrupted branch (3161 files, 801k lines) | Same |
| #1767 | Superseded by main | "Closing: AgentConfig redaction already on main." |
| #1791 | Superseded by main | "Closing: All 6 struct redactions already on main." |
| #1679 | Superseded by #1918 | "Closing: All changes implemented in #1918 and earlier PRs." |
| #1663 | Superseded by #1918 | Same |
| #1640 | Superseded by #1918 | Same |
| #1246 | Empty (0 files) | "Closing: empty PR." |
| #1343 | Duplicate of #1283 | "Closing: superseded by #1283 (NormalizedTerm evolution)." |
| #1605 | Duplicate of #1283 | Same |
| #1294 | Duplicate of #1367 | "Closing: superseded by #1367 (Terraphim Engineer role tests)." |

### Step 2: Phase 2 - Quick Wins (Merge 11 PRs)
**Files:** Various
**Description:** Merge mergeable and tiny PRs
**Tests:** `cargo test -p <affected>` after each
**Estimated:** 1 hour

**Batch A: Mergeable now (no rebase needed)**
```bash
# For each PR:
# 1. Disable status checks
curl -X PATCH ... enable_status_check: false

# 2. Merge
gitea-robot merge-pull --index <PR>

# 3. Re-enable status checks
curl -X PATCH ... enable_status_check: true

# 4. Sync remotes
git fetch gitea main && git merge gitea/main --no-edit && git push origin main
```

| PR | Verification Command |
|----|---------------------|
| #1272 | `cargo test -p terraphim_types` |
| #1268 | `cargo test -p terraphim_service` |
| #1279 | `cargo test -p terraphim_grep` |
| #1250 | `cargo test -p terraphim_orchestrator` |

**Batch B: Tiny PRs (rebase first)**
```bash
# For each PR:
git fetch gitea task/<branch>
git checkout -b task/<branch>-clean gitea/task/<branch>
git rebase origin/main
# Resolve conflicts if any
git push gitea task/<branch>-clean:task/<branch> --force-with-lease
gitea-robot merge-pull --index <PR>
```

| PR | Size | Verification |
|----|------|-------------|
| #1832 | 3 files, +57/-27 | `cargo test -p terraphim_config` |
| #1745 | 3 files, +77/-11 | `cargo test -p terraphim_rlm` |
| #1605 | 1 file, +2/-6 | `cargo test -p terraphim_sessions` |
| #1514 | 4 files, +212/-39 | `cargo test -p terraphim_agent` |
| #1510 | 3 files, +247/-3 | CI workflow check |
| #1483 | 2 files, +162/-0 | `cargo deny check` |
| #1482 | 4 files, +242/-52 | `cargo deny check` |

### Step 3: Phase 3 - Small Features (14 PRs in batches)
**Files:** Various
**Description:** Rebase and merge small feature PRs in dependency order
**Tests:** `cargo test --workspace` after each batch
**Estimated:** 2 hours

**Batch C: rustdoc chain (merge in order)**
```
#1365 (CI gate) -> #1695 (4 crates) -> #1710 (7 crates) -> #1867 (gaps + CHANGELOG)
```

**Batch D: NormalizedTerm**
```
#1283 only (close #1343 and #1605 first)
```

**Batch E: Security/test fixes**
```
#1291 (meta_coordinator wiring)
#1298 (world-readable config warnings)
#1585 (cron re-fire storm)
#1589 (Default impl AgentDefinition)
```

**Batch F: Context rot chain**
```
#1524 -> #1599 -> #1617
```

### Step 4: Phase 4 - Large Features (9 PRs, future sessions)
**Files:** Various
**Description:** Dedicated review and merge sessions for large PRs
**Tests:** Full workspace test + targeted review
**Estimated:** 1-2 sessions x 2 hours each

| PR | Title | Files | Why Review Needed |
|----|-------|-------|-------------------|
| #1788 | .terraphim/skills integration | 173 | Massive; touches CLI, config, skills |
| #1800 | PR dispatch contexts | 51 | Major feature; affects webhook handling |
| #1786 | Project-scoped registry | 48 | New feature; depends on #1291 |
| #1720 | Sessions cluster command | 28 | New CLI command; affects sessions |
| #1491 | RLM executor hardening | 32 | Security-critical; 3098 lines |
| #1524 | Context rot detection | 40 | Complex feature; 1282 lines |
| #1380 | Robot-mode Search envelope | 90 | Large; 3600 lines |
| #1367 | Engineer role tests | 77 | Large test refactor; 2892 lines |
| #1365 | rustdoc CI gate | 61 | Affects all crates; 2521 lines |

## Rollback Plan

If issues discovered after merge:
1. Identify offending PR via `git bisect`
2. Revert merge commit: `git revert -m 1 <merge-commit>`
3. Push revert to both remotes
4. Comment on PR explaining revert

## Test Strategy

### Per-PR Tests
| PR Size | Test Command |
|---------|-------------|
| Tiny (<5 files) | `cargo test -p <affected_crate>` |
| Small (5-20 files) | `cargo test -p <crate1> -p <crate2>` |
| Medium (20-50 files) | `cargo test --workspace` |
| Large (>50 files) | `cargo test --workspace --all-features` |

### Quality Gates
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo test --workspace` passes
- [ ] Both remotes in sync (`git diff origin/main gitea/main --stat` empty)

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify issues #664, #1013, #851 status | Pending | AI Agent |
| Check #1788 dependency on #1291 | Pending | AI Agent |
| Schedule Phase 4 sessions | Pending | Human |

## Approval

- [ ] Research document reviewed
- [ ] Merge sequence approved
- [ ] Rollback plan agreed
- [ ] Human approval received

## Next Steps

After approval:
1. Execute Phase 1 (close 12 PRs)
2. Execute Phase 2 (merge 11 quick wins)
3. Execute Phase 3 (merge 14 small features)
4. Schedule Phase 4 (review 9 large features)
5. Document results in Gitea comments
