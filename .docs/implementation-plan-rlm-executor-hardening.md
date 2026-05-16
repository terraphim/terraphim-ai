# Implementation Plan: RLM Executor & Agent-Search Hardening

**Status**: Draft
**Research Doc**: `.docs/research-rlm-executor-hardening.md`
**Author**: Claude Opus 4.7
**Date**: 2026-05-15
**Estimated Effort**: 6-8 hours implementation + 2 hours verification

## Overview

### Summary

Address every P1 and P2 finding from the structural review of the recently-landed RLM executor batch (`e4d896d3d` → `4442671e9`). Five independent commits, ordered by dependency: LocalExecutor contract → DockerExecutor concurrency/lifecycle/limits → backend selector → agent `concepts_matched` → hook/config peripherals.

### Approach

Follow the discipline established by the prior `docker-executor-robustness` pass:
- Smallest correct fix per defect.
- No new dependencies.
- No `ExecutionEnvironment` trait changes (use inherent methods, mirroring `FirecrackerExecutor::release_session_vm`).
- No public HTTP API changes (mitigate `concepts_matched` agent-side using `with_auto_id`).
- One additive `RlmError::NotSupported` variant for honest "snapshot not available" returns.
- All Docker/Local tests gated on real daemon/binary availability — no mocks.

### Scope

**In Scope:**
1. `LocalExecutor`: honour `ctx.timeout_ms`, `kill_on_drop(true)`, remove unused `output_dir`, return `NotSupported` for snapshots, drop the redundant `_session_id.clone()`.
2. `DockerExecutor`: per-session DashMap lock to close TOCTOU; add `release_session_container` inherent method; add `host_config` resource limits (memory cap, PIDs cap, dropped capabilities, network "none", read-only rootfs); replace `sleep 3600` with `sleep infinity`; remove `#[allow(dead_code)]`; widen `i32::try_from` for exit code; return `NotSupported` for snapshots.
3. `select_executor`: log `warn!` on Local fallback; correctly mark E2B unimplemented and `continue`; on Docker init Err, push to `tried` and `continue` rather than propagating with `?`.
4. `crates/terraphim_agent/src/main.rs`: extract `compute_concepts_matched` helper; server path uses `with_auto_id`; offline path stays the same; both log `debug!` on thesaurus fetch failure. Add `RlmError::NotSupported` variant.
5. `examples/opencode-plugin-rlm/terraphim-rlm-hook.sh`: replace string-built JSON with `jq -n --arg`; portable timeout via `( cmd & PID=$!; ... ) ` wrapper; document `jq` requirement in `install.sh`.
6. `crates/terraphim_orchestrator/src/config.rs`: redact `GiteaSkillRepoConfig.token` in `Debug`; default `cache_dir` to a meaningful path.

**Out of Scope:**
- Capability-driven `select_executor` rewrite.
- Enriching `/thesaurus/{role_name}` API.
- Adding `end_session` to the `ExecutionEnvironment` trait.
- Container pre-warm / pooling.
- Snapshot support for Docker / Local.
- Rewriting `terraphim-rlm.js` plugin (only minimal-impact tweaks).
- Adding `shellcheck` to CI (separate cycle).
- Multi-agent stress test infrastructure beyond a single `tokio::join!` regression test.

**Avoid At All Cost** (5/25 distractions):
- Adding new dependencies (e.g., `secrecy`, `nix` for `prctl`).
- Renaming `release_session_vm` → `release_session` (perfectly nice symmetry, a rabbit hole — defer).
- Generalising "no-isolation" warning into a configuration toggle.
- Rewriting `select_executor` body for "future capabilities".
- Generalising the `compute_concepts_matched` helper to a public crate API.
- Changing the `ExecutionContext` shape.
- Changing the `ExecutionResult` shape.
- Adding container restart-on-exit logic.
- Changing the `ThesaurusResponse` HTTP shape.
- Wrapping all `log::warn!` calls in `tracing` spans "while we're here".
- Adding bench harnesses for executors (out-of-scope per prior pass).
- Introducing a Mutex-of-HashMap fallback path for non-DashMap users.
- Splitting `docker.rs` into multiple files.
- Refactoring `LocalExecutor` to a builder pattern.
- Changing `bollard` version.
- Rewriting hook script in Python.
- Touching `FirecrackerExecutor` (out of scope by design).
- Adding `cargo-deny` rules for shell scripts.
- Replacing `parking_lot::RwLock` for the `session_to_container` map with `tokio::sync::RwLock`.
- Adding container labels for orphan-sweeping.

## Architecture

### Component Diagram

