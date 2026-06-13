# Implementation Plan: Telemetry-Driven Model Weather Report

**Status**: Approved (answers received)
**Research Doc**: `.docs/research-eliminate-probe.md`
**Gitea Issue**: [#1770](https://git.terraphim.cloud/terraphim/terraphim-ai/issues/1770)
**Author**: Agent
**Date**: 2026-05-20
**Estimated Effort**: 1-2 days

---

## Overview

### Summary
Remove the wasteful `probe_all()` / `probe_single()` process-spawning probe. Drive `ProviderHealthMap` from `TelemetryStore` real observed data. Add a `model-weather` CLI command that shows currently healthy models ranked by speed, cost, and tier. Add lightweight API key format validation for cold start. Ensure spawn pipeline falls back to next candidate on rate limit.

### Approach
1. **Eliminate** the active probe loop (process spawning, API calls)
2. **Reuse** TelemetryStore as primary health signal
3. **Add** ModelWeatherReport struct for agent/human consumption
4. **Add** key validation (format check, no API call)
5. **Add** fallback retry in spawn on rate limit

### Scope

**In Scope:**
- Remove `probe_all()` / `probe_single()` process spawning
- Drive `ProviderHealthMap` from telemetry data
- Add `ModelWeatherReport` with CLI output
- Add API key format validation (cold start)
- Add spawn fallback on rate limit / subscription limit
- Keep backward-compatible config (no-op for removed features)

**Out of Scope:**
- Web dashboard (CLI only, per user)
- Scheduled benchmark runs
- HTTP endpoint health checks
- Model capability evaluation
- Provider pricing API integration

**Avoid At All Cost:**
- Any feature that spawns processes for health checks
- Any feature that makes API calls to validate health
- Web UI (user explicitly said CLI first, agent first)
- Over-engineering the weather report with predictions/trends

---

## Architecture

### Component Diagram

```
+------------------+        +-------------------+
|   CLI / Agent    |        |   Spawn Pipeline  |
|  model-weather   |        |                   |
+--------+---------+        +--------+----------+
         |                           |
         v                           v
+------------------+        +-------------------+
| ModelWeatherReport|<------| ProviderHealthMap |
|   (new)          |        |   (modified)      |
+--------+---------+        +--------+----------+
         |                           |
         v                           v
+------------------+        +-------------------+
|  TelemetryStore  |        |  TelemetryStore   |
|  (existing)      |        |  (existing)       |
+------------------+        +-------------------+
         ^                           |
         |                           |
+--------+---------+                |
| CompletionEvent  |<---------------+
|  (from real      |
|   spawn output)  |
+------------------+
```

### Data Flow

**Health Update (existing, to use more):**
```
Spawn Agent -> Parse CLI Output -> CompletionEvent -> TelemetryStore.record()
                                                           |
                                                           v
                                              ModelPerformanceSnapshot
                                                           |
                                                           v
                                              ProviderHealthMap (updated)
```

**Weather Report Query (new):**
```
CLI: model-weather -> ModelWeatherReport::from_telemetry(store)
                            |
                            v
                    [Healthy Models Table]
                    - fastest: kimi-for-coding/k2p5 (avg 2.3s)
                    - cheapest: opencode-go/minimax-m2.5 ($0.001/1k)
                    - best free: claude-sonnet (95% success)
```

**Spawn Fallback (modified - per tier):**
```
Spawn Request -> RoutingDecisionEngine.decide_route_with_fallbacks()
                      |
                      v
              Primary Candidate (best score in tier)
                      |
                      v
              Try Spawn -> Rate Limit / Subscription Limit?
                      |
                      v
              Fallback Candidate (next best score, SAME tier)
                      |
                      v
              ...continue until success or tier exhausted
                      |
                      v
              (Optional: fall back to next tier if configured)
```

Fallback stays within the same tier to avoid accidentally switching from subscription to pay-per-use.

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Telemetry as primary health signal | Real observed data is better than synthetic probes | Keep probing every 30 min (rejected: wasteful) |
| Key format validation only (no API call) | Zero token cost for cold start | Make a cheap API call to validate (rejected: burns tokens) |
| CLI output first, agent-readable | User explicitly asked CLI + agent-first | Web dashboard (rejected: out of scope) |
| Fallback retry in spawn pipeline | Centralises retry logic, keeps routing simple | Each agent handles its own fallback (rejected: inconsistent) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| HTTP HEAD health checks | Most providers don't expose simple endpoints; wouldn't catch auth/rate-limit issues | Complexity for marginal benefit |
| Model capability benchmarking | We need availability/speed/cost, not MMLU scores | Scope creep - would need eval infrastructure |
| Predictive health (ML) | Overkill for current need | Complexity, training data, false positives |
| Real-time websocket feed | No consumer needs live updates | Infrastructure complexity |

### Simplicity Check

> "What if this could be easy?"

The simplest design:
1. Delete the probe loop
2. Add a method to TelemetryStore that returns a sorted list of model snapshots
3. Print it nicely in CLI
4. Add a `try_spawn_with_fallback()` that retries next candidate on rate limit

This is exactly what we will do. No overcomplication.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? **No.**

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

---

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_orchestrator/src/model_weather.rs` | ModelWeatherReport struct and formatting |
| `crates/terraphim_orchestrator/src/key_validator.rs` | API key format validation (cold start) |

### Modified Files
| File | Changes |
|------|---------|
| `provider_probe.rs` | Remove `probe_all()` and `probe_single()`; keep health map but drive from telemetry |
| `control_plane/telemetry.rs` | Add `weather_report()` method; add tier classification |
| `control_plane/routing.rs` | Add fallback candidate list to `RoutingDecision` |
| `lib.rs` | Wire up weather report CLI; remove startup probe call |
| `config.rs` | Deprecate `probe_on_startup`, `probe_ttl_secs`, `probe_results_dir` (keep for compat) |

### Deleted Files
| File | Reason |
|------|--------|
| None | We keep files but gut the wasteful functions |

---

## API Design

### New Public Types

```rust
/// A human and agent-readable weather report of currently available models.
/// Derived entirely from TelemetryStore - no API calls, no process spawning.
pub struct ModelWeatherReport {
    /// Timestamp when this report was generated
    pub generated_at: DateTime<Utc>,
    /// All known models with performance data
    pub models: Vec<ModelWeatherEntry>,
    /// Currently subscription-limited models
    pub limited_models: Vec<String>,
    /// Recommended model by category
    pub recommendations: ModelRecommendations,
}

/// Performance and health data for a single model.
pub struct ModelWeatherEntry {
    pub model: String,
    pub provider: String,
    pub tier: ModelTier,
    pub health: HealthStatus,
    pub avg_latency_ms: f64,
    pub avg_cost_per_1k_tokens: f64,
    pub success_rate: f64,
    pub total_completions: u64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub subscription_limited: bool,
}

/// Tier classification for model selection.
///
/// Determined by checking (in order):
/// 1. Config file `tier` field if present
/// 2. KG routing rule metadata if present
/// 3. Inferred from observed cost (zero cost = Free)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Free models (zero cost)
    Free,
    /// Subscription models (unlimited within plan)
    Subscription,
    /// Pay-per-use models
    PayPerUse,
    /// Unknown / not yet classified
    Unknown,
}

