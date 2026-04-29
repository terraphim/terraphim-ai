# Implementation Plan: ADF Fallback/Quota Bug (v2)

**Status**: Draft
**Research Doc**: `.docs/research-adf-fallback-quota-v2.md`
**Author**: CTO Executive (design synthesis)
**Date**: 2026-04-29
**Estimated Effort**: 4-6 hours (implementation + testing)

## Overview

### Summary

Fix the ADF orchestrator's quota-to-fallback chain so that subscription-limit exits (1) record the provider health failure even when `def.provider` is `None`, (2) parse the rate-limit reset window from the error message, (3) suppress routing to that provider until the reset window expires, and (4) respawn on the next healthy KG fallback route.

### Approach

Minimal changes to `poll_agent_exits()` in `lib.rs`, plus a new `ProviderRateLimitWindow` struct and a `parse_reset_time()` helper. Reuse existing `provider_key_for_model()` to derive the provider from the routed model. Use the existing tumbling-window `ProviderBudgetTracker.force_exhaust()` for budget-based suppression, and add a per-provider `blocked_until` timestamp map for precise reset-time suppression.

### Scope

**In Scope:**
- Fix duplicate `modelerror` PatternDef in `agent_run_record.rs`
- Fix misclassified quota patterns (should be `RateLimit` not `ModelError`)
- Derive effective provider from routed model when `def.provider` is `None`
- Record provider health failure + force budget exhaustion on quota exit
- Parse rate-limit reset time from error text (e.g. "resets 2am Europe/Berlin")
- Add per-provider `blocked_until` map to suppress routing during rate-limit window
- Respawn on next healthy KG fallback route after quota exit
- New integration test that passes

**Out of Scope:**
- `error_signatures` config for anthropic (G3) -- deferred; `detect_quota_error()` catches the text
- Reconciliation ordering verification (G4) -- already correct
- KG fallback design documentation (G5) -- separate PR
- Pattern refinement for `resets at`/`resets in` false positive risk -- acceptable

**Avoid At All Cost:**
- Modifying `KgRouter` or routing engine internals
- Adding new external crate dependencies
- Modifying `SpawnRequest` or the spawner
- Changes to the `FlowExecutor`

## Architecture

### Component Diagram

```
poll_agent_exits()
    |
    v
detect_quota_error(stdout, stderr) -> Option<String>
    |
    v
parse_reset_time(quota_line) -> Option<DateTime<Utc>>
    |
    v
provider_key_for_model(routed_model) -> Option<&str>
    |
    v
provider_health.record_failure(provider_key)
provider_budget.force_exhaust(provider_key)
provider_rate_limits.block_until(provider_key, reset_time)
    |
    v
kg_router.route_agent(task) -> KgRouteDecision
    |
    v
first_healthy_route(unhealthy ++ rate_limited_providers) -> Option<&RouteDirective>
    |
    v
spawn_agent(fallback_def)  [unique name: "{name}-retry-{N}"]
```

### Data Flow (Fixed)