```
                 ┌─────────────────────────┐
                 │    select_executor()    │  (FIX 3: warn! on Local;
                 │  Cap-style fallthrough  │  fix E2B; degrade on Docker init Err)
                 └────────────┬────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
   ┌────▼─────┐          ┌────▼─────┐          ┌────▼─────┐
   │Firecracker│          │ Docker   │          │  Local   │
   │ executor  │          │ executor │          │ executor │
   │(unchanged)│          │ (FIX 2)  │          │ (FIX 1)  │
   └───────────┘          └────┬─────┘          └────┬─────┘
                               │                     │
                ┌──────────────┼──────────────┐      │
                │              │              │      │
        ┌───────▼──────┐ ┌─────▼──────┐ ┌─────▼──────────┐
        │ DashMap-     │ │ HostConfig │ │ kill_on_drop   │
        │ guarded      │ │ resource   │ │ + ctx timeout  │
        │ ensure_*     │ │ limits     │ │ honoured       │
        └──────────────┘ └────────────┘ └────────────────┘
                │
        ┌───────▼──────────────────┐
        │ release_session_container │ (inherent, mirrors
        │   (new inherent method)   │  release_session_vm)
        └───────────────────────────┘

                 ┌─────────────────────────┐
                 │  agent::run_*_command    │  (FIX 4: extract helper,
                 │   compute_concepts_matched │   use with_auto_id)
                 └─────────────────────────┘

                 ┌─────────────────────────┐
                 │ terraphim-rlm-hook.sh    │  (FIX 5: jq -n --arg,
                 │  + GiteaSkillRepoConfig  │   portable timeout, redact token)
                 └─────────────────────────┘
```

### Data Flow

`Caller → DockerExecutor.execute_*(ctx) → ensure_container(session_id):`
- DashMap entry per session_id. First call: lock, create+start container, install in `session_to_container`, drop lock. Concurrent calls: wait on the entry's `Notify` until the container_id is published, then `clone()`.

`Caller → LocalExecutor.execute_*(ctx) → run_command:`
- `Command::kill_on_drop(true)`. `tokio::time::timeout(Duration::from_millis(ctx.timeout_ms), child.wait_with_output())`. On timeout, the `Child` future is dropped → tokio kills the OS process.

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|---|---|---|
| Per-session lock via `dashmap::DashMap<SessionId, Arc<tokio::sync::Mutex<Option<String>>>>` | DashMap already in workspace; per-key lock keeps unrelated sessions parallel; `Option<String>` lets first creator publish container_id under the lock. | Single `tokio::Mutex<HashMap>` (serialises everything); lock-free CAS (overkill); `tokio::sync::Notify` (more code, no benefit). |
| `release_session_container` as **inherent** method, not on the trait | Mirrors `FirecrackerExecutor::release_session_vm`; no other backend needs it; trait change ripples to 6 impls + mock. | Adding `end_session` to trait (defer until needed). |
| Fix `concepts_matched` server-mode using `NormalizedTerm::with_auto_id` | Helper already exists in `terraphim_types`; preserves uniqueness; no API change. | Enriching `/thesaurus/` HTTP API (frontend impact); new `find_matches` overload (changes a public API). |
| Add `RlmError::NotSupported { backend, op }` variant | Lets Docker/Local return honest "snapshot not supported" instead of fake `SnapshotId`. | Reusing `ExecutionFailed` (semantically wrong). |
| Replace `sleep 3600` with `sleep infinity` | Container outlives any reasonable session; `cleanup()`/`release_session_container`/Drop reliably tears it down. | Container `restart_policy: always` (could mask bugs); `tail -f /dev/null` (works but `sleep infinity` is more idiomatic for container keepalive). |
| `host_config: { memory: 512MB, pids_limit: 256, cap_drop: ["ALL"], network_mode: "none", readonly_rootfs: true }` defaults | Strengthens isolation claim; matches Docker security defaults; no resource accounting in callers yet. | Configurable per call (defer; YAGNI). |
| Hook portable timeout via `( "$cmd" & PID=$!; ( sleep 30 && kill -TERM $PID 2>/dev/null ) & WATCHDOG=$!; wait $PID; kill $WATCHDOG 2>/dev/null )` | Pure POSIX; works on macOS without coreutils. | `gtimeout` (assumes brew install); `expect` (heavyweight). |
| Hook safe JSON via `jq -n --arg prompt "$prompt" '{prompt: $prompt}'` | jq is already a dependency of the hook; `--arg` does correct JSON encoding. | `printf` with manual escaping (error-prone). |
| Redact `GiteaSkillRepoConfig.token` via manual `Debug` impl | No new dependency (`secrecy` not in workspace); keeps `Serialize` working for round-trips. | Adding `secrecy` crate (avoid new deps); `#[serde(skip)]` (would break round-trip). |
| `cache_dir` defaults to `dirs::cache_dir().unwrap_or_else(env::temp_dir).join("terraphim/skills")` | Sensible default; `dirs` already transitively in workspace via several crates. | Empty `PathBuf::default()` (current — bad); requiring user to set (annoying for local dev). |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|---|---|---|
| Add `end_session(SessionId)` to `ExecutionEnvironment` trait | No uniform call-site; ripples to 6 impls + mock; `release_session_vm` precedent says inherent is fine. | Trait churn for marginal benefit; risk to `FirecrackerExecutor`. |
| Capability-driven `select_executor` rewrite | Architectural improvement, not a bug fix. | Scope creep; would push this cycle from days to weeks. |
| Enrich `/thesaurus/{role}` API to return `Vec<NormalizedTerm>` | Public API change with frontend impact; `with_auto_id` is sufficient. | Frontend regression risk; coordination cost. |
| Configurable container resource limits | YAGNI — no caller demands this; defaults are conservative. | Per-call config plumbing for zero current callers. |
| Container `restart_policy: "always"` | Masks bugs; container should die when session ends, not silently respawn. | Hides cleanup failures. |
| `secrecy::SecretString` for token | New dependency for ~5 lines of `Debug` impl. | New crate, version coupling. |
| Container labels (`org.terraphim.session_id`) for orphan sweeps | Useful for production but not in scope; needs a separate sweep tool. | Half-built feature; design debate without delivery. |
| Refactor `compute_concepts_matched` to a public crate API | Premature; called from exactly two sites in one binary. | Public-API stability commitment for an internal helper. |
| `prctl(PR_SET_PDEATHSIG)` on LocalExecutor children | Linux-only; `kill_on_drop` is sufficient for `bash -c`/`python3 -c` snippets. | Platform divergence. |
| Add bench harnesses for the new container path | Out of scope per prior pass; correctness fixes first. | Distraction. |

