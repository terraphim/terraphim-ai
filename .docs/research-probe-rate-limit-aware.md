# Research Document: Make ADF Provider Probe Rate-Limit Aware

## 1. Problem Restatement and Scope

### Problem
The ADF orchestrator's provider health probe (`ProviderHealthMap::probe_all`) currently probes all configured providers on startup and periodically when cached results become stale. However, it has no knowledge of rate-limit windows that the orchestrator tracks via `ProviderRateLimitWindow`. When a provider returns HTTP 429 or quota-exceeded errors, the orchestrator blocks that provider for a cooldown period (e.g., "resets in 1 hour"). Despite this block, the probe continues to send test requests to the rate-limited provider, which:

1. Wastes API calls and tokens on probes that are guaranteed to fail
2. Risks exacerbating the rate-limit situation (some providers penalise repeated 429s)
3. Pollutes probe results with false "unhealthy" signals that conflate temporary rate limits with genuine provider outages
4. Prevents accurate health assessment — a rate-limited provider is not "down", it is temporarily unavailable

### IN Scope
- Making `probe_all` aware of `ProviderRateLimitWindow` blocks
- Distinguishing `ProbeStatus::RateLimited` from `ProbeStatus::Error` in probe results
- Preventing circuit breaker updates when probes are skipped due to rate limits
- Updating the orchestrator's `run()` and `tick()` methods to pass rate-limit state to the probe
- Ensuring `model_health()` and `provider_health()` correctly report `HealthStatus::Degraded` for rate-limited providers

### OUT of Scope
- Changes to the actual rate-limit detection logic (already handled in `lib.rs` exit classification)
- Changes to `ProviderRateLimitWindow` implementation (already working)
- Changes to agent respawn/fallback logic (already working)
- Changes to circuit breaker thresholds or configuration

## 2. User & Business Outcomes

### Visible Changes
- Providers that hit rate limits will show `Degraded` health status instead of `Unhealthy`
- Probe logs will clearly indicate "skipped: rate-limited until HH:MM" instead of opaque error messages
- Circuit breakers will not trip solely due to rate-limit probes, reducing unnecessary fallback routing
- Fewer wasted API calls during rate-limit windows, preserving quota for actual work

### Business Value
- More reliable provider health assessment reduces false-positive provider failover
- Preserves API quota during rate-limit windows (probes cost tokens/calls too)
- Faster recovery when rate limits expire: probe immediately marks provider healthy again without waiting for circuit breaker cooldown

## 3. System Elements and Dependencies

| Element | Location | Role | Dependencies |
|---------|----------|------|--------------|
| `ProviderHealthMap` | `provider_probe.rs:36` | Tracks circuit breakers and probe results | `CircuitBreaker`, `KgRouter` |
| `ProviderHealthMap::probe_all` | `provider_probe.rs:88` | Executes probes for all providers | `tokio::process::Command`, `KgRouter` |
| `ProviderHealthMap::model_health` | `provider_probe.rs:181` | Returns health for specific model | Probe results, circuit breaker state |
| `ProviderHealthMap::provider_health` | `provider_probe.rs:209` | Returns aggregate health for provider | Probe results, circuit breaker state |
| `ProviderRateLimitWindow` | `lib.rs:313` | Blocks providers until reset time | `HashMap<String, Instant>` |
| `ProviderRateLimitWindow::is_blocked` | `lib.rs:329` | Checks if provider is currently blocked | `Instant::now()` |
| `ProviderRateLimitWindow::blocked_providers` | `lib.rs:335` | Returns list of blocked providers | `Instant::now()` |
| `AgentRunRecord::ExitClass::RateLimit` | `agent_run_record.rs:57` | Classification for rate-limit exits | Pattern matching on stderr/stdout |
| `parse_reset_time` | `lib.rs:350` | Parses "resets in X minutes/hours" from output | String parsing |
| `Orchestrator::run` | `lib.rs:1023` | Startup probe invocation | `provider_health.probe_all()` |
| `Orchestrator::tick` | `lib.rs:5480` | Periodic probe when stale | `provider_health.probe_all()` |
| `ProbeResult` | `provider_probe.rs:16` | Result struct for single probe | Serde serialization |
| `ProbeStatus` | `provider_probe.rs:28` | Enum: Success, Error, Timeout | Needs new variant |

### Data Flow
```
Agent exits with RateLimit
    -> lib.rs:6226: parse_reset_time from stderr
    -> lib.rs:6262: provider_rate_limits.block_until(provider, reset_time)
    -> lib.rs:6246: provider_health.record_failure(provider) [circuit breaker]
    -> lib.rs:6527: respawn via KG fallback route

[tick() every N seconds]
    -> lib.rs:5492: if provider_health.is_stale()
    -> lib.rs:5494: provider_health.probe_all(kg_router)
    -> [MISSING] provider_health.probe_all does NOT check provider_rate_limits
    -> provider_probe.rs:118: spawns probe for rate-limited provider
    -> provider_probe.rs:560: probe fails with exit error
    -> provider_probe.rs:157: breaker.record_failure() [extra failure!]
    -> probe result shows Error, not RateLimited
```

