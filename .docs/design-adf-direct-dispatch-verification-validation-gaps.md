# Implementation Plan: ADF Direct Dispatch Verification and Validation Gaps

**Status**: Draft
**Research Doc**: `.docs/research-adf-direct-dispatch-verification-validation-gaps.md`
**Author**: OpenCode
**Date**: 2026-05-26
**Estimated Effort**: 2-3 hours
**Related Issue**: terraphim/terraphim-ai#1875
**Related PRs**: GitHub PR #888, Gitea PR #1876

## Overview

### Summary

This plan closes the remaining Phase 4 verification and Phase 5 validation gaps found by the latest structured PR review for ADF direct dispatch. It keeps the existing architecture and adds only the missing evidence: strict lint cleanliness, real Unix domain socket round-trip tests, and a documented live acceptance scenario.

### Approach

Apply the smallest correct changes:

1. Remove unused imports and the minor `to_string_in_format_args` warning.
2. Add two real `#[tokio::test]` Unix socket tests inside `direct_dispatch.rs` using real sockets and real tokio mpsc channels.
3. Add small test helpers for bounded socket readiness and response reads.
4. Run strict verification commands and update validation evidence.

No dispatch redesign, no new dependencies, no admin socket, no default-path change.

### Scope

**In Scope:**

- Remove unused imports from `direct_dispatch.rs`.
- Replace `writeln!(stream, "{}", payload.to_string())` with a clippy-clean equivalent in `adf-ctl.rs`.
- Add `test_direct_dispatch_socket_valid_agent_round_trip`.
- Add `test_direct_dispatch_socket_unknown_agent_returns_error`.
- Optionally add `test_direct_dispatch_socket_permissions_owner_only` if it is stable on Unix CI.
- Run strict verification commands.
- Document Phase 5 validation status and manual/live command evidence.

**Out of Scope:**

- Refactoring `cmd_trigger()` to async.
- Changing `/tmp/adf-ctl.sock` default path.
- Enabling direct dispatch from `.terraphim/adf.toml` without stakeholder approval.
- Adding HMAC/token auth to the UDS protocol.
- Implementing status/cancel/list admin socket commands.
- Rewriting webhook dispatch or `LoopEvent` architecture.

**Avoid At All Cost** (from 5/25 analysis):

- Do not create a second spawn pipeline outside `WebhookDispatch::SpawnAgent`.
- Do not add mocks for socket or channel behaviour.
- Do not add new crates for test synchronisation.
- Do not silently skip unknown-agent no-dispatch assertions.
- Do not claim live validation passed unless it was actually executed.

## Architecture

### Component Diagram

```text
#[tokio::test]
  |
  | create tempdir socket path
  | create tokio mpsc channel
  | start_direct_dispatch_listener(socket_path, tx, agent_names)
  v
UnixListener task
  |
  | wait_for_socket(socket_path) with bounded async loop
  v
tokio::net::UnixStream client
  |
  | write newline JSON
  | read one response line
  v
Assertions
  |
  | valid agent -> response status ok + rx receives SpawnAgent
  | unknown agent -> response status error + rx receives no dispatch
  v
handle.abort()
```

### Data Flow