/// Recommendations for common selection criteria.
pub struct ModelRecommendations {
    pub fastest: Option<String>,
    pub cheapest: Option<String>,
    pub best_free: Option<String>,
    pub most_reliable: Option<String>,
}

/// Result of a lightweight key validity check.
pub struct KeyValidationResult {
    pub provider: String,
    pub valid_format: bool,
    pub has_key: bool,
    pub error: Option<String>,
}
```

### New Public Functions

```rust
// model_weather.rs

/// Generate a weather report from telemetry data.
///
/// # Arguments
/// * `store` - TelemetryStore with observed completion data
/// * `kg_router` - Optional KG router for model discovery
///
/// # Returns
/// A weather report containing all known models and recommendations.
pub async fn generate_weather_report(
    store: &TelemetryStore,
    kg_router: Option<&KgRouter>,
) -> ModelWeatherReport;

/// Format the weather report for CLI output (human-readable).
pub fn format_weather_report(report: &ModelWeatherReport) -> String;

/// Format the weather report as JSON (agent-readable).
pub fn format_weather_report_json(report: &ModelWeatherReport) -> String;

// key_validator.rs

/// Validate API key format for all configured providers.
///
/// Checks:
/// - Key exists in environment or config
/// - Key matches expected format (prefix, length)
///
/// Does NOT make API calls.
pub fn validate_provider_keys() -> Vec<KeyValidationResult>;

