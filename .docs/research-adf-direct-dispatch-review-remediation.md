# Research Document: ADF Direct Dispatch Review Remediation

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-26
**Reviewers**: Pending

## Executive Summary

The direct-dispatch implementation provides the intended low-latency Unix domain socket path, but structural review identified four gaps that should be resolved before merge. The essential work is to make CLI semantics explicit, harden socket cleanup, align configuration documentation with behaviour, and add a real socket round-trip test.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This closes correctness and safety gaps in a feature that is meant to remove dispatch latency from ADF workflows. |
| Leverages strengths? | Yes | The work is concentrated in Rust CLI/orchestrator boundaries, async IO, and testable protocol contracts. |
| Meets real need? | Yes | The review found concrete merge-blocking or merge-relevant risks in the current implementation. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

The current direct-dispatch implementation is functional at the broad architecture level but has review findings that can lead to confusing runtime behaviour, unsafe cleanup of configured paths, inconsistent operator documentation, and insufficient test evidence for the UDS protocol.

### Impact

ADF operators and agents depend on `adf-ctl trigger --local --direct` doing exactly what it says: local direct socket dispatch without webhook or HMAC. If direct mode can silently fall back to webhook/SSH, if startup can remove the wrong file, or if config documentation points users at the wrong path, the feature becomes harder to operate safely and diagnose.

### Success Criteria

1. `adf-ctl trigger --direct ...` without `--local` fails fast with a clear error.
2. Direct-dispatch listener removes only stale Unix socket files and refuses to remove regular files or other filesystem entries.
3. `DirectDispatchConfig` documentation and implementation agree on the default path.
4. Tests exercise the actual UDS request/response path for both valid and invalid agents.
5. Existing `cargo test -p terraphim_orchestrator --lib` and `cargo test -p terraphim_orchestrator --bin adf-ctl` remain green.

## Current State Analysis

### Existing Implementation

`adf-ctl` accepts a new `--direct` flag on the `trigger` subcommand. `cmd_trigger` enters the direct socket path only when both `local` and `direct` are true, otherwise it continues through the existing HMAC/webhook path. Socket path discovery currently checks `ADF_DIRECT_SOCKET`, `.terraphim/adf.toml`, `ADF_ORCHESTRATOR_TOML`, `/opt/ai-dark-factory/orchestrator.toml`, then `/tmp/adf-ctl.sock`.

The orchestrator creates one `tokio::sync::mpsc` dispatch channel for webhook and direct dispatch, starts the direct listener when `config.direct_dispatch` is present, and forwards accepted socket commands as `WebhookDispatch::SpawnAgent` events.

The listener removes any existing filesystem entry at `socket_path` before binding. It then binds a `tokio::net::UnixListener`, attempts to set mode `0600`, accepts newline-delimited JSON, validates `agent` against configured names, sends `WebhookDispatch::SpawnAgent`, and returns JSON `{status}` responses.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Direct CLI flag and dispatch client | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Parses `--direct`, resolves socket path, sends JSON over `std::os::unix::net::UnixStream`. |
| Direct dispatch listener | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Binds UDS, validates command, forwards to orchestrator dispatch channel. |
| Orchestrator startup wiring | `crates/terraphim_orchestrator/src/lib.rs` | Creates shared dispatch channel and starts direct-dispatch listener when configured. |
| Direct dispatch config | `crates/terraphim_orchestrator/src/config.rs` | Defines `DirectDispatchConfig` and default socket path. |
| Project-local ADF conversion | `crates/terraphim_orchestrator/src/project_adf.rs` | Builds full `OrchestratorConfig` from `.terraphim/adf.toml`, currently sets `direct_dispatch: None`. |

### Data Flow

```text
adf-ctl trigger --local --direct AGENT
  -> resolve_socket_path()
  -> UnixStream::connect(socket_path)
  -> write JSON line { agent, context }
  -> direct_dispatch::handle_connection()
  -> validate agent name
  -> dispatch_tx.send(WebhookDispatch::SpawnAgent)
  -> orchestrator main loop handles dispatch
  -> CLI reads JSON response
```

### Integration Points

| Interface | Producer | Consumer | Notes |
|-----------|----------|----------|-------|
| CLI flag `--direct` | Clap in `adf-ctl.rs` | `cmd_trigger` | Must fail if used without `--local`. |
| JSON line protocol | `direct_dispatch_via_socket` | `handle_connection` | Existing tests cover JSON types, not socket IO. |
| `WebhookDispatch::SpawnAgent` | direct listener | orchestrator loop | Reuses existing webhook dispatch flow. |
| `direct_dispatch.socket_path` TOML | config files | CLI and orchestrator | Documentation/defaults need alignment. |

