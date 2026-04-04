# Synthetic Time Testing Guide

How to write deterministic, fast tests for time-dependent logic in the Terraphim ADF crates.

---

## The Problem

Three ADF crates make scheduling decisions based on elapsed time: stall detection, message expiry, retry eligibility, restart intensity windows. Before this change, all functions called `Utc::now()` internally, making tests either non-deterministic (race conditions at boundaries) or slow (real `sleep()` calls).

## The Solution

Two complementary techniques, zero new dependencies:

1. **Parameter injection** -- Logic-critical functions accept `now: DateTime<Utc>` instead of calling `Utc::now()` internally. Production code passes `Utc::now()` at the call site; tests pass explicit timestamps.

2. **`tokio::time::pause`** -- For tests involving tokio timers (sleep, interval, timeout), use the built-in tokio test infrastructure to freeze and advance time.

---

## Technique 1: Parameter Injection (Wall-Clock Decisions)

### Pattern

```rust
// BEFORE: untestable at boundaries
pub fn check_stall(started_at: DateTime<Utc>, timeout_ms: i64) -> bool {
    let elapsed = (Utc::now() - started_at).num_milliseconds();
    elapsed > timeout_ms
}

// AFTER: deterministically testable
pub fn check_stall(started_at: DateTime<Utc>, timeout_ms: i64, now: DateTime<Utc>) -> bool {
    let elapsed = (now - started_at).num_milliseconds();
    elapsed > timeout_ms
}
```

Production call site unchanged in intent:
```rust
let stalled = check_stall(entry.started_at, timeout, Utc::now());
```

Test with exact boundary control:
```rust
#[test]
fn no_stall_at_exact_boundary() {
    let started = Utc::now();
    let now = started + chrono::Duration::milliseconds(300_000);
    assert!(!check_stall(started, 300_000, now)); // elapsed == timeout, not >
}

#[test]
fn stall_one_ms_over_boundary() {
    let started = Utc::now();
    let now = started + chrono::Duration::milliseconds(300_001);
    assert!(check_stall(started, 300_000, now));
}
```

### Available Functions

#### Symphony -- Stall Detection

```rust
// crates/terraphim_symphony/src/orchestrator/reconcile.rs

/// Check a single running entry for stall.
pub fn check_stall(
    last_event_timestamp: Option<DateTime<Utc>>,
    started_at: DateTime<Utc>,
    stall_timeout_ms: i64,
    now: DateTime<Utc>,                          // <-- injected
) -> Option<ReconcileAction>

/// Collect stalled issue IDs from the runtime state.
pub fn find_stalled_issues(
    state: &OrchestratorRuntimeState,
    stall_timeout_ms: i64,
    now: DateTime<Utc>,                          // <-- injected
) -> Vec<String>
```

Example test:
```rust
use chrono::Utc;
use crate::orchestrator::reconcile::{check_stall, ReconcileAction};

#[test]
fn stall_uses_last_event_timestamp_when_present() {
    let started = Utc::now();
    let last_event = started + chrono::Duration::seconds(100);
    let now = last_event + chrono::Duration::seconds(200);
    // 200s since last event, within 300s timeout
    let result = check_stall(Some(last_event), started, 300_000, now);
    assert!(result.is_none());
}

#[test]
fn stall_falls_back_to_started_at_when_no_events() {
    let started = Utc::now();
    let now = started + chrono::Duration::milliseconds(300_001);
    let result = check_stall(None, started, 300_000, now);
    assert!(matches!(result, Some(ReconcileAction::StallDetected)));
}
```

#### Symphony -- State Snapshot

```rust
// crates/terraphim_symphony/src/orchestrator/state.rs

/// Create a snapshot of the current state for observability.
pub fn snapshot(&self, now: DateTime<Utc>) -> StateSnapshot
```

