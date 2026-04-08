# Implementation Plan: Restore CI Baseline Stability

**Status**: Draft | Review | Approved
**Research Doc**: `.docs/research-CI-baseline-restoration-2026-04-08.md`
**Author**: AI Agent (Phase 2 Discipline)
**Date**: 2026-04-08
**Estimated Effort**: 45 minutes total

## Overview

### Summary

Two independent CI failures block all development on `main`. This plan fixes both. **Track A** fixes the `ci-main.yml` rust-build failure (fff-search/zlob panic). **Track B** fixes the `performance-benchmarking.yml` YAML parse failure (0s, HTTP 422).

**Critical finding for Track B (just established):** The workflow fails at YAML parse time with HTTP 422 — "failed to parse workflow: (Line: 264, Col: 1): Unexpected value 'Auto-updated from CI run', (Line: 265, Col: 1): Unexpected value 'SLO Compliance'". The `git commit -m` command uses a multi-line string with embedded colons at the start of lines inside a YAML `run: |` block scalar. YAML block scalars do not suppress interpretation of YAML-like syntax at line-start position; GitHub's YAML parser is treating `Auto-updated from CI run:` and `SLO Compliance:` as top-level YAML key-value pairs. **Fix:** Replace the multi-line commit message with a compact single-line JSON string.

### Approach

**Track A**: Add `--features zlob` to the cargo build and cargo test commands in `ci-main.yml`. The `zlob` feature already exists in both `terraphim_mcp_server` and `terraphim_file_search` Cargo.toml files — it just needs to be enabled in CI.

**Track B**: Fix the malformed `git commit` multi-line string in the `update-baseline` job. Replace with a compact single-line commit message.

---

## Track A: ci-main.yml zlob Fix

### Step A1: Verify zlob Feature Locally

**File**: (verification only)
**Description**: Confirm `--features zlob` builds without side effects
**Tests**: `cargo build --release --workspace --features zlob` succeeds locally
**Estimated**: 15 minutes

```bash
cargo build --release --workspace --features zlob 2>&1 | tail -10
```

### Step A2: Add `--features zlob` to cargo build

**File**: `.github/workflows/ci-main.yml`
**Line**: ~185
**Description**: Add `--features zlob` to the workspace build command

**Current (line ~183–186):**
```yaml
      - name: Build release binaries
        run: |
          # Build workspace with default features (no rocksdb for faster CI)
          cargo build --release --target ${{ matrix.target }} --workspace
```

**Change to:**
```yaml
      - name: Build release binaries
        run: |
          # Build workspace. zlob feature required: fff-search build.rs
          # panics when CI=true unless zlob is explicitly enabled.
          cargo build --release --target ${{ matrix.target }} --workspace --features zlob
```

### Step A3: Add `--features zlob` to cargo test

**File**: `.github/workflows/ci-main.yml`
**Line**: ~209
**Description**: Add `--features zlob` to the workspace test command (tests must build same code)

**Current (line ~207–209):**
```yaml
      - name: Run tests
        run: |
          # Run unit and integration tests (exclude integration-signing which requires zitsign CLI)
          cargo test --release --target ${{ matrix.target }} --workspace --features "self_update/signatures"
```

**Change to:**
```yaml
      - name: Run tests
        run: |
          # Run unit and integration tests (exclude integration-signing which requires zitsign CLI)
          # zlob feature required: fff-search build.rs panics when CI=true without zlob.
          cargo test --release --target ${{ matrix.target }} --workspace --features "self_update/signatures,zlob"
```

### Rollback Plan (Track A)

If CI fails after A2/A3, replace `--workspace --features zlob` with package-level scoping:
```yaml
cargo build --release --target ${{ matrix.target }} \
  --package terraphim_server \
  --package terraphim_mcp_server \
  --package terraphim_agent \
  --features zlob
```

---

## Track B: performance-benchmarking.yml YAML Fix

### Step B1: Fix Malformed git commit Message

**File**: `.github/workflows/performance-benchmarking.yml`
**Lines**: ~261–266
**Description**: The multi-line `git commit -m` string with embedded colons at line-start causes YAML parse failure (HTTP 422). Replace with a compact single-line message.

**Current (lines ~261–266):**
```yaml
    - name: Commit baseline update
      run: |
        git config --global user.name 'github-actions[bot]'
        git config --global user.email 'github-actions[bot]@users.noreply.github.com'

        git add benchmark-results/baseline.json
        git commit -m "chore: update performance baseline

Auto-updated from CI run: ${{ github.run_id }}
SLO Compliance: ${{ needs.performance-benchmarks.outputs.slo-compliance }}%" || echo "No changes to commit"

        git push origin main
```

