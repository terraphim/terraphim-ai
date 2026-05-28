# Research Document: ADF Direct Dispatch Verification and Validation Gaps

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-26
**Reviewers**: Human maintainer, PR reviewers
**Related Issue**: terraphim/terraphim-ai#1875
**Related PRs**: GitHub PR #888, Gitea PR #1876

## Executive Summary

The ADF direct-dispatch feature has a sound architecture, but the latest structured PR review found that the implementation is not yet supported by sufficient Phase 4 verification or Phase 5 validation evidence. The remaining work is narrow: make the changed code lint-clean, add real Unix domain socket round-trip tests for the core IPC boundary, and run an end-to-end direct-dispatch acceptance scenario against a live orchestrator configured with `[direct_dispatch]`.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | The feature exists to remove local dispatch latency and simplify ADF agent workflows; the remaining gaps are concrete and close to completion. |
| Leverages strengths? | Yes | The repo already has Rust async tests, tokio UDS support, orchestrator channel abstractions, and V-model artefacts to trace the fix. |
| Meets real need? | Yes | The structural PR review identified missing IPC evidence and strict clippy failures that block confident review/merge. |

**Proceed**: Yes -- 3/3 YES.

## Problem Statement

### Description

The current direct-dispatch implementation adds a Unix domain socket listener and an `adf-ctl trigger --local --direct` client path, but its verification is incomplete. Existing tests cover JSON serialisation, socket cleanup safety, TOML socket-path parsing, and the `--direct`/`--local` CLI guard. They do not exercise a real listener/client round trip or verify that `WebhookDispatch::SpawnAgent` is emitted over the actual tokio channel.

The latest strict lint run also shows that the changed code does not pass `cargo clippy -p terraphim_orchestrator -- -D warnings` because of unused imports and one minor `to_string_in_format_args` warning.

### Impact

If this is not fixed:

- The feature can merge with the main IPC path untested.
- A subtle bug in line framing, response writing, listener startup, channel forwarding, or unknown-agent handling could reach production unnoticed.
- CI or reviewer quality gates that use `-D warnings` will fail.
- The PR cannot credibly claim disciplined verification or validation evidence.

### Success Criteria

1. `cargo clippy -p terraphim_orchestrator -- -D warnings` passes for the changed code.
2. `direct_dispatch.rs` includes real Unix socket round-trip tests for valid and unknown agents.
3. Tests prove a valid command results in `WebhookDispatch::SpawnAgent` with the expected agent and context.
4. Tests prove an unknown agent returns `{"status":"error"}` and does not send a dispatch.
5. Phase 5 validation documents at least one live or production-like `adf-ctl --local trigger <agent> --direct` run, or explicitly records why it is deferred.
6. The PR review can move from caution to approval or approval-with-follow-ups.

## Current State Analysis

### Existing Implementation

The implementation is split across the orchestrator, direct-dispatch module, CLI, and configuration:

- `DirectDispatchConfig` configures the socket path and defaults to `/tmp/adf-ctl.sock`.
- `start_direct_dispatch_listener()` binds a tokio `UnixListener`, applies 0600 permissions, accepts newline-delimited JSON, validates the agent name, and sends `WebhookDispatch::SpawnAgent` into the shared dispatch channel.
- `adf-ctl trigger --local --direct` resolves a socket path, writes JSON over `std::os::unix::net::UnixStream`, reads a JSON response, and optionally waits for the agent to exit.
- `lib.rs` bridges the shared dispatch channel into `LoopEvent::Webhook`, preserving the existing spawn path.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| CLI direct mode | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Parses `--direct`, resolves socket path, connects to UDS, handles response. |
| Direct listener | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Owns UDS listener, command parsing, validation, response writing, and dispatch forwarding. |
| Config type | `crates/terraphim_orchestrator/src/config.rs` | Defines `DirectDispatchConfig` and default socket path. |
| Orchestrator wiring | `crates/terraphim_orchestrator/src/lib.rs` | Starts listener when configured and bridges dispatch channel to event loop. |
| Local test config | `.terraphim/adf.toml` | Lists local agent names but does not enable listener configuration. |
| Original design | `.docs/design-adf-ctl-direct-dispatch.md` | Describes direct-dispatch architecture. |
| Remediation design | `.docs/design-adf-direct-dispatch-review-remediation.md` | Requested lint cleanup and UDS round-trip tests. |

### Data Flow

```text
adf-ctl --local trigger NAME --direct
  -> resolve_socket_path()
  -> UnixStream::connect(socket_path)
  -> write newline JSON { agent, context }
  -> direct_dispatch listener read_line()
  -> serde_json::from_str()
  -> validate against configured HashSet<String>
  -> dispatch_tx.send(WebhookDispatch::SpawnAgent)
  -> write {"status":"ok"}
  -> webhook_dispatch_rx bridge
  -> LoopEvent::Webhook
  -> handle_webhook_dispatch()
  -> spawn_agent()
```

