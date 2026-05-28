# Research Document: ADF Direct Dispatch Semantic Gap

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-27
**Reviewers**: Pending
**Issue**: #1875

## Executive Summary

The runtime listener now starts when `[direct_dispatch]` is configured, but direct dispatch still reuses the webhook mention handler. That handler requires `[mentions]`, so `adf-ctl --local trigger --direct` can receive a socket-level success response while the orchestrator silently drops the dispatch before spawning an agent.

The essential fix is to preserve the shared socket/listener implementation while separating direct dispatch semantics from webhook mention semantics inside the orchestrator event loop.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This closes the remaining acceptance gap for local direct dispatch, the central purpose of #1875. |
| Leverages strengths? | Yes | The fix is orchestration-flow design: event routing, Rust async tests, and disciplined verification. |
| Meets real need? | Yes | The user requirement is local `adf-ctl` dispatch without SSH/webhook/HMAC; the current path is not yet guaranteed to spawn agents. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

Direct dispatch currently enters the orchestrator as `WebhookDispatch::SpawnAgent` and is consumed by `handle_webhook_dispatch()`. That handler begins by reading `self.config.mentions`; when `mentions` is absent, it returns immediately.

The Unix socket listener acknowledges only that it accepted and queued a command. It does not know whether the orchestrator later spawned the agent. This creates a false-positive success path for `adf-ctl --local trigger --direct`.

### Impact

Operators using local direct dispatch may see a successful CLI response but no agent execution. This is worse than an explicit failure because it obscures the real configuration/dispatch issue and undermines confidence in `--direct` mode.

### Success Criteria

- Direct dispatch does not require `[mentions]` configuration.
- Direct dispatch either spawns the requested configured agent or returns/logs an explicit dispatch failure.
- Webhook mention dispatch behaviour remains unchanged.
- Existing socket-level validation for unknown agents remains unchanged.
- Tests prove request-to-orchestrator dispatch behaviour, not only socket creation.

## Current State Analysis

### Existing Implementation

`direct_dispatch::start_direct_dispatch_listener()` accepts newline-delimited JSON on a Unix domain socket and forwards valid commands as `WebhookDispatch::SpawnAgent` with synthetic `issue_number: 0` and `comment_id: 0`.

`AgentOrchestrator::run()` now creates a shared dispatch channel for webhook and direct dispatch producers. Both producers feed `LoopEvent::Webhook` through `webhook_dispatch_rx`.

`handle_webhook_dispatch()` implements mention-specific semantics: mention rate limits, mention chain context, Gitea issue assignment, comment processed marking, persona resolution, and event-only agent rejection.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Direct socket listener | `crates/terraphim_orchestrator/src/direct_dispatch.rs` | UDS protocol, agent allow-list, channel send, JSON response. |
| Orchestrator event loop | `crates/terraphim_orchestrator/src/lib.rs` | Consumes loop events and invokes dispatch handlers. |
| Webhook mention dispatch | `crates/terraphim_orchestrator/src/lib.rs::handle_webhook_dispatch` | Handles HTTP webhook-originated mention/persona/PR/push dispatches. |
| CLI direct mode | `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Sends direct dispatch commands over UDS. |
| Direct config | `crates/terraphim_orchestrator/src/config.rs` | Defines `[direct_dispatch] socket_path`. |

### Data Flow

Current flow:

```text
adf-ctl --direct
  -> Unix socket listener
  -> WebhookDispatch::SpawnAgent { issue_number: 0, comment_id: 0 }
  -> shared dispatch channel
  -> LoopEvent::Webhook
  -> handle_webhook_dispatch()
  -> returns early if config.mentions is None
```

Desired flow:

```text
adf-ctl --direct
  -> Unix socket listener
  -> direct-origin dispatch event
  -> direct dispatch handler
  -> resolve configured agent
  -> spawn_agent()
