# Design & Implementation Plan: opencode-weather ADF-Backed Model Weather

## 1. Summary of Target Behavior

Add an ADF-backed model weather capability that exposes current model recommendations as a versioned JSON snapshot and a human-readable report. The standalone `opencode-weather` repository will consume this snapshot and present it inside opencode through a tool/plugin.

The authoritative scoring and rationale live in `terraphim-ai`, not in `opencode-weather`.

Target user flows:

| Flow | Behaviour |
|---|---|
| Terminal | `adf-ctl weather report --json` returns a machine-readable snapshot |
| Terminal | `adf-ctl weather report` returns a compact weather report |
| opencode | User calls `model_weather`; plugin shells out to the Terraphim weather command and renders the result |
| Optional proxy | If configured, ADF weather enriches candidates with proxy performance/cost/health |

## 2. Key Invariants and Acceptance Criteria

Invariants:

| Invariant | Guarantee |
|---|---|
| Single scoring authority | ADF weather service owns scoring and rationale |
| Thin opencode integration | `opencode-weather` does not duplicate KG routing, telemetry scoring, or budget gates |
| Local-first | No usage data leaves the machine by default |
| Explicit provenance | Every recommendation shows contributing sources |
| Explicit freshness | Metrics show timestamps/staleness/confidence |
| No automatic switching | v1 recommends only |

Acceptance criteria:

| ID | Criterion |
|---|---|
| AC1 | ADF exposes a JSON weather snapshot with ranked categories |
| AC2 | Snapshot includes candidate model, provider, CLI tool, score, confidence, status, evidence, and rationale |
| AC3 | Snapshot categories include `fastest_now`, `cheapest_now`, `best_thinking`, `best_coding`, and `avoid_now` |
| AC4 | Weather service reuses `RoutingDecisionEngine`, `KgRouter`, `TelemetryStore`, `ProviderBudgetTracker`, and `ProviderHealthMap` |
| AC5 | Proxy data is optional enrichment and absence of proxy does not fail the report |
| AC6 | `opencode-weather` plugin registers a `model_weather` tool and renders the Terraphim report |
| AC7 | Tests verify routing reuse, telemetry effects, budget/quota penalties, missing data, and proxy absence |

## 3. High-Level Design and Boundaries

Architecture:

```text
opencode-weather repo
  opencode plugin + small CLI wrapper
        |
        v
terraphim-ai weather CLI/API
  control_plane::weather
        |
        +-- RoutingDecisionEngine dry-runs
        +-- KgRouter all_routes and route_agent
        +-- TelemetryStore model performance
        +-- ProviderBudgetTracker checks
        +-- ProviderHealthMap probe results
        +-- terraphim_usage history/pricing
        +-- optional terraphim-llm-proxy enrichment
```

Boundary rules:

| Boundary | Decision |
|---|---|
| ADF weather service | Owns candidate generation, scoring, category assignment, and rationale |
| `terraphim-llm-proxy` | Provides optional data only; does not become mandatory for weather |
| `opencode-weather` | Owns opencode UX, installation, command invocation, and formatting fallback |
| Shared schema | Versioned JSON contract, stable enough for plugin compatibility |

Recommended source precedence:

1. ADF live routing and telemetry.
2. Direct opencode captured observations, once available.
3. Proxy metrics when proxy is configured.
4. Terraphim usage history.
5. Static KG/pricing metadata.

## 4. File/Module-Level Change Plan

### `terraphim-ai`

| File/Module | Action | Before | After | Dependencies |
|---|---|---|---|---|
| `crates/terraphim_orchestrator/src/control_plane/weather.rs` | Create | No weather snapshot module | Defines weather snapshot types and service | `routing`, `telemetry`, `provider_budget`, `provider_probe`, `terraphim_usage` |
| `crates/terraphim_orchestrator/src/control_plane/mod.rs` | Modify | Exports routing/telemetry modules | Exports `weather` module | New module only |
| `crates/terraphim_orchestrator/src/bin/adf-ctl.rs` | Modify | No weather command | Adds `weather report [--json]` | `WeatherService` |
| `crates/terraphim_orchestrator/src/lib.rs` | Modify | Owns router, telemetry, provider health | Exposes method/helper to build weather service from orchestrator state | Existing fields only |
| `crates/terraphim_orchestrator/src/control_plane/routing.rs` | Minimal modify if needed | `decide_route()` returns decision for one context | Add helper for dry-run/category usage only if current API insufficient | Avoid changing scoring rules |
| `crates/terraphim_orchestrator/tests/weather_tests.rs` | Create | No weather tests | Unit/integration tests for snapshot generation | No mocks; use real stores/fixtures |
| `.docs/design-opencode-weather-adf.md` | Create | No formal design | Implementation contract | Planning artefact |

### `terraphim-llm-proxy`

| File/Module | Action | Before | After | Dependencies |
|---|---|---|---|---|
| Existing `/api/metrics/json` | Reuse | Provides aggregate/provider metrics | Optional weather enrichment source | No initial change required |
| Existing `/health/detailed` | Reuse | Provides provider/system health | Optional weather enrichment source | No initial change required |
| Existing `/v1/models` | Reuse | Provides configured models | Optional inventory source | No initial change required |
| Future `/weather` endpoint | Defer | Not present | Direct proxy weather feed if later needed | Not required for v1 |

