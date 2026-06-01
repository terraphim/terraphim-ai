# Open PR Backlog Investigation & Correction Plan

## Executive Summary

**Critical Finding**: All 19 open PRs in the terraphim-ai repository are **NOT MERGEABLE** (mergeable: false). This represents a systematic failure in the PR workflow where branches have diverged from main and cannot be merged without conflict resolution.

## PR Inventory

### Category A: Feature Implementation (Large, Complex)
| PR | Title | Files | +/- | Comments | Issue Status |
|----|-------|-------|-----|----------|--------------|
| #1524 | Fix #1443: context rot detection via wall-clock threshold | 40 | +1282/-36 | 4 | #1443 open |
| #1380 | fix(agent): populate Thesaurus_matched and wildcard_fallback | 90 | +3600/-79 | 6 | #851 open |
| #1491 | Fix #1488: RLM executor surface hardening | 32 | +3098/-141 | 0 | #1488 open |
| #1615 | Fix #821: add applied_by param to LearningStore | 11 | +1016/-36 | 1 | #821 open |
| #1604 | Fix #1577: add matrix expansion to FlowStepDef | 7 | +779/-4 | 2 | #1577 open |

### Category B: Bug Fixes & Hardening (Medium)
| PR | Title | Files | +/- | Comments | Issue Status |
|----|-------|-------|-----|----------|--------------|
| #1514 | Fix #1299: add check_permissions + --strict-permissions flag | 4 | +212/-39 | 1 | #1299 open |
| #1600 | Fix #842: verify meta.query and meta.role in robot mode JSON | 25 | +532/-553 | 2 | #842 open |
| #1599 | Fix #1572: resolve #1443 merge conflicts and fix integration test | 24 | +655/-553 | 0 | #1572 open |
| #1367 | Fix #1358: use Terraphim Engineer role in test_full_feature_matrix | TBD | TBD | 1 | #1358 open |
| #1365 | Fix #1362: gate rustdoc warnings with RUSTDOCFLAGS=-D warnings | TBD | TBD | 1 | #1362 open |

### Category C: Test Fixes & CI (Small, Focused)
| PR | Title | Files | +/- | Comments | Issue Status |
|----|-------|-------|-----|----------|--------------|
| #1356 | fix(test): use unique temp path to prevent parallel test interference | TBD | TBD | 1 | #1340 open |
| #1349 | Fix #251: enforce RetryBound invariant in Symphony on_retry_timer | TBD | TBD | 1 | #251 open |
| #1347 | Fix #1340: use unique tempdir in test_tool_index_save_and_load | TBD | TBD | 1 | #1340 open |
| #1319 | Fix #1313: harden compose and CI Redis/Ollama bindings | TBD | TBD | 1 | #1313 open |
| #1316 | Fix #446: exempt C1-blocked probes from circuit-breaker updates | TBD | TBD | 1 | #446 open |

### Category D: Configuration & Security (Small)
| PR | Title | Files | +/- | Comments | Issue Status |
|----|-------|-------|-----|----------|--------------|
| #1308 | Fix #1297: close persistent spec gaps | TBD | TBD | 1 | #1297 open |
| #1298 | Fix #826: warn on world-readable sensitive config files | TBD | TBD | 1 | #826 open |
| #1291 | Fix #1275: wire meta_coordinator module into lib.rs | TBD | TBD | 1 | #1275 open |
| #1283 | Fix #1266: NormalizedTerm missing fields break compilation | TBD | TBD | 1 | #1266 open |

## Root Cause Analysis

### 1. Merge Base Drift
All PRs were created from old merge bases (e.g., `050d496da0a1a62ebf739b271d741cd82caab66f`, `ed85f7fe92eb99ed76fa0fabdcb9f7d72ffd459c`, `2b6e2af1a8fd22c3134e832aefd5d61a79725473`) that significantly diverge from current main (`1051ff255fe6dfb3022bfbfe41448e7c7f3d2fe1`).

### 2. No Rebase Practice
Branches have not been rebased onto main, causing accumulated conflicts.

### 3. Long-lived Branches
Some PRs are weeks old (e.g., #1380 created 2026-05-09, #1283 even older), giving main time to diverge substantially.

### 4. Large PRs
Category A PRs touch 32-90 files with 1000+ additions, making conflict resolution difficult.

## Correction Plan

### Phase 1: Triage & Close Stale PRs (Immediate - This Session)

**Action**: For each PR, determine if the fix/feature already exists on main or if the branch is salvageable.

1. **Check if fix already on main**: Search for key functions/types from the PR in current main
2. **If yes**: Close PR as superseded, update linked issue
3. **If no**: Mark for rebase/reimplementation

**Priority Order**:
1. Start with smallest PRs (Category C/D) - easier to validate
2. Then medium PRs (Category B)
3. Finally large PRs (Category A) - may need extraction/reimplementation

### Phase 2: Rebase or Reimplement (Next Sessions)

For PRs marked as needed:

**Option A: Rebase** (for small, focused PRs with simple conflicts)
```bash
git checkout task/XXX-feature
git fetch origin
git rebase origin/main
# Resolve conflicts
git push --force-with-lease
```

**Option B: Cherry-pick** (for PRs with clean commits)
```bash
git checkout -b task/XXX-feature-rebased origin/main
git cherry-pick <commit-range>
# Resolve conflicts
```

**Option C: Reimplement** (for large, stale PRs)
- Create new branch from current main
- Manually reapply changes
- Run tests and validation
- Create new PR, close old one

### Phase 3: Prevent Future Backlog

1. **Enforce rebasing**: Require branches to be up-to-date before merge
2. **Smaller PRs**: Split large features into multiple focused PRs
3. **Regular triage**: Weekly review of open PRs
4. **Stale bot**: Auto-close PRs inactive for >14 days

## Immediate Next Steps

1. **Close clearly superseded PRs**: Where the fix already exists on main
2. **Post structured reviews**: On remaining PRs explaining why they're blocked
3. **Create tracking issue**: For systematic PR backlog cleanup
4. **Notify assignees**: Comment on linked issues about PR status

## Risk Assessment

- **Risk**: Large PRs (#1380, #1491, #1524) may be too stale to salvage
- **Mitigation**: Extract core changes into new focused PRs
- **Risk**: Closing PRs without merge may lose useful work
- **Mitigation**: Always verify whether changes exist on main before closing

## Success Criteria

- [ ] All 19 PRs triaged (closed or marked for rebase)
- [ ] Linked issues updated with PR status
- [ ] New PRs created for salvageable work
- [ ] Zero non-mergeable PRs remaining (except those in active rebase)
- [ ] Prevention measures in place
