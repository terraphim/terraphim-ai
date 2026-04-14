# Implementation Plan: Fix CI Git-Diff Test Failures

**Status**: Draft
**Research Doc**: `.docs/research-ci-git-diff-test-failures.md`
**Date**: 2026-04-14
**Estimated Effort**: 30 minutes

## Overview

Fix two failing orchestrator tests in CI by (1) correcting the test helper to detect shallow clones and (2) adding `fetch-depth: 0` to the CI rust-build checkout step.

## Approach

Dual fix: make tests resilient to shallow clones AND give CI full history. The test fix is the primary defense; the CI fix is belt-and-suspenders.

### Scope

**In Scope:**
- `git_diff_baseline()` in `orchestrator_tests.rs` -- detect shallow clone
- `.github/workflows/ci-main.yml` line 112 -- add `fetch-depth: 0`

**Out of Scope:**
- Production `git_diff_pre_check()` changes
- Other CI workflow files
- Test refactoring

**Avoid At All Cost:**
- Adding new dependencies
- Changing production code behavior
- Modifying the pre-check strategy enum or dispatch logic

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/tests/orchestrator_tests.rs` | Fix `git_diff_baseline()` to detect when baseline == HEAD (shallow clone) |
| `.github/workflows/ci-main.yml` | Add `fetch-depth: 0` to rust-build checkout (line 112) |

## Implementation Steps

### Step 1: Fix `git_diff_baseline()` to detect shallow clones

**File:** `crates/terraphim_orchestrator/tests/orchestrator_tests.rs`
**Description:** The current logic only falls back to the empty tree when `baseline.is_empty()`. In a shallow clone, `rev-list --max-parents=0 HEAD` returns HEAD itself. Detect this by comparing the baseline to the current HEAD -- if they match, we're in a single-commit shallow clone and should use the empty tree.

**New logic:**
```rust
fn git_diff_baseline() -> String {
    let output = std::process::Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .output()
        .expect("git rev-list failed");
    let commits = String::from_utf8_lossy(&output.stdout);
    let baseline = commits.lines().next().unwrap_or("").trim();

    if baseline.is_empty() {
        return "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string();
    }

    // In a shallow clone (fetch-depth=1), rev-list --max-parents=0 returns
    // HEAD itself as the only reachable rootless commit. If the baseline
    // equals HEAD, we can't diff against it meaningfully -- use the empty tree.
    let head_output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse failed");
    let head = String::from_utf8_lossy(&head_output.stdout).trim().to_string();

    if baseline == head {
        return "4b825dc642cb6eb9a060e54bf8d69288fbee4904".to_string();
    }

    baseline.to_string()
}
```

**Tests:** Existing `test_git_diff_matching_changes_spawns` and `test_spawn_agent_proceeds_with_git_diff_findings` will now pass in both shallow and full clones.

### Step 2: Add `fetch-depth: 0` to CI rust-build checkout

**File:** `.github/workflows/ci-main.yml` (line 112)
**Description:** The setup job already uses `fetch-depth: 0` (line 54). The rust-build job does not, defaulting to shallow clone. Add `fetch-depth: 0` to ensure full history.

**Change:**
```yaml
      - name: Checkout
        uses: actions/checkout@v6
        with:
          fetch-depth: 0
```

### Step 3: Verify locally

Run the failing tests locally to confirm they still pass:
```bash
cargo test -p terraphim_orchestrator --test orchestrator_tests
```

## Simplicity Check

**What if this could be easy?** The simplest fix is just the test helper change (Step 1). The CI fix (Step 2) is a nice-to-have that makes CI faster to fix if the test fix works. Both are 2-line changes.

**Senior Engineer Test:** This is the minimum viable fix. No abstractions, no new types, no production code changes.

## Rollback Plan

If tests still fail:
1. Check CI logs for the actual git commands being run
2. Consider using `git shallow-rev-list` or `git log --oneline --all` for better shallow detection
3. Worst case: mark the two tests as `#[ignore]` and create a follow-up issue
