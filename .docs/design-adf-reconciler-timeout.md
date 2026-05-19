# Design & Implementation Plan: ADF Reconciler Timeout and Process Leak Fix

**Date:** 2026-05-19
**Status:** Phase 2 design
**Research:** `.docs/research-adf-reconciler-timeout.md`

## 1. Summary of Target Behaviour

After implementation:

| Outcome | Before | After |
|---------|--------|-------|
| Build-runner spawns on push | Never (queue stuck behind probes) | Within 1-2 s of webhook receipt |
| reconcile_tick timeout | Every tick | Only when TTL expires and probes re-run |
| Orphaned opencode processes | 60+ accumulated over 50 min | Zero orphans — killed on timeout or periodic cleanup |
| Circuit breaker for timeout providers | Never opens (deadlock) | Opens after 5 consecutive failures within cooldown window |

## 2. Key Invariants and Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| AC1 | `handle_push()` called directly from `handle_webhook_dispatch()` for Push events, NOT via dispatcher queue | Code review: match arm in `handle_webhook_dispatch` calls `self.handle_push(task)` |
| AC2 | `provider_health.probed_at` updated at START of `probe_all()`, not end | Code review; `is_stale()` returns false after first probe attempt |
| AC3 | Probe process group (not just bash) killed on timeout | Manual: insert `sleep 300 &` into probe action, verify grandchild also killed |
| AC4 | Orphaned opencode processes reaped in `adf-cleanup.sh` at startup AND periodically during runtime | Manual: verify `pkill` matches running opencode processes |
| AC5 | Circuit breaker opens for providers that time out 5 times consecutively | Log output shows "probe skipped: circuit breaker open" after 5th timeout |
| AC6 | `reconcile_tick` completes within timeout when probes are skipped (circuit breaker open) | No "reconcile_tick exceeded timeout" in logs after fix |
| AC7 | Dispatch queue still drained for non-Push tasks (ReviewPr, AutoMerge, etc.) | Other task types still processed in Step 17 |

## 3. High-Level Design

### 3.1 Change 1: Direct push handling (bypass dispatcher)

**Current:** `handle_webhook_dispatch()` at `lib.rs:3714` → enqueues `DispatchTask::Push` → reconciled later in Step 17

**New:** Match arm for `WebhookDispatch::Push` calls `self.handle_push(task).await` directly instead of enqueuing.

**Why this is safe:**
- `handle_push()` already takes `&mut self` (same as `handle_webhook_dispatch`)
- `handle_push()` already checks if build-runner is active (line 2985) — prevents double-spawn
- Other task types (ReviewPr, AutoMerge, PostMergeTestGate) still use dispatcher — no change to PR workflow

### 3.2 Change 2: Set probed_at at start (break circuit breaker deadlock)

**Current:** `probe_all()` at `provider_probe.rs:200` sets `self.probed_at = Some(Instant::now())` ONLY after ALL probes complete

**New:** Move `self.probed_at = Some(Instant::now())` to line 98 (start of `probe_all()`)

**Effect:** After a single probe attempt (even if cancelled), `is_stale()` returns false for TTL duration. Probes only re-run every TTL seconds, not every tick. The reconciler gets TTL seconds of clean ticks to drain dispatcher.

**Additional:** Also update circuit breaker state for each probe result AS THEY ARRIVE, not only after all complete. This means even if `probe_all()` is cancelled partway through, the breakers for completed probes are already updated.

### 3.3 Change 3: Process group kill

**Current:** `probe_single()` at `provider_probe.rs:638`: `kill -9 <pid>` — kills bash, orphans opencode

**New:** `kill -9 -<pgid>` — kills entire process group including opencode and its children

**Mechanism:** After spawning the child, call `nix::unistd::getpgid(pid)` to get the process group ID. On timeout, use negative PID to signal the entire group. The `nix` crate is already in the dependency tree.

### 3.4 Change 4: Runtime orphan reaping

**Current:** `adf-cleanup.sh` runs at startup only

**New:** Add a step 0.5 to `reconcile_tick()` that runs `pkill -9 -f '\.opencode run'` every N ticks (e.g., every 12 ticks = ~1 minute at 5 s interval)

**Alternative:** Use `tokio::process::Command::new("pkill")` to run the cleanup from Rust.

## 4. File/Module Change Plan