## Constraints

### Technical Constraints

- Rust workspace with existing `tokio`, `serde_json`, and `toml`; no new dependencies requested.
- Unix domain socket behaviour is Unix-specific; tests that inspect file type or permissions should be `#[cfg(unix)]`.
- `adf-ctl` trigger implementation is synchronous today; the minimal fix should avoid making the whole CLI async.
- The direct-dispatch listener currently returns a `JoinHandle<()>`, so test code must abort or drop the listener task after assertions.

### Business Constraints

- This work is a review-remediation pass for issue #1875, not a new feature expansion.
- The change should be small enough to merge into the existing task branch.
- No broader redesign of ADF dispatch or authentication is in scope.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Local direct-dispatch latency | One socket round trip plus next orchestrator loop event | Architecture supports this, pending e2e verification. |
| Local socket authorisation | Owner-only socket permissions | Listener attempts `0600`, but startup cleanup needs hardening. |
| CLI predictability | Flags never silently change mode | `--direct` is ignored without `--local`. |
| Test evidence | UDS round-trip covered | Only JSON serialisation tests exist. |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Explicit CLI semantics | Prevents direct mode from silently falling back to webhook/SSH. | Structural review P1. |
| Safe socket path handling | Prevents deleting a misconfigured regular file. | Listener currently calls `remove_file` on any existing path. |
| Real protocol test | Catches regressions across framing, IO, validation, and channel handoff. | Existing tests stop at serialisation. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Remote direct dispatch over SSH tunnel | Not required for local direct mode and would add auth/network complexity. |
| Replacing `WebhookDispatch` with a new dispatch enum | Existing path works and reduces implementation surface. |
| Adding HMAC to UDS protocol | Single-user local socket permissions are the accepted security boundary. |
| Supporting non-Unix direct dispatch in this patch | Current feature is explicitly UDS-based. |
| Project-local listener enablement redesign | Ambiguous; can be documented or deferred unless product confirms `.terraphim/adf.toml` should start listeners. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `WebhookDispatch::SpawnAgent` | Direct dispatch reuses webhook processing and event-only validation. | Low; already tested by webhook paths. |
| `MentionCursor::mark_processed` | Direct events use `comment_id = 0`, which is marked processed after dispatch. | Low for local dispatch, but keep in mind if dedup semantics change. |
| `discover_local_config()` | CLI socket discovery may read `.terraphim/adf.toml`. | Medium; project-local config currently cannot start the listener. |
| `tokio::net::UnixListener` | Listener bind/accept behaviour and tests. | Low on Unix CI; unavailable or gated on non-Unix. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| Tokio | Existing workspace dependency | Low | Standard library sockets, but async listener already fits orchestrator. |
| serde_json | Existing workspace dependency | Low | Manual JSON not justified. |
| toml | Existing workspace dependency | Low | Existing config parser path. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `--direct` without `--local` silently does remote/webhook dispatch | Medium | High | Add fail-fast guard and CLI test. |
| Listener removes regular file at configured path | Low-Medium | Medium | Check `symlink_metadata().file_type().is_socket()` before removing. |
| Tests hang because listener task runs forever | Medium | Medium | Keep test-scoped `JoinHandle`, abort after assertions, use unique temp socket path. |
| `.terraphim/adf.toml` socket discovery misleads users | Medium | Medium | Decide whether to remove this discovery source or map project config into `direct_dispatch`. |
| Existing doc says `<working_dir>/.adf-ctl.sock` while code uses `/tmp/adf-ctl.sock` | High | Low-Medium | Pick one default and update docs/tests accordingly. |

### Open Questions