### Simplicity Check

**What if this could be easy?** It is. Each fix is local to one file and most are 5-30 lines:

- `LocalExecutor` timeout: change one constant + one method call (`kill_on_drop(true)`). ~10 lines.
- `DockerExecutor` per-session lock: replace one `parking_lot::RwLock<HashMap>` with `dashmap::DashMap` and rewrite `ensure_container` (~25 lines).
- `DockerExecutor` host_config: add a `default_host_config()` helper and pass it in `create_container` (~30 lines).
- `select_executor` fixes: three small surgical edits (~15 lines).
- `concepts_matched` helper: extract a private `compute_concepts_matched` function and call it from both branches (~30 lines, mostly deletion).
- Hook script: rewrite four small JSON-builder calls and one timeout invocation (~30 lines).
- Config: add a `Debug` impl and a `Default` for `cache_dir` (~10 lines).

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? No — these are textbook fixes for textbook bugs. The only judgment call (per-session lock pattern) is the cheapest correct option using a dependency we already have.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request — every change traces to a review finding.
- [x] No abstractions "in case we need them later" — `release_session_container` is inherent, not on the trait, until a second caller appears.
- [x] No flexibility "just in case" — resource limits are hardcoded sensible defaults, not configurable.
- [x] No error handling for scenarios that cannot occur — DashMap entry creation is infallible.
- [x] No premature optimization — DashMap is the cheapest correct primitive; no benchmark-driven choices.

## File Changes

### New Files

None.

### Modified Files

| File | Changes |
|---|---|
| `crates/terraphim_rlm/src/executor/local.rs` | Honour `ctx.timeout_ms`; `kill_on_drop(true)`; remove `output_dir` field + `with_output_dir`; return `RlmError::NotSupported` from snapshot methods; drop redundant `_session_id.clone()`; add `test_local_honours_timeout`, `test_local_kills_on_timeout`, `test_local_snapshot_returns_not_supported`. |
| `crates/terraphim_rlm/src/executor/docker.rs` | Replace `session_to_container: parking_lot::RwLock<HashMap<...>>` with `DashMap<SessionId, Arc<tokio::sync::Mutex<Option<String>>>>`; rewrite `ensure_container` to be race-free; add `release_session_container(&SessionId)` inherent; add `default_host_config()` helper and pass into `create_container`; replace `sleep 3600` with `sleep infinity`; remove `#[allow(dead_code)]`; replace `as i32` with `i32::try_from`; return `RlmError::NotSupported` from snapshot methods; add `test_docker_concurrent_ensure_no_leak`, `test_docker_release_session_container`, `test_docker_resource_limits_applied`, `test_docker_snapshot_returns_not_supported`. |
| `crates/terraphim_rlm/src/executor/mod.rs` | Fix E2B arm to log `debug!` and `continue`; fix Docker arm to push to `tried` and `continue` on `DockerExecutor::new` Err; warn-log on Local fallback. |
| `crates/terraphim_rlm/src/error.rs` | Add `NotSupported { backend: String, op: String }` variant; add JSON-RPC error code `-32070`; mark non-retryable. |
| `crates/terraphim_agent/src/main.rs` | Extract `async fn compute_concepts_matched_offline(service, role, query)` and `async fn compute_concepts_matched_server(api, role, query)`; server variant uses `with_auto_id`; both log `debug!` on Err; replace inline blocks at lines 2177-2183 and 4240-4263. |
| `crates/terraphim_agent/tests/robot_search_concepts_matched_test.rs` | New regression test asserting offline/server parity for a fixed thesaurus + query (uses real ripgrep haystack, no mocks). |
| `examples/opencode-plugin-rlm/terraphim-rlm-hook.sh` | Replace `call_mcp_tool` JSON construction with `jq -n --arg`; replace `timeout 30` with portable POSIX wrapper; harden `awk`-based parsing for quoted args. |
| `examples/opencode-plugin-rlm/install.sh` | Add `jq` dependency check (warn but do not block); document macOS portability. |
| `examples/opencode-plugin-rlm/README.md` | Document `jq` requirement; note macOS support. |
| `crates/terraphim_orchestrator/src/config.rs` | Implement manual `Debug` for `GiteaSkillRepoConfig` redacting `token`; change `cache_dir` default to `dirs::cache_dir().unwrap_or_else(env::temp_dir).join("terraphim/skills")` via a `default_cache_dir()` helper. |