```text
Test client -> UnixStream -> listener.accept()
  -> handle_connection()
  -> serde_json parse
  -> agent_names.contains()
  -> dispatch_tx.send(WebhookDispatch::SpawnAgent) OR error response
  -> JSON response line
  -> test assertion
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Put tests in `direct_dispatch.rs` | Gives access to private helpers and keeps evidence close to implementation. | New integration test crate with broader public API. |
| Use real `tokio::net::UnixStream` in tests | Exercises actual IPC boundary and line framing. | Unit-only validation of structs and `HashSet` membership. |
| Use `tokio::time::timeout` for bounded awaits | Prevents hangs without using command-line `timeout`. | Unbounded `await` or sleep-only polling. |
| Abort listener handles after tests | Listener is intentionally long-lived; tests must clean it up. | Rely on test process teardown. |
| Keep `/tmp/adf-ctl.sock` default | Current code and docs already align; changing default is a separate decision. | Reverting to `<working_dir>/.adf-ctl.sock` in this remediation. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Full orchestrator test harness in this patch | The gap is specifically the UDS boundary; a full harness is slower and broader. | Flaky tests and larger review surface. |
| Async CLI refactor | Not required for correctness or evidence. | Cascading changes to CLI signatures and tests. |
| New public `handle_connection` API | Tests can live in the module and use the listener. | API surface expansion for tests only. |
| Socket auth token | Not part of the current security model. | Secret management complexity and UX changes. |
| Changing local `.terraphim/adf.toml` semantics | Needs stakeholder decision. | Surprising listener enablement from project test config. |

### Simplicity Check

What if this could be easy?

The easy version is two real socket tests and four lint fixes. Start the listener with a one-agent set, connect over a temp socket, write a JSON line, read the response, and inspect the channel. Repeat with an unknown agent and assert no channel message. This directly covers the review finding without changing architecture.

**Senior Engineer Test**: This is not overcomplicated; it tests the behaviour users depend on and removes only obvious lint issues.

**Nothing Speculative Checklist**:

- [x] No features the user did not request.
- [x] No abstractions for future expansion.
- [x] No flexibility just in case.
- [x] No error handling for scenarios that cannot occur.
- [x] No premature optimisation.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `.docs/research-adf-direct-dispatch-verification-validation-gaps.md` | Phase 1 research for remaining evidence gaps. |
| `.docs/design-adf-direct-dispatch-verification-validation-gaps.md` | Phase 2 implementation plan for the remediation. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Remove unused imports; add test helpers; add valid-agent and unknown-agent UDS round-trip tests; optionally assert socket permissions. |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Replace direct-dispatch `writeln!` payload formatting with a clippy-clean form. |
| `.docs/validation-adf-direct-dispatch.md` or PR comment | Record Phase 5 live validation evidence or explicit deferral. |

### Deleted Files

None.

## API Design

### Public Types

No new public types.

### Public Functions

No new public functions.

### Internal Test Helpers

Add private test helpers in `direct_dispatch.rs` under `#[cfg(test)]`.

```rust
#[cfg(unix)]
async fn wait_for_socket(path: &std::path::Path) {
    for _ in 0..50 {
        if path.exists() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("socket was not created at {}", path.display());
}
```

Use bounded timeouts around operations that can hang:

```rust
let stream = tokio::time::timeout(
    std::time::Duration::from_secs(2),
    tokio::net::UnixStream::connect(&socket_path),
)
.await
.expect("socket connect timed out")
.expect("socket connect failed");
```

### Error Types

No new error types.

## Test Strategy

### Unit Tests

Existing unit tests remain:

| Test | Location | Purpose |
|------|----------|---------|
| `test_dispatch_command_deserialize` | `direct_dispatch.rs` | Valid command JSON with context. |
| `test_dispatch_command_deserialize_no_context` | `direct_dispatch.rs` | Valid command JSON without context. |
| `test_dispatch_response_ok` | `direct_dispatch.rs` | OK response serialisation. |
| `test_dispatch_response_error` | `direct_dispatch.rs` | Error response serialisation. |
| `test_remove_stale_socket_rejects_regular_file` | `direct_dispatch.rs` | Refuse to remove non-socket file. |
| `test_remove_stale_socket_removes_nonexistent` | `direct_dispatch.rs` | Missing socket path is acceptable. |
| `test_trigger_direct_requires_local` | `adf-ctl.rs` | `--direct` requires `--local`. |
| `test_parse_socket_path_from_toml*` | `adf-ctl.rs` | Direct-dispatch socket config parsing. |

### Integration-Style Module Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_direct_dispatch_socket_valid_agent_round_trip` | `direct_dispatch.rs`, `#[cfg(unix)]`, `#[tokio::test]` | Start listener, connect through real UDS, send known agent JSON, assert `status=ok`, assert channel receives `WebhookDispatch::SpawnAgent`. |
| `test_direct_dispatch_socket_unknown_agent_returns_error` | `direct_dispatch.rs`, `#[cfg(unix)]`, `#[tokio::test]` | Send unknown agent JSON, assert error response, assert no dispatch is emitted. |
| `test_direct_dispatch_socket_permissions_owner_only` | `direct_dispatch.rs`, optional `#[cfg(unix)]` | Assert socket mode masks to `0o600` after listener startup. |

