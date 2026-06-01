# Specification: Redacted Output Capture and Timeout Reporting

**Status**: Authoritative
**Sources**:
- `crates/terraphim_spawner/src/redaction.rs`
- `crates/terraphim_spawner/src/output.rs`
**Issue**: #1924 (re-scoped from PR #1788 Slice 8)
**Date**: 2026-06-01

---

## Overview

The spawner captures agent stdout/stderr line-by-line, detects `@mention` patterns,
and stores a bounded buffer of events for timeout diagnostics. All events stored in
the buffer are redacted before storage so that if the buffer is attached to a timeout
report or Gitea comment, no secrets leak into issue trackers or logs.

---

## Redaction

### Patterns

The `DEFAULT_REDACTION_PATTERNS` array in `redaction.rs` defines seven regex patterns:

| Pattern | What it matches |
|---------|----------------|
| `api[_-]?key\s*[:=]\s*([^\s]+)` | `api_key=<value>`, `api-key: <value>`, etc. |
| `token\s*[:=]\s*([^\s]+)` | `token=<value>`, `TOKEN: <value>`, etc. |
| `secret\s*[:=]\s*([^\s]+)` | `secret=<value>`, etc. |
| `password\s*[:=]\s*([^\s]+)` | `password=<value>`, etc. |
| `sk-[a-zA-Z0-9]{20,}` | OpenAI-style API keys |
| `ghp_[a-zA-Z0-9]{36}` | GitHub personal access tokens |
| `bearer\s+[a-zA-Z0-9_\-]{20,}` | Bearer tokens in Authorization headers |

All patterns are case-insensitive. The last capture group in each pattern holds the
secret value; prefix groups are preserved.

### Replacement Token

Matched secret values are replaced with the literal string `***REDACTED***`.

### `verify_redacted`

A companion function checks whether a string is clean. It returns `true` if either:
- no pattern matches, or
- the string already contains `***REDACTED***` (considered clean; was already processed)

---

## Output Capture

### `OutputEvent` Variants

| Variant | Redacted on storage? |
|---------|----------------------|
| `Stdout { process_id, line }` | Yes — `line` is passed through `redact()` |
| `Stderr { process_id, line }` | Yes — `line` is passed through `redact()` |
| `Mention { process_id, target, message }` | Yes — `message` is passed through `redact()`; `target` is not |
| `Completed { process_id, exit_code }` | n/a (no text payload) |

### Bounded Buffer

`OutputCapture` maintains a `VecDeque<OutputEvent>` capped at `MAX_CAPTURED_EVENTS = 4096`.
When the buffer is full, the oldest entry is evicted (`pop_front`) before inserting the
new event (`push_back`). This is a FIFO circular buffer.

### Live Streaming

In addition to the bounded buffer, every event is broadcast on a `tokio::sync::broadcast`
channel (capacity 256). Subscribers (e.g., WebSocket clients) receive raw events before
redaction, because they are direct observers of the live stream. Stored events are always
redacted.

---

## Invariants

| # | Invariant | Source |
|---|-----------|--------|
| I1 | Events written to the bounded buffer are always redacted. | `record_event` calls `event.redacted()` before `push_back` |
| I2 | The buffer never exceeds `MAX_CAPTURED_EVENTS` entries. | `pop_front` when `len >= MAX_CAPTURED_EVENTS` |
| I3 | `Completed` events carry no text; redaction is a no-op. | `OutputEvent::redacted()` match arm |
| I4 | `Mention.target` is never redacted (it is an agent name, not a secret). | `target: target.clone()` in `redacted()` |
| I5 | An empty `line` is skipped and never stored. | `if line.is_empty() { continue; }` in capture loops |

---

## Failure Modes

| Failure | Observable Effect | Recovery |
|---------|-------------------|---------|
| Regex compilation error in `redact()` | Pattern silently skipped; other patterns still apply | Validate patterns at startup |
| Buffer full | Oldest events evicted; tail of output retained | Increase `MAX_CAPTURED_EVENTS` if diagnosis is impaired |
| Live broadcast receiver lagged | Receiver is dropped; no data loss in buffer | Subscriber reconnects |
| `Stdout`/`Stderr` read error | Error logged at ERROR level; capture goroutine exits | Agent process has exited or EOF |

---

## Verification Note

The following tests were verified to pass on `gitea/main` as of 2026-06-01:

```bash
cargo test -p terraphim_spawner redaction -- --nocapture
cargo test -p terraphim_spawner output -- --nocapture
```

Tests verified (redaction):
- `test_redact_api_key`
- `test_redact_token`
- `test_redact_secret`
- `test_redact_password`
- `test_redact_sk_key`
- `test_redact_bearer_token`
- `test_no_false_positives_on_safe_text`
- `test_verify_redacted_detects_leak`
- `test_verify_redacted_after_redaction`
- `test_redact_preserves_structure`
- `test_redact_multiple_secrets`

Tests verified (output capture):
- `test_mention_regex`
- `test_output_event_redacted_scrubs_secrets`
- `test_output_event_redacted_preserves_structure`
- `test_captured_events_bounded`
- `test_captured_events_redacts_before_storage`
