# Implementation Plan: ADF Direct Dispatch Semantic Gap

**Status**: Draft
**Research Doc**: `.docs/research-adf-direct-dispatch-semantic-gap.md`
**Author**: OpenCode
**Date**: 2026-05-27
**Estimated Effort**: 2-4 hours
**Issue**: #1875

## Overview

### Summary

This plan removes the accidental dependency between local direct dispatch and webhook mention configuration. Direct socket commands will be routed as direct-origin events and handled by a small direct dispatch handler that resolves and spawns the requested configured agent without requiring `[mentions]`.

### Approach

Keep the existing Unix socket listener and JSON protocol. Change only the orchestrator event routing and handler semantics so direct dispatch no longer enters `handle_webhook_dispatch()`.

### Scope

**In Scope:**

- Add direct-origin event routing in `AgentOrchestrator::run()`.
- Add a direct dispatch handler for `WebhookDispatch::SpawnAgent` commands from the UDS listener.
- Ensure direct dispatch works when `config.mentions` is `None`.
- Add runtime tests that send a real direct socket command and prove the orchestrator accepts it beyond socket creation.
- Preserve all existing webhook mention behaviour.

**Out of Scope:**

- Protocol redesign for synchronous spawn-completion acknowledgements.
- Direct persona/PR/push support over UDS.
- New dependencies.
- Full lifecycle management for spawned listener tasks.
- Windows named pipe support.

**Avoid At All Cost:**

- Reusing `handle_webhook_dispatch()` for direct dispatch after this fix.
- Adding a broad dispatch framework abstraction.
- Changing webhook mention rate limits, chain tracking, or Gitea issue semantics.

## Architecture

### Component Diagram

```text
adf-ctl --direct
    -> direct_dispatch::start_direct_dispatch_listener
    -> mpsc::Sender<WebhookDispatch>
    -> direct dispatch bridge task
    -> LoopEvent::DirectDispatch(WebhookDispatch)
    -> AgentOrchestrator::handle_direct_dispatch
    -> AgentOrchestrator::spawn_agent
```

Webhook remains:

```text
HTTP webhook
    -> webhook::webhook_router
    -> mpsc::Sender<WebhookDispatch>
    -> webhook bridge task
    -> LoopEvent::Webhook(WebhookDispatch)
    -> AgentOrchestrator::handle_webhook_dispatch
```

### Data Flow

Direct command data flow:

```text
{ agent, context }
  -> WebhookDispatch::SpawnAgent { agent_name, context, issue_number: 0, comment_id: 0 }
  -> exact configured agent lookup
  -> cloned AgentDefinition with task augmented by direct context
  -> spawn_agent(&direct_def)
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Add `LoopEvent::DirectDispatch` | Separates origin semantics without changing socket protocol. | Reusing `LoopEvent::Webhook`, because it preserves mention dependency. |
| Add `handle_direct_dispatch()` | Keeps direct behaviour explicit and testable. | Adding boolean flags to `handle_webhook_dispatch()`, because that risks cross-contaminating webhook semantics. |
| Exact-name agent lookup | Direct CLI already validates configured names at listener layer. | `mention::resolve_mention`, because it carries mention/project semantics. |
| Do not set `gitea_issue` for direct dispatch | No real issue exists for `issue_number: 0`. | Keeping synthetic issue `0`, because it can pollute agent context and dedup logic. |
| Allow direct triggering of configured enabled agents, including event-only | Direct trigger is an explicit operator action, not a user comment mention. | Reusing mention event-only rejection, because that would block valid manual operations. |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| New `DirectDispatchCommand` type across listener and CLI | More churn than needed; current payload is sufficient. | Type drift with existing tests. |
| Response channel from orchestrator back to socket listener | Turns queue acknowledgement into synchronous spawn acknowledgement and complicates async flow. | Deadlocks/timeouts and broader protocol changes. |
| Feature flag for direct semantics | No need; direct dispatch config already gates the feature. | Extra config surface. |

### Simplicity Check

The simplest correct design is one new loop event plus one small handler. It avoids broad enum refactors, keeps existing listener tests, and isolates direct dispatch from mention-only behaviour.

**Nothing Speculative Checklist:**

- [x] No features the user did not request
- [x] No abstractions for future dispatch modes
- [x] No new dependencies
- [x] No protocol redesign
- [x] No premature optimisation

## File Changes

### New Files

No new source files are required.

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Add `LoopEvent::DirectDispatch`, route direct receiver to it, add `handle_direct_dispatch()`, add/adjust tests. |
| `.docs/research-adf-direct-dispatch-semantic-gap.md` | Phase 1 research artefact. |
| `.docs/design-adf-direct-dispatch-semantic-gap.md` | Phase 2 implementation plan. |

### Deleted Files

None.

## API Design

No public API changes are required.

### Internal Function

```rust
async fn handle_direct_dispatch(&mut self, dispatch: webhook::WebhookDispatch) {
    match dispatch {
        webhook::WebhookDispatch::SpawnAgent { agent_name, context, .. } => {
            // exact-name configured agent lookup
            // clone definition, append direct context if present
            // spawn_agent(&direct_def).await
        }
        other => {
            warn!(dispatch = ?other, "direct dispatch ignored unsupported dispatch type");
        }
    }
}
```

Direct handler behaviour:

- Find `AgentDefinition` by exact `name == agent_name`.
- If not found, log warning and return. The listener should already reject unknown names.
- If `enabled == false`, log warning and return.
- Clone the definition.
- If context is non-empty, append it to `task` with a short direct-dispatch marker.
- Do not set `gitea_issue` from synthetic issue `0`.
- Do not require `MentionConfig`.
- Do not call `MentionChainTracker`.
- Do not call `should_skip_dispatch()` because it is issue/Gitea-assignment specific.
- Call `spawn_agent(&direct_def).await` and log success/failure.

## Test Strategy

### Unit/Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_direct_dispatch_config_starts_socket_listener` | `lib.rs` tests | Keep existing proof that runtime creates socket. |
| `test_direct_dispatch_without_mentions_spawns_agent` | `lib.rs` tests | Prove direct handler does not require `[mentions]`. |
| `test_direct_dispatch_socket_request_reaches_orchestrator_without_mentions` | `lib.rs` tests | Send real UDS command after `run()` startup and verify observable dispatch result. |
| `test_direct_dispatch_rejects_disabled_agent` | `lib.rs` tests or handler test | Prove disabled agents are not manually spawned. |
| Existing `direct_dispatch::*` tests | `direct_dispatch.rs` | Ensure protocol behaviour is unchanged. |