```
Agent exits with "You've hit your limit - resets 2am Europe/Berlin"
    |
    v
poll_agent_exits()
    |-- drain stdout/stderr
    |-- detect_quota_error() -> Some("You've hit your limit - resets 2am Europe/Berlin")
    |-- parse_reset_time() -> Some(2026-04-30T00:00:00Z) [2am Berlin = 00:00 UTC]
    |-- provider_key_for_model("sonnet") -> Some("claude-code")
    |-- provider_health.record_failure("claude-code")
    |-- provider_budget.force_exhaust("claude-code")
    |-- provider_rate_limits.block_until("claude-code", 2026-04-30T00:00:00Z)
    |-- record AgentRunRecord (exit_class = RateLimit)
    |
    v  (after removing from active_agents)
    |-- if is_quota_exit:
    |   |-- build local_unhealthy = unhealthy_providers() ++ rate_limited_providers()
    |   |-- kg_router.route_agent(task) -> KgRouteDecision
    |   |-- first_healthy_route(local_unhealthy) -> Some(RouteDirective { provider: "kimi", ... })
    |   |-- spawn fallback_def with name "{name}-retry-1"
    |   |-- else: no healthy route -> handle_agent_exit() (agent exits permanently)
    |
    v
next reconcile() cycle:
    |-- provider_rate_limits.clean_expired()  [remove entries past their reset time]
    |-- check_schedules() uses updated unhealthy ++ rate_limited set
    |-- "claude-code" is excluded until 2026-04-30T00:00:00Z
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Per-provider `blocked_until` map instead of modifying `ProviderHealthMap` | Minimal change; avoids touching probe infrastructure; rate-limit window is a different signal than health | Modifying `unhealthy_providers()` to also check circuit breakers |
| Reuse `provider_key_for_model()` | Already exists; handles bare names and provider-prefixed models; single source of truth | Writing new `provider_key_from_routed_model()` |
| Parse reset time from error text | Provider tells us exactly when the window expires; avoids probing during rate-limit | Fixed 1-hour backoff (wasteful if window is 30min, too short if window is 24h) |
| Local unhealthy set for fallback route selection | Combines health data + rate-limit data without global state mutation | Only using `unhealthy_providers()` (misses rate-limited providers) |
| Unique fallback name `"{name}-retry-{N}"` | Avoids key collision in `active_agents`; distinguishable in logs | Same name as primary (causes confusion) |

### Simplicity Check

**What if this could be easy?**

The simplest version: on quota exit, skip the failed provider and respawn on the next KG route. No parsing, no windows. But this risks probing the rate-limited provider during the window, wasting API calls and time.

**What we add beyond the simplest version**: reset-time parsing + `blocked_until` map. This is ~40 lines of new code but prevents wasted probes and ensures we don't retry the same provider before its window resets.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? No -- parsing the reset time from the provider's own error message is standard practice (HTTP `Retry-After` header equivalent). The `blocked_until` map is a simple HashMap with Instant keys.

## File Changes

### New Code (within existing files)

| File | Changes | Lines |
|------|---------|-------|
| `lib.rs` | `ProviderRateLimitWindow` struct, `parse_reset_time()` helper, modified `poll_agent_exits()`, new integration test | ~120 |
| `agent_run_record.rs` | Remove duplicate `modelerror` block, move quota patterns from `modelerror` to `ratelimit` | ~10 changed |
| `output_parser.rs` | Keep v1 additions (12 quota patterns + 7 tests) | No change |
| `telemetry.rs` | Keep v1 additions (12 patterns + 1 test) | No change |

### Modified Files Detail

| File | Changes |
|------|---------|
| `lib.rs` | Add `provider_rate_limits: ProviderRateLimitWindow` field to `AgentOrchestrator` |
| `lib.rs` | Modify `poll_agent_exits()`: derive provider, record health, parse reset, block provider, respawn with KG fallback |
| `lib.rs` | Modify `reconcile()` / main loop: call `provider_rate_limits.clean_expired()` |
| `lib.rs` | Add `parse_reset_time(quota_line: &str) -> Option<DateTime<Utc>>` helper |
| `lib.rs` | Add `ProviderRateLimitWindow` struct with `block_until()`, `is_blocked()`, `blocked_providers()`, `clean_expired()` |
| `agent_run_record.rs` | Remove duplicate `modelerror` PatternDef (lines 310-318); move "out of quota", "quota exhausted", "subscription quota", "insufficient balance" from `modelerror` to `ratelimit` patterns |
| `agent_run_record.rs` | Fix `classify_quota_out_of_quota` test: assert `RateLimit` not `ModelError` |

## API Design

### New Types

```rust
/// Tracks per-provider rate-limit windows parsed from provider error messages.
///
/// When a provider returns "resets 2am Europe/Berlin", we store the parsed
/// UTC timestamp. The routing layer excludes blocked providers from candidate
/// selection, preventing wasted probes during the rate-limit window.
struct ProviderRateLimitWindow {
    /// Provider id -> UTC instant when the rate-limit window expires.
    blocked_until: HashMap<String, Instant>,
}

