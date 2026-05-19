# Research Document: ADF Reconciler Tick Timeout and Process Leak

**Date:** 2026-05-19
**Status:** Phase 1 research (thorough code review)

## 1. Problem Restatement and Scope

### Problem

The ADF orchestrator logs `reconcile_tick exceeded timeout, forcing continuation` every 90 s. Build-runner never spawns automatically on push events. Sixty-plus orphaned `opencode` processes accumulate during runtime.

### IN scope

- Why reconcile_tick always times out
- Why build-runner never dispatches
- Why opencode processes leak
- The circuit breaker deadlock

### OUT of scope

- Changing tick interval configuration
- Removing providers from KG routing rules
- General memory optimisation

## 2. System Elements — Trace of a Push Event

Read from source at `crates/terraphim_orchestrator/src/`:

### 2.1 Push webhook arrival

1. **Gitea push** → webhook handler at `webhook.rs:498` (`handle_push_event`)
2. Parses payload, creates `WebhookDispatch::Push`, sends via `dispatch_tx` (mpsc channel)
3. Arrives in event loop at `lib.rs:1280` — tokio task forwards to main mpsc `loop_tx`
4. Main loop `lib.rs:1301` receives `LoopEvent::Webhook(dispatch)`
5. Calls `handle_webhook_dispatch()` at `lib.rs:3432`
6. Match arm at `lib.rs:3714` — **enqueues `DispatchTask::Push` into `self.dispatcher`** — does NOT spawn build-runner here
7. Sends `LoopEvent::Tick` to trigger reconciliation

### 2.2 Tick event — where dispatch SHOULD happen

8. Main loop receives `LoopEvent::Tick` at `lib.rs:1314`
9. Drains queued events (webhooks, schedules, alerts), coalescing stale ticks
10. Calls `reconcile_tick()` wrapped in `tokio::time::timeout(90s)` at `lib.rs:1336`
11. `reconcile_tick()` has 18 numbered steps at `lib.rs:5618`

### 2.3 reconcile_tick step ordering is the problem

| Step | Line | What | Duration |
|------|------|------|----------|
| 0 | 5621 | Clean rate limits, poll timeouts | Fast |
| 1 | 5629 | Poll agent exits | Fast |
| 2 | 5632 | Restart safety agents | Fast |
| 3 | 5635 | Check cron schedules | Fast |
| 4 | 5638 | Drain output events | Fast |
| 5-12 | 5644-5717 | Nightwatch, handoff, budget, flows, mentions, KG reload | Fast |
| **13** | **5719** | **Provider probes** — `probe_all(kg_router).await` | **Blocks > 90 s** |
| 14 | 5748 | Update `last_tick_time` | Fast |
| 15-16 | 5752-5764 | Telemetry, budget persist | Fast |
| **17** | **5795** | **Dispatch queue drain** — `handle_push()` for each Push task | **Never reached** |
| 18 | 5859 | Poll pending reviews | Never reached |

The tick is always cancelled at Step 13, so Steps 14-18 never execute.

### 2.4 Why probes take > 90 s

`probe_all()` at `provider_probe.rs:98`:

1. Iterates KG routing rules, collects unique `(provider, model, action)` triples
2. For each, spawns `tokio::spawn(async { probe_single(...) })`
3. Awaits all results sequentially

`probe_single()` at `provider_probe.rs:465`:

1. Spawns `bash -c "opencode run -m <model> --format json echo hello"`
2. Waits up to 120 s for completion (`Duration::from_secs(120)` at line 542)
3. On timeout: `kill -9 <pid>` (line 638)
4. Returns `ProbeResult { status: Timeout }`

**Two connected problems:**

**Problem A — Timing mismatch:** The probe's internal 120 s timeout exceeds reconcile_tick's 90 s timeout. Even a single slow probe causes the entire tick to timeout.

**Problem B — Circuit breaker deadlock:** `probed_at` and circuit breaker state are only updated at lines 199-200 of `probe_all()`, AFTER all probes complete. When `probe_all()` is cancelled by the reconcile_tick timeout:

- `probed_at` is never set → `is_stale()` always returns `true` → probes re-run on every tick
- Circuit breakers are never updated → providers never transition to `Open` state → probes always include timeout providers
- Each tick spawns a new set of probe tasks → tasks accumulate

### 2.5 Process leak mechanism

When `reconcile_tick()` is cancelled at 90 s, the `probe_all().await` is dropped. But `tokio::spawn` tasks are NOT cancelled — they continue running independently.

Each probe task:
1. `bash -c "opencode run ..."` is spawned
2. The 120 s internal timeout fires
3. `kill -9 <pid>` kills bash
4. opencode (child of bash) becomes orphaned (PPID=1)
5. orphan spawns child processes (`gtr`, `cached-context`, `sentrux`)
6. All continue running indefinitely

Over 50 minutes (~33 tick cycles × 3 timeout providers ≈ 99 leaked opencode instances), 60+ orphans observed.

### 2.6 Why cleanup is insufficient

`adf-cleanup.sh` already has `pkill -9 -f '\.opencode run'` — but it only runs at startup. Orphans that accumulate during runtime are never cleaned up.

## 3. Constraints and Their Implications

### C1: Probe timeout (120 s) exceeds reconcile timeout (90 s)
- **Effect:** Multiple probes running in `tokio::spawn` will always exceed 90 s if any provider is slow/unavailable
- **Implication:** reconcile_tick is ALWAYS cancelled; dispatch never happens

### C2: Circuit breaker never opens for timeout providers
- **Effect:** `probed_at` and breaker state only updated after `probe_all()` completes — which never happens
- **Implication:** `is_stale()` always true → probes always run with full set of providers → self-perpetuating

### C3: tokio::spawn handles not cancelled on parent drop
- **Effect:** Probe tasks continue running after reconcile_tick is cancelled
- **Implication:** Accumulating orphans drive memory/swapping, further slowing the reconciler (death spiral)

### C4: Dispatch drain positioned after probes
- **Effect:** Push events are enqueued but never dequeued because Step 17 is never reached
- **Implication:** Build-runner never spawns automatically

## 4. Risks

| Risk | Severity | Cause |
|------|----------|-------|
| OOM killer terminates critical processes | Critical | 60+ orphaned opencode consuming 20+ GB |
| Push events silently dropped | High | Dispatcher queue never drained |
| System becomes unresponsive | Medium | Load 23+, swap 3.9/4 GB exhausted |

## 5. Simplification Opportunities

1. **Handle push events directly** — Don't use the dispatch queue for push events. Call `handle_push()` directly from `handle_webhook_dispatch()`. This completely decouples build-runner dispatch from probe health.

2. **Set probed_at immediately** — Move `self.probed_at = Some(Instant::now())` to the START of `probe_all()`. This prevents `is_stale()` from returning true for the TTL duration, giving the reconciler breathing room.

3. **Kill process groups** — Use `kill -9 -<pgid>` to kill opencode children, not just bash parent.

4. **Add periodic orphan reaping** — Add a cleanup step to reconcile_tick that kills opencode processes older than 2 minutes.