### Deleted Files

None.

## API Design

### Public Types

No new public types in `terraphim_rlm` (one new private helper).

In `RlmError`:
```rust
#[derive(Debug, thiserror::Error)]
pub enum RlmError {
    // ... existing variants ...

    /// Operation is not supported by the selected backend.
    #[error("backend '{backend}' does not support operation '{op}'")]
    NotSupported {
        backend: String,
        op: String,
    },
}
```

### Public Functions

**New inherent on `DockerExecutor`** (mirrors `FirecrackerExecutor::release_session_vm`):
```rust
impl DockerExecutor {
    /// Release the container associated with `session_id`, removing it from
    /// Docker and from the internal session map. Returns the container id if one
    /// was bound to this session, or `None` if no container existed.
    ///
    /// Errors from `docker.remove_container` are logged but not propagated, so
    /// the session map is always cleaned up even if the daemon is unreachable.
    pub async fn release_session_container(&self, session_id: &SessionId) -> Option<String>;
}
```

**Existing trait methods unchanged**, but `delete_snapshot` / `create_snapshot` etc. on Docker and Local now return `Err(RlmError::NotSupported { backend, op })`.

### Private Helpers

**`crates/terraphim_agent/src/main.rs`** (new private fns):
```rust
/// Compute matched concepts for the offline (in-process service) path.
async fn compute_concepts_matched_offline(
    service: &TuiService,
    role_name: &RoleName,
    query: &str,
) -> Vec<String>;

/// Compute matched concepts for the server (HTTP client) path. Reconstructs a
/// minimal `Thesaurus` from the lossy `/thesaurus/{role}` API using
/// `NormalizedTerm::with_auto_id` so each term has a unique id.
async fn compute_concepts_matched_server(
    api: &ApiClient,
    role_name: &RoleName,
    query: &str,
) -> Vec<String>;
```

**`crates/terraphim_rlm/src/executor/docker.rs`** (new private fn):
```rust
/// Build the default `HostConfig` applied to every session container.
///
/// Memory: 512MB. PIDs: 256. All capabilities dropped. Read-only rootfs.
/// Network: "none" (containers cannot make outbound connections).
fn default_host_config() -> bollard::models::HostConfig;
```

### Error Types

`RlmError::NotSupported` is the only addition. JSON-RPC error code `-32070`. Marked as non-retryable (`is_retryable() == false`) and not a budget error.

## Test Strategy

### Unit Tests (no Docker required)