| File | Action | What Changes | Risk |
|------|--------|-------------|------|
| `lib.rs:3714-3738` | Modify | `WebhookDispatch::Push` arm: call `handle_push(task)` instead of `dispatcher.enqueue()` | Low — push events no longer go through dispatcher |
| `provider_probe.rs:98` | Modify | Add `self.probed_at = Some(Instant::now())` at start of `probe_all()` | Low — single line |
| `provider_probe.rs:156-188` | Modify | Update circuit breakers inside the `for task in tasks` loop as results arrive, not after | Medium — changes control flow |
| `provider_probe.rs:637-641` | Modify | Replace `kill -9 <pid>` with `kill -9 -<pgid>` | Low — requires `nix` crate |
| `lib.rs:5621` | Add | New step 0.5: `pkill -9 -f '\.opencode run'` every 12 ticks | Low — new step in reconcile_tick |

## 5. Step-by-Step Implementation Sequence

### Step 1: Direct push handling (highest value, lowest risk)
**File:** `lib.rs` lines 3714-3738

Change the `WebhookDispatch::Push` match arm from:
```rust
self.dispatcher.enqueue(dispatcher::DispatchTask::Push { ... });
```
to:
```rust
let task = dispatcher::DispatchTask::Push { ... };
if let Err(e) = self.handle_push(task).await {
    warn!(error = %e, "handle_push failed in webhook handler");
}
```

**Verification:** Push to Gitea, check if build-runner spawns within seconds.

### Step 2: Set probed_at at start
**File:** `provider_probe.rs` line 98

Add after the function signature: `self.probed_at = Some(Instant::now());`

Also move the circuit breaker update loop (`for result in &results`) to inside the `for task in tasks` loop, updating each breaker as results arrive.

### Step 3: Process group kill
**File:** `provider_probe.rs` lines 634-642

1. After `let child = match tokio::process::Command::new("bash")...`, capture pgid:
   ```rust
   let pgid = child.id().and_then(|pid| {
       nix::unistd::getpgid(Some(nix::unistd::Pid::from_raw(pid as i32))).ok()
   });
   ```
2. On timeout, use: `kill -9 -<pgid>`

### Step 4: Runtime orphan reaping
**File:** `lib.rs` around line 5621

Add as Step 0.5 (before rate limit cleaning):
```rust
if self.tick_count % 12 == 0 {
    let _ = std::process::Command::new("pkill")
        .arg("-9")
        .arg("-f")
        .arg(".opencode run")
        .spawn();
}
```

## 6. Testing Strategy

| Criterion | Test Method | Location |
|-----------|------------|----------|
| AC1: Direct push handling | Manual: push to Gitea, check build-runner log | Bigbox |
| AC2: probed_at at start | Unit: call probe_all(), cancel, check is_stale() | New test in provider_probe.rs |
| AC3: Process group kill | Unit: spawn shell that spawns grandchild, kill pgid, verify grandchild dead | New test in provider_probe.rs |
| AC5: Circuit breaker opens | Integration: let ADF run 5+ ticks with timeout providers, verify breaker state | Bigbox |
| AC6: reconciler completes | System: monitor logs for absence of "exceeded timeout" | Bigbox |
| AC7: Other tasks still dispatched | Existing: PR review and auto-merge tests still pass | Existing test suite |

## 7. Risk Review

| Risk | Mitigation | Residual |
|------|------------|----------|
| Push events double-dispatched (webhook + dispatcher) | Change 1 removes `dispatcher.enqueue()` for Push — no double-enqueue | Low |
| probe_all() with probed_at at start prevents re-probing when provider recovers | TTL ensures re-probing after TTL expires — normal behaviour | None |
| kill -9 -<pgid> may not work on all systems | `nix::unistd::getpgid` returns EPERM on some kernels — fall back to kill -9 <pid> if pgid unavailable | Low |
| Runtime pkill may match legitimate opencode processes | `-f '\.opencode run'` pattern is specific to probe commands | Low |

## 8. Questions for Reviewer

1. **Should `handle_push()` call from webhook handler be conditional on `reconcile_tick` completion?** Currently proposed: always call directly. Alternative: only call directly if `provider_health.is_stale()` is true (indicating probes are blocking ticks).

2. **Should the TTL for provider probes be increased from current value to reduce probe frequency?** Currently probes run every `self.ttl` (configurable in config). A longer TTL means less frequent probe re-attempts.

3. **Should the `DispatchTask::Push` variant remain in the enum, or be removed?** Since push events are now handled directly, the variant is dead code. Should we clean it up?
