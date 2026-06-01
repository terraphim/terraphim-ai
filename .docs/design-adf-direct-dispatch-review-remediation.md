# Implementation Plan: ADF Direct Dispatch Review Remediation

**Status**: Draft
**Research Doc**: `.docs/research-adf-direct-dispatch-review-remediation.md`
**Author**: OpenCode
**Date**: 2026-05-26
**Estimated Effort**: 2-3 hours

## Overview

### Summary

This plan remediates the structural review findings for the ADF direct-dispatch feature with the smallest correct code changes. It makes `--direct` semantics explicit, prevents unsafe socket-path cleanup, aligns operator documentation with runtime defaults, and adds real UDS protocol tests.

**2026-05-28 update:** direct dispatch is explicitly allowed to run `event_only` agents locally. The plan preserves that behaviour and improves it by reusing existing synthetic event, project resolution, and validation primitives rather than blocking event-only agents.

### Approach

Keep the existing architecture: `adf-ctl` sends newline-delimited JSON over a Unix domain socket, and the listener forwards valid commands as `WebhookDispatch::SpawnAgent` into the existing orchestrator dispatch channel. Apply targeted guardrails and tests around the current implementation rather than redesigning dispatch.

### Scope

**In Scope:**

- Reject `adf-ctl trigger --direct` unless `--local` is also set.
- Replace broad stale socket removal with socket-type-checked cleanup.
- Update direct-dispatch socket path documentation to match the current `/tmp/adf-ctl.sock` default.
- Add socket path and direct flag tests in `adf-ctl.rs`.
- Add real Unix domain socket round-trip tests in `direct_dispatch.rs`.
- Run targeted package tests after implementation.
- Preserve direct local/admin dispatch for `event_only` agents.
- Add optional synthetic push/PR event context for event-only local runs.
- Reuse existing project-aware agent resolution for `project/agent` direct dispatch.
- Validate CLI agent names before they are used in shell-backed status/wait/cancel paths.

**Out of Scope:**

- Changing the dispatch channel architecture.
- Adding remote direct dispatch.
- Adding HMAC or another auth layer to UDS.
- Changing project-local `.terraphim/adf.toml` to start the listener.
- Adding new crates.
- Blocking event-only agents from local direct dispatch.
- Replacing direct dispatch with a full admin socket protocol.
- Routing direct dispatch through the fairness queue unless explicitly requested later.

**Avoid At All Cost** (from 5/25 analysis):

- Do not create a second agent-spawn path outside `WebhookDispatch`.
- Do not make `cmd_trigger` async only for this socket path.
- Do not add speculative cross-platform named pipe support.
- Do not introduce compatibility fallbacks that silently switch dispatch modes.
- Do not broaden `.terraphim/adf.toml` semantics without explicit approval.
- Do not remove the local escape hatch for event-only agents.
- Do not invent a second event metadata model when `SyntheticEvent` already exists.

## Architecture

### Component Diagram

```text
adf-ctl Trigger args
  |
  |-- if direct && !local -> error and exit
  |
  |-- if local && direct -> resolve socket -> build DirectDispatchCommand
  |                         optional SyntheticEvent push/pr metadata
  |                         -> UnixStream JSON line
  |                                |
  |                                v
  |                         direct_dispatch listener
  |                                |
  |                     validate agent/project + send direct dispatch
  |                                |
  |                                v
  |                         orchestrator event loop
  |                                |
  |              resolve project/agent + inject SyntheticEvent env vars
  |
  |-- otherwise -> existing webhook/HMAC path
```

### Data Flow

