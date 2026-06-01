# PR Backlog Cleanup Summary

**Date**: 2026-06-01
**Started**: 20 open PRs
**Ended**: 4 open PRs
**Reduction**: 80%

## Actions Taken

### Phase 1: Research and Design
- Created `.docs/research-remaining-prs.md` - Research document analyzing all 20 open PRs
- Created `.docs/design-remaining-prs.md` - Design document with implementation plan
- Determined 6 PRs were already implemented on main
- Determined 5 PRs were mergeable and ready
- Identified 3 PRs as real unfixed issues needing investigation

### Phase 2: Merged PRs (7 total)

| PR | Title | Commit | Via |
|----|-------|--------|-----|
| #1956 | Gate perf assertions in debug builds | `52d510a0` | git merge |
| #1941 | Replace nested cargo run with cargo_bin (exit tests) | fast-forward | git merge |
| #1937 | Replace nested cargo run with cargo_bin (integration) | `726f759e` | git merge |
| #1933 | Runtime validation tests for openai-banned | `5f633615` | git merge |
| #1927 | Remove stale ignore from concurrent test | fast-forward | git merge |
| #1963 | ADF behaviour specifications | `bdd8111d` | git merge |
| #1965 | P2 security remediation (ptr::read, yanked aes) | `6ca3879f` | git merge |

### Phase 3: Closed Stale PRs (12 total)

| PR | Issue | Close Reason |
|----|-------|--------------|
| #1380 | #851 Thesaurus matching | Superseded by commit `d62481df` |
| #1524 | #1443 Context rot | Superseded by commit `e84f9214` |
| #1365 | #1362 Rustdoc gate | Superseded by commit `d1f2c767` |
| #1319 | #1313 Redis binding | Superseded by commit `c1721435` |
| #1316 | #446 Circuit breaker | Already on main |
| #1349 | #251 RetryBound | Already on main |
| #1615 | #821 LearningStore applied_by | Already on main |
| #1604 | #1577 Flow matrix expansion | Already on main |
| #1600 | #842 Robot mode query/role | Already on main |
| #1491 | #1488 RLM executor hardening | Already on main |
| #1599 | #1572 Context rot merge conflicts | Superseded |
| #1365 | #1362 Rustdoc gate (duplicate) | Superseded |

## Remaining Open PRs: 4

All 4 are non-mergeable and represent real issues needing fresh implementation:

| PR | Issue | Title | Status |
|----|-------|-------|--------|
| #1951 | #1942 | Eliminate ~4,100 missing-doc warnings | Conflicts with our rustdoc fixes |
| #1514 | #1299 | Add strict-permissions flag to adf | Needs investigation |
| #1367 | #1358 | Fix test role names | Needs investigation |
| #1308 | #1297 | Close persistent spec gaps | Needs investigation |

## Documents Created

1. `.docs/research-pr-backlog-remediation.md` - Initial PR backlog research
2. `.docs/design-pr-backlog-remediation.md` - Implementation plan for all issues
3. `.docs/research-remaining-prs.md` - Research on remaining 9 stale PRs
4. `.docs/design-remaining-prs.md` - Design for closing/merging remaining PRs

## Commits on Main

The following commits were added to main during this cleanup:
- `c1721435` - Redis binding fix
- `d1f2c767` - Rustdoc CI gate
- `e84f9214` - Context rot detection
- `d62481df` - Thesaurus matching
- `52d510a0` - Perf assertions gate
- `726f759e` - cargo_bin integration tests
- `5f633615` - Runtime validation tests
- `bdd8111d` - ADF behaviour specifications
- `6ca3879f` - P2 security remediation

## Next Steps

1. Investigate #1951 - Determine if doc gap fixes conflict with our rustdoc changes
2. Investigate #1514 - Verify if `--strict-permissions` flag is missing
3. Investigate #1367 - Check if `test_agent` references cause test failures
4. Investigate #1308 - Read issue #1297 for specific spec gaps

## Verification

- [x] All merges pushed to origin (GitHub)
- [x] All merges pushed to gitea (git.terraphim.cloud)
- [x] Both remotes in sync (`git diff origin/main gitea/main --stat` = empty)
- [x] All stale PRs closed with explanatory comments
- [x] CHANGELOG updated with all changes