1. Should `.terraphim/adf.toml` be able to configure and enable the direct-dispatch listener, or should only full `orchestrator.toml` do that?
2. Should the long-term default socket path be `/tmp/adf-ctl.sock` for easy discovery or `<working_dir>/.adf-ctl.sock` to avoid cross-project collisions?

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `--direct` is valid only with `--local`. | CLI help says direct dispatch is local-mode only; review finding agrees. | Remote users may expect a tunnelled direct path. | Yes. |
| No new dependencies should be added. | Original implementation constraint and current workspace already has required crates. | Tests may need more manual setup. | Yes. |
| `/tmp/adf-ctl.sock` is the current implementation default. | `DEFAULT_SOCKET_PATH` and `DirectDispatchConfig::default_socket_path`. | Documentation may need updating rather than code. | Yes. |
| Hardening stale socket cleanup is preferable to ignoring bind failures. | Prevents accidental deletion and gives clear operator feedback. | Existing stale non-socket paths require manual cleanup. | Yes. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| `--direct` without `--local` should imply local mode. | Convenient but hides a mode change and still bypasses remote host options. | Rejected; explicit failure is safer. |
| `--direct` without `--local` should tunnel over SSH. | Larger feature, auth semantics, latency assumptions change. | Rejected as out of scope. |
| Listener can remove any existing path as stale. | Simple but unsafe under misconfiguration. | Rejected by review; use socket type check. |
| Default socket is `/tmp/adf-ctl.sock`. | Easy global discovery, but possible project collision. | Current implementation; acceptable if documented. |
| Default socket is `<working_dir>/.adf-ctl.sock`. | Avoids project collision, but CLI must resolve working directory consistently. | Defer unless user prefers changing behaviour. |

## Research Findings

### Key Insights

1. The architectural approach is sound: direct dispatch should continue to reuse `WebhookDispatch::SpawnAgent` rather than creating a second spawn path.
2. The most important code fix is a small guard in `cmd_trigger`: `if direct && !local { bail!(...) }` before secret resolution or webhook payload construction.
3. Socket cleanup can be hardened locally in `direct_dispatch.rs` without changing the public protocol or config shape.
4. A meaningful integration test can be added inside `direct_dispatch.rs` using a temp directory under `std::env::temp_dir()`, `tokio::net::UnixStream`, and an mpsc receiver assertion; no mocks are required.
5. The `.terraphim/adf.toml` discovery path is the only design ambiguity. The safest remediation is to document the current split and avoid changing enablement semantics until explicitly approved.

### Relevant Prior Art

- Existing webhook dispatch path: validates agent names and uses `WebhookDispatch` to feed the main orchestrator loop.
- Existing direct-dispatch draft design: planned socket discovery through `.terraphim/adf.toml`, but implementation currently only enables the listener from `OrchestratorConfig.direct_dispatch`.
- Unix stale socket cleanup patterns: unlink stale socket files only after checking the existing path is a socket.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| UDS listener round-trip test | Confirm test harness can bind, connect, assert response, receive dispatch, and abort listener cleanly. | 30-45 minutes |
| Socket default decision | Decide `/tmp/adf-ctl.sock` vs working-dir socket before final docs. | 10 minutes stakeholder decision |

## Recommendations

### Proceed/No-Proceed

Proceed with a minimal remediation patch. The P1 and P2 findings are concrete, bounded, and low-risk to fix.

### Scope Recommendations

Implement exactly four remediation items:

1. Fail fast when `--direct` is used without `--local`.
2. Harden stale socket cleanup to remove sockets only.
3. Align default socket path documentation and tests with `/tmp/adf-ctl.sock` unless the user explicitly chooses working-dir default.
4. Add real UDS round-trip tests for valid and invalid agents.

### Risk Mitigation Recommendations

- Keep all socket path tests isolated with unique paths under a temporary directory.
- Use `#[cfg(unix)]` for socket-file-type and permission assertions.
- Avoid broadening `.terraphim/adf.toml` semantics in this patch unless clarified.
- Run the narrow tests first, then `cargo test -p terraphim_orchestrator --lib` and `cargo test -p terraphim_orchestrator --bin adf-ctl`.

## Next Steps

If approved:

1. Implement the CLI guard and test.
2. Implement socket cleanup helper and tests.
3. Add UDS round-trip tests.
4. Update config documentation to match implementation.
5. Re-run targeted and package-level tests.

## Appendix

### Reference Materials

- `.docs/research-adf-ctl-direct-dispatch.md`
- `.docs/design-adf-ctl-direct-dispatch.md`
- Structural review from 2026-05-26 for direct-dispatch changes.

### Code Snippets

Current CLI direct-mode branch:

```rust
if local && direct {
    let socket_path = resolve_socket_path()?;
    direct_dispatch_via_socket(&socket_path, name, Some(context))?;
    ...
    return Ok(());
}
```

Current listener cleanup:

```rust
if socket_path.exists() {
    if let Err(e) = std::fs::remove_file(&socket_path) {
        tracing::warn!(...);
    }
}
```
