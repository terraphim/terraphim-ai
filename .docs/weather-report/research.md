# Research Document: weather-report CLI

**Status**: Review
**Author**: opencode session
**Date**: 2026-06-17
**Reviewers**: alex

## Executive Summary

We need a small CLI that reports which LLM models are **currently available**
across the ADF routing tiers, classifying each tier as "thinking" (deep
reasoning) or "fast and cheap" (verification/review), with a live "weather"
condition per model. The Terraphim codebase already contains everything needed
to build this without inventing new probing logic: the orchestrator's
`provider_probe` module executes each route's `action::` template and records
success / latency / rate-limit / timeout, and the `kg_router` loads the tier
taxonomy from markdown. The new CLI is therefore a thin **presentation and
orchestration layer** over two existing crates.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Gives instant visibility into which paid/free models are alive right now -- replaces guesswork before dispatching agents. |
| Leverages strengths? | Yes | Reuses the orchestrator's battle-tested probe + circuit-breaker + C1 allow-list gate rather than re-implementing them. |
| Meets real need? | Yes | ADF dispatchers currently have no quick "what's up?" command; `adf-ctl status` shows processes, not model health. |

**Proceed**: Yes (3/3).

## Problem Statement

### Description
There is no single command that answers "which models can I actually use right
now, and are they the fast/cheap ones or the thinking ones?". The taxonomy
files list *configured* routes, but a configured route may point at an API that
is down, rate-limited, missing its CLI tool, or blocked by the C1
subscription allow-list.

### Impact
ADF dispatchers pick routes that silently fail or time out. Operators must
read orchestrator logs to discover that a provider is unhealthy. Cost/tier
(free vs paid, thinking vs cheap) is not surfaced anywhere machine-readable.

### Success Criteria
1. One command prints every tier's model roster with a live condition.
2. Tiers are bucketed into THINKING / WORKHORSE / FAST & CHEAP.
3. Free vs paid is visible per model.
4. JSON output exists for automation (mirrors `adf-ctl --format json`).
5. A `--no-probe` mode lists the configured roster instantly without firing
   any API calls.
6. No new probing logic; reuses `terraphim_orchestrator::provider_probe`.

## Current State Analysis

### Existing Implementation
The probe already exists and runs inside the orchestrator reconciliation loop.
We are exposing it as a standalone CLI, not building it.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| Probe executor | `terraphim_orchestrator::provider_probe::probe_single` | Runs a route's action template with `"echo hello"`, 15s timeout, records status+latency. (registry 1.20.2) |
| Health cache | `terraphim_orchestrator::provider_probe::ProviderHealthMap` | Concurrent `probe_all(&KgRouter)`, circuit breakers, rate-limit tracking, `.results()`. |
| Tier loader | `terraphim_orchestrator::kg_router::KgRouter::load(dir)` | Parses a taxonomy dir of markdown into routing rules. |
| Taxonomy parser | `terraphim_automata::markdown_directives::parse_markdown_directives_dir` | Returns `HashMap<concept, MarkdownDirectives>` keyed by filename stem -- gives tier-grouped routes. |
| Route type | `terraphim_types::RouteDirective` | `{provider, model, action, is_free}` + `cli_basename()` + `route_key()`. |
| Directives | `terraphim_types::MarkdownDirectives` | `{routes, priority, synonyms, heading}` per tier file. |
| CLI pattern | `terraphim_orchestrator src/bin/adf-ctl.rs` | clap subcommands, `OutputFormat::{Human,Json}`, config discovery -- the template to mimic. |
| Tier taxonomy | `docs/taxonomy/routing_scenarios/adf/*.md` | The 4 tier files being reported on. |
| C1 allow-list | `terraphim_orchestrator::config::is_allowed_provider` | Subscription gate the probe already enforces. |

### Tier Taxonomy (the data being reported)

| Tier file | Priority | Routes | Classification |
|-----------|----------|--------|----------------|
| `planning_tier.md` | 80 | 6 | Thinking (opus, k2p6, gpt-5.4/5.5, glm-5.1) |
| `decision_tier.md` | 65 | 4 | Thinking (gpt-5.5, k2p6, glm-5.1) |
| `implementation_tier.md` | 50 | 6 | Workhorse (sonnet, k2p5, gpt-5.3-codex, MiniMax-M2.7, glm-5.1) |
| `review_tier.md` | 40 | 5 | Fast & Cheap (haiku, k2p5, glm-5.1, gpt-5.4-mini, MiniMax-M2.5) |

Total: 4 tiers, 21 routes. Several routes are marked `is_free:: true`
(glm-5.1 via zai-coding-plan, MiniMax-M2.5).

