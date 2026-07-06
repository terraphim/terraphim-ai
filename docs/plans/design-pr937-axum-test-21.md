# Implementation Plan: axum-test 19.1.1 -> 21.0.0

**Status**: Approved
**Research Doc**: `docs/plans/research-pr937-axum-test-21.md`
**Author**: OpenCode
**Date**: 2026-07-06
**Estimated Effort**: 5 minutes (merge-only)

## Overview

### Summary
Merge the already-cleaned Dependabot PR that bumps `axum-test` to 21.0.0.

### Scope
**In scope:**
- Merge PR #937.
- Verify the two affected packages still compile and test.

**Out of scope:**
- Source code changes.
- Additional dependency bumps.

**Avoid At All Cost:**
- Reintroducing unrelated `Cargo.lock` drift.
- Manually editing `Cargo.lock` instead of using `cargo update` for future bumps.

## Architecture

No architecture changes. The dependency is a test helper; its public usage is confined to `TestServer` wrappers.

## File Changes

### Modified Files
| File | Changes |
|------|---------|
| `Cargo.lock` | `axum-test` 19.1.1 -> 21.0.0; adds `educe`, `enum-ordinalize`, `enum-ordinalize-derive` |
| `crates/terraphim_validation/Cargo.toml` | `axum-test = "19.1.1"` -> `axum-test = "21.0.0"` |
| `terraphim_server/Cargo.toml` | `axum-test = "19"` -> `axum-test = "21"` |

## Test Strategy

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `ollama_api_test.rs` | `terraphim_server/tests/` | Validates Ollama-compatible endpoints |
| `agent_web_flows_test.rs` | `terraphim_server/tests/` | Validates agent web flows |
| `workflow_e2e_tests.rs` | `terraphim_server/tests/` | Validates workflow endpoints |
| `testing/server_api/harness.rs` | `crates/terraphim_validation/src/` | Shared test harness |

Run after merge:
```bash
cargo test -p terraphim_server -p terraphim_validation
```

## Implementation Steps

### Step 1: Merge
**Action:** `gh pr merge 937 --repo terraphim/terraphim-ai --squash`
**Verification:** Confirm merge commit reaches GitHub `main`.

### Step 2: Mirror to Gitea
**Action:** Push GitHub `main` to Gitea `main`.
**Verification:** `git diff github/main origin/main --stat` is empty.

### Step 3: Post-merge smoke test
**Action:** `cargo test -p terraphim_server -p terraphim_validation`
**Verification:** Compilation succeeds; tests pass or skipped paths remain skipped.

## Rollback Plan

If CI fails:
1. Revert merge commit on `main`.
2. Push revert to both GitHub and Gitea.
3. Reopen PR #937 with failure details.

## Dependencies

| Crate | From | To | Reason |
|-------|------|-----|--------|
| `axum-test` | 19.1.1 | 21.0.0 | Keep test helper aligned with axum 0.8 |
| `educe` | - | 0.6.0 | New transitive dependency of axum-test |
| `enum-ordinalize` | - | 4.4.1 | New transitive dependency of educe |
