# Research Document: Direct Dispatch for adf-ctl --local

**Status**: Draft
**Author**: AI Agent
**Date**: 2026-05-25
**Issue**: terraphim/terraphim-ai#1875

## Executive Summary

`adf-ctl --local trigger` currently requires a running webhook server, HMAC negotiation, and mention-polling latency (~30s minimum) to dispatch agents. The orchestrator has no IPC mechanism beyond HTTP. The simplest correct approach is a Unix domain socket for fire-and-forget dispatch commands, mirroring the existing `WebhookDispatch::SpawnAgent` path in the orchestrator event loop.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Eliminates webhook dependency for local dev; 30s latency removed |
| Leverages strengths? | Yes | Existing `WebhookDispatch::SpawnAgent` + `LoopEvent` channel are perfect extension points |
| Meets real need? | Yes | Verified: webhook dispatch adds 30s+ latency; 5 agents tested locally all hit this |

**Proceed**: Yes -- 3/3 YES.

## Problem Statement

### Description
`adf-ctl --local trigger` requires HTTP roundtrip through the orchestrator's webhook server (127.0.0.1:9091/webhooks/gitea), constructing fake Gitea payloads with HMAC signatures. This adds latency (mention poll_modulo * tick_interval_secs) and requires:
1. Webhook server to be configured and running
2. HMAC secret to be negotiated (env var or config file)
3. Fake Gitea JSON payload construction
4. Mention polling delay

### Impact
- Local development feedback loop is ~30-60 seconds per agent trigger
- Cannot dispatch agents when webhook is not configured
- `adf-ctl trigger` is coupled to Gitea webhook format despite being a CLI tool

### Success Criteria
1. `adf-ctl --local trigger <agent> --direct` dispatches without HTTP webhook
2. Agent spawns on the NEXT tick (latency <= tick_interval_secs, not poll_modulo * tick_interval_secs)
3. No HMAC secret required for direct dispatch
4. Works when orchestrator webhook is not configured (`[webhook]` section absent)
5. Same output format as current trigger (HTTP-like status)

## Current State Analysis

### Existing Implementation

The orchestrator main loop receives events through a single `std::sync::mpsc::Receiver<LoopEvent>` channel:

```
LoopEvent::Tick | Schedule | DriftAlert | Webhook(WebhookDispatch)
```

`WebhookDispatch::SpawnAgent` is the exact variant needed for direct dispatch. Currently it's only produced by the axum webhook handler, but nothing prevents its construction from other sources.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `LoopEvent` enum | `lib.rs:1295` | Central event type received by main loop |
| `WebhookDispatch::SpawnAgent` | `webhook.rs:82` | The dispatch variant for agent spawn |
| `handle_webhook_dispatch` | `lib.rs:3527` | Processes WebhookDispatch variants |
| `spawn_agent` | `lib.rs:1853` | Single funnel for all agent spawns |
| `dispatch_tx` (mpsc) | `lib.rs:1253` | Channel from webhook handler to main loop |
| `adf-ctl trigger` | `adf-ctl.rs:313` | CLI trigger command |
| `adf --local --agent` | `adf.rs:554` | Direct spawn bypassing orchestrator |

### Data Flow

```
Current (webhook):
  adf-ctl → curl POST → axum → dispatch_tx → loop_rx → handle_webhook_dispatch → spawn_agent

Current (adf --local --agent):
  adf → AgentSpawner::spawn_with_fallback() directly (no orchestrator)

Proposed (UDS):
  adf-ctl → Unix socket connect → orchestrator UDS listener → loop_tx.send(Webhook(SpawnAgent)) → spawn_agent
```

### Integration Points

1. **LoopEvent channel** (`lib.rs:1303`): `Arc<Mutex<Sender<LoopEvent>>>` shared across tick thread, scheduler, nightwatch. Adding a UDS listener that sends to this same channel requires minimal change.
2. **WebhookDispatch** (`webhook.rs:80`): Already has `SpawnAgent { agent_name, detected_project, issue_number, comment_id, context }`. All fields have sensible defaults for direct dispatch.
3. **Config** (`config.rs`): A new `[direct_dispatch]` section can be added, or the UDS path can be derived from `working_dir`.

## Constraints

### Technical Constraints
- Rust async runtime (tokio) -- UDS listener must integrate with tokio's event loop
- No external dependencies needed -- `tokio::net::UnixListener` is in std/tokio
- Must not block the main event loop
- Must handle concurrent connects gracefully

### Business Constraints
- Must work when webhook is not configured
- Must not require HMAC secret for local communication (UDS is filesystem-permission-gated)
- Must match existing `adf-ctl trigger` UX