### Observable Test Design

Preferred test approach:

- Use a temp socket path.
- Configure a single fast `echo` agent with `mentions = None` and `direct_dispatch = Some(...)`.
- Start `run()` with controlled shutdown as current test does, or add a bounded helper that starts only event bridges and listener.
- Send a real socket command to the listener.
- Assert one of:
  - `active_agents` contains the agent after direct handling, or
  - a test-safe ledger/output event records the spawn.

If `run()` is difficult to observe after shutdown, add a focused handler test that calls `handle_direct_dispatch()` directly and checks `active_agents`, plus retain socket-level listener tests.

No mocks of internal code should be introduced.

## Implementation Steps

### Step 1: Split Direct Event Routing

**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Add `LoopEvent::DirectDispatch(webhook::WebhookDispatch)` and route direct dispatch receiver into that event instead of `LoopEvent::Webhook`.
**Tests:** Existing focused direct socket startup test should still pass.
**Estimated:** 30 minutes.

Concrete edit:

- Keep shared channel creation if practical, but distinguish bridge origin.
- If one shared receiver cannot distinguish origin, create separate `direct_dispatch_rx` storage or wire direct listener sender to a separate channel.
- Prefer separate channels if that keeps origin explicit and code simple.

### Step 2: Add Direct Handler

**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Implement `handle_direct_dispatch()` with exact-name resolution and no `MentionConfig` dependency.
**Tests:** Add no-mentions handler test.
**Estimated:** 45 minutes.

### Step 3: Preserve Webhook Behaviour

**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Ensure webhook bridge still emits `LoopEvent::Webhook` and calls `handle_webhook_dispatch()` unchanged.
**Tests:** Existing webhook/mention tests in full lib suite.
**Estimated:** 30 minutes.

### Step 4: Add Request-to-Orchestrator Evidence

**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:** Add a real socket request test or direct handler integration test proving no-mentions direct dispatch reaches spawn intent.
**Tests:** New test plus existing `direct_dispatch` suite.
**Estimated:** 1-2 hours.

### Step 5: Verification and Validation

**Files:** N/A
**Description:** Run quality gates and update #1875.
**Tests/Commands:**

```bash
cargo fmt -- --check
cargo test -p terraphim_orchestrator --lib direct_dispatch
cargo test -p terraphim_orchestrator --bin adf-ctl
cargo clippy -p terraphim_orchestrator -- -D warnings
cargo test -p terraphim_orchestrator --lib
cargo llvm-cov -p terraphim_orchestrator --lib --summary-only -- direct_dispatch
ubs crates/terraphim_orchestrator/src/lib.rs crates/terraphim_orchestrator/src/direct_dispatch.rs crates/terraphim_orchestrator/src/bin/adf-ctl.rs crates/terraphim_orchestrator/src/config.rs
```

## Rollback Plan

If the direct handler causes regressions:

1. Revert the `LoopEvent::DirectDispatch` routing and handler changes.
2. Keep the planning artefacts for the next design iteration.
3. Existing webhook path remains unaffected if changed as specified.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Dispatch overhead | No meaningful increase over existing channel/event path | Existing tests and absence of new blocking work. |
| Socket response latency | Still returns after queue send | Existing direct socket tests. |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm direct trigger policy for `event_only` agents | Recommended: allow if configured/enabled | User/maintainer |
| Confirm queue ack vs spawn ack semantics | Recommended: keep queue ack for this issue | User/maintainer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received before implementation
