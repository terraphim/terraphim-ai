# Implementation Plan: Direct Dispatch for adf-ctl --local

**Status**: Draft
**Research Doc**: `.docs/research-adf-ctl-direct-dispatch.md`
**Author**: AI Agent
**Date**: 2026-05-25
**Issue**: terraphim/terraphim-ai#1875
**Estimated Effort**: 4-6 hours

## Overview

### Summary
Add a Unix domain socket listener to the ADF orchestrator and a `--direct` flag to `adf-ctl --local trigger`, enabling agent dispatch without webhook HTTP roundtrip, HMAC negotiation, or mention-polling latency.

### Approach
Unix domain socket at `<working_dir>/.adf-ctl.sock`. The orchestrator spawns a tokio task that listens for JSON dispatch commands and injects them into the existing `LoopEvent` channel. `adf-ctl` connects, writes `{"agent":"name","context":"..."}`, reads the response, and exits.

### Scope

**In Scope:**
- New `[direct_dispatch]` config section with `socket_path`
- UDS listener tokio task in orchestrator startup
- `adf-ctl --local trigger <agent> --direct` subcommand
- Socket path auto-discovery via `.terraphim/adf.toml`

**Out of Scope:**
- Admin socket for status/cancel/list (Phase 2)
- Authentication beyond filesystem permissions
- Multi-command batching

**Avoid At All Cost:**
- Embedding spawner logic in adf-ctl (200MB binary)
- Adding new LoopEvent variant (reuse existing Webhook path)
- TLS on UDS (unnecessary for local IPC)

## Architecture

### Component Diagram
```
adf-ctl --local trigger NAME --direct
  │
  │ connect to <working_dir>/.adf-ctl.sock
  │ write {"agent":"NAME","context":"..."}
  │ read {"status":"ok"}
  ▼
┌─────────────────────────────────────────────────┐
│  Unix Domain Socket Listener (tokio task)        │
│  orchestrator startup (lib.rs)                   │
│  1. unlink stale socket                          │
│  2. bind + listen                                │
│  3. accept connection                            │
│  4. read JSON command                            │
│  5. validate agent name against config           │
│  6. construct WebhookDispatch::SpawnAgent        │
│  7. send to loop_tx channel                      │
│  8. write {"status":"ok"} response               │
└──────────────────────┬──────────────────────────┘
                       │ loop_tx.send(Webhook(SpawnAgent))
                       ▼
┌──────────────────────────────────────────────────┐
│  Main Event Loop (existing)                       │
│  handle_webhook_dispatch(dispatch).await          │
│  → should_skip_dispatch(issue_number=0) → false   │
│  → spawn_agent(&mention_def)                      │
└──────────────────────────────────────────────────┘
```

### Data Flow
```
adf-ctl → connect(UDS) → write(JSON) → orchestrator validates → loop_tx.send(SpawnAgent) → spawn_agent
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Reuse `WebhookDispatch::SpawnAgent` | No new code for dispatch logic | New LoopEvent variant (unnecessary) |
| UDS path in orchestrator config, not adf.toml | Orchestrator owns the socket; adf-ctl discovers it | Store in adf.toml (complex discovery) |
| Response is `{"status":"ok"}` or `{"status":"error","message":"..."}` | Simple, parseable, matches HTTP-like UX | No response (fire-and-forget less testable) |
| Tokio UnixListener in existing runtime | Reuses tokio runtime, no new threads | std::os::unix::net (blocking, needs thread) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Named pipe/FIFO | Unidirectional, harder to handle concurrent clients | Race conditions, blocking reads |
| Embedded spawner in adf-ctl | Links 200MB of orchestrator deps into CLI binary | Binary bloat, maintenance burden |
| New `DirectDispatch` LoopEvent variant | Adds code with identical logic to Webhook path | Duplication, divergence |

### Simplicity Check

**What if this could be easy?**
The simplest design: a Unix socket where adf-ctl writes a name and the orchestrator spawns it. No auth, no complex protocol, no new types. The existing `WebhookDispatch::SpawnAgent` already accepts `issue_number=0` (bypasses dedup). We just need a new way to produce it.

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/direct_dispatch.rs` | UDS listener implementation |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/config.rs` | Add `DirectDispatchConfig` struct with `socket_path` field |
| `crates/terraphim_orchestrator/src/lib.rs` | Import module; start UDS listener in orchestrator startup; send `LoopEvent::Webhook(SpawnAgent)` from listener |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Add `--direct` flag to `Trigger` subcommand; implement UDS client logic |

## API Design

### Config Type
```rust
/// Configuration for direct dispatch via Unix domain socket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectDispatchConfig {
    /// Path to the Unix domain socket for adf-ctl direct dispatch.
    /// Default: "<working_dir>/.adf-ctl.sock"
    #[serde(default = "DirectDispatchConfig::default_socket_path")]
    pub socket_path: PathBuf,
}

impl Default for DirectDispatchConfig {
    fn default() -> Self {
        Self { socket_path: Self::default_socket_path() }
    }
}