// control_plane/telemetry.rs (additions)

impl TelemetryStore {
    /// Get models sorted by success rate (descending).
    pub async fn models_by_reliability(&self) -> Vec<ModelPerformanceSnapshot>;

    /// Get models sorted by cost (ascending).
    pub async fn models_by_cost(&self) -> Vec<ModelPerformanceSnapshot>;

    /// Get models filtered by tier.
    pub async fn models_by_tier(&self, tier: ModelTier) -> Vec<ModelPerformanceSnapshot>;
}

// control_plane/routing.rs (additions)

impl RoutingDecisionEngine {
    /// Decide route with fallback candidates ordered by preference (same tier).
    ///
    /// Returns the primary candidate plus an ordered list of fallback candidates
    /// within the same tier. This ensures we don't accidentally fall back from
    /// a subscription model to a pay-per-use model.
    pub async fn decide_route_with_fallbacks(
        &self,
        ctx: &DispatchContext,
        budget_verdict: &BudgetVerdict,
    ) -> (RoutingDecision, Vec<RouteCandidate>);
}
```

### Modified Functions

```rust
// provider_probe.rs

/// Drive health map from telemetry instead of spawning probes.
///
/// Updates circuit breakers based on TelemetryStore success rates.
/// No process spawning. No API calls.
pub async fn update_from_telemetry(&mut self, store: &TelemetryStore);

/// DEPRECATED: This is now a no-op. Health is derived from telemetry.
/// Kept for API compatibility.
pub async fn probe_all(&mut self, _kg_router: &KgRouter) {
    tracing::warn!("probe_all() is deprecated; health is now derived from telemetry");
}
```

---

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_weather_report_empty_store` | `model_weather.rs` | Empty telemetry -> all models from KG router, unknown health |
| `test_weather_report_reliability_sort` | `model_weather.rs` | Models sorted by success rate descending |
| `test_weather_report_fastest_recommendation` | `model_weather.rs` | Fastest model identified correctly |
| `test_weather_report_cheapest_recommendation` | `model_weather.rs` | Cheapest model identified correctly |
| `test_weather_report_free_filter` | `model_weather.rs` | Only free models returned when filtered |
| `test_key_validation_format` | `key_validator.rs` | Valid/invalid key formats detected |
| `test_key_validation_missing` | `key_validator.rs` | Missing keys reported |
| `test_fallback_candidates_ordered` | `routing.rs` | Fallbacks ordered by score descending |
| `test_fallback_on_rate_limit` | spawn module | Spawn retries next candidate on rate limit |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_spawn_with_fallback` | `tests/spawn_fallback.rs` | Full spawn -> rate limit -> fallback flow |
| `test_weather_report_cli_output` | `tests/weather_cli.rs` | CLI command produces expected output |
| `test_cold_start_key_validation` | `tests/cold_start.rs` | Startup validates keys without API calls |

---

## Implementation Steps

### Step 1: Add Model Weather Report Types and Generation
**Files:** `src/model_weather.rs` (new)
**Description:** Define `ModelWeatherReport`, `ModelWeatherEntry`, `ModelTier`, `ModelRecommendations`. Implement `generate_weather_report()` that queries `TelemetryStore` and `KgRouter`.
**Tests:** Unit tests for empty store, sorting, filtering.
**Dependencies:** None
**Estimated:** 3 hours

```rust
// Key code to write:
pub struct ModelWeatherReport { ... }
pub async fn generate_weather_report(store: &TelemetryStore, kg_router: Option<&KgRouter>) -> ModelWeatherReport;
pub fn format_weather_report(report: &ModelWeatherReport) -> String;
pub fn format_weather_report_json(report: &ModelWeatherReport) -> String;
```

### Step 2: Add Key Validation (Cold Start)
**Files:** `src/key_validator.rs` (new)
**Description:** Check API key presence and format for each provider. No API calls.
**Tests:** Unit tests for valid/invalid/missing keys.
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to write:
pub fn validate_provider_keys() -> Vec<KeyValidationResult>;
```

