# Implementation Plan: TinyClaw Slack Scrubber Scope

**Status**: Approved — Alexander Mikhalev, 2026-08-23
**Issue**: terraphim/terraphim-ai#3281
**Research Reference**: Issue #3281 reproduction on clean Gitea `main` at `4aa1f8f271bfe8461ad01032a1410374699e67ce`; related regression from closed #3161; blocks #3143 acceptance.
**Author**: Codex design agent
**Date**: 2026-08-23
**Estimated Effort**: 15 minutes implementation, plus verification time

## Overview

### Summary

Fix the `terraphim_tinyclaw` all-features Slack integration test compile failure by making the nested `slack_tests` module resolve the root integration-test helper module explicitly.

The expected implementation is a one-file test-harness compile fix in `crates/terraphim_tinyclaw/tests/slack_integration.rs`. It does not change production behavior, public API, feature flags, dependencies, lockfiles, Slack adapter logic, or credential-scrubbing behavior.

### Scope

**In Scope:**
- Resolve E0433 in `crates/terraphim_tinyclaw/tests/slack_integration.rs`.
- Preserve the existing `mod common;` root test helper module.
- Preserve the current `common::scrub_env()` first executable line in both Slack integration tests.
- Verify default-feature and all-feature TinyClaw test commands.

**Out of Scope:**
- Production code changes.
- New tests or rewritten test behavior.
- Changes to `crates/terraphim_tinyclaw/tests/common/mod.rs` scrub list.
- Changes to Slack live-test credential policy.
- Changes to Cargo features, dependencies, or `Cargo.lock`.
- Refactoring Slack adapter code or integration-test structure beyond the module import needed for compilation.

**Avoid At All Cost:**
- Moving `crates/terraphim_tinyclaw/tests/common/mod.rs` into the library crate.
- Adding a helper crate or dependency for test setup.
- Removing the nested `slack_tests` feature-gated module.
- Disabling, deleting, or weakening Slack integration tests.
- Changing the credential scrubber to allow real secrets through by default.

## Root-Cause Evidence

The file currently declares the helper module at integration-test crate root:

```rust
mod common;
```

The Slack tests are nested under:

```rust
#[cfg(feature = "slack")]
mod slack_tests {
```

Inside that nested module, both tests call:

```rust
common::scrub_env();
```

In Rust, an unqualified `common::...` path inside `slack_tests` resolves relative to `slack_tests` first. There is no `slack_tests::common`, while the available helper is the root sibling `crate::common`.

Verification command run on this branch:

```bash
cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run
```

Observed failure:

```text
error[E0433]: cannot find module or crate `common` in this scope
  --> crates/terraphim_tinyclaw/tests/slack_integration.rs:36:9

error[E0433]: cannot find module or crate `common` in this scope
  --> crates/terraphim_tinyclaw/tests/slack_integration.rs:60:9
```

Rustc suggested importing the root module inside `slack_tests`:

```rust
use crate::common;
```

The reported baseline is consistent with the issue statement: `cargo test -p terraphim_tinyclaw` passes under default features, while `cargo test -p terraphim_tinyclaw --all-features` reaches the Slack-gated nested module and fails to compile the helper path.

## Simplest Design

### Design

Add one import inside the feature-gated nested module:

```rust
#[cfg(feature = "slack")]
mod slack_tests {
    use crate::common;
    use std::sync::Arc;
    use terraphim_tinyclaw::bus::MessageBus;
    use terraphim_tinyclaw::channel::Channel;
```

No call sites need to change. The existing `common::scrub_env()` statements remain the first executable lines in both test functions.

### Why This Is Sufficient

The helper module already exists at crate root. The failing scope is only the nested module name lookup. Importing `crate::common` into `slack_tests` makes the current calls resolve without changing behavior.

### Component Diagram

```text
slack_integration.rs integration-test crate
|
+-- common                 crates/terraphim_tinyclaw/tests/common/mod.rs
|
+-- slack_tests            #[cfg(feature = "slack")]
    |
    +-- use crate::common  brings root helper into nested scope
    +-- test functions     call common::scrub_env()
```

## Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
| --- | --- | --- |
| Replace calls with `crate::common::scrub_env()` | Correct, but touches two call sites instead of one import line. | Larger diff for the same compile outcome. |
| Move `mod common;` inside `slack_tests` | Would look for a different module path and duplicate test-helper topology. | Confusing module layout and possible helper divergence. |
| Make `common` public from production `src/lib.rs` | Test helper is not production API. | Public API pollution and security-sensitive helper exposure. |
| Remove `common::scrub_env()` from Slack live tests | Would avoid the missing path but weakens the scrubber invariant. | Real credentials could influence tests unexpectedly. |
| Gate Slack integration tests differently | The feature gate is not the bug. | Could hide all-features regressions instead of fixing them. |

## File Changes

### New Files

None during implementation.

### Modified Files

| File | Planned Change |
| --- | --- |
| `crates/terraphim_tinyclaw/tests/slack_integration.rs` | Add `use crate::common;` inside `mod slack_tests`. |

### Deleted Files

None.

## API Design

No public API changes.

No new public types, functions, traits, errors, feature flags, environment variables, or configuration keys.

## Security Credential-Scrubbing Invariant

The implementation must preserve this invariant:

> Every executable Slack integration test body continues to call `common::scrub_env()` as its first executable statement, and the common scrubber continues to remove Slack credentials from process environment by default.

This plan intentionally does not edit `crates/terraphim_tinyclaw/tests/common/mod.rs`, does not remove Slack credentials from `SCRUB_VARS`, and does not add any bypass for `SLACK_BOT_TOKEN`, `SLACK_APP_TOKEN`, or `SLACK_SIGNING_SECRET`.

The compile fix only changes name resolution so the scrubber can be reached from the nested module.

## Test Strategy

### Strict RED/GREEN Sequence

**RED: confirm current failure before implementation**

```bash
cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run
```

Expected RED result before the change:

```text
error[E0433]: cannot find module or crate `common` in this scope
```

at `tests/slack_integration.rs:36` and `tests/slack_integration.rs:60`.

**GREEN: confirm the targeted compile fix**

```bash
cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run
```

Expected GREEN result after the change: the Slack integration test target compiles. The ignored live tests are compiled but not executed by this command.

**Regression: default TinyClaw suite remains healthy**

```bash
cargo test -p terraphim_tinyclaw
```

Expected result: default-feature test suite passes. Issue ground truth reports 592 passed and 1 ignored on clean Gitea `main` at `4aa1f8f271bfe8461ad01032a1410374699e67ce`.

**Acceptance: all-features no longer fails on Slack helper scope**

```bash
cargo test -p terraphim_tinyclaw --all-features
```

Expected result: no E0433 for `common::scrub_env()` in `tests/slack_integration.rs`.

If this command exposes unrelated all-features failures after the scope fix, record them separately and do not broaden this issue's implementation.

### Full Verification Commands

```bash
cargo fmt --all -- --check
cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run
cargo test -p terraphim_tinyclaw
cargo test -p terraphim_tinyclaw --all-features
cargo clippy -p terraphim_tinyclaw --all-targets --all-features -- -D warnings
ubs crates/terraphim_tinyclaw/tests/slack_integration.rs
```

No live Slack credential command is required for this issue because the acceptance target is all-features compilation and normal ignored-test handling, not live Slack API validation.

### Observed Verification Results

The orchestrator ran the approved verification sequence after the one-line import:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run`: passed; the prior Slack `E0433` is removed.
- `cargo test -p terraphim_tinyclaw`: passed with 592 tests and 1 ignored.
- `cargo test -p terraphim_tinyclaw --all-features`: the scoped Slack target compiles, then the suite reaches an unrelated failure in untouched `src/tools/voice_transcribe.rs`; 436 tests passed and `test_voice_feature_disabled_message` failed. This downstream defect is tracked separately as #3282 and is not caused by this import.
- `cargo clippy -p terraphim_tinyclaw --all-targets --all-features -- -D warnings`: failed on the untouched pre-existing `clippy::collapsible_if` at `src/tools/voice_transcribe.rs:391`; clean-main stash triage reproduced the same finding. #3282 owns that repair.
- `ubs crates/terraphim_tinyclaw/tests/slack_integration.rs`: unavailable because `ubs` is not installed on this host; no UBS result is claimed.

Accordingly, #3281 proves and fixes the Slack module-scope regression without claiming that unrelated repository-wide gates are green. Merge order is #3281 first, then #3282, because #3281 unlocks the full all-feature test build that exposes the voice failure.

## Implementation Steps

1. Edit `crates/terraphim_tinyclaw/tests/slack_integration.rs`.
2. Inside `#[cfg(feature = "slack")] mod slack_tests`, add `use crate::common;` before the existing imports.
3. Do not modify any test body.
4. Run the GREEN and regression verification commands.
5. Obtain an independent different-model structural review at 5/5 with P0=0, P1=0, and P2=0.
6. Report results against issue #3281 and #3143 acceptance unblock.

## Rollback Plan

Revert the single import line from `crates/terraphim_tinyclaw/tests/slack_integration.rs`.

Rollback has no data migration, no dependency impact, and no production behavior impact.

## Acceptance Traceability

| Acceptance Need | Design Element | Verification |
| --- | --- | --- |
| Fix #3281 all-features E0433 | Add `use crate::common;` inside nested `slack_tests`. | `cargo test -p terraphim_tinyclaw --all-features --test slack_integration --no-run` |
| Preserve #3161 scrubber intent | Keep `common::scrub_env()` as first executable line in both tests. | Code review plus Slack integration target compile. |
| Unblock #3143 acceptance | All-features TinyClaw test build no longer fails on Slack helper path. | `cargo test -p terraphim_tinyclaw --all-features` |
| Avoid scope creep | One planned test-file import, no production/test behavior rewrite. | `git diff --stat` after implementation. |
| Preserve strict Rust quality | The scoped import formats cleanly and introduces no lint; unrelated clean-main voice Clippy failure is tracked in #3282. | `cargo fmt --all -- --check`, focused Slack compile, clean-main triage, and strict all-target/all-feature Clippy caveat above. |
| Meet fleet merge bar | Independent different-model review finds no P0/P1/P2 and scores 5/5. | Verified review artifact and PR comment. |

## Approval Gate

- [x] Technical review complete
- [x] Scope approved as one-file test-harness compile fix
- [x] Security scrubber invariant approved
- [x] Verification sequence approved
- [x] Human approval received — Alexander Mikhalev, 2026-08-23