### Acceptance / Validation Scenario

If a local orchestrator can be run:

```bash
cargo run -p terraphim_orchestrator --bin adf-ctl -- --local trigger meta-learning --direct --context "direct dispatch validation"
```

Expected evidence:

- CLI prints `Agent dispatched via direct socket: meta-learning`.
- Orchestrator logs show direct dispatch socket listening.
- Orchestrator logs show `spawning agent=meta-learning` or equivalent spawn classification.
- No `ADF_WEBHOOK_SECRET` is required for this command.

If no live orchestrator is available, record validation as conditional and include exact required config:

```toml
[direct_dispatch]
socket_path = "/tmp/adf-ctl.sock"
```

## Implementation Steps

### Step 1: Lint Cleanup

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`, `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`

**Description:** Remove unused imports and replace unnecessary `payload.to_string()` in `writeln!` format args.

**Tests:** `cargo clippy -p terraphim_orchestrator -- -D warnings` after all steps.

**Estimated:** 10 minutes.

Key changes:

```rust
// direct_dispatch.rs: remove unused imports
use std::collections::HashSet;
use std::path::PathBuf;

use tokio::net::UnixListener;
use tracing::{error, info};
```

```rust
// adf-ctl.rs
writeln!(stream, "{payload}")
    .context("failed to write to direct dispatch socket")?;
```

### Step 2: Add Valid-Agent UDS Round-Trip Test

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`

**Description:** Start listener on a temp socket, send a valid JSON command, assert OK response and exact `WebhookDispatch::SpawnAgent` contents.

**Tests:** New `test_direct_dispatch_socket_valid_agent_round_trip`.

**Dependencies:** Step 1 independent.

**Estimated:** 35-45 minutes.

Key code shape:

```rust
#[cfg(unix)]
#[tokio::test]
async fn test_direct_dispatch_socket_valid_agent_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("adf.sock");
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let agent_names = ["meta-learning".to_string()].into_iter().collect();

    let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_names);
    wait_for_socket(&socket_path).await;

    let response = send_command(&socket_path, r#"{"agent":"meta-learning","context":"test"}"#).await;
    assert_eq!(response["status"], "ok");

    let dispatch = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("dispatch receive timed out")
        .expect("dispatch channel closed");

    match dispatch {
        WebhookDispatch::SpawnAgent { agent_name, context, issue_number, comment_id, .. } => {
            assert_eq!(agent_name, "meta-learning");
            assert_eq!(context, "test");
            assert_eq!(issue_number, 0);
            assert_eq!(comment_id, 0);
        }
        other => panic!("unexpected dispatch: {other:?}"),
    }

    handle.abort();
}
```

### Step 3: Add Unknown-Agent UDS Error Test

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`

**Description:** Start listener with a known agent set, send an unknown agent, assert error response and no channel dispatch.

**Tests:** New `test_direct_dispatch_socket_unknown_agent_returns_error`.

**Dependencies:** Step 2 helper functions.

**Estimated:** 25-35 minutes.

Key code shape:

```rust
#[cfg(unix)]
#[tokio::test]
async fn test_direct_dispatch_socket_unknown_agent_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("adf.sock");
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let agent_names = ["meta-learning".to_string()].into_iter().collect();

    let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_names);
    wait_for_socket(&socket_path).await;

    let response = send_command(&socket_path, r#"{"agent":"unknown-agent"}"#).await;
    assert_eq!(response["status"], "error");
    assert!(response["message"].as_str().unwrap().contains("unknown agent"));
    assert!(rx.try_recv().is_err(), "unknown agent must not dispatch");

    handle.abort();
}
```

### Step 4: Optional Socket Permission Test

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`