| Test | Location | Purpose |
|---|---|---|
| `test_local_honours_ctx_timeout` | `local.rs::tests` | Pass `ctx.timeout_ms = 100`, run `sleep 5`, assert `timed_out == true` and elapsed < 1s. |
| `test_local_kills_on_timeout` | `local.rs::tests` | Run `sleep 30 & echo $!`, capture PID, assert `kill -0 $PID` returns non-zero (process killed) within 200ms of timeout. |
| `test_local_snapshot_returns_not_supported` | `local.rs::tests` | Assert `create_snapshot/restore_snapshot/delete_snapshot/list_snapshots/delete_session_snapshots` all return `Err(RlmError::NotSupported)`. |
| `test_local_command_kill_on_drop` | `local.rs::tests` | Build a `Command` via the public helper and assert `kill_on_drop(true)` was called (via behaviour: dropped child terminates within 200ms). |
| `test_select_executor_falls_back_to_local_on_docker_init_err` | `mod.rs::tests` | Use a `RlmConfig` whose `backend_preference` is `[Docker, Local]` and a stubbed `is_docker_available` that returns true while the daemon is actually unavailable; assert returned executor is `Local`. (Gated on `cfg(unix)` and absence of Docker daemon.) |
| `test_select_executor_e2b_fallthrough` | `mod.rs::tests` | With E2B api key set but feature unavailable, assert `select_executor` returns Local (not Docker, not error). |
| `test_compute_concepts_matched_server_uses_unique_ids` | `crates/terraphim_agent/tests/robot_search_concepts_matched_test.rs` | Build a mock-free `ApiClient` against a real test server; assert returned `concepts_matched` equals a known-good list and that the helper does not panic when thesaurus values collide on the same id. |
| `test_robot_search_concepts_matched_offline_server_parity` | same | Run both `compute_concepts_matched_offline` and `_server` against the same fixture; assert sets are equal. |
| `test_normalized_term_with_auto_id_unique` | (existing in `terraphim_types`) — verify expected behaviour, no new test needed unless missing. | Defensive — confirm helper. |
| `test_rlm_error_not_supported_is_not_retryable` | `error.rs::tests` | Assert `NotSupported` returns `false` from `is_retryable()` and the right RPC code. |
| `test_rlm_error_not_supported_display` | `error.rs::tests` | Assert `Display` formatting includes backend + op. |
| `test_gitea_skill_repo_config_token_redacted_in_debug` | `crates/terraphim_orchestrator/src/config.rs::tests` | Construct with token "secret", `format!("{:?}", config)` does NOT contain "secret". |
| `test_gitea_skill_repo_config_default_cache_dir_non_empty` | same | Assert default `cache_dir` ends in `terraphim/skills`. |

### Integration Tests (Docker required, gated)

| Test | Location | Purpose |
|---|---|---|
| `test_docker_concurrent_ensure_no_leak` | `docker.rs::tests` | Spawn 8 `tokio::join!` tasks calling `execute_command("echo hi", &ctx)` with the **same** `session_id`; assert `docker ps --filter name=terraphim-rlm-${session_id} -q | wc -l == 1` after all complete. |
| `test_docker_release_session_container_removes` | `docker.rs::tests` | Create one container via `execute_command`, call `release_session_container(&sid)`; assert returned `Some(id)`, container is gone from `docker ps -a`, and a subsequent `execute_command` creates a fresh container. |
| `test_docker_release_session_container_unknown_returns_none` | `docker.rs::tests` | Call `release_session_container(&unknown_sid)`; assert `None`. |
| `test_docker_resource_limits_applied` | `docker.rs::tests` | Create container, exec `cat /sys/fs/cgroup/memory.max`; assert value matches the configured 512MB cap. |
| `test_docker_snapshot_returns_not_supported` | `docker.rs::tests` | Mirror the LocalExecutor snapshot-not-supported test. |
| `test_docker_keepalive_sleep_infinity` | `docker.rs::tests` | Exec `ps -o args= -p 1`; assert it shows `sleep infinity` (not `sleep 3600`). |
| `test_docker_exit_code_truncation_safe` | `docker.rs::tests` | Run `bash -c 'exit 137'`; assert `exit_code == 137` (sanity for the `i32::try_from` change). |

### Hook Script Tests

| Test | Location | Purpose |
|---|---|---|
| `test_hook_jq_safe_quoted_prompt` | `examples/opencode-plugin-rlm/tests/test_hook.sh` (new) | Pipe `{"tool_name":"Bash","tool_input":{"command":"rlm_query \"hello \\\"world\\\"\""}}` through hook; assert MCP receives valid JSON (use `jq` to validate the captured output). |
| `test_hook_portable_timeout_no_gtimeout_dependency` | same | Run hook with `PATH=/usr/bin:/bin` (no `timeout`/`gtimeout`); assert hook does not error. |
| `test_hook_passthrough_non_rlm_command` | same | Pipe a non-RLM Bash command; assert hook outputs identical JSON. |

These shell tests are simple bash assertions; do not invoke a real MCP server (the script is the SUT).

### Property Tests

None — no fuzzing surface required.

### Deferred (out of scope this cycle)

- Multi-process stress test (process-level concurrency, not in-process).
- Container orphan-sweep CI job.
- `shellcheck` CI step.

## Implementation Steps

Each step is one logically-coherent commit. Steps are independent and can land in any order, but the listed order minimises rebase noise.

### Step 1: LocalExecutor honours `ctx.timeout_ms` + reaps timed-out children + honest snapshot

**Files**: `crates/terraphim_rlm/src/executor/local.rs`, `crates/terraphim_rlm/src/error.rs`
**Description**: Add `RlmError::NotSupported` variant. In `LocalExecutor::run_command`, change `Duration::from_millis(30000)` → `Duration::from_millis(ctx.timeout_ms)`; thread `ctx` through. Add `command.kill_on_drop(true)` in `build_command`/`build_python_command`. Remove `output_dir`, `with_output_dir`. Replace snapshot method bodies with `Err(RlmError::NotSupported { backend: "local".into(), op: "<name>".into() })`. Drop the `_session_id.clone()`.
**Tests**: `test_local_honours_ctx_timeout`, `test_local_kills_on_timeout`, `test_local_snapshot_returns_not_supported`, `test_rlm_error_not_supported_is_not_retryable`, `test_rlm_error_not_supported_display`.
**Estimated**: 1 hour.