```text
CLI args -> cmd_trigger guard -> optional SyntheticEvent parse
  -> resolve_socket_path -> UnixStream write DirectDispatchCommand
  -> listener accept -> JSON parse -> basic command validation
  -> orchestrator event loop -> project-aware resolution
  -> optional SyntheticEvent::env_vars() into SpawnContext
  -> spawn_agent -> JSON response -> CLI status/error
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Fail fast on `--direct` without `--local` | Prevents silently using the wrong dispatch mode. | Imply local mode; SSH-tunnel direct dispatch. |
| Keep `/tmp/adf-ctl.sock` as default for this patch | Matches current implementation and CLI fallback. | Switching to working-dir default would change behaviour and require more discovery work. |
| Check existing path is a Unix socket before removal | Prevents deleting regular files under misconfiguration. | Continue broad `remove_file`; ignore existing paths until bind fails. |
| Add tests inside existing modules | Minimises visibility/API churn and avoids new dependencies. | New integration test crate with extra harness. |
| Allow event-only direct dispatch | Direct dispatch is the intended local/admin way to run push/PR-only agents manually. | Applying mention-dispatch event-only rejection to direct mode. |
| Reuse `SyntheticEvent` for local event context | Existing type already maps push/PR fields into env vars. | New direct-dispatch-specific event type. |
| Reuse `mention::resolve_mention` for project-aware lookup | Existing resolver handles qualified and unqualified agent names. | First matching `agent.name`, which is ambiguous in multi-project fleets. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| New direct dispatch enum | Existing `WebhookDispatch` already carries required data. | Duplicates dispatch logic and validation. |
| Async CLI socket client | Current blocking client is enough for one-shot CLI use. | Wider refactor and more test surface. |
| Config migration for `.terraphim/adf.toml` direct listener enablement | Behavioural ambiguity needs stakeholder decision. | Unexpected listener startup from project config. |
| Socket auth token | UDS permissions are the chosen local security boundary. | Secret management complexity. |
| Blocking event-only agents in direct mode | Conflicts with the intended local/admin use case. | Makes `build-runner`/`pr-reviewer` hard to test locally. |
| Full admin socket now | Bigger than the immediate direct-dispatch refinement. | Scope creep and delayed remediation. |

### Simplicity Check

The simplest correct design is to keep the current direct socket protocol and add three guardrails: explicit CLI validation, safe stale socket handling, and a real socket test. This avoids speculative abstractions and keeps each review finding mapped to one localised change.

For event-only local runs, the simplest correct extension is to reuse `SyntheticEvent::env_vars()` instead of creating a parallel event metadata structure. Direct dispatch remains an admin/local override, but can optionally carry enough event context to make event-only agents behave like their webhook-triggered equivalents.

**Senior Engineer Test**: This is not overcomplicated; each change removes ambiguity or risk from the existing design.

**Nothing Speculative Checklist:**

- [x] No features the user did not request.
- [x] No abstractions for future expansion.
- [x] No flexibility just in case.
- [x] No new dependencies.
- [x] No premature optimisation.

## File Changes

### New Files

No new implementation files. This plan document is the only new design artefact.

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Add `direct && !local` validation before local-mode print/secret resolution; add tests for direct flag semantics and socket path parsing. |
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | Add stale-socket cleanup helper; use Unix file type checks; add UDS round-trip tests for valid, invalid, project-qualified, and synthetic-event commands. |
| `crates/terraphim_orchestrator/src/config.rs` | Update `DirectDispatchConfig.socket_path` documentation to state `/tmp/adf-ctl.sock`, unless default behaviour is intentionally changed before implementation. |
| `crates/terraphim_orchestrator/src/lib.rs` | Make direct dispatch use project-aware resolution; preserve event-only local dispatch; inject optional synthetic event env vars into the spawned agent context. |
| `crates/terraphim_orchestrator/src/agent_runner.rs` | Reuse `SyntheticEvent` and `env_vars()`; add serde derives only if required for socket command payloads. |

### Deleted Files

None.

## API Design

### Internal Helper Functions

```rust
#[cfg(unix)]
fn remove_stale_socket_if_present(socket_path: &Path) -> std::io::Result<()>;
```

Purpose: remove `socket_path` only when it exists and is a Unix socket. If it exists and is not a socket, return an error so the listener logs and exits without deleting user data.

Expected behaviour:

```rust
match std::fs::symlink_metadata(socket_path) {
    Ok(metadata) if metadata.file_type().is_socket() => std::fs::remove_file(socket_path),
    Ok(_) => Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "direct dispatch path exists and is not a socket",
    )),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(e) => Err(e),
}
```

### CLI Guard

```rust
if direct && !local {
    bail!("--direct requires --local");
}

if local {
    println!("[local mode]");
}

if direct {
    let socket_path = resolve_socket_path()?;
    direct_dispatch_via_socket(&socket_path, name, Some(context))?;
    ...
    return Ok(());
}
```

### Direct Dispatch Command

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DirectDispatchCommand {
    agent: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    synthetic_event: Option<SyntheticEvent>,
}
```

Agent name parsing rule:

```rust
fn split_project_agent(input: &str) -> (Option<String>, String) {
    match input.split_once('/') {
        Some((project, agent)) => (Some(project.to_string()), agent.to_string()),
        None => (None, input.to_string()),
    }
}
```

### CLI Event Flags

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
enum DirectEventKind {
    Push,
    Pr,
}

