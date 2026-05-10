# Design & Implementation Plan: Make ADF Provider Probe Rate-Limit Aware

## 1. Summary of Target Behavior

After implementation, the ADF provider probe will:

1. **Check rate-limit windows before probing**: Before executing a probe for any provider, `probe_all` will check whether that provider is currently blocked by `ProviderRateLimitWindow`. If blocked, the probe is skipped and recorded as `ProbeStatus::RateLimited`.

2. **Distinguish rate limits from errors**: A new `ProbeStatus::RateLimited` variant will be added to clearly distinguish "provider is temporarily blocked by rate limit" from "provider is experiencing an error".

3. **Preserve circuit breaker state for rate limits**: When a probe is skipped due to rate limits, the circuit breaker is neither incremented (failure) nor decremented (success). The existing breaker state is preserved.

4. **Report degraded health for rate-limited providers**: `model_health()` and `provider_health()` will return `HealthStatus::Degraded` (not `Unhealthy`) for providers that are currently rate-limited. This signals "temporarily unavailable, expected to recover".

5. **Exclude rate-limited providers from unhealthy list**: `unhealthy_providers()` will not include rate-limited providers, since they are not candidates for escalation or investigation.

6. **Enable faster recovery**: When a rate-limit window expires, the next probe tick will immediately probe the provider again (no circuit breaker cooldown required).

## 2. Key Invariants and Acceptance Criteria

### Invariants

| ID | Invariant |
|----|-----------|
| I-1 | A provider blocked by `ProviderRateLimitWindow` must never be probed |
| I-2 | Skipping a probe due to rate limits must not modify the circuit breaker state |
| I-3 | A rate-limited provider must report `HealthStatus::Degraded`, not `Unhealthy` |
| I-4 | `unhealthy_providers()` must not include rate-limited providers |
| I-5 | When the rate-limit window expires, the provider must be probed on the next tick (if stale) |
| I-6 | Existing `ProbeStatus::Success` and `ProbeStatus::Error` behaviour must remain unchanged |
| I-7 | `ProbeResult` JSON serialization must remain backward-compatible |

### Acceptance Criteria

| ID | Criterion | Test Type |
|----|-----------|-----------|
| AC-1 | When `ProviderRateLimitWindow::is_blocked(provider)` returns true, `probe_all` skips the probe and emits a `ProbeResult` with `status = RateLimited` | Unit |
| AC-2 | When a probe is skipped due to rate limits, the provider's circuit breaker failure count does not increase | Unit |
| AC-3 | `model_health(provider, model)` returns `Degraded` when the provider is rate-limited, even if the circuit breaker is Open | Unit |
| AC-4 | `provider_health(provider)` returns `Degraded` when the provider is rate-limited | Unit |
| AC-5 | `unhealthy_providers()` does not include a provider that is only rate-limited (no other failures) | Unit |
| AC-6 | After the rate-limit window expires, `is_stale()` returns true, and the next `probe_all` includes the previously blocked provider | Integration |
| AC-7 | `ProbeResult` JSON serialization with `RateLimited` status deserialises correctly and does not break existing `latest.json` readers | Unit |
| AC-8 | Existing tests for `ProbeStatus::Success`, `Error`, `Timeout` continue to pass | Regression |

## 3. High-Level Design and Boundaries

### Architecture

```
+------------------------------------------+
|  Orchestrator::tick() / run()            |
|  - Checks provider_health.is_stale()     |
|  - Calls provider_health.probe_all()     |
+------------------------------------------+
                    |
                    v
+------------------------------------------+
|  ProviderHealthMap::probe_all()          |
|  - NEW: Accepts rate_limit_checker       |
|  - For each provider:                    |
|    - Check if blocked by rate limits     |
|    - If blocked: skip, record RateLimited|
|    - If not blocked: run existing probe  |
+------------------------------------------+
                    |
        +-----------+-----------+
        |                       |
        v                       v
+---------------+     +------------------+
| Skip probe    |     | Execute probe    |
| - No breaker  |     | - Existing logic |
|   update      |     | - Success/Error  |
| - Record      |     |   /Timeout       |
|   RateLimited |     | - Update breaker |
+---------------+     +------------------+
```

### Component Boundaries

**Changes inside existing components:**
- `provider_probe.rs`: Add `RateLimited` variant, update `probe_all`, `model_health`, `provider_health`, `unhealthy_providers`
- `lib.rs`: Pass rate-limit checker to `probe_all` in `run()` and `tick()`

**No new components introduced.**

**Interfaces:**
- `probe_all` will accept a new parameter: `rate_limit_checker: &dyn Fn(&str) -> bool` (or a trait object)
- This is the minimal interface — a closure that answers "is this provider currently blocked?"

### Complected Areas to Avoid

