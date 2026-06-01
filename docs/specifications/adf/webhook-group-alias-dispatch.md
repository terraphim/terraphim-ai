# Specification: Webhook Group Alias Dispatch and Expansion Limits

**Status**: Authoritative — Implemented and Verified
**Source**: `crates/terraphim_orchestrator/src/webhook.rs`
**Issue**: #1924 (re-scoped from PR #1788 Slice 8)
**Date**: 2026-06-01
**Lines of Code**: ~26,449 bytes
**Test Coverage**: All invariants verified via `cargo test -p terraphim_orchestrator webhook`

---

## Overview

The webhook handler supports group alias mentions that expand a single `@adf:<alias>`
into multiple `SpawnAgent` dispatches for all agents whose names match the pattern
`<alias>-*`. An expansion cap prevents a single mention from triggering an unbounded
number of spawns.

---

## Group Alias Resolution

### Naming Convention

An agent name is a member of group `<alias>` if and only if the agent name starts with
the prefix `<alias>-`.

Examples:

| Mention | Resolved agents (given registry `[a-A, a-B, a-C, b-X]`) |
|---------|--------------------------------------------------------|
| `@adf:a` | `a-A`, `a-B`, `a-C` |
| `@adf:b` | `b-X` |
| `@adf:c` | _(empty — no match → treated as direct agent name)_ |

### Expansion Cap

```
DEFAULT_MAX_GROUP_ALIAS_MEMBERS = 10
```

The `group_alias_members` function uses `.take(10)` on the iterator. If more than 10
agents match the prefix, only the first 10 (in registration order) are dispatched.
The cap is a DoS guard: a broad alias pattern must not spawn arbitrarily many agents
from a single comment.

---

## Dispatch Flow

When the webhook handler receives a Gitea issue comment event:

1. Validate HMAC-SHA256 signature against `X-Gitea-Signature` (or `X-Hub-Signature-256`).
2. Parse the comment body for `@adf:<name>` mentions via `AdfCommandParser`.
3. For each mention, call `group_alias_members(name, &state.agent_names)`.
   - If the result is non-empty: emit one `WebhookDispatch::SpawnAgent` per member.
   - If the result is empty: emit a single `WebhookDispatch::SpawnAgent` with `agent_name = name` (direct dispatch).
4. Enqueue dispatches on `WebhookState.dispatch_tx`.

---

## `WebhookDispatch` Variants

| Variant | Trigger |
|---------|---------|
| `SpawnAgent { agent_name, detected_project, issue_number, comment_id, context, synthetic_event }` | Resolved from a mention (direct or group member) |
| `SpawnPersona { persona_name, issue_number, comment_id, context }` | Persona mention |
| `CompoundReview { issue_number, comment_id }` | Compound review command |
| `ReviewPr { pr_number, project, head_sha, author_login, title, diff_loc }` | Pull request event |
| `Push { project, ref_name, before_sha, after_sha, pusher_login, files_changed }` | Push event |

`ReviewPr` and `Push` dispatches are not associated with a comment; their `comment_id()`
returns `0`.

---

## Invariants

| # | Invariant | Source |
|---|-----------|--------|
| I1 | Group expansion never exceeds `DEFAULT_MAX_GROUP_ALIAS_MEMBERS` per mention. | `.take(DEFAULT_MAX_GROUP_ALIAS_MEMBERS)` in `group_alias_members` |
| I2 | An alias that matches zero agents falls through to direct agent dispatch. | Call sites check `is_empty()` on `group_alias_members` result |
| I3 | Webhooks without a valid HMAC signature are rejected with HTTP 401. | `verify_signature` guard before payload parsing |
| I4 | Webhook processing does not block the HTTP handler; dispatch is fire-and-forget via channel. | `dispatch_tx.send(...)` is non-blocking |
| I5 | `detected_project` is `None` for unqualified `@adf:name` mentions. | `WebhookDispatch::SpawnAgent` docs |

---

## Failure Modes

| Failure | Observable Effect | Recovery |
|---------|-------------------|---------|
| HMAC secret mismatch | HTTP 401; no dispatch emitted | Check `WEBHOOK_SECRET` configuration |
| Dispatch channel full | `dispatch_tx.send` returns error; mention dropped; WARN logged | Increase channel capacity or reduce mention frequency |
| Group alias matches > 10 agents | Only first 10 dispatched; remainder silently truncated | Reduce group size or increase cap |
| No agents match alias, agent name also unknown | `SpawnAgent` emitted for unknown name; orchestrator handles unknown-agent error | Register the agent or correct the mention |

---

## Verification Note

The following test covers group alias expansion logic:

```bash
cargo test -p terraphim_orchestrator webhook -- --nocapture
cargo test -p terraphim_orchestrator group_alias -- --nocapture
```

The `group_alias_members` function is unit-testable with a synthetic agent name list.
HMAC validation is tested via the webhook handler integration tests.