### Integration Points

- **Unix domain socket**: local IPC boundary and filesystem-permission security boundary.
- **Tokio listener task**: async server accepting one JSON command per connection.
- **Tokio mpsc channel**: forwards `WebhookDispatch` into existing orchestrator dispatch handling.
- **Blocking CLI client**: appropriate for one-shot command-line use, but must not hang indefinitely in tests.
- **Configuration discovery**: CLI reads `ADF_DIRECT_SOCKET`, `.terraphim/adf.toml`, `ADF_ORCHESTRATOR_TOML`, `/opt/ai-dark-factory/orchestrator.toml`, then falls back to `/tmp/adf-ctl.sock`.

## Constraints

### Technical Constraints

- Unix domain sockets are only available on Unix targets; tests must be `#[cfg(unix)]`.
- No new external dependencies should be added.
- Tests must use real Unix sockets and real tokio channels; project instructions prohibit mocks.
- Tests must avoid command-line `timeout`; bounded async waits should use tokio primitives.
- `adf-ctl` currently uses a blocking std UnixStream by design to avoid making `cmd_trigger` async.
- The listener task runs forever; tests must abort the returned join handle.
- The repo has a dirty local worktree; implementation must avoid touching unrelated changes.

### Business Constraints

- Keep the patch small because the feature is already in PR review.
- Avoid redesigning dispatch or widening scope beyond verification/validation gaps.
- Preserve the existing `/tmp/adf-ctl.sock` default unless stakeholders explicitly change the decision.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Direct dispatch IPC correctness | Valid command produces one `WebhookDispatch::SpawnAgent` | Not tested through real socket. |
| Unknown-agent safety | Unknown agent returns error and sends no dispatch | Logic implied; not tested through real socket. |
| Lint hygiene | `cargo clippy -p terraphim_orchestrator -- -D warnings` passes | Fails on unused imports and formatting warning. |
| Test coverage on direct dispatch | Critical listener/client paths covered | `direct_dispatch.rs` line coverage around 41.92%. |
| Live acceptance evidence | One direct trigger against configured orchestrator | Not yet captured. |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Test the real IPC boundary | The feature's core behaviour is socket IO plus channel forwarding; unit-only tests miss that. | Structured PR review P1 finding. |
| Keep the patch lint-clean | Repo standards and CI expectations require clippy-clean code. | Strict clippy currently fails on changed code. |
| Keep scope to verification and validation gaps | The architecture is sound; redesign would add risk late in review. | Prior remediation decisions and PR review. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Replacing blocking CLI UnixStream with async client | Not required to close the review gap; would broaden API and tests. |
| Adding authentication tokens to UDS | Existing design deliberately uses 0600 filesystem permissions. |
| Changing the default socket path | The review remediation already aligned docs with `/tmp/adf-ctl.sock`; changing now would require broader stakeholder decision. |
| Adding admin socket for status/cancel/agents | Explicitly Phase 2/future work. |
| Full orchestrator integration harness in this patch | Real module-level UDS round-trip tests cover the critical boundary; live e2e can be documented as validation evidence. |
| New crates or test frameworks | Existing tokio/tempfile/serde_json support is enough. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `WebhookDispatch::SpawnAgent` | The direct listener constructs this event for the existing spawn path. | Low; type already exists and is used by webhook path. |
| `tokio::sync::mpsc::Sender<WebhookDispatch>` | Test must receive dispatch from the listener. | Low; channel can be created in test without mocks. |
| `tokio::net::UnixListener` and `UnixStream` | Required for real UDS tests. | Medium; startup race must be handled with bounded polling. |
| `remove_stale_socket_if_present()` | Ensures test paths are safe and stale sockets are handled. | Low; existing helper already covers regular-file rejection. |
| `adf-ctl` direct client | Live validation depends on built CLI and configured listener. | Medium; live orchestrator may not be running in session. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| None new | N/A | N/A | Existing tokio and tempfile are sufficient. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test flakiness from listener startup race | Medium | Medium | Poll for socket existence with a bounded async loop before connecting. |
| Test hang if response is never written | Low/Medium | Medium | Use bounded `tokio::time::timeout` around connect/read/recv operations. This is tokio API usage, not command-line timeout. |
| Listener task leaks after test | Medium | Low | Always call `handle.abort()` after assertions. |
| Unknown-agent test receives a stale dispatch | Low | Medium | Use a fresh channel per test and assert `try_recv()` or bounded timeout returns no message. |
| Live validation cannot be run in current environment | Medium | Medium | Document as deferred with exact manual command and required config; do not claim it passed. |

### Open Questions

