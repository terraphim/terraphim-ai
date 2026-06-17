# Implementation Plan: weather-report CLI

**Status**: Review
**Research Doc**: `.docs/weather-report/research.md`
**Author**: opencode session
**Date**: 2026-06-17
**Estimated Effort**: ~0.5 day

## Overview

### Summary
A new crate `terraphim_weather_report` providing a `weather-report` binary that
loads the ADF tier taxonomy, optionally live-probes every model route via the
orchestrator's `provider_probe`, and prints a "weather report" bucketing tiers
into THINKING / WORKHORSE / FAST & CHEAP with a per-model condition.

### Approach
Option A from the research doc: **reuse** `KgRouter` + `ProviderHealthMap` for
probing (single source of truth for C1 gate / circuit breakers / env errors),
and `parse_markdown_directives_dir` for tier grouping (because `RoutingRule` is
private and `all_routes()` loses tier association).

### Scope
**In Scope:**
- New crate `crates/terraphim_weather_report` (lib + bin).
- Subcommands: `report` (default), `thinking`, `fast`, `workhorse`, `tiers`.
- Human + JSON output.
- `--no-probe` (forecast mode), `--taxonomy <PATH>` / `ADF_TAXONOMY_DIR`,
  `--slow-threshold <MS>`.
- Unit tests for all pure logic; one test against the real in-repo taxonomy.

**Out of Scope:** persistence/TTL cache, watch mode, cost-per-token,
`--timeout` (probe timeout is hard-coded 15s upstream).

**Avoid At All Cost** (5/25):
- Re-implementing the probe (duplicates allow-list/env-error logic).
- `chrono` -- **the project uses `jiff`**; the earlier draft's `chrono`
  dependency and `chrono_now`/`now_timestamp` must be replaced with `jiff`.
- A `--timeout` flag that does nothing (probe timeout is not configurable).
- A live-change `watch` mode.
- Inherent methods defined on lib types from the binary crate (orphan rule).

## Architecture

### Component Diagram
```
                +-----------------------------+
   taxonomy --> | load_tier_routes (lib)      | -- (concept, MarkdownDirectives)[] sorted
                +-----------------------------+
                          |
          +---------------+--------------------------------+
          | (probe branch)                                | (no-probe branch)
          v                                               |
   KgRouter::load(dir)                                    | probes = []
          |                                               |
   ProviderHealthMap::probe_all(&router) ----->  Vec<ProbeResult>
          |                                               |
          +---------------+--------------------------------+
                          |
                          v
                +-----------------------------+
                | build_report (lib, pure)    | -- WeatherReport
                +-----------------------------+
                          |        |
                  filter_by_kind  print_human / print_json
```