```rust
// New error variant in error.rs
#[error("backend '{backend}' does not support operation '{op}'")]
NotSupported { backend: String, op: String },

// In local.rs:
async fn run_command(&self, mut cmd: Command, ctx: &ExecutionContext) -> RlmResult<ExecutionResult> {
    let start = Instant::now();
    let timeout_duration = Duration::from_millis(ctx.timeout_ms);
    let output = timeout(timeout_duration, cmd.output()).await;
    // ... rest as before, with the timeout error message using ctx.timeout_ms
}

fn build_command(&self, cmd: &str, ctx: &ExecutionContext) -> Command {
    let mut command = Command::new("bash");
    command
        .arg("-c").arg(cmd)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .envs(&ctx.env_vars)
        .kill_on_drop(true);  // <-- new
    if let Some(cwd) = &ctx.working_dir { command.current_dir(cwd); }
    command
}
```

### Step 2: DockerExecutor concurrency, lifecycle, and resource limits

**Files**: `crates/terraphim_rlm/src/executor/docker.rs`
**Description**:
- Replace `session_to_container: parking_lot::RwLock<HashMap<SessionId, String>>` with `session_to_container: dashmap::DashMap<SessionId, Arc<tokio::sync::Mutex<Option<String>>>>`.
- Rewrite `ensure_container` to acquire/insert the per-session entry, take the inner `Mutex` lock, then read or create+publish the container_id.
- Add `default_host_config()` returning `HostConfig` with `memory: Some(512 * 1024 * 1024)`, `pids_limit: Some(256)`, `cap_drop: Some(vec!["ALL".into()])`, `network_mode: Some("none".into())`, `readonly_rootfs: Some(true)`.
- Pass `host_config: Some(default_host_config())` in `ContainerCreateBody`.
- Replace `cmd: Some(vec!["sleep".into(), "3600".into()])` with `cmd: Some(vec!["sleep".into(), "infinity".into()])`.
- Add inherent `pub async fn release_session_container(&self, session_id: &SessionId) -> Option<String>` — looks up the entry, removes it, force-removes the container, logs warn on remove failure.
- Replace snapshot methods with `Err(RlmError::NotSupported { backend: "docker".into(), op: "<name>".into() })`.
- Replace `as i32` cast with `i32::try_from(exit).unwrap_or(-1)`.
- Remove `#[allow(dead_code)]` on the struct.
- Update `cleanup()` and `Drop` to drain the DashMap.

**Tests**: `test_docker_concurrent_ensure_no_leak`, `test_docker_release_session_container_removes`, `test_docker_release_session_container_unknown_returns_none`, `test_docker_resource_limits_applied`, `test_docker_snapshot_returns_not_supported`, `test_docker_keepalive_sleep_infinity`, `test_docker_exit_code_truncation_safe`.
**Dependencies**: Step 1 (NotSupported variant).
**Estimated**: 3 hours.

```rust
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DockerExecutor {
    config: RlmConfig,
    docker: Docker,
    session_to_container: DashMap<SessionId, Arc<Mutex<Option<String>>>>,
    image: String,
    capabilities: Vec<Capability>,
}

async fn ensure_container(&self, session_id: &SessionId) -> RlmResult<String> {
    let entry = self
        .session_to_container
        .entry(*session_id)
        .or_insert_with(|| Arc::new(Mutex::new(None)))
        .clone();

    let mut guard = entry.lock().await;
    if let Some(id) = guard.as_ref() {
        return Ok(id.clone());
    }
    let new_id = self.create_container(session_id).await?;
    *guard = Some(new_id.clone());
    Ok(new_id)
}

pub async fn release_session_container(&self, session_id: &SessionId) -> Option<String> {
    let removed = self.session_to_container.remove(session_id)?;
    let id_opt = removed.1.lock().await.take();
    if let Some(id) = &id_opt {
        if let Err(e) = self.delete_container(id).await {
            log::warn!("release_session_container: failed to remove {id}: {e}");
        }
    }
    id_opt
}

fn default_host_config() -> bollard::models::HostConfig {
    bollard::models::HostConfig {
        memory: Some(512 * 1024 * 1024),
        pids_limit: Some(256),
        cap_drop: Some(vec!["ALL".to_string()]),
        network_mode: Some("none".to_string()),
        readonly_rootfs: Some(true),
        ..Default::default()
    }
}
```

### Step 3: Backend selector — fix E2B fallthrough, degrade Docker init, warn on Local

