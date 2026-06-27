# Implementation Plan: Fix terraphim-agents native-ci (PR #33)

**Status**: Completed (native-ci run 73 success, 2026-06-10)
**Research Doc**: `.docs/research-agents-native-ci-failure.md`
**Author**: Terraphim AI
**Date**: 2026-06-10
**Estimated Effort**: 30 minutes

## Overview

### Summary

Restore `native-ci` compliance on terraphim-agents PR #33 through a three-commit fix chain: fmt drift, clippy `collapsible_if`, then ReviewPr `lib_tests` gate stubs (`echo` → `cat` for stdin prompts).

### Approach

1. `cargo fmt --all` + clippy fixes (done: `8d361169`)
2. Update `review_pr_config*` helpers to use `cat` for PR gate agents that receive `with_stdin()` prompts; keep `echo` for `build-runner`
3. Verify with `RUST_TEST_THREADS=32 cargo test -p terraphim_orchestrator --lib` and full BUILD.md; push

### Scope

**In Scope:**
- `cargo fmt --all` on PR branch
- Local verification: fmt, clippy (orchestrator crate), lib tests
- Push + poll Gitea workflow run for success

**Out of Scope:**
- Workflow YAML changes
- Runner instance repair (not needed — main green run 68)
- Merging PR #33 (separate step after CI green)
- ADF deploy (post-merge)

**Avoid At All Cost:**
- Manually posting fake `native-ci` success statuses
- Disabling branch protection to force merge
- Broad refactors unrelated to fmt

## Architecture

No architectural change. This is BUILD.md contract enforcement on existing code.

```
push → Gitea Actions (terraphim-native) → cargo fmt (FAIL today) → stop
push → Gitea Actions (terraphim-native) → cargo fmt (PASS) → clippy → build → test
```

## File Changes

| File | Change |
|------|--------|
| `crates/terraphim_orchestrator/src/pr_handlers_impl.rs` | rustfmt + (prior) clippy |
| `crates/terraphim_orchestrator/src/reconcile_impl.rs` | rustfmt + clippy |
| `crates/terraphim_orchestrator/src/lib_tests.rs` | `PR_GATE_TEST_CLI=cat`, `gate_test_cli()` / `build_runner_test_cli()` helpers |

Test-only change for step 4; no production logic changes.

## Test Strategy

| Step | Command | Expected |
|------|---------|----------|
| 1 | `cargo fmt --all -- --check` | exit 0 |
| 2 | `cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings` | exit 0 |
| 3 | `cargo test -p terraphim_orchestrator --lib pr_gate` | all pass |
| 3b | `RUST_TEST_THREADS=32 cargo test -p terraphim_orchestrator --lib` | 947 pass (no broken-pipe flakes) |
| 4 | `cargo test --workspace --lib --no-fail-fast` | all pass |
| 5 | Gitea workflow run on push | `native-ci / build (push)` success, duration ~30–60s |

## Implementation Sequence

1. `cd terraphim-agents` worktree on `task/2301-pr-gate-result-contract`
2. `cargo fmt --all`
3. `cargo fmt --all -- --check` (verify)
4. `cargo clippy -p terraphim_orchestrator --all-targets -- -D warnings`
5. `cargo test -p terraphim_orchestrator --lib pr_gate`
6. Commit fmt/clippy: `style(orchestrator): satisfy native-ci fmt and clippy Refs #2301` (done)
7. Commit tests: `test(orchestrator): use cat stubs for PR gate stdin paths Refs #2301`
8. Push to Gitea
9. Poll commit statuses until `native-ci / build (push)` success

## Rollback

Revert the fmt commit if unexpected clippy regressions appear (unlikely — formatting only).