Trigger {
    #[arg(long, value_enum)]
    event: Option<DirectEventKind>,
    #[arg(long)]
    sha: Option<String>,
    #[arg(long)]
    ref_name: Option<String>,
    #[arg(long)]
    pr: Option<u64>,
    #[arg(long)]
    head_sha: Option<String>,
}
```

Build existing events:

```rust
fn build_synthetic_event(args: &TriggerArgs) -> Result<Option<SyntheticEvent>>;
```

### Orchestrator Direct Dispatch Resolution

```rust
fn resolve_direct_dispatch_agent<'a>(
    agents: &'a [AgentDefinition],
    project_hint: Option<&str>,
    default_project: &str,
    agent_name: &str,
) -> Option<&'a AgentDefinition> {
    mention::resolve_mention(project_hint, default_project, agent_name, agents)
}
```

Event-only handling remains permissive in direct mode:

```rust
if def.event_only {
    info!(
        agent = %agent_name,
        event = ?synthetic_event,
        "direct dispatch override: spawning event-only agent locally"
    );
}
```

### Public Types

No public type changes are required.

### Error Types

No new error enum is required. Use `anyhow::bail!` in the CLI and `std::io::Error` for the cleanup helper.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_trigger_direct_requires_local` | `adf-ctl.rs` tests | Verifies `cmd_trigger(false, ..., direct=true)` returns an error before webhook/secret resolution. |
| `test_parse_socket_path_from_toml` | `adf-ctl.rs` tests | Verifies `[direct_dispatch] socket_path = "..."` parsing. |
| `test_direct_dispatch_default_socket_path_documented` | `config.rs` tests or existing config tests | Optional: verify `DirectDispatchConfig::default().socket_path == /tmp/adf-ctl.sock` if helper visibility allows; otherwise rely on existing default implementation. |
| `test_remove_stale_socket_rejects_regular_file` | `direct_dispatch.rs` tests, `#[cfg(unix)]` | Ensures regular files are not removed. |

### Integration-Style Module Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_direct_dispatch_socket_valid_agent_round_trip` | `direct_dispatch.rs` tests, `#[tokio::test]`, `#[cfg(unix)]` | Starts listener on unique socket, sends JSON with a real Unix stream, asserts `status: ok`, and receives `WebhookDispatch::SpawnAgent`. |
| `test_direct_dispatch_socket_unknown_agent_returns_error` | `direct_dispatch.rs` tests, `#[tokio::test]`, `#[cfg(unix)]` | Sends unknown agent, asserts error response, and verifies no dispatch is sent. |
| `test_direct_dispatch_socket_permissions_owner_only` | `direct_dispatch.rs` tests, `#[cfg(unix)]` | Optional if stable in CI: assert socket mode masks to `0o600`. |
| `test_direct_dispatch_accepts_project_qualified_agent` | `direct_dispatch.rs` or `lib.rs` tests | Verifies `project/agent` is parsed and resolved against the intended project. |
| `test_direct_dispatch_event_only_agent_allowed_with_override_log` | `lib.rs` tests | Verifies event-only agents remain runnable through direct dispatch. |
| `test_direct_dispatch_push_event_env_vars_injected` | `lib.rs` tests | Verifies synthetic push fields become `ADF_EVENT_TYPE`, `ADF_PUSH_SHA`, `ADF_PUSH_REF`, etc. |
| `test_adf_ctl_rejects_invalid_agent_name_for_shell_paths` | `adf-ctl.rs` tests | Verifies shell-backed status/wait/cancel helpers reject unsafe names before command construction. |

### No Mocks

Tests should use real Unix sockets, real tokio mpsc channels, and real temporary filesystem paths. Do not introduce mocks.

## Implementation Steps

### Step 1: CLI direct-mode guard

**Files:** `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`

**Description:** Add a fail-fast guard near the start of `cmd_trigger` before printing `[local mode]` and before resolving secrets.

**Tests:** Add `test_trigger_direct_requires_local` using the existing test style in `adf-ctl.rs`.

**Dependencies:** None.

**Estimated:** 20 minutes.

```rust
if direct && !local {
    bail!("--direct requires --local");
}
```

### Step 2: Socket cleanup hardening

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`

**Description:** Extract existing-path handling into a helper that only unlinks Unix socket files. Replace the current `if socket_path.exists() { remove_file(...) }` block with this helper. If the helper returns an error, log and return from listener startup.

**Tests:** Add `test_remove_stale_socket_rejects_regular_file` and, if easy, `test_remove_stale_socket_removes_socket_file` using a listener-created socket path.

**Dependencies:** Step 1 independent.

**Estimated:** 35 minutes.

### Step 3: UDS round-trip tests

**Files:** `crates/terraphim_orchestrator/src/direct_dispatch.rs`

**Description:** Add async tests that start `start_direct_dispatch_listener` with a unique path, wait until the socket exists by polling with `try_exists` in a bounded loop, connect with `tokio::net::UnixStream`, send newline JSON, read one response line, and assert channel results.

**Tests:** The new tests are the verification.

**Dependencies:** Step 2, because startup cleanup should be final before exercising listener startup.

**Estimated:** 60 minutes.

Implementation notes:

```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(1);
let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_names);
wait_for_socket(&socket_path).await;