impl DirectDispatchConfig {
    fn default_socket_path() -> PathBuf {
        PathBuf::from("/tmp/adf-ctl.sock")
    }
}
```

### UDS Listener Function (in direct_dispatch.rs)
```rust
/// Start the Unix domain socket listener for direct dispatch.
/// Spawns a tokio task that listens for adf-ctl connections.
///
/// # Arguments
/// * `socket_path` - Path to bind the Unix domain socket
/// * `loop_tx` - Channel to the main event loop (same type as webhook handler uses)
/// * `agent_names` - Set of configured agent names for validation
pub fn start_direct_dispatch_listener(
    socket_path: PathBuf,
    loop_tx: Arc<Mutex<std::sync::mpsc::Sender<LoopEvent>>>,
    agent_names: HashSet<String>,
) -> tokio::task::JoinHandle<()>;
```

### Dispatch Command (JSON protocol)
```json
{"agent": "meta-learning", "context": "optional context string"}
```

### Dispatch Response (JSON protocol)
```json
{"status": "ok"}
```
```json
{"status": "error", "message": "unknown agent: no-such-agent"}
```

### adf-ctl subcommand change
```rust
Trigger {
    // ... existing fields ...
    /// Dispatch directly via Unix domain socket (local mode only)
    #[arg(long)]
    direct: bool,
}
```

## Test Strategy

### Unit Tests (direct_dispatch.rs)
| Test | Purpose |
|------|---------|
| `test_direct_dispatch_config_defaults` | Verify default socket_path |
| `test_validate_command_known_agent` | Known agent returns Ok |
| `test_validate_command_unknown_agent` | Unknown agent returns Err |
| `test_parse_dispatch_json_valid` | Valid JSON parses correctly |
| `test_parse_dispatch_json_invalid` | Invalid JSON returns error |

### Unit Tests (adf-ctl.rs)
| Test | Purpose |
|------|---------|
| `test_resolve_socket_path_from_config` | Discovers socket_path from .terraphim/adf.toml |
| `test_resolve_socket_path_fallback` | Falls back to /tmp/adf-ctl.sock |
| `test_build_direct_dispatch_payload` | Builds correct JSON payload |

### Integration Tests
| Test | Purpose |
|------|---------|
| `test_direct_dispatch_e2e` | Start orchestrator with UDS, connect, dispatch, verify spawn |
| `test_direct_dispatch_unknown_agent` | Returns error for unknown agent |
| `test_direct_dispatch_graceful_no_orchestrator` | adf-ctl reports connection error |

## Implementation Steps

### Step 1: Config and Types (1h)
**Files:** `config.rs`, `direct_dispatch.rs`
**Description:** Add `DirectDispatchConfig`, default socket path, command/response types
**Tests:** `test_direct_dispatch_config_defaults`, `test_parse_dispatch_json_*`
```rust
// Key code:
pub struct DirectDispatchConfig { pub socket_path: PathBuf }
pub struct DispatchCommand { pub agent: String, pub context: Option<String> }
pub struct DispatchResponse { pub status: String, pub message: Option<String> }
```

### Step 2: UDS Listener in Orchestrator (2h)
**Files:** `lib.rs`, `direct_dispatch.rs`
**Description:** Start UDS listener tokio task, send `WebhookDispatch::SpawnAgent` to loop
**Tests:** `test_validate_command_*`, integration test
```rust
// Key code in lib.rs startup (after webhook server start):
if let Some(ref dd_config) = config.direct_dispatch {
    let agent_names: HashSet<String> = config.agents.iter().map(|a| a.name.clone()).collect();
    let dd_handle = direct_dispatch::start_direct_dispatch_listener(
        dd_config.socket_path.clone(),
        loop_tx.clone(),
        agent_names,
    );
    self.direct_dispatch_handle = Some(dd_handle);
}
```

### Step 3: adf-ctl --direct flag (1.5h)
**Files:** `adf-ctl.rs`
**Description:** Add `--direct` flag, implement UDS client, auto-discover socket path
**Tests:** `test_resolve_socket_path_*`, `test_build_direct_dispatch_payload`
```rust
// Key code:
if direct {
    let socket_path = resolve_socket_path()?;
    let mut stream = UnixStream::connect(&socket_path)?;
    let payload = serde_json::json!({"agent": name, "context": context});
    stream.write_all(payload.to_string().as_bytes())?;
    // read response
}
```

### Step 4: End-to-End Verification (0.5h)
**Description:** Full integration test: start orchestrator, direct-dispatch agent, verify spawn
**Tests:** `test_direct_dispatch_e2e`

## Rollback Plan
- Remove `[direct_dispatch]` from orchestrator.toml -- listener not started
- `adf-ctl trigger --direct` fails with "direct dispatch not configured" -- graceful

## Dependencies
**No new external dependencies.** Uses `tokio::net::UnixListener` (already in dependency tree via tokio) and `serde_json` (already imported).

## Performance Considerations
| Metric | Target |
|--------|--------|
| UDS connect + dispatch latency | < 10ms |
| Agent spawn latency (dispatch to spawn) | < 1 tick (~30s) |
| Socket path discovery | < 1ms (single stat call per directory level) |

## Open Items
| Item | Status |
|------|--------|
| Socket path default location | Needs decision: `<working_dir>/.adf-ctl.sock` vs `/tmp/adf-ctl.sock` |
| Socket permissions | `umask 077` before bind or `chmod 600` after |

## Approval
- [ ] Design review requested
