# Research Document: Open PR Backlog Merge Strategy

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-05-31
**Scope**: 50 open PRs across terraphim-ai repository

## Executive Summary

We have 50 open PRs with a combined ~92,000 lines of changes across ~7,000 files. Most (45) have merge conflicts. Many PRs overlap thematically (rustdoc, NormalizedTerm evolution, credential redaction, test fixes). The backlog grew because PRs were opened faster than they were merged, creating a cascading conflict problem. This research identifies quick wins, dependency chains, and a rational merge sequence.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Clearing backlog unblocks feature work and reduces cognitive load |
| Leverages strengths? | Yes | We have direct merge access and can batch-process related PRs |
| Meets real need? | Yes | 50 open PRs = blocked features, security fixes, and docs |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description

The PR backlog has grown to 50 open pull requests. The majority (45) have merge conflicts against `main`. Many PRs touch overlapping files (especially `terraphim_orchestrator/src/lib.rs`, `terraphim_types`, and test files), creating a cascading rebase problem: merging one PR often breaks others.

### Impact

- **Blocked features**: ADF agent registry, skills integration, sessions cluster command
- **Blocked security fixes**: credential redaction (already addressed separately), world-readable config warnings
- **Blocked docs**: rustdoc gaps across 15+ crates
- **Blocked reliability fixes**: cron re-fire storm, context rot detection, retry bounds
- **Developer friction**: Contributors see their PRs stagnate; new PRs compound the problem

### Success Criteria

- [ ] Close or merge all 50 open PRs
- [ ] No regressions in `cargo test --workspace`
- [ ] Both remotes (GitHub + Gitea) stay in sync
- [ ] Document which PRs were closed vs merged and why

## Current State Analysis

### PR Categories

| Category | Count | Total Files | Total +/- | Strategy |
|----------|-------|-------------|-----------|----------|
| **Mergeable now** | 5 | 34 | +7,964/-1,875 | Merge immediately |
| **Tiny fixes (<5 files, <100 lines)** | 6 | 16 | +887/-78 | Rebase and merge |
| **Small features (5-20 files, <1500 lines)** | 14 | 191 | +9,357/-1,606 | Rebase in batches |
| **Large features (>20 files or >2000 lines)** | 9 | 542 | +38,932/-4,073 | Dedicated review sessions |
| **Corrupted/stale** | 3 | 9,697 | ~2.4M lines | Close immediately |
| **Already superseded** | 4 | - | - | Close with explanation |
| **Docs-only** | 5 | 111 | +2,946/-124 | Merge after code PRs |
| **Duplicate/similar** | 4 | - | - | Pick best, close others |

### Mergeable PRs (Quick Wins)

| PR | Title | Files | +/- | Risk |
|----|-------|-------|-----|------|
| #1279 | flush compiled thesaurus cache | 10 | +3082/-1775 | Medium - large but isolated to KG system |
| #1272 | spec gaps C4/C5 | 2 | +38/-2 | Low - tiny |
| #1268 | reduce server startup retry | 1 | +2/-2 | Low - trivial |
| #1250 | agent role-aware tier routing | 20 | +572/-52 | Medium - routing logic |
| #1246 | CHANGELOG update | 0 | +0/-0 | None - empty |

### Corrupted/Stale PRs (Close Immediately)

| PR | Title | Files | +/- | Reason |
|----|-------|-------|-----|--------|
| #1045 | concurrency limits | 3277 | +812k/-74k | Based on ancient branch; impossible to rebase |
| #1023 | --format json | 3259 | +808k/-74k | Same as above |
| #969 | concepts_matched | 3161 | +801k/-74k | Same as above |

These three PRs show ~800k lines added across 3000+ files. The repository has ~670k lines total. These are clearly based on a branch that diverged massively from main (possibly an old merge commit or branch contamination). They should be closed and re-created from current main if still needed.

### Already Superseded (Close)

| PR | Superseded By | Reason |
|----|--------------|--------|
| #1767 | main | AgentConfig redaction already on main |
| #1791 | main | 6 struct redactions already on main |
| #1679 | #1918 + main | All structs except one already implemented |
| #1663 | #1918 + main | All structs except one already implemented |
| #1640 | #1918 + main | Config struct redactions already on main |