### Non-Functional Requirements
| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Dispatch latency | < 1 tick (~30s) | Current: poll_modulo * tick (~60s typical, 30s with poll_modulo=1) |
| Socket path discovery | Automatic | Same CWD walk-up as `.terraphim/adf.toml` |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Must integrate with existing `LoopEvent` channel | Only way to reach `spawn_agent` funnel | All agent spawns flow through this; adding new path would duplicate gates |
| Must be filesystem-permission-gated | No HMAC needed for local; UDS permissions replace authentication | Standard Unix security model |
| Must discover socket path automatically | Same UX as `--local agents` | `discover_local_config()` pattern already exists |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Named pipe/FIFO approach | UDS is bidirectional, supports concurrent clients, and is more standard for IPC |
| Embedded spawner in adf-ctl | Would require linking terraphim_spawner + terraphim_orchestrator into adf-ctl binary (~200MB); defeats purpose of lightweight CLI |
| Phase 2 admin socket (full control plane) | Out of scope for this issue; UDS for dispatch only |
| Multi-tool protocol (msgpack, protobuf) | JSON is acceptable for local IPC; low throughput requirement |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `LoopEvent` enum | Must add `DirectDispatch` variant or reuse `Webhook` | Low -- enum is local to lib.rs |
| `handle_webhook_dispatch` | Can reuse existing dispatch logic | Low -- already handles `SpawnAgent` |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| None new | - | - | `tokio::net::UnixListener` is in std |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Socket file left behind on crash | Medium | Low (next startup cleans it) | Unlink before bind; use abstract socket on Linux |
| Concurrent dispatch flooding | Low | Medium (channel capacity 64) | Channel already has capacity bound; backpressure is graceful |

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Unix domain sockets are available on all target platforms | Linux is primary target; macOS also supports UDS | Windows would fail (not a target) | Yes -- Linux only |
| Socket file permissions (0600) are sufficient auth for local dispatch | UDS is accessible only to same user | Multi-user systems need additional auth | Yes -- single-user dev laptop |
| `discover_local_config()` can discover socket path | Socket path can be stored in `.terraphim/adf.toml` or derived from `working_dir` | Config file must exist | Partially -- `.terraphim/adf.toml` exists |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| UDS listener in separate thread | Simpler, synchronous | Rejected: tokio runtime already exists; spawning OS thread adds complexity |
| UDS listener as tokio task | Async, integrates with existing runtime | **Chosen**: matches webhook listener pattern |
| Reuse webhook handler for UDS | Minimal code change | Rejected: webhook handler expects HTTP headers, HMAC verification -- unnecessary overhead |

## Research Findings

### Key Insights

1. **`adf --local --agent NAME` already bypasses the orchestrator entirely** but blocks until agent completion. We need fire-and-forget dispatch, not blocking spawn.
2. **All agent spawns funnel through `spawn_agent()`** in `lib.rs:1853`. Adding a new entry point that calls this function is architecturally correct but requires refactoring it to be callable from outside the orchestrator.
3. **The existing `LoopEvent` channel is the cleanest extension point**. Adding a UDS listener that sends to this channel follows the exact pattern of the webhook server.
4. **`WebhookDispatch::SpawnAgent` with `issue_number=0`** already exists and bypasses dedup checks. This is the same variant used by `adf-ctl trigger` today.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Verify `tokio::net::UnixListener` integrates with existing tokio runtime | Confirm compatibility | ~30 min |
| Test socket discovery from `.terraphim/adf.toml` | Confirm config pattern works | ~15 min |

## Recommendations

### Proceed/No-Proceed
**Proceed** with Unix domain socket dispatch approach.

### Scope Recommendations
Phase 1 (this issue):
- Add UDS listener to orchestrator (`[direct_dispatch].socket_path` config, defaults to `<working_dir>/.adf-ctl.sock`)
- Add `--direct` flag to `adf-ctl trigger --local`
- `adf-ctl` writes JSON `{"agent": "name", "context": "..."}` to socket
- No HMAC, no webhook payload construction

Phase 2 (future):
- Admin socket for `status`, `cancel`, `agents` with authoritative answers

### Risk Mitigation Recommendations
- Socket cleanup on startup (unlink before bind)
- Use `tokio::net::UnixListener` for async integration
- Default socket path: `<working_dir>/.adf-ctl.sock` for automatic discovery

## Appendix

### Reference Code

**WebhookDispatch::SpawnAgent** (webhook.rs:82-88):
```rust
SpawnAgent {
    agent_name: String,
    detected_project: Option<String>,
    issue_number: i64,
    comment_id: i64,
    context: String,
}
```

**LoopEvent channel creation** (lib.rs:1303):
```rust
let loop_tx: Arc<Mutex<Sender<LoopEvent>>> = Arc::new(Mutex::new(loop_tx));
```

**Existing UDS mention** (adf-ctl.rs:709):
```rust
"(Phase 2 admin socket will provide authoritative cancel)"
```