Example test:
```rust
#[test]
fn snapshot_calculates_elapsed_from_injected_now() {
    let mut state = OrchestratorRuntimeState::new(30_000, 10);
    // ... add a running entry with known started_at ...
    let now = entry_started_at + chrono::Duration::seconds(120);
    let snap = state.snapshot(now);
    // Verify elapsed calculation uses injected time
    assert!(snap.codex_totals.seconds_running >= 120.0);
}
```

#### Messaging -- Message Expiry

```rust
// crates/terraphim_agent_messaging/src/message.rs

/// Check if message has expired.
pub fn is_expired(&self, now: DateTime<Utc>) -> bool
```

Example test:
```rust
use std::time::Duration;

#[test]
fn message_expires_after_timeout() {
    let options = DeliveryOptions {
        timeout: Duration::from_secs(30),
        ..Default::default()
    };
    let envelope = MessageEnvelope::new(
        AgentPid::new(),
        "test".to_string(),
        serde_json::Value::String("payload".to_string()),
        options,
    );

    // Not expired at 29s
    let before = envelope.created_at + chrono::Duration::seconds(29);
    assert!(!envelope.is_expired(before));

    // Expired at 30s + 1ms
    let after = envelope.created_at + chrono::Duration::milliseconds(30_001);
    assert!(envelope.is_expired(after));
}
```

#### Messaging -- Retry Eligibility

```rust
// crates/terraphim_agent_messaging/src/delivery.rs

/// Check if a message should be retried.
fn should_retry(&self, record: &DeliveryRecord, now: DateTime<Utc>) -> bool
```

This is a private method. Test it indirectly through `get_retry_candidates()`, or write tests within the `delivery.rs` module.

#### Supervisor -- Agent Uptime

```rust
// crates/terraphim_agent_supervisor/src/agent.rs

/// Get uptime duration.
pub fn uptime(&self, now: DateTime<Utc>) -> chrono::Duration
```

Example test:
```rust
#[test]
fn uptime_calculation() {
    let info = SupervisedAgentInfo::new(/* ... */);
    let now = info.start_time + chrono::Duration::hours(2);
    assert_eq!(info.uptime(now).num_hours(), 2);
}
```

#### Supervisor -- Restart Window

```rust
// crates/terraphim_agent_supervisor/src/supervisor.rs

/// Check if agent should be restarted based on policy.
async fn should_restart(
    &self,
    agent_id: &AgentPid,
    reason: &ExitReason,
    now: DateTime<Utc>,                          // <-- injected
) -> SupervisionResult<bool>
```

The `RestartIntensity::is_restart_allowed()` already accepts `time_since_first_restart: Duration` as a parameter, so it is directly testable without the async wrapper:

```rust
use std::time::Duration;

#[test]
fn restart_window_resets_after_time_window() {
    let intensity = RestartIntensity::new(3, Duration::from_secs(60));

    // At max restarts within window: denied
    assert!(!intensity.is_restart_allowed(3, Duration::from_secs(30)));

    // Exactly at window boundary: still denied (> not >=)
    assert!(!intensity.is_restart_allowed(3, Duration::from_secs(60)));

    // One second past window: counter resets, allowed
    assert!(intensity.is_restart_allowed(3, Duration::from_secs(61)));
}
```

---

## Technique 2: tokio::time::pause (Async Timers)

For tests that need to exercise `tokio::time::sleep`, `tokio::time::interval`, or `tokio::time::timeout` without real delays.

### Pattern

```rust
#[tokio::test(start_paused = true)]
async fn retry_timer_fires_after_backoff() {
    // Time is frozen at test start.
    // tokio::time::sleep/interval/timeout will NOT resolve
    // until we explicitly advance time.

    let retry_delay = Duration::from_secs(10);
    let timer = tokio::time::sleep(retry_delay);
    tokio::pin!(timer);

    // Timer has not fired yet
    assert!(futures::poll!(&mut timer).is_pending());

    // Advance time by 10 seconds
    tokio::time::advance(Duration::from_secs(10)).await;

    // Timer has now fired
    assert!(futures::poll!(&mut timer).is_ready());
}
```