### Data Flow
```
CLI args -> resolve_taxonomy -> load_tier_routes
         -> (probe? run_probes : []) -> build_report -> [filter] -> render
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Two parses (automata + KgRouter) | Need tier grouping (KgRouter flattens); 4-file dir is trivial to parse twice | Mirror private `RoutingRule` (fragile) |
| Default = live probe | User said "currently available" | Default `--no-probe` (rejected: defeats "weather") |
| `jiff` for timestamps | Project standard; matches orchestrator | `chrono` (rejected -- not used here) |
| Text condition tokens, no emoji | Project rule (no emoji) | Unicode weather glyphs |
| Probe the FULL taxonomy even for filtered subcommands | Summary stays honest regardless of view | Probe only filtered set (misleading counts) |

### Eliminated Options (Essentialism)
| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| `--timeout` flag | Hard-coded 15s upstream | Misleads users |
| `chrono` | Not the project's time lib | Dependency noise / drift |
| Watch/poll mode | Run-once tool | Complexity, resource use |
| Cost-per-token | No price data in taxonomy | Speculative feature |

### Simplicity Check
**What if this could be easy?** It is: two crates do the work, the new code is
classification + a report struct + rendering. The binary is a clap shell over
three lib calls.

**Senior Engineer Test:** a senior engineer would *not* call this
overcomplicated -- it is a thin viewer. The only wart (double parse) is forced
by `RoutingRule` being private and is explicitly justified.

**Nothing Speculative Checklist:**
- [x] No features the user didn't request
- [x] No abstractions "for later"
- [x] No `--timeout` flexibility we can't honour
- [x] No error handling for impossible scenarios

## File Changes

### New Files
| File | Purpose |
|------|---------|
| `crates/terraphim_weather_report/Cargo.toml` | Crate manifest; deps via Gitea `terraphim` registry |
| `crates/terraphim_weather_report/src/lib.rs` | Pure logic: `TierKind`, `WeatherCondition`, report types, `load_tier_routes`, `build_report`, `filter_by_kind` + unit tests |
| `crates/terraphim_weather_report/src/main.rs` | clap CLI, probe orchestration, human/JSON rendering |

### Modified Files
| File | Changes |
|--------|---------|
| (none mandatory) | `crates/*` glob auto-members the crate; workspace `exclude` list does not name it |
| `Cargo.toml` (workspace) | Optional: explicit member add for clarity -- not required |

### Deleted Files
None.

## API Design

### Public Types (lib.rs)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierKind { Thinking, Workhorse, FastCheap }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherCondition { Sunny, Fair, Cloudy, Stormy, Offline, Unknown, Configured }

#[derive(Debug, Clone, Serialize)]
pub struct ModelRow {
    pub provider: String,
    pub model: String,
    pub cli: String,
    pub is_free: bool,
    pub condition: WeatherCondition,
    pub latency_ms: Option<u64>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TierSection {
    pub concept: String,
    pub heading: String,
    pub kind: TierKind,
    pub priority: Option<u8>,
    pub models: Vec<ModelRow>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ConditionSummary {
    pub sunny: usize, pub fair: usize, pub cloudy: usize,
    pub stormy: usize, pub offline: usize, pub unknown: usize, pub configured: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeatherReport {
    pub generated_at: String,        // jiff RFC3339
    pub taxonomy_path: String,
    pub probed: bool,
    pub total_models: usize,
    pub summary: ConditionSummary,
    pub tiers: Vec<TierSection>,
}
```

### Public Functions (lib.rs) -- signatures
```rust
/// Classify a tier by concept name with priority fallback.
pub fn classify_tier(concept: &str, priority: Option<u8>) -> TierKind;

/// Map an optional probe result to a weather condition.
pub fn from_probe(probe: Option<&ProbeResult>, slow_threshold_ms: u64) -> WeatherCondition;

/// Load `(concept, directives)` pairs with >=1 route, sorted by priority desc.
pub fn load_tier_routes(taxonomy: &Path) -> anyhow::Result<Vec<(String, MarkdownDirectives)>>;

/// Find a probe matching a route's (cli, provider, model) triple.
pub fn find_probe<'a>(probes: &'a [ProbeResult], route: &RouteDirective) -> Option<&'a ProbeResult>;

/// Assemble the report from tiers + probes. `probed=false` => all Configured.
pub fn build_report(
    taxonomy_path: &Path,
    entries: &[(String, MarkdownDirectives)],
    probes: &[ProbeResult],
    probed: bool,
    slow_threshold_ms: u64,
) -> WeatherReport;

/// Retain only one tier kind and recompute the summary.
pub fn filter_by_kind(report: WeatherReport, kind: TierKind) -> WeatherReport;
```

> Note: `WeatherCondition::from_probe` is an **inherent method on a type in the
> same crate (lib)** -- legal. The earlier draft's bug was defining an inherent
> method on `WeatherCondition` *from the binary crate* (E0116). Fix: call
> `m.condition.token()` directly; keep `token()` and `from_probe` in the lib.

### CLI (main.rs) -- clap shape
```rust
#[derive(Parser)]
struct Cli {
    #[arg(long, env = "ADF_TAXONOMY_DIR")] taxonomy: Option<PathBuf>,
    #[arg(long)] no_probe: bool,
    #[arg(long, default_value_t = 3000)] slow_threshold: u64, // ms
    #[arg(long, value_enum, default_value_t)] format: OutputFormat, // Human | Json
    #[command(subcommand)] command: Option<ReportCommand>,
}
enum ReportCommand { Report, Thinking, Fast, Workhorse, Tiers }
```

### Error Strategy
- `anyhow::Result` throughout (matches `adf-ctl`).
- Bad taxonomy path / empty taxonomy -> `bail!` with actionable message.
- `KgRouterError` mapped via `anyhow!` (it impls `std::error::Error`).

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `terraphim_orchestrator` | 1.20.2 (registry `terraphim`, default-features=false) | probe + router |
| `terraphim_automata` | 1.20.2 (registry `terraphim`) | taxonomy parser |
| `terraphim_types` | 1.20.2 (registry `terraphim`) | `RouteDirective`, `MarkdownDirectives` |
| `jiff` | 0.2 | Timestamps (project standard; **replaces chrono**) |
| `clap` | 4 (derive) | CLI |
| `serde` / `serde_json` | 1 | JSON output |
| `tokio` | 1 (rt, macros, time, process) | probe is async |
| `anyhow` | 1 | errors |

> **chrono is deliberately NOT a dependency.** Remove it from the current draft
> `Cargo.toml` and replace `chrono::Utc::now()` calls with `jiff::Timestamp::now()`.

## Test Strategy

### Unit Tests (lib.rs, no mocks -- real types)
| Test | Purpose |
|------|---------|
| `classifies_known_tiers_by_name` | planning->Thinking, review->FastCheap, etc. |
| `classifies_unknown_tier_by_priority` | priority bands fallback |
| `condition_from_probe_maps_all_statuses` | None->Configured, Success fast->Sunny, slow->Fair, RateLimited->Cloudy, Timeout->Stormy |
| `condition_distinguishes_env_error_from_offline` | missing-CLI/allow-list -> Unknown; HTTP 503 -> Offline |
| `find_probe_matches_on_cli_provider_model` | join key correctness |
| `build_report_summarises_conditions` | summary counts + tier kind + available() |
| `no_probe_marks_everything_configured` | `probed=false` path |
| `filter_by_kind_recomputes_summary` | subcommand filtering recomputes counts |
| `loads_real_adf_taxonomy` | reads `docs/taxonomy/routing_scenarios/adf`, asserts >=4 tiers, priority-sorted, contains Thinking + FastCheap |

### Integration (manual, after build)
| Check | Command |
|-------|---------|
| Forecast instant | `weather-report tiers` |
| Live report | `weather-report` (run from repo root) |
| Thinking only | `weather-report thinking` |
| Fast only | `weather-report fast` |
| JSON | `weather-report --format json --no-probed` |
| Custom taxonomy | `weather-report --taxonomy <dir>` |

### Property / Benchmarks
None needed for v1 (no perf targets beyond the NFR table).

## Implementation Steps

### Step 1: Fix manifest + timestamp library
**Files:** `Cargo.toml`, `src/lib.rs` (chrono_now), `src/main.rs` (now_timestamp)
- Remove `chrono`; add `jiff = "0.2"`.
- `generated_at` / timestamp via `jiff::Timestamp::now().to_string()`.
**Test:** crate still compiles.
**Estimated:** 10 min

### Step 2: Pure lib logic + unit tests (TDD)
**Files:** `src/lib.rs`
- Types: `TierKind`, `WeatherCondition` (with `token()` + `from_probe()`),
  `ModelRow`, `TierSection`, `ConditionSummary`, `WeatherReport`.
- Functions: `classify_tier`, `is_environment_error`, `load_tier_routes`,
  `find_probe`, `model_row`, `build_report`, `filter_by_kind`, `now()` (jiff).
- All 9 unit tests pass.
**Test:** `cargo test -p terraphim_weather_report --lib`
**Estimated:** 1.5 h

### Step 3: Binary -- CLI + probing + rendering
**Files:** `src/main.rs`
- clap `Cli`/`ReportCommand`/`OutputFormat`.
- `resolve_taxonomy` (explicit > env > walk-up).
- `run_probes` (current-thread tokio runtime, `KgRouter::load` +
  `ProviderHealthMap::new(...).probe_all(...)`).
- `print_human` / `print_tier` / `print_model_row` / `print_summary` using
  `m.condition.token()` (NOT a cross-crate inherent impl).
- `print_json` via `serde_json::to_writer_pretty`.
- Probe the full taxonomy; apply `filter_by_kind` only for display.
**Test:** `cargo build -p terraphim_weather_report`; manual run.
**Estimated:** 1 h

### Step 4: Verify against real taxonomy
**Files:** none
- From repo root: `weather-report --no-probe`, `weather-report`, `--format json`.
- Confirm 4 tiers / 21 routes, free/paid labels, conditions render.
**Test:** manual + `loads_real_adf_taxonomy` unit test.
**Estimated:** 20 min

### Step 5: Quality gates + land
- `cargo fmt`, `cargo clippy -p terraphim_weather_report -- -D warnings`.
- `cargo test -p terraphim_weather_report`.
- Commit; file Gitea issue; push origin + gitea per remote-sync protocol.
**Estimated:** 20 min

## Rollback Plan
The crate is additive and auto-membered. Rollback = delete
`crates/terraphim_weather_report/`. No other code depends on it, so removal is
safe and complete.

## Performance Considerations
| Metric | Target | Measurement |
|--------|--------|-------------|
| `--no-probe` cold run | < 50 ms | manual timing |
| Live probe wall time | <= 15 s (concurrent, capped by 15s/model) | manual timing |

No benchmarks added for v1.

## Open Items
| Item | Status | Owner |
|------|--------|-------|
| Confirm "default = live probe" with user | Pending user | user |
| Decide explicit workspace member add vs glob | Lean: glob (no edit) | session |

## Approval
- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Default-probe assumption confirmed by user
- [ ] Human approval received
