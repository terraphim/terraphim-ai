# Research Document: CI Git-Diff Test Failures

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-04-14
**CI Run**: https://github.com/terraphim/terraphim-ai/actions/runs/24403573885

## Executive Summary

Two orchestrator tests fail in CI because the rust-build job uses a shallow clone (`fetch-depth: 1` default) while the tests depend on comparing git history between commits. The `git_diff_baseline()` fallback to git's empty tree doesn't trigger because `git rev-list --max-parents=0 HEAD` returns HEAD itself in a shallow clone (not empty), making baseline == HEAD, so the diff is empty and the agent is never spawned.

## Problem Statement

### Description
Tests `test_git_diff_matching_changes_spawns` and `test_spawn_agent_proceeds_with_git_diff_findings` fail in CI with `assertion failed: orch.is_agent_active("sentinel")`.

### Impact
CI Main Branch is broken. All subsequent jobs (Integration Tests, Docker Build) are skipped.

### Success Criteria
- Both tests pass in CI with shallow clone
- Tests continue to pass in full clone (local development)

## Current State Analysis

### Root Cause Chain

1. **CI Workflow** (`.github/workflows/ci-main.yml:112`): rust-build job uses `actions/checkout@v6` **without** `fetch-depth: 0` (defaults to `fetch-depth: 1`, a single-commit shallow clone)

2. **`git_diff_baseline()`** (`crates/terraphim_orchestrator/tests/orchestrator_tests.rs:15-30`): Runs `git rev-list --max-parents=0 HEAD` to find the root commit. In a shallow clone of depth 1, this returns **HEAD itself** (the only commit available), not an empty string.

3. **Empty-tree fallback never triggers**: The fallback `4b825dc642cb6eb9a060e54bf8d69288fbee4904` only activates when `baseline.is_empty()`, but in the shallow clone, `rev-list` returns a valid (non-empty) commit hash -- HEAD.

4. **`set_last_run_commit("sentinel", &baseline_commit)`**: Sets baseline to HEAD (the only commit in the shallow clone).

5. **`git_diff_pre_check()`** (`src/lib.rs:1308-1378`): Compares `last_run_commit` to `get_current_head()`. Both are HEAD. `head == last_commit` is TRUE, so `PreCheckResult::NoFindings` is returned and the agent is never spawned.

6. **Test assertion fails**: `assert!(orch.is_agent_active("sentinel"))` fails because the pre-check skipped the spawn.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `git_diff_baseline()` | `tests/orchestrator_tests.rs:15-30` | Baseline commit lookup for tests |
| `git_diff_pre_check()` | `src/lib.rs:1308-1378` | Pre-check comparing last_run_commit to HEAD |
| `spawn_agent()` | `src/lib.rs:916-1035` | Agent spawn with pre-check gate |
| CI checkout | `.github/workflows/ci-main.yml:112` | Shallow clone (missing `fetch-depth: 0`) |

### Data Flow (in CI shallow clone)

```
git_diff_baseline()
  -> git rev-list --max-parents=0 HEAD
  -> returns HEAD (not empty, not root)
  -> baseline = HEAD hash

set_last_run_commit("sentinel", baseline)
  -> last_run_commits["sentinel"] = HEAD hash

spawn_agent_for_test("sentinel")
  -> run_pre_check()
    -> git_diff_pre_check()
      -> get_current_head() = HEAD hash
      -> HEAD == last_commit => TRUE
      -> return NoFindings
  -> return Ok(()) without spawning
  -> is_agent_active("sentinel") = FALSE
  -> ASSERTION FAILS
```

## Constraints

### Technical Constraints
- CI uses self-hosted runners with `actions/checkout@v6`
- Default `fetch-depth` is 1 (shallow)
- Only the setup job (line 54) has `fetch-depth: 0`; rust-build job (line 112) does not

### Multiple Interpretations Considered

| Interpretation | Implications | Verdict |
|----------------|--------------|---------|
| Fix CI: add `fetch-depth: 0` to rust-build | Full history available, tests work as-is | Correct but heavier checkout |
| Fix tests: detect shallow clone and use empty tree | Tests work in both shallow and full | More robust |
| Fix both | Belt and suspenders | Recommended |

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Shallow clone detection varies by git version | Low | Medium | Use `git rev-parse --verify` to test commit reachability |
| Other jobs also missing `fetch-depth: 0` | Medium | Low | Audit all checkout steps |

### Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `rev-list --max-parents=0 HEAD` returns HEAD in depth-1 clone | Git documentation | Root cause is different | Yes - confirmed by CI behavior |
| Tests pass locally | Local test run | N/A | Yes |

## Recommendations

**Fix approach: Dual fix for robustness**

1. **Fix CI**: Add `fetch-depth: 0` to the rust-build checkout step (line 112)
2. **Fix tests**: Make `git_diff_baseline()` detect shallow clones and always use the empty tree when the baseline == HEAD (indicating single-commit history)

### Scope

**In Scope:**
- Fix `git_diff_baseline()` to handle shallow clones correctly
- Fix CI workflow `fetch-depth` for rust-build job
- Verify both tests pass

**Out of Scope:**
- Refactoring the pre-check system
- Changes to production `git_diff_pre_check()` logic

## Next Steps

1. Create implementation plan (Phase 2)
2. Implement and verify locally
3. Push and verify CI passes