### Data Flow
```
taxonomy/*.md
   |-- parse_markdown_directives_dir --> (concept, MarkdownDirectives)  [tier grouping + roster]
   |-- KgRouter::load                 --> KgRouter                       [probe input]
          |
          v
   ProviderHealthMap::probe_all(&router) --executes action templates--> Vec<ProbeResult>
          |
          v
   join (cli, provider, model) --> ModelRow { condition, latency } --> WeatherReport
```

### Integration Points
- Reads: markdown taxonomy dir (filesystem only).
- Executes (probe mode): the CLI tools named in `action::` templates
  (`claude`, `opencode`, `pi-rust`) via `bash -c`, with the orchestrator's
  `PATH` augmentation (`~/.local/bin`, `~/.bun/bin`, etc.).
- Emits: stdout (human table or JSON).

## Constraints

### Technical Constraints
- **Timestamps use `jiff`, not `chrono`.** The project standard is `jiff` (the
  orchestrator and `adf-ctl` both depend on `jiff::Timestamp`). The earlier
  draft accidentally added `chrono`; it must be removed.
- Edition 2021 (Rust toolchain is 1.91; edition 2024 also viable but 2021
  matches the orchestrator registry crates).
- Must consume `terraphim_orchestrator` etc. from the Gitea `terraphim`
  registry (`sparse+https://git.terraphim.cloud/...`), versions 1.20.2.
- No mocks in tests (project rule). Tier classification / condition mapping /
  report assembly are pure functions and are tested with real `RouteDirective`
  and `ProbeResult` values; one test reads the real in-repo taxonomy.
- The orchestrator's probe timeout is **hard-coded at 15s** and **not
  configurable** through the public API -- so a `--timeout` flag would be a
  lie. Do not add one.

### Business Constraints
- A live probe fires real API calls (one `"echo hello"` per route). That costs
  a tiny number of tokens and up to ~15s wall time (routes probe concurrently).
  Default must therefore be deliberate, and `--no-probe` must exist.

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| `--no-probe` latency | < 50 ms | n/a (new) |
| Live probe wall time | <= 15 s (concurrent) | orchestrator already does this |
| Output stability | JSON schema stable | new -- define now |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)
| Constraint | Why It's Vital | Evidence |
|------------|----------------|---------|
| Reuse `provider_probe`, do not reinvent | Correctness of allow-list/circuit-breaker/env-error semantics; one source of truth | `provider_probe.rs` already encodes these |
| Think/cheap dichotomy must be obvious | The user explicitly framed the need as "fast and cheap or thinking" | User request |
| No live API calls unless asked | Surprising operators with 21 token-bearing calls is hostile | probe_single runs real CLIs |