### Step 3: Deprecate Active Probe
**Files:** `src/provider_probe.rs`, `src/lib.rs`
**Description:** Make `probe_all()` and `probe_single()` no-ops with deprecation warnings. Remove startup probe call from `Orchestrator::run()`. Keep `ProviderHealthMap` but add `update_from_telemetry()`.
**Tests:** Ensure no process spawning in tests.
**Dependencies:** None
**Estimated:** 2 hours

```rust
// Key code to write:
pub async fn probe_all(&mut self, _kg_router: &KgRouter) {
    tracing::warn!("probe_all() is deprecated; health derived from telemetry");
}
```

### Step 4: Add Telemetry-Driven Health Updates
**Files:** `src/control_plane/telemetry.rs`
**Description:** Add `models_by_reliability()`, `models_by_cost()`, `models_by_tier()` to `TelemetryStore`.
**Tests:** Unit tests for sorting and filtering.
**Dependencies:** Step 1
**Estimated:** 2 hours

```rust
// Key code to write:
impl TelemetryStore {
    pub async fn models_by_reliability(&self) -> Vec<ModelPerformanceSnapshot>;
    pub async fn models_by_cost(&self) -> Vec<ModelPerformanceSnapshot>;
}
```

### Step 5: Add Fallback Candidates to Routing
**Files:** `src/control_plane/routing.rs`
**Description:** Add `decide_route_with_fallbacks()` that returns ordered fallback candidates. Modify spawn to try each in order on rate limit.
**Tests:** Unit tests for fallback ordering, integration test for spawn retry.
**Dependencies:** Step 3
**Estimated:** 3 hours

```rust
// Key code to write:
pub async fn decide_route_with_fallbacks(...) -> (RoutingDecision, Vec<RouteCandidate>);
```

### Step 6: Wire Up CLI Command
**Files:** `src/lib.rs` (or CLI module)
**Description:** Add `adf model-weather` subcommand that prints formatted report. Support `--json` for agents.
**Tests:** CLI integration test.
**Dependencies:** Steps 1, 2
**Estimated:** 2 hours

### Step 7: Update Config Defaults
**Files:** `src/config.rs`
**Description:** Deprecate probe-related config fields. Add comment that they are ignored. Keep for backward compatibility.
**Tests:** Config parse test with deprecated fields.
**Dependencies:** Step 3
**Estimated:** 1 hour

---

## Rollback Plan

If issues discovered:
1. Revert `provider_probe.rs` changes (restore `probe_all` / `probe_single`)
2. Re-enable startup probe call in `lib.rs`
3. Remove new files (`model_weather.rs`, `key_validator.rs`)

No database migrations or external state changes.

---

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| None | - | All functionality uses existing crates |

### Dependency Updates
| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | - | - | No updates needed |

---

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Startup time | < 5s | Remove probe delay |
| Weather report generation | < 10ms | In-memory telemetry query |
| Key validation | < 1ms | Format regex checks |
| Spawn fallback latency | < 100ms | Try next candidate immediately |

### Benchmarks to Add
```rust
#[bench]
fn bench_weather_report_generation(b: &mut Bencher) {
    let store = TelemetryStore::new(3600);
    // populate with sample data...
    b.iter(|| generate_weather_report(&store, None));
}
```

---

## User Answers (2026-05-20)

| Question | Answer |
|----------|--------|
| CLI command name | `adf model-weather` |
| Tier classification | Knowledge graph + read from config (KG routing rules + config file `tier` field) |
| Fallback depth | Per tier - try candidates within same tier first, as now |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| User approval on design | Approved | User |
| Exact CLI command name | Resolved: `adf model-weather` | User |
| Tier classification rules | Resolved: KG + config | User |
| Number of fallback retries | Resolved: per tier | User |

---

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

---

## Questions Before Implementation

1. **CLI command name**: Should it be `terraphim model-weather`, `terraphim weather`, or something else?

2. **Tier classification**: How do we know if a model is "free" vs "subscription" vs "pay-per-use"? Should this be:
   - Hardcoded based on model prefix (e.g., `kimi-for-coding/*` = subscription)?
   - Read from a config file?
   - Inferred from observed cost (zero cost = free)?

3. **Fallback depth**: When spawn hits rate limit, should we:
   - Try ALL remaining candidates in order?
   - Stop after N retries (e.g., max 3)?
   - Only try candidates within the same tier (don't fall back from subscription to pay-per-use)?