**Files**: `crates/terraphim_rlm/src/executor/mod.rs`
**Description**: Rewrite the three problematic arms in `select_executor`. E2B arm becomes `log::debug!("E2B not yet implemented; trying next backend"); tried.push("e2b (not implemented)".into());` (no return, no spurious "Selected" log). Docker arm changes to:
```rust
BackendType::Docker if is_docker_available() => {
    match DockerExecutor::new(config.clone()) {
        Ok(executor) => {
            log::info!("Selected Docker backend (container isolation)");
            return Ok(Box::new(executor));
        }
        Err(e) => {
            log::warn!("Docker init failed: {e}; trying next backend");
            tried.push(format!("docker (init failed: {e})"));
        }
    }
}
```
Local arm changes `log::info!` → `log::warn!("Falling back to LocalExecutor: NO ISOLATION. Tried: {:?}", tried)`.

**Tests**: `test_select_executor_falls_back_to_local_on_docker_init_err`, `test_select_executor_e2b_fallthrough`. Tests use environment-driven backend preference; no mocks.
**Dependencies**: None (independent of Steps 1, 2).
**Estimated**: 30 minutes.

### Step 4: Agent `concepts_matched` helper + parity test

**Files**: `crates/terraphim_agent/src/main.rs`, `crates/terraphim_agent/tests/robot_search_concepts_matched_test.rs`
**Description**:
- Extract two private async helpers as defined in API Design.
- Server helper builds `Thesaurus` using `NormalizedTerm::with_auto_id(NormalizedTermValue::from(value))`.
- Both helpers `log::debug!` on `Err`.
- Replace inline blocks at `main.rs:2177-2183` and `main.rs:4240-4263` with single helper calls.
- Add a regression test that runs both helpers against a fixture role + query and asserts identical output sets.

**Tests**: `test_compute_concepts_matched_server_uses_unique_ids`, `test_robot_search_concepts_matched_offline_server_parity`. Both use real (not mocked) `TuiService`/`ApiClient` against a fixture haystack — gated on `cargo test --features ...` if needed.
**Dependencies**: None.
**Estimated**: 1.5 hours.

```rust
async fn compute_concepts_matched_server(
    api: &ApiClient,
    role_name: &RoleName,
    query: &str,
) -> Vec<String> {
    let resp = match api.get_thesaurus(role_name.as_str()).await {
        Ok(r) => r,
        Err(e) => {
            log::debug!("get_thesaurus failed for {role_name}: {e}; concepts_matched empty");
            return vec![];
        }
    };
    let mut thesaurus = terraphim_types::Thesaurus::new(format!("role-{role_name}"));
    for value in resp.thesaurus.into_iter().flatten().values() {
        let nt_value = terraphim_types::NormalizedTermValue::from(value.clone());
        let term = terraphim_types::NormalizedTerm::with_auto_id(nt_value.clone());
        thesaurus.insert(nt_value, term);
    }
    terraphim_automata::find_matches(query, thesaurus, false)
        .map(|matches| matches.into_iter().map(|m| m.term).collect())
        .unwrap_or_default()
}
```

### Step 5: Hook script + Gitea config

**Files**: `examples/opencode-plugin-rlm/terraphim-rlm-hook.sh`, `examples/opencode-plugin-rlm/install.sh`, `examples/opencode-plugin-rlm/README.md`, `crates/terraphim_orchestrator/src/config.rs`, `examples/opencode-plugin-rlm/tests/test_hook.sh` (new).
**Description**:
- Replace every `"{\"key\":\"$var\"}"` JSON construction with a `jq -n --arg key "$var" '{key: $key}'` call.
- Replace `timeout 30 "$TERRAPHIM_MCP"` with portable `(  "$TERRAPHIM_MCP" & PID=$!; ( sleep 30 && kill -TERM $PID 2>/dev/null ) & WATCHDOG=$!; wait $PID; STATUS=$?; kill $WATCHDOG 2>/dev/null; exit $STATUS )`.
- In `install.sh`, add a non-blocking `command -v jq >/dev/null || echo "WARNING: jq not found"` check.
- In `README.md`, document `jq` and macOS support.
- In `config.rs`, manually `impl Debug for GiteaSkillRepoConfig` to print `token: Some("***")` / `None`. Add `default_cache_dir()` helper using `dirs::cache_dir().unwrap_or_else(env::temp_dir).join("terraphim/skills")` and reference it via `#[serde(default = "default_cache_dir")]`.

**Tests**: `test_hook_jq_safe_quoted_prompt`, `test_hook_portable_timeout_no_gtimeout_dependency`, `test_hook_passthrough_non_rlm_command`, `test_gitea_skill_repo_config_token_redacted_in_debug`, `test_gitea_skill_repo_config_default_cache_dir_non_empty`.
**Dependencies**: None.
**Estimated**: 1.5 hours.

## Rollback Plan

Each step is a self-contained commit on a `task/rlm-executor-hardening` branch. To roll back:

| Step | Roll-back action |
|---|---|
| 1 | `git revert <step1-sha>` — restores `LocalExecutor` to ignore `ctx.timeout_ms`. No data loss. |
| 2 | `git revert <step2-sha>` — restores `parking_lot::RwLock<HashMap>` and re-introduces TOCTOU. Does not affect Firecracker. Requires also reverting Step 1 if `RlmError::NotSupported` was first added there. |
| 3 | `git revert <step3-sha>` — restores logger lies and prevents Local fallback after Docker init Err. No state to migrate. |
| 4 | `git revert <step4-sha>` — restores duplicated inline `concepts_matched` blocks. No persistence impact. |
| 5 | `git revert <step5-sha>` — restores the leaky JSON construction and unportable timeout. No installation state to clean up. |

No feature flags are needed; this is a pure correctness pass with no behaviour the user explicitly opts into.

## Migration

None. No persisted state changes. No HTTP API changes. No config-file format changes (`GiteaSkillRepoConfig` adds a sensible default to a previously-empty path; existing configs that explicitly set `cache_dir` continue to work).

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

### Existing Dependencies Used

| Crate | Version | Use |
|---|---|---|
| `dashmap` | 6.1 | Per-session lock map in `DockerExecutor`. |
| `tokio::sync::Mutex` | 1.x | Inner lock per session entry. |
| `bollard::models::HostConfig` | 0.20 | Resource limits on container creation. |
| `tokio::process::Command::kill_on_drop` | 1.x | Reap timed-out children in `LocalExecutor`. |
| `terraphim_types::NormalizedTerm::with_auto_id` | workspace | Server-mode thesaurus reconstruction. |
| `terraphim_automata::find_matches` | workspace | `concepts_matched` computation. |
| `dirs::cache_dir` | (transitively in workspace; if not, add via `directories-next` already in workspace via desktop crates) | Default `cache_dir`. |

If `dirs` is not directly available to `terraphim_orchestrator`, fall back to `std::env::var("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(env::temp_dir).join(".cache")).join("terraphim/skills")` — purely stdlib.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|---|---|---|
| `ensure_container` first-call latency | unchanged (~bollard create+start) | container test |
| `ensure_container` cached-call latency | sub-millisecond (DashMap entry + tokio Mutex acquire on a held value) | unit test, debug build |
| `LocalExecutor` timeout latency | within 50ms of `ctx.timeout_ms` | unit test |
| `LocalExecutor` zombie count after timeout | 0 | `pgrep` after test |
| Concurrent same-session `execute_command` (8-way) | exactly 1 container created | integration test |
| Container memory cap | hard 512MB | `cat /sys/fs/cgroup/memory.max` |

### Benchmarks to Add

None — out of scope per prior pass and per the "Avoid At All Cost" list.

## Open Items

| Item | Status | Owner |
|---|---|---|
| Confirm bollard 0.20's `HostConfig` field names match the spike values used here (`pids_limit` vs `pids-limit`, `readonly_rootfs` vs `readonly_root_fs`) | To verify in Step 2 by reading `~/.cargo/registry/src/.../bollard-0.20.*/src/models.rs` before writing code | Implementer |
| Confirm `dirs::cache_dir` is reachable from `terraphim_orchestrator` Cargo deps | To verify in Step 5; if not, switch to stdlib fallback | Implementer |
| Confirm `tokio::sync::Mutex` usage inside `dashmap::Entry` does not deadlock under contention | Spike with the concurrent regression test | Implementer |
| Decide whether `compute_concepts_matched_*` helpers move to a shared module | Defer — keep in `main.rs` until a third caller appears | Alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Gate Checklist

### Standard Gates
- [x] All file changes listed (8 modified, 2 new test files, 0 deleted)
- [x] All public APIs defined (`RlmError::NotSupported`, `DockerExecutor::release_session_container`)
- [x] Test strategy complete (13 unit tests, 7 integration tests, 3 hook script tests)
- [x] Steps sequenced with dependencies (5 steps, only Step 2 depends on Step 1's `NotSupported` variant)
- [x] Performance targets set (cached-call sub-ms, zero zombies, single container under contention)
- [ ] Human approval received

### Essentialism Gates
- [x] 5 or fewer major components/features in scope (LocalExecutor, DockerExecutor, selector, agent helper, hook+config)
- [x] "Eliminated Options" section populated (10+ items)
- [x] "Avoid At All Cost" list documented (20 items)
- [x] Simplicity Check answered — design is local, mechanical, and uses already-available primitives
- [x] 5/25 Rule applied — 5 in-scope groups, 20 explicit out-of-scope distractions

### Quality Evaluation
Recommended next: invoke `disciplined-quality-evaluation` skill on this design before committing to implementation. Flag if any of the four `Open Items` resolve in a way that materially changes step ordering or required dependencies.