1. **Do not merge `ProviderRateLimitWindow` into `ProviderHealthMap`**: Keep the time-based blocking separate from the circuit breaker. They serve different purposes.
2. **Do not change `ProviderRateLimitWindow`**: It works correctly; we only need to read from it.
3. **Do not modify agent respawn logic**: The fallback routing already handles rate-limited providers correctly.

## 4. File/Module-Level Change Plan

| File | Action | Before | After | Dependencies |
|------|--------|--------|-------|--------------|
| `provider_probe.rs` | Modify | `ProbeStatus` has Success/Error/Timeout | Add `RateLimited` variant | None |
| `provider_probe.rs` | Modify | `probe_all(&mut self, kg_router: &KgRouter)` | `probe_all(&mut self, kg_router: &KgRouter, is_blocked: &dyn Fn(&str) -> bool)` | `ProbeStatus` variant |
| `provider_probe.rs` | Modify | `probe_all` spawns all non-Open providers | `probe_all` skips rate-limited providers before spawning | `is_blocked` parameter |
| `provider_probe.rs` | Modify | Circuit breaker updated for all Error/Timeout | Skip breaker update for RateLimited probes | `ProbeStatus` match |
| `provider_probe.rs` | Modify | `model_health()` checks results then breaker | Check rate limit status first (via new field) | `ProviderHealthMap` field |
| `provider_probe.rs` | Modify | `provider_health()` checks results then breaker | Check rate limit status first | `ProviderHealthMap` field |
| `provider_probe.rs` | Modify | `unhealthy_providers()` filters by results/breakers | Also filter out rate-limited providers | `ProviderHealthMap` field |
| `provider_probe.rs` | Modify | `ProviderHealthMap` has breakers, results, probed_at, ttl, cb_config | Add `rate_limited: HashSet<String>` field (provider keys currently blocked) | None |
| `lib.rs` | Modify | `run()`: `probe_all(kg_router)` | `probe_all(kg_router, &|p| self.provider_rate_limits.is_blocked(p))` | `probe_all` signature |
| `lib.rs` | Modify | `tick()`: `probe_all(kg_router)` | `probe_all(kg_router, &|p| self.provider_rate_limits.is_blocked(p))` | `probe_all` signature |
| `provider_probe.rs` | Modify | Tests cover Success/Error/Timeout | Add tests for RateLimited status and health queries | All above changes |

## 5. Step-by-Step Implementation Sequence

### Step 1: Add `ProbeStatus::RateLimited` variant
**Purpose**: Extend the status enum to represent rate-limited probes.  
**Deployable**: Yes — adding an unused variant is a no-op.  
**Feature flag**: None needed.

1. Add `RateLimited` to `ProbeStatus` enum
2. Add `#[serde(rename_all = "snake_case")]` ensures it serialises as `"rate_limited"`
3. Verify `serde_json` serialization/deserialization in a quick test

### Step 2: Add rate-limit tracking to `ProviderHealthMap`
**Purpose**: Track which providers are currently rate-limited so health queries can report correctly.  
**Deployable**: Yes — adding a field that starts empty is a no-op.  
**Feature flag**: None needed.

1. Add `rate_limited: HashSet<String>` to `ProviderHealthMap`
2. Initialise as empty in `ProviderHealthMap::new()`
3. Add helper methods: `is_rate_limited(&self, provider: &str) -> bool`

### Step 3: Update `probe_all` signature and logic
**Purpose**: Make probe_all aware of rate limits and skip blocked providers.  
**Deployable**: Yes — behaviour change only affects callers that pass the new parameter.  
**Feature flag**: None needed.

1. Change signature to accept `is_blocked: &dyn Fn(&str) -> bool`
2. Before spawning a probe, check `is_blocked(&rule.provider)`
3. If blocked, generate a `ProbeResult` with `ProbeStatus::RateLimited` and skip the spawn
4. In the breaker update loop, match on `ProbeStatus::RateLimited` and skip the breaker update
5. Add the provider to `self.rate_limited` when a probe is skipped
6. Clear `self.rate_limited` at the start of `probe_all` (rebuild it each tick)

### Step 4: Update health query methods
**Purpose**: Report Degraded (not Unhealthy) for rate-limited providers.  
**Deployable**: Yes — changes health reporting only.  
**Feature flag**: None needed.

1. Update `model_health()` to check `self.is_rate_limited(provider)` first, return `Degraded` if true
2. Update `provider_health()` to check `self.is_rate_limited(provider)` first, return `Degraded` if true
3. Update `unhealthy_providers()` to filter out rate-limited providers

### Step 5: Update orchestrator call sites
**Purpose**: Wire the rate-limit checker into the probe invocation.  
**Deployable**: Yes — connects the new logic to the orchestrator.  
**Feature flag**: None needed.

1. In `lib.rs::run()` line 1038: pass `|p| self.provider_rate_limits.is_blocked(p)` as closure
2. In `lib.rs::tick()` line 5494: pass the same closure