### `opencode-weather` new repository

| File/Module | Action | Responsibility |
|---|---|---|
| `package.json` | Create | Plugin package and scripts |
| `plugin/model-weather.js` | Create | Registers opencode `model_weather` tool |
| `src/report.ts` or simple JS helper | Create | Shells out to `adf-ctl weather report` and handles errors |
| `README.md` | Create | Install and usage instructions |
| `schemas/weather-snapshot.schema.json` | Copy/reference | Validates plugin contract |
| `tests/model-weather.test.js` | Create | Tests plugin formatting with fixture JSON |

## 5. Step-by-Step Implementation Sequence

1. Define weather snapshot schema in `terraphim-ai`.
   Deployable state: types compile; no behaviour changes.

2. Implement `WeatherService` in `control_plane::weather` using existing ADF objects.
   Deployable state: service can produce a snapshot from injected router/telemetry/budget/health state.

3. Add predefined weather categories and dry-run tasks.
   Deployable state: `best_thinking`, `best_coding`, `fastest_now`, `cheapest_now`, and `avoid_now` can be generated without spawning agents.

4. Add CLI command in `adf-ctl`.
   Deployable state: `adf-ctl weather report --json` works with no proxy configured.

5. Add optional proxy enrichment adapter.
   Deployable state: if proxy URL is present, enrich; otherwise report continues with ADF-only data.

6. Add tests for snapshot generation.
   Deployable state: routing, telemetry, provider-budget, quota, and missing-proxy scenarios covered.

7. Create `opencode-weather` repository.
   Deployable state: plugin can call `adf-ctl weather report` and render fixture data.

8. Add opencode plugin install documentation.
   Deployable state: user can install plugin and invoke `model_weather` from opencode.

9. Add optional direct opencode event capture later.
   Deployable state: direct opencode usage can feed Terraphim telemetry without proxy.

## 6. Testing & Verification Strategy

| Acceptance Criteria | Test Type | Test Location |
|---|---|---|
| AC1 | CLI integration | `crates/terraphim_orchestrator/tests/weather_cli_tests.rs` |
| AC2 | JSON schema/unit | `crates/terraphim_orchestrator/tests/weather_tests.rs` |
| AC3 | Category snapshot test | `weather_tests.rs` |
| AC4 | Routing reuse test | `weather_tests.rs` with real `RoutingDecisionEngine` |
| AC5 | Proxy absent test | `weather_tests.rs` |
| AC6 | Plugin fixture test | `opencode-weather/tests/model-weather.test.js` |
| AC7 | Telemetry/budget/quota cases | `weather_tests.rs` |

Important test cases:

| Case | Expected Result |
|---|---|
| No proxy configured | Weather report still succeeds |
| KG and keyword agree | Candidate source shows combined/routing evidence |
| Provider budget exhausted | Candidate appears in `avoid_now` or is excluded from best categories |
| Subscription limited telemetry | Candidate heavily penalised and marked limited |
| Sparse telemetry | Candidate can rank but confidence is low |
| Fast but unreliable model | Reliability penalty prevents misleading fastest recommendation |
| Unknown pricing | Candidate not selected as cheapest unless no better evidence exists |

Verification commands after implementation:

```bash
cargo test -p terraphim_orchestrator weather
cargo test -p terraphim_orchestrator --test weather_tests
ubs crates/terraphim_orchestrator/src/control_plane/weather.rs
```

For `opencode-weather`:

```bash
bun test
bun run lint
```

## 7. Risk & Complexity Review

| Risk | Mitigation | Residual Risk |
|---|---|---|
| Weather duplicates routing logic | Call `RoutingDecisionEngine`; do not copy scoring | Some category-specific ranking still needed |
| Proxy availability varies | Treat proxy as optional enrichment | Proxy-only metrics may be absent |
| Direct opencode traffic is invisible | Add opencode capture as later source | v1 may rely on ADF/proxy history only |
| Too much logic leaks into plugin | Plugin shells out and renders only | Plugin may need fallback formatting |
| JSON schema churn breaks plugin | Version schema and keep backward-compatible fields | Version migration needed later |
| Active probes cost quota | Reuse ADF probes; do not trigger by default from plugin | Explicit probes still carry cost |

## 8. Open Questions / Decisions for Human Review

| Decision | Recommended Default |
|---|---|
| CLI entry point | Start with `adf-ctl weather report --json`; optionally alias through `terraphim-agent weather` later |
| Proxy enrichment | Enable only when `TERRAPHIM_LLM_PROXY_URL` is set |
| Direct opencode capture | Phase 2 follow-up after weather report works |
| Active probes from plugin | Do not allow in v1 |
| Plugin language | JavaScript using `@opencode-ai/plugin`, shelling out to Terraphim CLI |
| Schema ownership | `terraphim-ai` owns canonical schema; `opencode-weather` vendors/validates it |
