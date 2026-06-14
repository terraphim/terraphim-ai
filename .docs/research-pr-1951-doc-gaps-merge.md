# Research Document: Merge PR #1951 Doc Fixes into Main

**Status**: Draft
**Author**: Opencode (GLM-5.1)
**Date**: 2026-06-01
**Related PRs**: #1951 (closed unmerged), #1968 (open release)

## Executive Summary

PR #1951 (`task/doc-gaps-2026-06-01`) was closed without merging despite containing 19 commits of doc-comment additions across 17 crates (~3,346 lines added, 99 files pure additions). The branch also carries non-doc changes (guard.rs removal, code refactorings) that create merge complexity. A cherry-pick or selective merge strategy is needed to recover the doc-comment work without pulling in undesired changes.

PR #1968 (`release/v2026.05.31`) is a release PR pointing at the same commit as current main (`615a57b32`). It is open, mergeable, and has 0 diff from main — purely a release marker.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | CI rustdoc warnings gate (`d1f2c7671`) will fail PRs until doc gaps are filled |
| Leverages strengths? | Yes | Doc comments are mechanical, low-risk, high-impact for crate quality |
| Meets real need? | Yes | `cargo doc --workspace` produces warnings; CI gate blocks PRs with missing docs |

**Proceed**: Yes (3/3)

## Problem Statement

### Description
PR #1951 was closed unmerged. It contains 1,732 lines of pure doc-comment additions across 99 source files, plus ~1,311 lines of mixed doc+refactor changes. The work eliminates missing-doc warnings across 17 crates but is stranded on a diverged branch.

### Impact
- CI rustdoc warnings gate (`d1f2c7671 ci: add rustdoc warnings gate to PR validation`) will flag missing docs on future PRs
- Crates publish without adequate documentation
- `cargo doc --workspace` produces noisy output

### Success Criteria
1. All doc-comment additions from PR #1951 land on main
2. No non-doc code changes introduced without review
3. `cargo doc --workspace` produces zero missing-doc warnings
4. All existing tests continue to pass

## Current State Analysis

### PR #1951 Branch Analysis

| Metric | Value |
|--------|-------|
| Total commits on branch (not on main) | 19 |
| Files changed | 184 |
| Net additions (crates/ only) | +2,182 |
| Pure-addition files (doc-only) | 99 files, 1,732 lines |
| Files with deletions (mixed) | 31 files |
| Merge conflicts vs main | 3 files |

### Files with Conflicts
1. `CHANGELOG.md` — trivial, additive
2. `crates/terraphim_persistence/src/lib.rs` — doc additions vs main's code changes
3. `crates/terraphim_rolegraph/src/lib.rs` — doc additions vs main's code changes

### Doc-Only Commits (Pure Additions)
19 commits, all prefixed `docs(crate_name):`. These add `///` and `//!` doc comments to public API items.

### Non-Doc Changes on Branch (Risk)
| File | Lines Removed | Nature | Risk |
|------|--------------|--------|------|
| `crates/terraphim_agent/src/learnings/guard.rs` | 347 | File deletion | HIGH — guard.rs exists on main |
| `crates/terraphim_agent/src/main.rs` | 56 removed, 31 added | Refactoring | MEDIUM — different structure |
| `crates/terraphim_orchestrator/src/config.rs` | 97 removed, 30 added | Significant refactor | HIGH — diverged from main |
| `crates/terraphim_types/src/lib.rs` | 118 removed, 1 added | Code removal | HIGH — different content on main |
| `crates/terraphim_agent/src/repl/commands.rs` | 17 removed, 133 added | Major expansion | MEDIUM — mixed doc+code |

### PR #1968 (Release v2026.05.31)
- State: open, mergeable
- Head: `release/v2026.05.31` at `615a57b32` (identical to main)
- 0 changed files — purely a release tag PR
- Should be merged as-is to mark the release

## Constraints

### Technical Constraints
- Branch `gitea/task/doc-gaps-2026-06-01` has diverged from main with non-doc changes
- Direct merge would bring in 31 files with mixed doc+code changes
- 3 merge conflicts to resolve
- CI rustdoc warnings gate is active

### Quality Constraints
- All 5,367 tests must continue to pass
- No new clippy warnings
- Cargo fmt must be clean

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cherry-pick conflicts from diverged code | Medium | Medium | Manual conflict resolution per commit |
| Non-doc code changes slip in | Medium | High | Careful cherry-pick review |
| guard.rs deletion breaks main | Low | High | Exclude guard.rs deletion |
| CI doc gate blocks other PRs until merged | High | Medium | Prioritise this merge |

### Assumptions
| Assumption | Basis | Risk if Wrong |
|------------|-------|---------------|
| Doc-only commits can be cherry-picked cleanly | Most files are pure additions | Some may conflict with main's evolved code |
| Non-doc changes are NOT wanted | They were part of a broader cleanup PR | Verify with user before excluding |

## Research Findings

### Key Insights
1. PR #1951 is NOT purely a doc PR — it carries significant code changes (guard.rs removal, config refactoring, main.rs restructuring)
2. The 19 doc commits can be cherry-picked individually, but some will have conflicts because the files they modify have been refactored on main
3. PR #1968 is a clean release marker — merge it independently
4. The CI rustdoc warnings gate makes this work time-sensitive

### Recommended Strategy
**Cherry-pick the 19 doc commits onto a fresh branch from main**, resolving conflicts as needed. This isolates doc work from non-doc changes.

## Recommendations

### Proceed: Yes
### Strategy: Cherry-pick doc commits onto fresh branch

1. Merge PR #1968 first (clean release marker)
2. Create new branch from main
3. Cherry-pick each of the 19 `docs(...)` commits
4. Resolve conflicts (expect ~5-7 due to main's evolved code)
5. Run `cargo doc --workspace` to verify zero warnings
6. Run `cargo test --workspace` to verify no regressions
7. Open new PR

### Alternative: Selective merge with manual file filtering
Rebase the branch onto main, then revert non-doc changes. More complex but preserves full history.

## Next Steps
1. Get human approval on this research
2. Proceed to Phase 2 (Design)
