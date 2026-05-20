# Research Document: Eliminate Wasteful Provider Probe

**Status**: Approved (via user directive)
**Author**: Agent
**Date**: 2026-05-20
**Reviewers**: User

---

## Executive Summary

Our current `provider_probe.rs` spawns OS processes and makes real API calls every 30 minutes to check provider health. This is wasteful. We already have `TelemetryStore` that records real observed latency, cost, and success rate from every actual completion. Others (OpenRouter, LMSYS, Artificial Analysis) do not probe continuously - they observe real traffic or run scheduled benchmarks. We should eliminate the active probe, derive health from telemetry, add a CLI "weather report" for model selection, and ensure fallback during spawn.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Eliminating waste and improving reliability energises any engineer |
| Leverages strengths? | Yes | We have rich telemetry already; this uses what we built |
| Meets real need? | Yes | User explicitly asked: probe is wasteful, agents need model selection, cold start needs key validation, spawn needs fallback |

**Proceed**: Yes - 3/3

---

## Problem Statement

### Description
`provider_probe.rs` executes `action::` templates (via bash) for every provider+model combination found in KG routing rules. It runs `echo hello` as a test prompt. This:
- Spawns processes that leak if timed out (observed: 1,244 leaked tasks in systemd)
- Consumes real API tokens/credits even for health checks
- Takes up to 120s per provider (slow startup)
- Duplicates information already captured by `TelemetryStore` from real traffic

### Impact
- Startup is slow and expensive
- Tokens are burned on health checks instead of real work
- Process leaks accumulate over time
- TelemetryStore already has strictly better data (real observed latency, not synthetic)

### Success Criteria
- No process spawning for health checks
- Model health derived from real telemetry or lightweight key validation
- Agents can query a "weather report" to select healthy models by tier
- If a model hits rate limit during spawn, work continues via fallback

---

## Current State Analysis

### Existing Implementation

| Component | Location | Purpose |
|-----------|----------|---------|
| ProviderHealthMap | `provider_probe.rs:39` | Cached probe results with TTL, circuit breakers |
| ProbeResult | `provider_probe.rs:18` | Result of a single probe attempt |
| probe_all | `provider_probe.rs:98` | Spawns tasks for every provider+model combo |
| probe_single | `provider_probe.rs:465` | Executes bash command with 120s timeout |
| TelemetryStore | `control_plane/telemetry.rs:279` | In-memory store of real completion events |
| CompletionEvent | `control_plane/telemetry.rs:26` | Real observed latency, cost, success per completion |
| ModelPerformanceSnapshot | `control_plane/telemetry.rs:57` | Aggregated success_rate, avg_latency, avg_cost |
| RoutingDecisionEngine | `control_plane/routing.rs:128` | Uses telemetry to select Fastest/Cheapest/FreeThenCheapest |
| CircuitBreaker | `terraphim_spawner::health` | Already updated by real spawn failures |

### Data Flow (Current - Wasteful)
```
Startup -> probe_all() -> spawn bash per provider -> API call "echo hello"
                                    |
                                    v
                           ProbeResult -> CircuitBreaker -> save JSON
                                    |
                                    v
                              Quickwit sink
```

### Data Flow (Proposed - Telemetry-Driven)
```
Real Spawn -> CompletionEvent -> TelemetryStore -> ModelPerformanceSnapshot
                                                       |
                                                       v
                                              WeatherReport (CLI)
                                                       |
                                                       v
                                              RoutingDecisionEngine
```

---

## Constraints

### Technical Constraints
- TelemetryStore persists across restarts via `TelemetrySummary`
- Circuit breakers already exist in `terraphim_spawner::health`
- CLI tools must be on PATH (existing `cli_tool_on_path` check)
- Key validation must not consume API tokens

### Business Constraints
- Must not break existing routing behaviour
- Must support tiered model selection (free vs paid)
- Must handle subscription limit errors gracefully

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Startup time | < 5s | 30-120s (probe delay) |
| Health check cost | Zero tokens | ~1-5 tokens per provider per probe |
| Process leaks | Zero | ~1,244 leaked tasks observed |

---

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Zero token cost for health checks | Health checks should not burn budget | User directive |
| Fallback on rate limit during spawn | Work must not stop | User directive |
| Telemetry as primary health signal | We already have better data than probes | Code analysis |