## 4. Constraints and Their Implications

| Constraint | Implication |
|------------|-------------|
| **Probe must remain async and non-blocking** | Checking rate limits is synchronous (HashMap lookup), so no async changes needed |
| **Circuit breaker must not open for rate-limited probes** | Rate limits are temporary; opening the breaker would extend downtime beyond the rate-limit window |
| **ProbeStatus is serialised to JSON** | Adding a new variant requires backward-compatible serde handling or version bump |
| **ProviderRateLimitWindow lives in lib.rs, probe in provider_probe.rs** | Need to either pass the window to probe_all or extract a shared trait/interface |
| **Tick loop runs every 5 seconds; probe TTL is typically 300s** | Probes are infrequent enough that checking rate limits has negligible overhead |
| **Must not break existing tests** | Any new ProbeStatus variant or probe_all signature change requires test updates |
| **Provider health queries must be fast (synchronous)** | model_health() and provider_health() are called frequently; they should not do async I/O |

## 5. Risks, Unknowns, and Assumptions

### Risks
| Risk | Severity | De-risking |
|------|----------|------------|
| Adding `RateLimited` to `ProbeStatus` breaks downstream JSON consumers | Medium | Use `#[serde(other)]` or explicit variant handling; check if `save_results` is consumed by external tools |
| Passing `ProviderRateLimitWindow` to `probe_all` creates coupling between lib.rs and provider_probe.rs | Low | Extract a `RateLimitCheck` trait or pass a simple `Fn(&str) -> bool` closure |
| Rate-limited provider shows `Degraded` but circuit breaker may still be `Open` from prior failures | Medium | Ensure `model_health()` prioritises rate-limit status over breaker state when both apply |
| Probe skipped for rate limit but TTL expires; next tick probes again while still rate-limited | Low | `is_blocked()` checks `Instant::now() < until`, so expired blocks are automatically ignored |

### Unknowns
1. **Does any external tool consume `probe_results_dir/latest.json`?** If so, adding a new `ProbeStatus` variant may break it.
2. **What is the typical rate-limit window duration?** Some providers say "resets in 1 hour", others "resets in 1 minute". This affects how long probes are skipped.
3. **Do providers distinguish between per-model and per-account rate limits?** The current `ProviderRateLimitWindow` blocks by provider key, not model.

### Assumptions
- **Assumption**: Rate-limited providers should show `HealthStatus::Degraded`, not `Unhealthy`. Degraded means "temporarily unavailable but expected to recover"; Unhealthy means "needs investigation".
- **Assumption**: Probes that are skipped due to rate limits should NOT update the circuit breaker at all (no success, no failure).
- **Assumption**: The `ProviderRateLimitWindow` block duration is authoritative; we trust the provider's "resets in" message.

## 6. Context Complexity vs. Simplicity Opportunities

### Sources of Complexity
1. **Two health systems**: `ProviderHealthMap` (circuit breakers + probes) and `ProviderRateLimitWindow` (time-based blocks) operate independently but affect the same providers.
2. **Multiple health query paths**: `model_health()`, `provider_health()`, `is_healthy()`, `is_model_healthy()`, `unhealthy_providers()` all need to account for rate limits.
3. **Probe result JSON serialisation**: Adding a new status variant affects the pi-benchmark compatible format.

### Simplification Opportunities
1. **Unified health query**: Create a single method that checks probe results → rate limits → circuit breakers in priority order, rather than duplicating the logic across `model_health()` and `provider_health()`.
2. **Trait-based rate limit check**: Instead of passing the entire `ProviderRateLimitWindow` to `probe_all`, pass a `&dyn RateLimitChecker` trait object or closure. This decouples the modules.
3. **Skip rather than fail**: Instead of executing the probe and getting an Error, skip the probe entirely when rate-limited. This is the simplest change and avoids conflating rate limits with genuine errors.

## 7. Questions for Human Reviewer

1. **Should rate-limited probes show `Degraded` or `Unhealthy`?** My recommendation is `Degraded` since recovery is expected. Confirm.
2. **Is `ProbeStatus::RateLimited` needed, or should we skip the probe entirely with no result?** Skipping is simpler but loses visibility. Preference?
3. **Does any external system consume `probe_results_dir/latest.json`?** Adding `rate_limited` to JSON may break consumers.
4. **Should `unhealthy_providers()` include rate-limited providers?** Currently it returns providers with all models unhealthy. Rate-limited providers are not truly "unhealthy" for fallback routing purposes.
5. **What is the desired probe TTL when rate limits are active?** Should we shorten TTL to probe sooner after rate limit expiry, or keep the default?
6. **Should we add metrics/telemetry for skipped rate-limited probes?** This would help quantify how many probes we are saving.
7. **Does the current `ProviderRateLimitWindow` handle per-model rate limits, or only per-provider?** The struct keys by provider string, not provider:model.