### Eliminated from Scope (5/25 AVOID list)
| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| `--timeout` flag | Probe timeout is hard-coded 15s in orchestrator; flag would mislead |
| Writing probe results to disk / TTL cache | `ProviderHealthMap` already does this in the orchestrator; a one-shot CLI does not need persistence |
| Routing / dispatching prompts | That is `kg_router::route_agent`'s job, not a weather report's |
| Cost-per-token display | No price data in taxonomy; `is_free` boolean is enough for v1 |
| Subscribing to live changes / `watch` mode | Out of scope; run-once tool |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_orchestrator` (registry) | Provides probe + router | Low -- already used widely; pin 1.20.2 |
| `terraphim_automata` (registry) | Taxonomy parser | Low |
| `terraphim_types` (registry) | `RouteDirective`, `MarkdownDirectives` | Low |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| clap | 4 | None | -- |
| serde / serde_json | 1 | None | -- |
| tokio | 1 (rt + process + time) | None | -- |
| jiff | 0.2 | None -- matches orchestrator | -- |
| anyhow | 1 | None | -- |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Probe fires tokens on default run | High | Low (tiny prompt) | Make live mode explicit via status line + keep `--no-probe`; document it |
| CLI tools have absolute `/home/alex/...` paths in templates | High | Med | Probe already detects missing CLI -> `Unknown` (env error); surfaced honestly in output |
| Join mismatch if `cli_basename` differs from probe's `cli_tool` | Low | Med | Both derive basename identically (`RouteDirective::cli_basename`); unit-test the join |
| `RoutingRule` is private, so tier grouping needs the automata parser (double parse) | Certain | Low | Two parses of a 4-file dir is negligible |

### Open Questions
1. Should live probe be the **default** or opt-in (`--probe`)? -- *Resolved
   below under Assumptions:* default ON, because the user said "currently
   available"; `--no-probe` is the escape hatch. Confirmable with the user.
2. Workspace membership vs standalone build? -- *Resolved:* add under
   `crates/*` (auto-membered), build with `cargo build -p
   terraphim_weather_report`.

### Assumptions Explicitly Stated
| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| "currently available" means live-probe by default | User wording | If wrong, default should flip to `--no-probe`; trivial change | No -- needs user confirm |
| `parse_markdown_directives_dir` keys by filename stem (e.g. `planning_tier`) | Read automata source `markdown_directives.rs:54` | Tier names mislabelled | Yes |
| Probe results join to routes on `(cli_basename, provider, model)` | orchestrator uses same key | Rows show `Configured` instead of live | Yes -- unit tested |
| `jiff` available transitively via orchestrator | orchestrator Cargo.toml lists jiff | Need to add jiff as direct dep | Add directly to be safe |

### Multiple Interpretations Considered
| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| A) Reuse `KgRouter` + `ProviderHealthMap` for probing | Full reuse incl. C1 gate & circuit breakers | **Chosen** -- maximal reuse, single source of truth |
| B) Parse taxonomy ourselves + re-implement probe | Lighter dep graph (drop orchestrator) | Rejected -- duplicates allow-list/env-error logic, drift risk |
| C) Shell out to `adf-ctl` | Zero new Rust | Rejected -- `adf-ctl` has no probe subcommand; would need orchestrator HTTP anyway |

## Research Findings

### Key Insights
1. **The hard part already exists.** `probe_single` + `ProviderHealthMap`
   already do live probing, C1 gating, environment-error detection, and
   concurrent execution. The CLI is presentation.
2. **Tier grouping requires the automata parser**, because `KgRouter`'s
   `all_routes()` flattens routes and loses the tier association
   (`RoutingRule` is private). So we parse twice: automata for grouping,
   KgRouter for probing. Cheap on a 4-file dir.
3. **The think/cheap split is derivable** from tier name + priority -- no new
   config needed. planning/decision/research/design -> Thinking;
   review/verif/valid/check -> FastCheap; implementation -> Workhorse;
   priority fallback otherwise.
4. **No emoji** (project rule). Weather conditions are uppercase text tokens
   (SUNNY / FAIR / CLOUDY / STORMY / OFFLINE / UNKNOWN / CONFIG).

### Relevant Prior Art
- `adf-ctl status`: shows running *processes*, not model health -- this CLI
  fills the gap.
- `adf-ctl agents --format json`: the JSON envelope style to mirror.

### Technical Spikes Needed
None. The public APIs are confirmed against registry source.

## Recommendations

### Proceed / No-Proceed
**Proceed.** The work is low-risk, reuses proven modules, and fills a real
operational gap. Estimated effort small (one crate, ~2 source files + tests).

### Scope Recommendations
- v1 ships `report` (default), `thinking`, `fast`, `workhorse`, `tiers`
  subcommands, human + json output, `--no-probe`, `--taxonomy`,
  `--slow-threshold`, env `ADF_TAXONOMY_DIR`.
- Defer: persistence, watch mode, cost-per-token, `--timeout`.

### Risk Mitigation Recommendations
- Add `jiff` as a direct dependency (do not rely on transitive).
- One unit test that loads the real in-repo taxonomy to guard against
  parser-contract drift.
- Print a clear "Mode: live probe (fires N API calls)" line so the default is
  never surprising.

## Next Steps
If approved:
1. Produce the Phase 2 Implementation Plan (next document).
2. Get human sign-off on default-probe assumption.
3. Implement TDD (Step sequence per the design doc).

## Appendix -- Key API signatures (verified against registry 1.20.2)
```rust
// terraphim_automata::markdown_directives
pub fn parse_markdown_directives_dir(root: &Path)
    -> Result<MarkdownDirectivesParseResult>; // { directives: HashMap<String, MarkdownDirectives>, warnings }

// terraphim_types
pub struct MarkdownDirectives {
    pub routes: Vec<RouteDirective>,
    pub priority: Option<u8>,
    pub heading: Option<String>,
    /* ... */
}
pub struct RouteDirective {
    pub provider: String,
    pub model: String,
    pub action: Option<String>,
    pub is_free: bool,
    pub fn cli_basename(&self) -> Option<&str>;
    pub fn route_key(&self) -> String;
}

// terraphim_orchestrator::kg_router
impl KgRouter { pub fn load(dir: impl Into<PathBuf>) -> Result<Self, KgRouterError>; }

// terraphim_orchestrator::provider_probe
pub struct ProbeResult {
    pub provider: String, pub model: String, pub cli_tool: String,
    pub status: ProbeStatus, pub latency_ms: Option<u64>,
    pub error: Option<String>, pub timestamp: String,
}
pub enum ProbeStatus { Success, Error, Timeout, RateLimited }
impl ProviderHealthMap {
    pub fn new(ttl: Duration) -> Self;
    pub async fn probe_all(&mut self, router: &KgRouter);
    pub fn results(&self) -> &[ProbeResult];
}
// probe_single internal: 15s hard timeout, "echo hello" prompt,
//   skips if !is_allowed_provider(model) (C1 gate),
//   returns env error if CLI not on PATH.
```