### Step 6: Add tests
**Purpose**: Verify all acceptance criteria.  
**Deployable**: Yes — tests only.  
**Feature flag**: None needed.

1. Unit test: `probe_skips_rate_limited_provider`
2. Unit test: `rate_limited_does_not_open_circuit_breaker`
3. Unit test: `model_health_returns_degraded_for_rate_limited`
4. Unit test: `provider_health_returns_degraded_for_rate_limited`
5. Unit test: `unhealthy_providers_excludes_rate_limited`
6. Unit test: `probe_result_json_serializes_rate_limited`
7. Integration test: `rate_limit_expiry_triggers_reprobe`

## 6. Testing & Verification Strategy

| Criterion | Test Type | Location | Description |
|-----------|-----------|----------|-------------|
| AC-1 | Unit | `provider_probe.rs` tests | Mock `is_blocked` closure returning true; assert `ProbeResult.status == RateLimited` |
| AC-2 | Unit | `provider_probe.rs` tests | Record 4 failures, then skip probe due to rate limit, assert breaker still at 4 (not 5) |
| AC-3 | Unit | `provider_probe.rs` tests | Set `rate_limited` HashSet, call `model_health`, assert `Degraded` |
| AC-4 | Unit | `provider_probe.rs` tests | Set `rate_limited` HashSet, call `provider_health`, assert `Degraded` |
| AC-5 | Unit | `provider_probe.rs` tests | Set `rate_limited` for provider with no other failures, assert not in `unhealthy_providers()` |
| AC-6 | Integration | `tests/` or lib.rs tests | Block provider, wait for expiry (or mock time), verify next tick probes it |
| AC-7 | Unit | `provider_probe.rs` tests | Serialize `ProbeResult` with `RateLimited`, deserialize, assert round-trip correct |
| AC-8 | Regression | Existing tests | Run all existing `provider_probe` tests, assert 0 failures |

### Test Data

```rust
// Mock rate-limit checker for unit tests
let blocked_providers = std::collections::HashSet::from(["anthropic".to_string()]);
let is_blocked = |provider: &str| blocked_providers.contains(provider);

// Mock KG router with one route per provider
let kg_router = KgRouter::from_rules(vec![
    KgRoute { provider: "anthropic".to_string(), model: "claude-sonnet".to_string(), action: Some("claude -p hello".to_string()) },
    KgRoute { provider: "kimi".to_string(), model: "kimi-for-coding/k2p5".to_string(), action: Some("opencode -m kimi-for-coding/k2p5 hello".to_string()) },
]);
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|------|------------|---------------|
| JSON serialization breakage | Use `#[serde(rename_all = "snake_case")]` which automatically handles new variants; test round-trip serialization | Low — serde is backward-compatible for readers that ignore unknown fields |
| `probe_all` signature change breaks external callers | Search codebase for all `probe_all` calls (only 2 in lib.rs); no external callers expected | None |
| Health query methods called frequently; `is_rate_limited` lookup adds overhead | `HashSet` lookup is O(1) and negligible compared to existing HashMap lookups in the same methods | None |
| Rate-limited provider with Open breaker shows Degraded, but `should_allow()` returns false | This is correct behaviour: `Degraded` means "don't route new work here, but don't escalate either"; existing fallback routing already handles this | None |
| `ProviderRateLimitWindow` keys by provider, not provider:model, so all models for a provider are skipped | This matches current behaviour — `ProviderRateLimitWindow` blocks the entire provider, not individual models | None |
| Tests require mocking time for rate-limit expiry | Use `tokio::time::pause()` in async tests or mock the `is_blocked` closure directly | Low |

## 8. Open Questions / Decisions for Human Review

1. **Should `ProbeStatus::RateLimited` include an optional `resets_at: Option<String>` field?** This would make the probe result self-describing ("rate limited, resets at 14:30"). Recommendation: No — keep it simple; the orchestrator already logs the reset time when it blocks the provider.

2. **Should we add a metric counter for skipped probes?** This would help quantify how many API calls we are saving. Recommendation: Yes — add a `probes_skipped_rate_limited` counter to the existing telemetry, but this can be a follow-up issue to keep the scope minimal.

3. **Should `is_healthy()` return true for rate-limited providers?** Currently `is_healthy()` returns true for `Healthy | Degraded`. If we keep this, rate-limited providers would still be considered "healthy enough" for `should_allow()` checks. Recommendation: Yes — keep `is_healthy()` returning true for `Degraded`, since `Degraded` already means "available but not preferred". The KG router's `first_healthy_route` will skip `Degraded` providers if better options exist.

4. **Should the probe TTL be shortened when rate limits are active?** For example, if TTL is 300s but a rate limit expires in 60s, should we probe sooner? Recommendation: No — keep the default TTL. The orchestrator tick loop (every 5s) checks `is_stale()`, and when the TTL expires, the next tick will probe. Shortening TTL adds complexity with minimal benefit.
