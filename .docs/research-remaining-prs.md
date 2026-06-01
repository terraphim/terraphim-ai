# Research Document: Remaining Stale PR Backlog

**Status**: Approved
**Author**: Root (orchestrator agent)
**Date**: 2026-06-01
**Reviewers**: TBD

## Executive Summary

Investigation of 9 remaining stale PRs reveals that **6 are already implemented on main** and should be closed immediately. Only **3 require fresh implementation work**: test role name fixes, strict permissions flag, and spec gap investigation.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Cleaning backlog improves velocity and reduces confusion |
| Leverages strengths? | Yes | Full codebase access and understanding from previous fixes |
| Meets real need? | Yes | Stale PRs create cognitive overhead and hide real work |

**Proceed**: Yes - 3/3 YES

## Problem Statement

### Description
Nine open PRs remain from the original backlog of 20. After triaging 11 PRs (6 closed as superseded + 5 merged), we need to determine the fate of the remaining 9.

### Impact
- Confusion about which issues are actually fixed
- Wasted review effort on stale branches
- Risk of duplicate work

### Success Criteria
- All stale/superseded PRs closed with documentation
- Real unfixed issues identified and queued for fresh implementation
- Backlog reduced to only actionable work

## Current State Analysis

### PR Triage Results

#### Category 1: Already Implemented (Close Immediately)

| PR | Issue | Investigation Result | Status |
|----|-------|---------------------|--------|
| #1615 | #821 LearningStore applied_by | `applied_by: &str` parameter exists in trait `SharedLearningStore` at `terraphim_types/src/shared_learning.rs:139-140` and all implementations (InMemoryLearningStore, FileLearningStore, etc.) | **ALREADY DONE** |
| #1604 | #1577 Flow matrix expansion | `MatrixConfig` struct exists at `terraphim_orchestrator/src/flow/config.rs:28-40` with `params: Vec<MatrixParams>`, `max_parallel`, `fail_strategy` | **ALREADY DONE** |
| #1600 | #842 Robot mode query/role | `ResponseMeta.query` and `ResponseMeta.role` fields exist with `with_query()` and `with_role()` builders. Tests verify serialization at `schema.rs:518-543` | **ALREADY DONE** |
| #1491 | #1488 RLM executor hardening | Full executor implementations exist: `LocalExecutor`, `DockerExecutor`, `FirecrackerExecutor`, `SshExecutor` in `crates/terraphim_rlm/src/executor/` | **ALREADY DONE** |
| #1599 | #1572 Context rot merge conflicts | Superseded by our fresh implementation in commit `e84f9214` | **SUPERSEDED** |

#### Category 2: Still Relevant (Need Fresh Implementation)

| PR | Issue | Investigation Result | Action Needed |
|----|-------|---------------------|---------------|
| #1367 | #1358 Test role names | 55 references to `"test_agent"` string literal across codebase. Need to verify if these are causing test failures or just inconsistent naming | **FRESH PR** |
| #1514 | #1299 Strict permissions | `PermissionCheck` exists in `commands/validator.rs` but no `--strict-permissions` CLI flag found in adf-ctl. Need to verify scope | **INVESTIGATE** |
| #1308 | #1297 Spec gaps | Title is vague. Need to read issue description to understand what spec gaps remain | **INVESTIGATE** |

## Constraints

### Technical Constraints
- Must not break existing tests
- Must maintain backward compatibility
- Changes must pass CI (rustdoc, clippy, tests)

### Business Constraints
- Each fix should be a focused PR (< 20 files)
- Must update CHANGELOG

## Vital Few

### Essential Constraints
1. **Close stale PRs first** - Reduce noise before doing new work
2. **Verify before implementing** - Confirm issue still exists on main
3. **Small focused PRs** - One issue per PR

### Eliminated from Scope
- Rebase existing PRs (all branches are stale)
- Bulk changes (each issue gets its own PR)

## Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| Test suite | Must pass after role name changes | Medium |
| CI pipeline | Must handle new rustdoc gate | Low |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test role rename breaks tests | Medium | High | Run full test suite before PR |
| Strict permissions already implemented | Medium | Low | Check codebase thoroughly first |

### Open Questions
1. Does `test_agent` cause actual test failures or is it just naming inconsistency?
2. What exactly are the "spec gaps" in #1297?
3. Is `--strict-permissions` flag already present under a different name?

## Research Findings

### Key Insights
1. **Most work is already done**: 6 of 9 remaining PRs represent features already on main
2. **Naming inconsistency is real**: 55 `test_agent` references suggest #1367 is valid
3. **Permission system exists but may be incomplete**: `PermissionCheck` action exists but CLI integration unclear

### Code Locations
```
# LearningStore applied_by (ALREADY DONE)
crates/terraphim_types/src/shared_learning.rs:139-140
crates/terraphim_orchestrator/src/learning.rs:314-339

# Flow matrix expansion (ALREADY DONE)
crates/terraphim_orchestrator/src/flow/config.rs:28-40

# RLM executors (ALREADY DONE)
crates/terraphim_rlm/src/executor/mod.rs
crates/terraphim_rlm/src/executor/local.rs
crates/terraphim_rlm/src/executor/docker.rs
crates/terraphim_rlm/src/executor/firecracker.rs

# Robot mode query/role (ALREADY DONE)
crates/terraphim_agent/src/robot/schema.rs:74-123

# Test role names (NEEDS FIX)
grep -r '"test_agent"' crates/ --include="*.rs" | wc -l = 55

# Permission check (NEEDS INVESTIGATION)
crates/terraphim_agent/src/commands/validator.rs:30
```

## Recommendations

### Proceed/No-Proceed
**PROCEED** - All three essential questions answered YES.

### Scope Recommendations
**Phase 1**: Close 6 stale PRs immediately with explanatory comments
**Phase 2**: Investigate 3 remaining issues (#1367, #1514, #1308)
**Phase 3**: Create fresh focused PRs for confirmed issues

### Risk Mitigation
- Run full test suite before any rename operations
- Verify issue still exists before creating fresh PR

## Next Steps

1. Close PRs #1615, #1604, #1600, #1491, #1599 with "already implemented" comments
2. Investigate #1367: Determine if `test_agent` causes failures
3. Investigate #1514: Check if `--strict-permissions` flag exists
4. Investigate #1308: Read issue #1297 for spec gap details
5. Create implementation plan for confirmed issues

## Appendix

### Reference Materials
- `.docs/pr-backlog-correction-plan.md` - Original triage results
- `crates/terraphim_orchestrator/src/flow/config.rs` - MatrixConfig implementation
- `crates/terraphim_types/src/shared_learning.rs` - LearningStore trait