### What it controls

| Primitive | Controlled by pause/advance? |
|-----------|------------------------------|
| `tokio::time::sleep()` | Yes |
| `tokio::time::interval()` | Yes |
| `tokio::time::timeout()` | Yes |
| `tokio::time::Instant::now()` | Yes |
| `chrono::Utc::now()` | **No** -- use parameter injection |
| `std::time::Instant::now()` | **No** -- not used in logic decisions |

### When to use which technique

| Decision type | Technique | Example |
|--------------|-----------|---------|
| "Has N milliseconds elapsed since X?" | Parameter injection (`now`) | Stall detection, message expiry, restart windows |
| "Fire after a tokio sleep/interval" | `tokio::time::pause` | Retry timer, health check loop, poll interval |
| Both | Combine both | Test that a stalled session triggers a retry timer |

### Combined example

```rust
#[tokio::test(start_paused = true)]
async fn stalled_session_triggers_retry_after_backoff() {
    // Setup orchestrator with known start time
    let started_at = Utc::now();

    // Advance tokio time past stall threshold
    tokio::time::advance(Duration::from_secs(301)).await;

    // Check stall with injected wall-clock time
    let now = started_at + chrono::Duration::seconds(301);
    let result = check_stall(None, started_at, 300_000, now);
    assert!(matches!(result, Some(ReconcileAction::StallDetected)));

    // Retry timer (tokio sleep) resolves because time is paused+advanced
    let backoff = Duration::from_secs(10);
    tokio::time::sleep(backoff).await; // resolves instantly with paused time
}
```

---

## Boundary Testing Conventions

All time comparisons in the codebase use **strict greater-than** (`>`), not greater-than-or-equal (`>=`). This means:

| Condition | Result |
|-----------|--------|
| `elapsed < threshold` | No action |
| `elapsed == threshold` | No action |
| `elapsed > threshold` | Action triggered |

Always test three points:
1. **Below threshold** -- verify no action
2. **At exact threshold** -- verify no action (the `==` case)
3. **One unit above threshold** -- verify action triggers

```rust
#[test]
fn boundary_below() {
    let now = started + chrono::Duration::milliseconds(threshold - 1);
    assert!(result.is_none());
}

#[test]
fn boundary_exact() {
    let now = started + chrono::Duration::milliseconds(threshold);
    assert!(result.is_none()); // > not >=
}

#[test]
fn boundary_above() {
    let now = started + chrono::Duration::milliseconds(threshold + 1);
    assert!(result.is_some());
}
```

---

## Running Tests

```bash
# All three ADF crates
cd crates/terraphim_symphony && cargo test --lib
cd crates/terraphim_agent_supervisor && cargo test
cd crates/terraphim_agent_messaging && cargo test

# Specific boundary tests
cargo test -p terraphim_symphony stall_boundary
cargo test -p terraphim_agent_messaging message_expired
cargo test -p terraphim_agent_messaging message_not_expired
cargo test -p terraphim_agent_supervisor restart_window_boundary
```

---

## Adding Synthetic Time to New Functions

When writing a new function that makes decisions based on elapsed time:

1. **Add `now: DateTime<Utc>` as the last parameter**
2. **Replace `Utc::now()` with `now`** in the function body
3. **Pass `Utc::now()` at the call site** in production code
4. **Write three boundary tests** (below, at, above threshold)
5. **Do not change timestamp-recording calls** (`created_at`, `started_at`, etc.) -- only decision-making comparisons need the parameter

```rust
// Good: decision function parameterised
pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
    (now - self.deadline).num_seconds() > 0
}

// Good: recording function keeps Utc::now()
pub fn record_start(&mut self) {
    self.started_at = Utc::now(); // this is fine
}
```