1. Should live validation be required before merge, or is a real UDS round-trip test plus documented manual validation sufficient?
2. Should `.terraphim/adf.toml` gain a `[direct_dispatch]` section for local acceptance testing, or should the live orchestrator config remain the only listener-enabling source?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Tests may use `tokio::time::timeout` for bounded async waits. | Project instruction only forbids command-line `timeout`; test waits need bounded behaviour. | Tests could hang or violate local style if misunderstood. | Partially. |
| A module-level UDS test is enough to verify the direct-dispatch boundary. | It exercises listener bind, socket IO, JSON parsing, validation, response writing, and channel send. | Full orchestrator runtime bugs may remain. | No. |
| The `/tmp/adf-ctl.sock` default remains accepted. | Current code and remediation doc selected it for this patch. | CLI and docs may continue to diverge from original research preference. | Yes for current patch. |
| Strict clippy is expected for merge quality. | AGENTS.md requires lint cleanliness; review asked for evidence. | If CI does not use `-D warnings`, this may be non-blocking but still low-cost to fix. | Yes. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Verification requires full orchestrator e2e test in CI | Highest confidence, but requires a full orchestrator harness and may be slow/flaky. | Rejected for this patch; too broad late in review. |
| Verification requires direct listener/client round-trip module tests | Exercises the new IPC boundary with low overhead. | Chosen as vital and sufficient for Phase 4 remediation. |
| Validation requires live orchestrator command before merge | Strongest acceptance evidence. | Conditional; depends on environment availability and stakeholder preference. |
| Validation can be documented as manual follow-up | Fastest path, but weaker evidence. | Acceptable only if explicitly approved. |

## Research Findings

### Key Insights

1. The architecture is not the problem; evidence is. The listener forwards into the existing dispatch path rather than duplicating spawn logic.
2. The current tests miss the functions most likely to fail in production: `start_direct_dispatch_listener()`, `handle_connection()`, and `write_response()`.
3. The clippy failures are trivial and should be fixed in the same remediation patch.
4. The direct-dispatch module has low coverage because the listener and connection handler are untested; real UDS tests will materially improve confidence.
5. `.terraphim/adf.toml` currently lists agents but does not enable the listener, so it cannot by itself validate direct dispatch against a running orchestrator.

### Relevant Prior Art

- Existing webhook tests verify HTTP-facing behaviour but do not cover the UDS listener.
- The prior remediation design already specified two UDS round-trip tests and a bounded socket wait helper.
- Rust/tokio idioms support spawning a listener, waiting for the socket path, connecting with `tokio::net::UnixStream`, and aborting the join handle after assertions.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Minimal UDS round-trip test | Confirm listener scheduling and socket readiness pattern in this repo. | 20-30 minutes |
| Live orchestrator validation | Confirm `adf-ctl --local trigger <agent> --direct` works with a configured local orchestrator. | 30-60 minutes, environment-dependent |

## Recommendations

### Proceed/No-Proceed

Proceed with a narrow verification/validation remediation patch. The core implementation appears structurally sound, and the remaining issues are small, testable, and high-value.

### Scope Recommendations

Implement only:

1. Lint cleanup in changed Rust files.
2. Real UDS valid-agent round-trip test.
3. Real UDS unknown-agent error/no-dispatch test.
4. Optional socket-permissions assertion if stable on CI.
5. Validation report update documenting the live acceptance scenario and whether it was executed.

### Risk Mitigation Recommendations

- Use `tokio::time::timeout` around connect/read/channel receive to prevent hangs.
- Use tempdirs for socket paths.
- Abort listener join handles at the end of each test.
- Keep tests under `#[cfg(unix)]`.
- Do not edit unrelated dirty worktree files.

## Next Steps

If approved:

1. Write the Phase 2 implementation plan for the minimal remediation.
2. Implement lint cleanup and UDS round-trip tests.
3. Run strict verification commands.
4. Run or document Phase 5 live validation.
5. Update PR review evidence and issue #1875.

## Appendix

### Reference Materials

- `.docs/research-adf-ctl-direct-dispatch.md`
- `.docs/design-adf-ctl-direct-dispatch.md`
- `.docs/research-adf-direct-dispatch-review-remediation.md`
- `.docs/design-adf-direct-dispatch-review-remediation.md`
- Structured PR review for commit `f980fec82`

### Current Failing Evidence

```text
cargo clippy -p terraphim_orchestrator -- -D warnings

error: unused import: `std::sync::Arc`
  --> crates/terraphim_orchestrator/src/direct_dispatch.rs:10:5

error: unused import: `tokio::sync::Mutex`
  --> crates/terraphim_orchestrator/src/direct_dispatch.rs:13:5

error: unused import: `AsyncWriteExt`
   --> crates/terraphim_orchestrator/src/direct_dispatch.rs:154:38
```

### Current Passing Evidence

```text
cargo test -p terraphim_orchestrator direct_dispatch
7 passed

cargo test -p terraphim_orchestrator --bin adf-ctl
26 passed

cargo test -p terraphim_orchestrator --lib
788 passed

cargo llvm-cov -p terraphim_orchestrator --lib --summary-only
TOTAL line coverage: 72.88%
direct_dispatch.rs line coverage: 41.92%
```