### Duplicate/Similar PRs (Pick One)

| Theme | PRs | Recommendation |
|-------|-----|----------------|
| NormalizedTerm evolution | #1283, #1343, #1605 | #1283 is largest and most comprehensive; close #1343 and #1605 |
| rustdoc gaps | #1867, #1710, #1695, #1365 | Merge in order: #1365 (CI gate), then #1695, #1710, #1867 |
| Terraphim Engineer role tests | #1367, #1294 | #1367 is larger; close #1294 as duplicate |
| cargo-deny scans | #1483, #1482 | Merge both - they are complementary (compliance vs advisory) |

### Dependency Chains

```
#1365 (rustdoc CI gate)
  -> #1695 (rustdoc 4 crates)
    -> #1710 (rustdoc 7 crates)
      -> #1867 (rustdoc gaps + CHANGELOG)

#1283 (NormalizedTerm - largest)
  -> #1343 (NormalizedTerm builder) [close, superseded]
  -> #1605 (NormalizedTerm enrichment) [close, superseded]

#1524 (context rot detection)
  -> #1599 (#1443 merge conflicts + isolation)
  -> #1617 (context_rot_token_budget)

#1291 (meta_coordinator wiring)
  -> #1786 (project-scoped registry) [depends on meta_coordinator]
```

## Constraints

### Technical Constraints

- **Branch protection**: Gitea requires status checks; must temporarily disable for merge
- **Dual remotes**: Must merge to Gitea first, then sync to GitHub
- **Workspace size**: 70+ crates; `cargo test --workspace` takes 10+ minutes
- **Feature flags**: Some tests require `--features` combinations

### Business Constraints