let mut stream = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
stream.write_all(br#"{"agent":"meta-learning","context":"test"}\n"#).await.unwrap();

let mut reader = tokio::io::BufReader::new(stream);
let mut line = String::new();
reader.read_line(&mut line).await.unwrap();
assert_eq!(serde_json::from_str::<serde_json::Value>(&line).unwrap()["status"], "ok");

let dispatch = rx.recv().await.unwrap();
match dispatch { ... }

handle.abort();
```

Avoid unbounded waits. Use a short bounded loop with `tokio::task::yield_now().await` or a small `tokio::time::sleep` in tests only if needed by tokio scheduling. Do not use command-line `timeout`.

### Step 4: Documentation alignment

**Files:** `crates/terraphim_orchestrator/src/config.rs`, optionally `.docs/design-adf-ctl-direct-dispatch.md`

**Description:** Update the `socket_path` field doc comment to match the current default `/tmp/adf-ctl.sock`. If stakeholders choose `<working_dir>/.adf-ctl.sock` instead, change the implementation and tests consistently rather than only changing the comment.

**Tests:** Existing compile checks; optional default-path assertion.

**Dependencies:** Decision on default path. Current recommendation: keep `/tmp/adf-ctl.sock`.

**Estimated:** 10 minutes.

### Step 5: Verification

**Files:** None.

**Description:** Run targeted and package-level checks.

**Commands:**

```bash
cargo test -p terraphim_orchestrator direct_dispatch
cargo test -p terraphim_orchestrator --bin adf-ctl
cargo test -p terraphim_orchestrator --lib
cargo clippy -p terraphim_orchestrator
```

**Estimated:** 30-45 minutes.

### Step 6: Synthetic event context for event-only local runs

**Files:** `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`, `crates/terraphim_orchestrator/src/direct_dispatch.rs`, `crates/terraphim_orchestrator/src/agent_runner.rs`, `crates/terraphim_orchestrator/src/lib.rs`

**Description:** Add optional direct-dispatch event fields and CLI flags for push/PR simulation. Reuse `SyntheticEvent::env_vars()` when spawning through direct dispatch.

**Tests:** Add CLI parsing tests, direct command serialisation tests, and a lib test confirming env injection for event-only direct runs.

**Dependencies:** Steps 1-3.

**Estimated:** 2-3 hours.

### Step 7: Project-aware direct dispatch resolution

**Files:** `crates/terraphim_orchestrator/src/lib.rs`, `crates/terraphim_orchestrator/src/direct_dispatch.rs`, `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`

**Description:** Accept `project/agent` in `adf-ctl`, transport project separately in the socket command, and resolve through `mention::resolve_mention`.

**Tests:** Add duplicate-agent-name fixture across two projects and verify direct dispatch selects the project-qualified agent.

**Dependencies:** Step 6 can be independent, but both touch the command payload.

**Estimated:** 1-2 hours.

### Step 8: Shell command hardening in adf-ctl

**Files:** `crates/terraphim_orchestrator/src/bin/adf-ctl.rs`

**Description:** Add strict agent-name validation and remove user-controlled interpolation from shell command strings where feasible. Run broad fixed commands and filter in Rust.

**Tests:** Add invalid-name tests for trigger/wait/cancel paths and parser tests for Rust-side output filtering.

**Dependencies:** None, but should be completed before relying on remote/status automation.

**Estimated:** 1-2 hours.

## Rollback Plan

If implementation causes regressions:

1. Revert only the remediation commit while preserving the original direct-dispatch feature commit.
2. Keep `--direct` unpublished or document it as experimental until fixed.
3. If socket cleanup is the only failing area, temporarily remove automatic stale cleanup and let bind errors surface.

Feature disablement remains config-based: omit `[direct_dispatch]` from orchestrator config to avoid starting the listener.

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
| CLI direct dispatch overhead | One local socket connect/write/read | UDS round-trip test verifies functional path, not benchmark. |
| Listener startup | Negligible | Existing orchestrator startup path. |
| Memory | No meaningful change | One helper and tests only. |

### Benchmarks to Add

No benchmark required for this remediation patch.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide whether to keep `/tmp/adf-ctl.sock` or switch to working-dir default | Pending | User |
| Decide whether `.terraphim/adf.toml` should enable listener config in a later patch | Pending | User |
| Event-only bare direct dispatch | Allowed by product decision; should log as admin/local override when no synthetic event is supplied. | User |
| Direct dispatch audit event | Decide whether `EventOrigin::Direct` is required in this patch or can wait for admin socket/status work. | User |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Default socket path decision confirmed
- [ ] Human approval received

## Phase 3 Handoff

Implementation should be done in the following order:

1. CLI guard plus test.
2. Socket cleanup helper plus tests.
3. UDS round-trip tests.
4. Documentation alignment.
5. Synthetic event support for event-only local runs.
6. Project-aware direct dispatch resolution.
7. Shell command hardening.
8. Verification commands.

Do not implement until this plan is approved.