```

### Integration Points

- `tokio::sync::mpsc::Sender<WebhookDispatch>` currently couples producers to webhook dispatch type.
- `LoopEvent::Webhook` currently conflates dispatch origin with payload type.
- `spawn_agent(&AgentDefinition)` is the existing spawning boundary and should remain the final operation.

## Constraints

### Technical Constraints

- No new external dependencies.
- Preserve existing webhook semantics and tests.
- Preserve existing direct socket protocol unless a protocol-level acknowledgement change is explicitly required later.
- Keep the fix small; do not redesign ADF dispatch broadly.
- Unix domain socket implementation remains Unix-only.

### Business Constraints

- #1875 acceptance depends on a reliable local workflow.
- Avoid broad runtime lifecycle work, such as storing and aborting all listener handles, unless necessary.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Local dispatch correctness | Direct trigger spawns configured agent without SSH/webhook | Partial; listener starts but handler can drop dispatch. |
| Security | Local UDS permission model, no HMAC for local direct path | Socket sets `0600`; unchanged. |
| Compatibility | Existing webhook behaviour unchanged | Currently shared handler preserved; future fix must maintain this. |

## Vital Few (Essentialism)

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Direct dispatch must not require `[mentions]` | This is the active P1 acceptance blocker. | `handle_webhook_dispatch()` returns when `mentions` is absent. |
| Webhook semantics must remain untouched | Existing mention dispatch has rate limiting, chain tracking, and Gitea behaviour. | The same handler supports several webhook event types. |
| Tests must prove request-to-spawn intent | Socket creation alone missed the semantic bug. | Structural PR review P2 finding. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| New socket protocol with synchronous spawn acknowledgement | Broader than needed; current protocol can remain queue-acknowledgement if dispatch semantics are correct. |
| Replacing `WebhookDispatch` enum everywhere | Risky broad rename/refactor; origin can be represented at loop level first. |
| Full listener lifecycle manager | Existing webhook server also lacks stored handle; not required for acceptance. |
| Windows/named pipe support | Current issue is Linux/Unix local dispatch. |
| Direct persona/PR/push dispatch over UDS | User story only covers triggering a named agent. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `AgentOrchestrator::spawn_agent` | Direct dispatch should converge here. | Low; already used widely. |
| `mention::resolve_mention` | Could resolve agent names, but carries mention/project semantics. | Medium; direct dispatch may need simpler exact-name resolution. |
| `AgentDefinition.event_only` | Existing mention path rejects event-only agents. | Medium; direct dispatch policy must decide whether manual direct triggers can run event-only agents. |
| `should_skip_dispatch` | Tied to Gitea issue assignment and issue number. | Medium; synthetic issue `0` may produce misleading dedup behaviour. |

### External Dependencies

No new external dependencies are needed.

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Direct path bypasses important rate limiting | Medium | Medium | Use existing global spawn safeguards; document that mention-specific limits do not apply. |
| Direct path incorrectly rejects event-only agents | Medium | Medium | Decide policy explicitly in design. Recommended: direct manual trigger can spawn any configured enabled agent because it is operator-initiated. |
| Tests accidentally spawn real long-running agents | Low | Medium | Use `echo` test agent and existing real spawner only if bounded; otherwise test handler state with minimal run harness. |
| Queue-level `ok` response still does not guarantee spawn success | High | Medium | Document semantics; add logs/tests proving handler receives direct event. Protocol-level spawn acknowledgement is out of scope unless required. |

### Open Questions

1. Should direct manual dispatch allow `event_only` agents? Recommended answer: yes, because direct dispatch is an explicit operator command rather than an issue mention.
2. Should direct dispatch attach synthetic Gitea issue `0` to spawned agent config? Recommended answer: no; leave `gitea_issue` unchanged unless a real issue is provided later.
3. Should direct dispatch use mention chain context? Recommended answer: no; use CLI context as direct operator context only.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Direct dispatch means manual operator-triggered named-agent execution. | `adf-ctl trigger --local --direct` requirement. | If direct dispatch should simulate a mention, bypassing mention logic would be wrong. | Partially |
| Socket `ok` can mean queued, not completed spawn. | Existing listener writes response after channel send. | Operators may expect spawn-completed acknowledgement. | Yes |
| No new dependencies remain required. | Existing code has all needed primitives. | None significant. | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Direct dispatch is a faster webhook mention | Requires `[mentions]`, chain context, Gitea issue semantics. | Rejected because it fails the local direct acceptance goal without mention config. |
| Direct dispatch is an operator command | Resolves configured agent and spawns directly with provided context. | Chosen because it matches `adf-ctl --local trigger --direct`. |
| Direct dispatch should synchronously report spawn success/failure | Requires response channel or protocol redesign. | Deferred because current issue is semantic drop after queueing. |

## Research Findings

### Key Insights

1. The active bug is not the listener anymore; it is origin conflation in the orchestrator event loop.
2. `WebhookDispatch` can remain the payload for now, but `LoopEvent` should carry direct origin distinctly.
3. Direct dispatch needs a dedicated handler or a shared lower-level spawn helper that does not require `MentionConfig`.
4. The current runtime test must be extended from socket creation to actual direct request consumption.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Direct handler extraction | Identify minimum code needed to spawn exact-name agent without mention context. | 30-45 minutes |
| Runtime request-to-spawn test | Prove a real socket request reaches a bounded test agent path. | 1-2 hours |

## Recommendations

### Proceed/No-Proceed

Proceed. This is a small, high-value correction required before the feature can be called production-ready.

### Scope Recommendations

- Add `LoopEvent::DirectDispatch(WebhookDispatch)` or a smaller direct command type.
- Bridge direct dispatch receiver to `LoopEvent::DirectDispatch`, not `LoopEvent::Webhook`.
- Add `handle_direct_dispatch()` for `SpawnAgent` only.
- Keep webhook dispatch untouched.

### Risk Mitigation Recommendations

- Do not call `should_skip_dispatch()` for direct dispatch until issue semantics are explicit.
- Do not add mention chain context for direct dispatch.
- Log direct dispatch attempts, rejections, and spawn failures with `direct_dispatch` terminology.
- Add tests for no-mentions config and real socket request path.

## Next Steps

If approved:

1. Implement a dedicated direct dispatch loop event and handler.
2. Add no-mentions request-to-spawn test coverage.
3. Re-run verification and validation checks.