impl ProviderRateLimitWindow {
    fn block_until(&mut self, provider: &str, until: Instant);
    fn is_blocked(&self, provider: &str) -> bool;
    fn blocked_providers(&self) -> Vec<String>;
    fn clean_expired(&mut self);
}
```

### New Helper Function

```rust
/// Parse a rate-limit reset time from a provider error message.
///
/// Handles patterns like:
/// - "resets 2am Europe/Berlin" -> next 02:00 in that timezone, converted to UTC
/// - "resets at 14:00 UTC" -> today's 14:00 UTC (or tomorrow if already past)
/// - "resets in 1 hour" -> now + 1 hour
/// - "resets in 30 minutes" -> now + 30 minutes
///
/// Returns None if no parseable reset time is found.
fn parse_reset_time(quota_line: &str) -> Option<Instant>;
```

### New Field on AgentOrchestrator

```rust
pub struct AgentOrchestrator {
    // ... existing fields ...
    provider_rate_limits: ProviderRateLimitWindow,
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `parse_reset_time_utc_format` | `lib.rs` | "resets at 14:00 UTC" parses correctly |
| `parse_reset_time_timezone` | `lib.rs` | "resets 2am Europe/Berlin" converts to UTC correctly |
| `parse_reset_time_relative` | `lib.rs` | "resets in 1 hour" / "resets in 30 minutes" |
| `parse_reset_time_none` | `lib.rs` | Non-matching text returns None |
| `rate_limit_window_block_and_expire` | `lib.rs` | Provider blocked during window, unblocked after |
| `rate_limit_window_clean_expired` | `lib.rs` | Expired entries removed by clean_expired() |
| `classify_quota_out_of_quota` | `agent_run_record.rs` | Fixed: assert RateLimit not ModelError |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_quota_exit_triggers_kg_fallback` | `lib.rs` | Quota exit spawns fallback on next healthy KG route |
| `test_quota_exit_no_healthy_route` | `lib.rs` | All providers unhealthy/rate-limited -> agent exits normally |
| `test_quota_blocked_provider_not_selected` | `lib.rs` | Rate-limited provider excluded from route selection |

## Implementation Steps

### Step 1: Revert v1 diff, re-apply correct patterns

**Files:** `agent_run_record.rs`, `output_parser.rs`, `telemetry.rs`
**Description:** Revert all 4 files to clean state. Re-apply the 12 quota patterns to `output_parser.rs` and `telemetry.rs` (these are correct). Fix `agent_run_record.rs`: remove the duplicate `modelerror` block entirely; move "out of quota", "quota exhausted", "subscription quota", "insufficient balance" from `modelerror` to `ratelimit` patterns. Fix `classify_quota_out_of_quota` test to assert `RateLimit`.
**Tests:** `cargo test -p terraphim_orchestrator --lib -- output_parser telemetry agent_run_record`
**Estimated:** 30 min

### Step 2: Add ProviderRateLimitWindow + parse_reset_time

**Files:** `lib.rs`
**Description:** Add `ProviderRateLimitWindow` struct with `block_until`, `is_blocked`, `blocked_providers`, `clean_expired` methods. Add `parse_reset_time()` helper that handles "resets at HH:MM TZ", "resets in N hour(s)/minute(s)", and "resets Nam timezone" patterns. Add field to `AgentOrchestrator::new()`.
**Tests:** Unit tests for `parse_reset_time` variants, `rate_limit_window_block_and_expire`, `rate_limit_window_clean_expired`.
**Estimated:** 1.5 hours

### Step 3: Fix poll_agent_exits -- derive provider, record health, parse reset

**Files:** `lib.rs`
**Description:** In the output drain loop of `poll_agent_exits()`:
1. After `detect_quota_error()` returns `Some(quota_line)`, call `parse_reset_time(&quota_line)`.
2. Derive effective provider using `provider_budget::provider_key_for_model(routed_model)`.
3. Call `provider_health.record_failure(provider_key)` (even when `def.provider` is `None`).
4. Call `provider_budget.force_exhaust(provider_key)` if tracker available.
5. Call `provider_rate_limits.block_until(provider_key, reset_time)` if reset time parsed.
6. Keep existing exit class override to `RateLimit`.
7. Add `provider_rate_limits.clean_expired()` call at the start of `reconcile()`.
**Tests:** `cargo test -p terraphim_orchestrator --lib -- quota`
**Dependencies:** Step 1, Step 2
**Estimated:** 1 hour

### Step 4: Add KG router fallback respawn in poll_agent_exits

**Files:** `lib.rs`
**Description:** After removing the quota-exited agent from `active_agents`:
1. Build `local_unhealthy` set: `provider_health.unhealthy_providers()` ++ `provider_rate_limits.blocked_providers()`.
2. If `kg_router` is available, call `route_agent(&def.task)` to get the `KgRouteDecision`.
3. Call `first_healthy_route(&local_unhealthy)` to find next healthy route.
4. If found: clone `def`, set model + cli_tool from the healthy route, name = `"{name}-retry-{retry_count}"`, clear fallback fields, spawn.
5. If no healthy route: fall through to `handle_agent_exit()`.
6. Track retry count in a `HashMap<String, u32>` to prevent infinite loops (max 3 retries).
**Tests:** New integration test `test_quota_exit_triggers_kg_fallback`.
**Dependencies:** Step 3
**Estimated:** 1.5 hours

### Step 5: Integration test and full suite

**Files:** `lib.rs`
**Description:** Write integration test that:
1. Creates a primary agent that exits with quota text and a reset time.
2. Verifies the provider is recorded as unhealthy.
3. Verifies the provider is blocked until the reset time.
4. Verifies a fallback agent with unique name is spawned.
5. Verifies the fallback agent uses a different model/provider.
6. Run full targeted test suite: `cargo test -p terraphim_orchestrator --lib -- quota rate_limit fallback`.
**Tests:** All tests passing.
**Dependencies:** Step 4
**Estimated:** 1 hour

## Rollback Plan

Each step is independently revertible:
1. Steps 1-3 are additive (new struct + helper + modified exit handling)
2. Step 4 adds the respawn block -- can be commented out without breaking exit handling
3. If integration tests fail, the existing behaviour (agent exits, no fallback) is preserved

No feature flag needed -- the changes are all within `poll_agent_exits()` which is already the exit handling path.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| `parse_reset_time` | < 1us | Regex on short string |
| `blocked_providers()` | < 1us | HashMap iteration (< 10 entries) |
| `clean_expired()` | < 10us | HashMap drain_filter (< 10 entries) |
| Fallback spawn latency | < 5s from exit to spawn | End-to-end integration test |

No benchmarks needed -- all operations are O(n) where n < 10 providers.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify `provider_key_for_model("sonnet")` returns `"claude-code"` | Pending | Implementer |
| Verify `parse_reset_time` handles "resets 2am Europe/Berlin" correctly (CEST vs CET) | Pending | Implementer |
| Check if `KgRouter` requires `use_routing_engine` config flag to be active | Pending | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