**Change to:**
```yaml
    - name: Commit baseline update
      run: |
        git config --global user.name 'github-actions[bot]'
        git config --global user.email 'github-actions[bot]@users.noreply.github.com'

        git add benchmark-results/baseline.json
        git commit -m "chore: update performance baseline from run ${{ github.run_id }}" || echo "No changes to commit"

        git push origin main
```

**Why this fixes it:**
- The original multi-line string has `Auto-updated from CI run:` at the start of a line inside a YAML `run: |` block scalar. GitHub's YAML parser interprets YAML-like syntax at line-start even inside block scalars, treating it as a new key-value pair rather than string content.
- The `SLO Compliance:` continuation line has the same issue. GitHub Actions' YAML parser (which is stricter than pure YAML) fails at parse time with HTTP 422.
- The fix uses a compact single-line commit message, avoiding any multi-line string interpretation.

**Alternative considered:** Quoting the entire multi-line string with `>` (folded scalar) instead of `|` (literal). Rejected because YAML fold scalars convert newlines to spaces, which would mangle the intended formatting without solving the root cause.

### Step B2: Validate Fixed Workflow

**File**: `.github/workflows/performance-benchmarking.yml`
**Description**: Verify the workflow can be dispatched without HTTP 422

```bash
gh workflow run .github/workflows/performance-benchmarking.yml --repo terraphim/terraphim-ai
```

Expected: HTTP 202 (workflow dispatch accepted), not HTTP 422.

---

## Implementation Sequence

```
Step A1 (local zlob build test)
    │
    ├─────────────────────────────────────────────────────────────
    ▼                                                             ▼
Step A2 (ci-main.yml build)                              Step B1 (perf benchmark git commit fix)
    │                                                             │
    ▼                                                             ▼
Step A3 (ci-main.yml test) ──────────────────────► Verify: push to test branch
                                                            │
                                                            ▼
                                                    Both tracks: PR to main
```

Both tracks can be implemented in parallel since they touch different files. Verification on a test branch is required before merging to `main`.

---

## File Changes Summary

| File | Lines | Change |
|------|-------|--------|
| `.github/workflows/ci-main.yml` | ~185 | `cargo build --release --target ${{ matrix.target }} --workspace` → `... --workspace --features zlob` |
| `.github/workflows/ci-main.yml` | ~209 | `cargo test --release ... --features "self_update/signatures"` → `... --features "self_update/signatures,zlob"` |
| `.github/workflows/performance-benchmarking.yml` | ~261–266 | Replace multi-line `git commit -m` with compact single-line |

---

## Simplicity Check

**Track A**: Two one-word additions (`--features zlob`) to two existing commands. The feature already exists and is correctly wired — it simply needs to be enabled.

**Track B**: One line replacement in a shell command. The multi-line string is replaced with an equivalent single-line message. No behavioral change to what is committed.

Both changes are minimal and reversible.

---

## Verification Matrix

| Step | Verification Command | Expected Result |
|------|---------------------|-----------------|
| A1 | `cargo build --release --workspace --features zlob` | Build succeeds |
| A2 | Push `ci-main.yml` change to test branch → CI rust-build | Job passes (no zlob panic) |
| A3 | Push `ci-main.yml` change to test branch → CI tests | Tests pass |
| B1 | Apply fix → `gh workflow run .github/workflows/performance-benchmarking.yml` | HTTP 202, not 422 |
| B2 | Push fix to test branch → workflow completes | `performance-benchmarks` job starts (not 0s failure) |

---

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Audit `ci-native.yml` build coverage (lint-only, no build) | Pending | Human |
| Cluster and close duplicate Gitea ADF issues | Pending (after CI stable) | Human |

---

## Approval Checklist

Before Phase 3 (Implementation):

- [x] Track A: Root cause confirmed (`fff-search/zlob` panic, `CI=true` on bare self-hosted runner)
- [x] Track B: Root cause confirmed (HTTP 422 YAML parse failure on multi-line `git commit -m`)
- [x] Track A: Two-line change in `ci-main.yml` identified (lines ~185, ~209)
- [x] Track B: One-line fix in `performance-benchmarking.yml` identified (lines ~261-266)
- [x] Track A: Local verification command provided (`cargo build --release --workspace --features zlob`)
- [x] Track B: Validation command provided (`gh workflow run .github/workflows/performance-benchmarking.yml`)
- [ ] Human approval received on this implementation plan