**Description:** If stable on local and CI Unix platforms, assert socket permissions are owner read/write only (`0o600`).

**Tests:** New `test_direct_dispatch_socket_permissions_owner_only`.

**Dependencies:** Step 2 helper functions.

**Estimated:** 10-15 minutes.

Acceptance rule: omit this test if platform behaviour makes it flaky; keep permission setting covered by code review and live validation instead.

### Step 5: Verification Commands

**Files:** None.

**Description:** Run strict evidence commands.

**Dependencies:** Steps 1-3.

**Estimated:** 30-45 minutes.

Commands:

```bash
ubs crates/terraphim_orchestrator/src/bin/adf-ctl.rs \
    crates/terraphim_orchestrator/src/config.rs \
    crates/terraphim_orchestrator/src/direct_dispatch.rs \
    crates/terraphim_orchestrator/src/lib.rs

cargo test -p terraphim_orchestrator direct_dispatch
cargo test -p terraphim_orchestrator --bin adf-ctl
cargo test -p terraphim_orchestrator --lib
cargo fmt -- --check
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo llvm-cov -p terraphim_orchestrator --lib --summary-only
```

Pass criteria:

- Direct-dispatch tests include at least 9 tests: existing 7 plus 2 UDS round-trip tests.
- `adf-ctl` tests remain 26 passing, unless a new CLI test is added.
- Lib tests remain 788 passing or expected count if new tests are included in lib count.
- Strict clippy passes.
- Coverage report is recorded; no hard global coverage threshold is introduced in this patch.

### Step 6: Validation Evidence Update

**Files:** `.docs/validation-adf-direct-dispatch.md` or PR/Gitea issue comment.

**Description:** Record whether live direct dispatch was executed.

**Dependencies:** Step 5.

**Estimated:** 20-40 minutes if environment is ready; otherwise 10 minutes to document deferral.

Validation checklist:

- [ ] Orchestrator config includes `[direct_dispatch] socket_path = "/tmp/adf-ctl.sock"` or equivalent.
- [ ] Orchestrator is running and logs `direct dispatch socket listening`.
- [ ] `adf-ctl --local trigger meta-learning --direct --context "direct dispatch validation"` succeeds.
- [ ] No `ADF_WEBHOOK_SECRET` is required for the direct command.
- [ ] Logs show the agent spawn request reached the orchestrator.
- [ ] If validation is deferred, reason and exact reproduction commands are documented.

## Rollback Plan

If remediation tests are unstable or fail unexpectedly:

1. Keep lint cleanup; it is independently safe.
2. Remove only the unstable optional permission test if platform-dependent.
3. Keep valid-agent and unknown-agent round-trip tests unless they reveal a real implementation defect.
4. If round-trip tests expose a listener bug, loop back to Phase 3 implementation and fix the listener, not the test.

Feature disablement remains configuration-based: omit `[direct_dispatch]` from the orchestrator config to avoid starting the listener.

## Migration

No data migration is required.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| UDS test runtime | Sub-second for new tests | `cargo test -p terraphim_orchestrator direct_dispatch` output. |
| Direct dispatch overhead | One socket connect/write/read | Functional test proves path; benchmark not required. |
| Memory | No meaningful change | No new runtime allocations beyond test helpers. |

### Benchmarks to Add

None. This remediation is correctness/evidence work, not performance tuning.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide whether live validation is required before merge | Pending | Human maintainer |
| Decide whether `.terraphim/adf.toml` should include `[direct_dispatch]` in a later patch | Deferred | Human maintainer |
| Confirm optional permission test stability | Pending during implementation | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Validation requirement decided
- [ ] Human approval received

## Phase 3 Handoff

Implementation order:

1. Remove lint warnings.
2. Add shared test helpers in `direct_dispatch.rs` tests.
3. Add valid-agent UDS round-trip test.
4. Add unknown-agent no-dispatch test.
5. Optionally add permission assertion if stable.
6. Run verification commands.
7. Update validation evidence and issue/PR comments.

Do not broaden the patch beyond these items without returning to Phase 2 design.