### Eliminated from Scope
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Scheduled benchmark suite | Not vital - telemetry gives real data |
| HTTP endpoint health checks | Most providers don't expose simple health endpoints |
| Model capability benchmarking | Out of scope - we need availability, not capability |
| Web dashboard | User asked CLI first |

---

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| TelemetryStore | Primary health data source | Low - already tested |
| CircuitBreaker | Failure tracking from real traffic | Low - already tested |
| RoutingDecisionEngine | Consumes telemetry for routing | Low - already tested |
| KgRouter | Provides model list for weather report | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| chrono | ^0.4 | Low | time crate |
| serde | ^1.0 | Low | - |

---

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cold start with no telemetry | High (first run) | Medium | Key validity check + assume healthy |
| Telemetry stale after long idle | Medium | Low | Staleness check + re-validate keys |
| Agent picks unhealthy model | Low | Medium | Fallback retry in spawn pipeline |

### Open Questions (Resolved by User)
1. ~Remove active probe?~ **Yes**
2. ~CLI dashboard?~ **Yes, agent-first**
3. ~Cold start handling?~ **Key validation + fallback on rate limit**

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| TelemetryStore has sufficient data after first hour | Real traffic patterns | Cold-start models appear healthy | Yes - acceptable |
| CLI tool on PATH implies provider is configured | Existing check logic | Missing env var causes false negative | Partial - will check |
| Rate limit errors are detectable from spawn output | Existing error classification | Some providers return weird errors | Yes - tested |

---

## Research Findings

### Key Insights
1. **Nobody probes continuously.** OpenRouter uses real usage telemetry. Artificial Analysis runs periodic benchmarks. LMSYS uses crowdsourced votes.
2. **Our TelemetryStore is better than a probe.** It records actual latency, cost, success rate from real work - not synthetic "echo hello" tests.
3. **The probe actively harms.** It burns tokens, spawns processes that leak, and delays startup by up to 120s per provider.

### Relevant Prior Art
- **OpenRouter Rankings**: Aggregates real API usage from millions of calls. No probing.
- **Artificial Analysis**: Independent scheduled benchmarks (not continuous).
- **LMSYS Chatbot Arena**: Human preference crowdsourcing. No probing.
- **Vellum Leaderboard**: Static benchmark tables. No live probing.

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Key validation without API call | Verify API key format without spending tokens | 1 hour |
| Fallback retry in spawn pipeline | Ensure spawn continues if first model rate-limited | 2 hours |

---

## Recommendations

### Proceed/No-Proceed
**Proceed.** The user has explicitly directed removal of the probe, addition of CLI weather report, and fallback handling.

### Scope Recommendations
- Remove `probe_all()` and `probe_single()` from `provider_probe.rs`
- Keep `ProviderHealthMap` but drive it from telemetry instead of probes
- Add `ModelWeatherReport` CLI command
- Add key validity check (format only, no API call)
- Add fallback retry in spawn pipeline on rate limit

### Risk Mitigation Recommendations
- Keep `probe_results_dir` config for backward compatibility (ignore it)
- Keep `probe_on_startup` config but make it a no-op (log deprecation)
- Ensure `TelemetryStore` restores from persistence on startup

---

## Next Steps

1. Create design document (Phase 2)
2. Get user approval on design
3. Implement (Phase 3)

---

## Appendix

### Reference Materials
- `provider_probe.rs` - current wasteful probe implementation
- `control_plane/telemetry.rs` - telemetry store with real data
- `control_plane/routing.rs` - routing engine that consumes telemetry

### Code Snippets

Current wasteful probe loop (to remove):
```rust
// provider_probe.rs:98-201
pub async fn probe_all(&mut self, kg_router: &KgRouter) {
    for rule in kg_router.all_routes() {
        tasks.push(tokio::spawn(async move {
            probe_single(&provider, &model, action.as_deref()).await
        }));
    }
    // ... wait for all, update circuit breakers
}
```

Telemetry we already have (to use instead):
```rust
// control_plane/telemetry.rs:57-83
pub struct ModelPerformanceSnapshot {
    pub model: String,
    pub successful_completions: u64,
    pub failed_completions: u64,
    pub throughput: f64,
    pub avg_latency_ms: f64,
    pub success_rate: f64,
    pub subscription_limit_reached: bool,
    pub avg_cost_per_1k_tokens: f64,
    pub is_free: bool,
}
```