- **No force-push to main**: Must use merge commits or rebase-then-merge
- **No breaking changes**: Must maintain backward compatibility
- **Documentation first**: rustdoc PRs should not conflict with code changes

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Merge velocity | 5-10 PRs per session | ~2-3 |
| Conflict resolution time | <10 min per PR | 30+ min |
| Test suite pass | 100% | ~95% (some flaky tests) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Close corrupted PRs first | They clutter the queue and confuse prioritisation | 3 PRs with 800k+ lines each |
| Merge tiny PRs before large ones | Reduces conflict surface for bigger PRs | 6 tiny PRs = <100 lines each |
| Batch related PRs | Avoids repeated rebasing of same files | 4 rustdoc PRs touch similar crates |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Rebase corrupted PRs (#1045, #1023, #969) | Impossible - based on ancient divergent branch |
| Merge all 50 PRs in one session | Too much cognitive load; risk of mistakes |
| Rebase #1788 (173 files) without dedicated review | Too large to merge blindly |
| Rebase #1800 (51 files, +4783 lines) without review | Major feature - needs review |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Merge conflict cascade | High | Medium | Merge in dependency order; close duplicates |
| Test regressions from batch merges | Medium | High | Run `cargo test` after each batch |
| Gitea status check blocks | High | Low | Temporary disable/restore workaround |
| PR descriptions outdated | Medium | Low | Verify diff still makes sense before merge |

### Open Questions

1. **Are #1045/#1023/#969 still needed?** The issues they reference (#664, #1013, #851) may have been fixed in other PRs. Check issue status before closing.
2. **Does #1788 depend on #1291?** Both touch `.terraphim/skills/` and meta_coordinator. Merge #1291 first if so.
3. **Are rustdoc PRs (#1867, #1710, #1695, #1365) independent?** They likely touch different crates, but #1365 adds CI gate which affects all.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Corrupted PRs (#1045/#1023/#969) are not needed | Massive divergence suggests branch contamination | Features may be lost | No - need to check issues |
| Tiny PRs have no hidden dependencies | Small diff = small blast radius | Unexpected breakage | Partial - will verify with tests |
| Large PRs (#1788, #1800) are still valid | They reference open issues | May be stale | No - need issue verification |

## Research Findings

### Key Insights

1. **The backlog is self-sustaining**: PRs conflict because they sit too long. The solution is batch merging, not individual rebasing.

2. **80/20 rule applies**: 5 mergeable PRs + 6 tiny PRs = 11 PRs that can be cleared with minimal effort. These represent 22% of the backlog but probably 5% of the total diff.

3. **Three PRs are corrupted**: #1045, #1023, #969 show impossible diff stats. They should be closed immediately.

4. **Security PRs are mostly done**: After #1918, all credential redaction from #1663/#1679/#1640 is complete.

5. **Docs PRs should merge last**: They touch many files and are likely to conflict with code PRs. Merge code first, docs second.

### Relevant Prior Art

- **PR #1850 cleanup**: Demonstrated that focused, cleaned PRs merge faster than bundled ones
- **PR #1912**: Showed that removing unused dependencies is cleaner than adding exceptions
- **Gitea status check workaround**: Established pattern for merging when checks block

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify issue status for corrupted PRs | Check if #664, #1013, #851 are still open | 5 min |
| Check #1788 vs #1291 dependency | Determine if skills integration needs meta_coordinator | 10 min |
| Verify rustdoc PR independence | Check if #1867, #1710, #1695 touch same crates | 10 min |

## Recommendations

### Proceed/No-Proceed

**Proceed** with phased approach:
1. Phase 1: Close corrupted + superseded (8 PRs)
2. Phase 2: Merge quick wins (11 PRs)
3. Phase 3: Rebase small features in batches (14 PRs)
4. Phase 4: Dedicated review sessions for large features (9 PRs)

### Scope Recommendations

- **Close immediately**: #1045, #1023, #969, #1767, #1791, #1679, #1663, #1640, #1246, #1343, #1605, #1294
- **Merge immediately**: #1272, #1268, #1279, #1250, #1246 (if not empty)
- **Rebase and merge**: #1832, #1745, #1268, #1605, #1514, #1510, #1483, #1482, #1291
- **Batch merge**: rustdoc series (#1365 -> #1695 -> #1710 -> #1867)
- **Dedicated review**: #1788, #1800, #1786, #1720, #1491, #1524, #1380

### Risk Mitigation Recommendations

1. **Before each merge**: Run `cargo test -p <affected_crate>`
2. **After each batch**: Run `cargo test --workspace`
3. **Before closing**: Comment explaining why with reference to superseding PR
4. **Sync protocol**: Always push to Gitea first, then GitHub, then verify diff

## Next Steps

If approved:
1. Execute Phase 1 (close 12 PRs)
2. Execute Phase 2 (merge 5 mergeable PRs)
3. Execute Phase 3 (rebase 6 tiny PRs)
4. Execute Phase 4 (batch rustdoc PRs)
5. Schedule dedicated sessions for large features

## Appendix

### PR Matrix

| PR | Title | Files | +/- | Category | Action |
|----|-------|-------|-----|----------|--------|
| #1279 | flush thesaurus cache | 10 | +3082/-1775 | Mergeable | Merge |
| #1272 | spec gaps C4/C5 | 2 | +38/-2 | Mergeable | Merge |
| #1268 | reduce retry time | 1 | +2/-2 | Mergeable | Merge |
| #1250 | tier routing | 20 | +572/-52 | Mergeable | Merge |
| #1246 | CHANGELOG | 0 | +0/-0 | Mergeable | Close (empty) |
| #1045 | concurrency limits | 3277 | +812k/-74k | Corrupted | Close |
| #1023 | --format json | 3259 | +808k/-74k | Corrupted | Close |
| #969 | concepts_matched | 3161 | +801k/-74k | Corrupted | Close |
| #1767 | AgentConfig redaction | - | - | Superseded | Close |
| #1791 | 6 struct redaction | - | - | Superseded | Close |
| #1679 | 5 struct redaction | - | - | Superseded | Close |
| #1663 | credential redaction | - | - | Superseded | Close |
| #1640 | config redaction | - | - | Superseded | Close |
| #1343 | NormalizedTerm builder | - | - | Duplicate | Close |
| #1605 | NormalizedTerm enrichment | - | - | Duplicate | Close |
| #1294 | Engineer role tests | - | - | Duplicate | Close |

### Reference Materials

- Gitea PR list: https://git.terraphim.cloud/terraphim/terraphim-ai/pulls
- Current main: `e36c9ac7`
