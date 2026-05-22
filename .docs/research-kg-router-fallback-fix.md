# Research Document: ADF KG-Router Fallback Fix

**Status**: Draft
**Author**: alex
**Date**: 2026-05-22
**Reviewers**: alex
**Scope**: `crates/terraphim_orchestrator`

## Executive Summary

ADF agents whose task text matches an `implementation_tier` synonym are forcibly re-routed by the KG tier router to `anthropic/sonnet` on every spawn, even when the per-agent config or a quota-fallback respawn explicitly selected a different provider/model. When Anthropic's session quota is exhausted, this produces a tight respawn loop that exits with "no healthy KG route available" within ~30-90 s, repeatedly. Three compounding causes: (1) `parse_reset_time` cannot parse Claude Code's actual quota message, so the rate-limit window is never set; (2) `provider_probe`'s circuit breaker needs 5 consecutive failures before flipping the provider to unhealthy; (3) the fallback respawn path mutates `cli_tool` and `model` on the spawn def, but `spawn_agent` re-runs KG tier routing on the unchanged `task` text and overwrites both. This document scopes the research for two work items: a combined fix for (1)+(2)+(3) (the "Option 2+1" plan), and an ADR capturing why we are deferring Option 3 (invert the static-config vs KG-router precedence).

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | ADF North Star priority -- 5+ overnight agents reliable by 2026-06-15. Today's bigbox journal shows 0% throughput on implementation-swarm after Anthropic quota hit. |
| Leverages strengths? | Yes | Lives entirely inside `terraphim_orchestrator`, a crate we own. No external coordination. |
| Meets real need? | Yes | implementation-swarm-A has been a stuck zombie process since 2026-05-21 17:00 (~17 h); swarm-B exits 1 every cron tick. compliance-watchdog hit the same bug at 2026-05-22 00:05 UTC. |

**Proceed**: Yes (3/3).

## Problem Statement

### Description

The KG tier router in `crates/terraphim_orchestrator/src/lib.rs:1917-1964` re-routes every spawn based on the task prompt's matched concept, even when the spawn was constructed precisely to bypass the failed primary route. The fallback respawn at `lib.rs:6821-6841` carefully sets `fallback_def.cli_tool = opencode`, `fallback_def.model = kimi-for-coding/k2p6`, `fallback_def.provider = None`, `fallback_def.fallback_provider = None`, then calls `spawn_agent(&fallback_def)` -- which then ignores all of that and re-selects anthropic/sonnet from `implementation_tier.md`, respawning into the very provider that just rate-limited.

Two secondary issues amplify the loop: Claude Code's quota error format ("5-hour limit reached ∙ resets 11pm" or similar US-locale strings) does not match `parse_reset_time`'s "resets in N hour/minute" or "resets at HH:MM utc" patterns, so `ProviderRateLimitWindow::block_until` is never called. And `provider_probe::ProviderHealthMap` requires `failure_threshold: 5` consecutive failures (`provider_probe.rs:68`) before a breaker opens, so a single quota exit does not put `claude-code` into `unhealthy_providers()`.

### Impact

- **Today on bigbox**: implementation-swarm-A is a zombie since May 21 17:00; implementation-swarm-B and compliance-watchdog exit `rate_limit` within ~30 s on every cron firing (~every 25-30 min in active window). Zero useful work produced today.
- **Pattern**: any agent on a sonnet-tier task (i.e. any agent whose task text contains "implement / code / fix / cargo / PR review / test / security audit / merge / documentation") will exhibit this when Anthropic quota is hit, regardless of its `cli_tool` and `fallback_provider` config.
- **North Star miss**: ADF stabilisation target (5+ agents reliable overnight by 2026-06-15) cannot be met while quota-recovery is broken.

### Success Criteria

1. After a Claude Code session-limit error, the next cron firing of the same agent (or any other implementation-tier agent) must spawn against the configured `fallback_provider`/`fallback_model` (e.g. `opencode/kimi-for-coding/k2p6`), not against `anthropic/sonnet`.
2. The configured fallback choice persists at least until the quota reset window expires, without requiring 5 prior failures to accumulate.
3. compliance-watchdog and implementation-swarm-{A,B} successfully complete a full cycle (claim → branch → comment) on at least one cron firing within 6 hours of a quota event.
4. No new regressions: agents that have not configured a fallback continue to use KG tier routing as today; healthy-state behaviour is unchanged.

## Current State Analysis

### Existing Implementation

#### KG tier-routing call site (every spawn)

```rust
// crates/terraphim_orchestrator/src/lib.rs:1917-1964
} else if supports_model_flag {
    // KG routing first (phase-aware tier selection from markdown rules).
    // Takes priority over static model config so tier routing controls selection.
    let mut unhealthy = self.provider_health.unhealthy_providers();
    unhealthy.extend(self.provider_rate_limits.blocked_providers());
    let kg_decision = self.kg_router.as_ref().and_then(|router| {
        let decision = router.route_agent(&def.task)?;
        if !unhealthy.is_empty() {
            if let Some(healthy_route) = decision.first_healthy_route(&unhealthy) {
                return Some(KgRouteDecision { /* fallback route */ });
            }
        }
        Some(decision) // <-- BUG: returns primary even if unhealthy, when filter returned None
    });
    if let Some(ref kg) = kg_decision {
        // ...
        if let Some(ref action) = kg.action {
            if let Some(cli) = action.split_whitespace().next() {
                kg_cli_override = Some(cli.to_string()); // <-- overrides fallback_def.cli_tool
            }
        }
        Some(kg.model.clone()) // <-- overrides fallback_def.model
    } else if let Some(m) = &def.model { /* static fallback */ }
```

Two design points: (a) routing input is `def.task` (the prompt body), not the AgentDef's CLI/model fields, so any operator intent encoded in those fields is invisible to the router; (b) when `unhealthy` is non-empty but no healthy fallback route exists, the closure still returns `Some(decision)` with the unhealthy primary -- there is no "abort" path.

#### Quota exit detection and provider blocking

```rust
// crates/terraphim_orchestrator/src/lib.rs:6493-6520
if let Some(provider_key) = effective_provider {
    warn!(/* quota exit detected */);
    self.provider_health.record_failure(provider_key);    // +1 failure on breaker
    if let Some(tracker) = self.provider_budget_tracker.as_ref() {
        tracker.force_exhaust(provider_key);
    }
    let quota_line = stderr_lines.iter().chain(stdout_lines.iter())
        .find(|l| l.to_lowercase().contains("resets "))
        .map(|s| s.as_str()).unwrap_or("");
    if let Some(reset_time) = parse_reset_time(quota_line) {
        self.provider_rate_limits.block_until(provider_key, reset_time);
    }
    // No else branch: if parse_reset_time returns None, NO window is set.
}
```

#### parse_reset_time (current parser)

```rust
// crates/terraphim_orchestrator/src/lib.rs:384-430
fn parse_reset_time(quota_line: &str) -> Option<Instant> {
    let line = quota_line.to_lowercase();
    if let Some(idx) = line.find("resets in ") { /* N hour|minute */ }
    if let Some(idx) = line.find("resets at ") { /* HH:MM utc */ }
    None
}
```

Patterns recognised:

- `"5-hour limit reached, resets in 4 hours"` -> Some(+4 h)
- `"resets at 23:00 UTC"` -> Some(today/tomorrow 23:00 UTC)

Patterns NOT recognised (observed in Claude Code stderr):

- `"5-hour limit reached ∙ resets 11pm"` (no "in", no "at", AM/PM not UTC)
- `"You've hit your session limit. Resets 11:00pm"` (no "at")
- `"resets in 4h"` (`h` abbreviation, no word "hour")

#### Quota-exit fallback chain

```rust
// crates/terraphim_orchestrator/src/lib.rs:6763-6845
if quota_exits.contains(&name) {
    let mut local_unhealthy = self.provider_health.unhealthy_providers();
    local_unhealthy.extend(self.provider_rate_limits.blocked_providers());

    let respawned = if let Some(ref kg_router) = self.kg_router {
        if let Some(decision) = kg_router.route_agent(&def.task) {
            if let Some(healthy_route) = decision.first_healthy_route(&local_unhealthy) {
                // KG fallback respawn -- works when breaker is already open
                /* ... build fallback_def from healthy_route, spawn ... */
                true
            } else {
                info!("no healthy KG route available, agent exits permanently");
                false
            }
        } else { false }
    } else { false };

    if !respawned {
        if def.fallback_provider.is_some() {
            info!("KG routing failed, respawning with configured fallback provider");
            let mut fallback_def = def.clone();
            fallback_def.cli_tool = def.fallback_provider.clone().unwrap();
            fallback_def.model = def.fallback_model.clone();
            fallback_def.provider = None;
            fallback_def.fallback_provider = None;
            fallback_def.fallback_model = None;
            self.spawn_agent(&fallback_def).await  // <-- re-enters KG routing, overrides everything
        }
    }
}
```

#### Circuit breaker threshold

```rust
// crates/terraphim_orchestrator/src/provider_probe.rs:60-75
pub fn new(ttl: Duration) -> Self {
    Self {
        cb_config: CircuitBreakerConfig {
            failure_threshold: 5,           // <-- 5 consecutive failures before opening
            cooldown: Duration::from_secs(300),
            success_threshold: 1,
        },
        // ...
    }
}
```

`record_failure` (`provider_probe.rs:352-380`) increments breakers prefixed `{provider}:`. If none exist, it creates `{provider}:*` and increments. So the first quota exit creates `claude-code:*` with 1 failure; the breaker needs 4 more to open. Until then, `unhealthy_providers()` does not list `claude-code`.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| KG tier routing call site | `crates/terraphim_orchestrator/src/lib.rs:1917-1964` | Re-routes on every spawn from `def.task`; overrides cli_tool & model |
| Quota detection | `crates/terraphim_orchestrator/src/lib.rs:6480-6520` | Detects rate-limit exit, calls `record_failure` + (maybe) `block_until` |
| `parse_reset_time` | `crates/terraphim_orchestrator/src/lib.rs:384-430` | Parses reset window from stderr text |
| Quota-exit fallback chain | `crates/terraphim_orchestrator/src/lib.rs:6763-6845` | KG-fallback then configured-fallback respawn |
| `KgRouter::route_agent` | `crates/terraphim_orchestrator/src/kg_router.rs:182-245` | Matches `def.task` against taxonomy synonyms |
| `KgRouteDecision::first_healthy_route` | `crates/terraphim_orchestrator/src/kg_router.rs:54-63` | Filters routes by unhealthy provider list (canonical-key aware) |
| `ProviderHealthMap::record_failure` | `crates/terraphim_orchestrator/src/provider_probe.rs:352-380` | Increments breaker, auto-creates if missing |
| `ProviderHealthMap::unhealthy_providers` | `crates/terraphim_orchestrator/src/provider_probe.rs:291-329` | Returns providers whose breakers are all-open |
| `CircuitBreakerConfig` | `crates/terraphim_orchestrator/src/provider_probe.rs:60-75` | `failure_threshold: 5`, `cooldown: 300 s` |
| `canonical_quota_key` | `crates/terraphim_orchestrator/src/provider_budget.rs:380-403` | Collapses `anthropic` -> `claude-code` |
| `AgentDefinition` | `crates/terraphim_orchestrator/src/config.rs:689-778` | Per-agent config; extension point for `bypass_kg_routing` |
| `ProviderRateLimitWindow` | `crates/terraphim_orchestrator/src/lib.rs:347-382` | Time-bounded block list, fed by `parse_reset_time` |
| Routing taxonomy (data) | `docs/taxonomy/routing_scenarios/adf/implementation_tier.md` | anthropic/sonnet primary; kimi, openai, zai fallbacks |

### Data Flow (current, quota-hit case)

```
cron fires implementation-swarm-B
  -> spawn_agent(def)
     -> KG route_agent(def.task) -> implementation_tier -> anthropic/sonnet
     -> unhealthy = [] (first quota of the day; breaker not yet open)
     -> kg_cli_override = "/home/alex/.local/bin/claude"
     -> Some("sonnet") overrides def.model="kimi-for-coding/k2p6"
  -> claude exits 1, stderr "you've hit your session limit"
  -> exit classifier: ExitClass::RateLimit
  -> record_failure("claude-code") (breaker: 1/5 failures)
  -> parse_reset_time("you've hit your session limit") -> None
  -> ProviderRateLimitWindow: no block set
  -> quota_exits branch:
     -> first_healthy_route(local_unhealthy=[]) -> Some(anthropic/sonnet)  [no filter]
     -> respawn into anthropic/sonnet again -> fails identically
     -> after 5 cycles, breaker opens, claude-code in unhealthy
     -> first_healthy_route([claude-code]) -> Some(kimi)  [now works]
     -> BUT: spawn_agent(fallback_def with cli=opencode, model=kimi)
        -> re-runs route_agent(def.task) -> implementation_tier
        -> if claude-code in unhealthy, first_healthy_route picks kimi  [works]
        -> if claude-code NOT in unhealthy (e.g. breaker reset by `cooldown=300s`),
           re-picks anthropic/sonnet  [back to the loop]
```

Observed deviation in today's journal: "no healthy KG route available" was logged at 06:31:49 -- this means by then the breaker had already opened from prior runs and pushed claude-code into `unhealthy`, AND `first_healthy_route` couldn't find a non-unhealthy route. Hypothesis: `kimi`, `openai`, `zai` were also marked unhealthy by stale entries from earlier today (e.g. from probe failures or other agents). Worth confirming when the orchestrator is restarted -- expected fresh state to show "no healthy route" only after multiple failures across multiple providers.

### Integration Points

- **`ExitClassifier` (`control_plane::output_parser`)** classifies stderr; matches `"you've hit your session limit"` to `ExitClass::RateLimit`. Authoritative source of "this was a quota".
- **`canonical_quota_key`** (`provider_budget.rs:398`) collapses `anthropic` -> `claude-code`. Used by both `record_failure` attribution and `first_healthy_route` filter, so the mapping is consistent. No change needed here.
- **Taxonomy markdown** (`docs/taxonomy/routing_scenarios/adf/implementation_tier.md`) lists routes in priority order. Healthy data; no change in scope.

## Constraints

### Technical Constraints

- **Rust workspace, edition 2024**: changes must compile under the workspace toolchain.
- **`tokio` async**: all spawn paths are async; new flags must thread through `spawn_agent` without changing its `async fn` signature in a breaking way (it has many call sites).
- **Backwards compatibility with existing TOML config**: `AgentDefinition` is `#[serde(default)]`-heavy; any new field must default such that existing `conf.d/*.toml` files keep working unchanged. `bool` default `false` is fine.
- **Subscription-based providers**: cannot probe by issuing a free request -- a probe-on-quota-detect approach is not possible. Must rely on stderr signatures + recorded state.
- **No external dependencies** for parse changes -- regex would be acceptable but plain `str::find` matches existing style.

### Business Constraints

- **ADF stabilisation target 2026-06-15**: this fix is on the critical path.
- **Anthropic budget already committed**: cannot drop sonnet usage entirely; just need to honour cooldown.
- **Cannot restart orchestrator casually**: it holds child-process state and recovery is expensive; design must be deployable via a `systemctl restart adf-orchestrator` not via in-place hot reload.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Quota-recovery latency | < 1 cron tick (~30 min) | Effectively never (loops indefinitely) |
| Per-tick CPU cost | Unchanged | -- |
| No new dependencies | true | -- |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| A single quota exit must mark the provider unhealthy for the cooldown window | Otherwise the next cron tick re-spawns into the same blocked provider | Today's journal: 5 cycles wasted before breaker opened |
| `spawn_agent` must distinguish "operator chose this CLI/model" from "no choice yet" | Without this, fallback respawn cannot escape KG re-routing | `lib.rs:6829-6841` mutates fields that `lib.rs:1917-1964` ignores |
| Claude Code's actual quota message must be parseable | Otherwise `ProviderRateLimitWindow` is dead code for the most common provider | `parse_reset_time` returns None for observed strings |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Generic per-provider stderr regex configuration | Two patterns (Claude + the existing UTC pattern) suffice today; deferred until a third provider needs a custom format. |
| Probe-on-quota-detect (issuing a tiny probe request to confirm the block) | Subscription providers don't support cheap probes; reset-time parsing covers 99% of cases. |
| Refactor `spawn_agent` into a state machine | Out of scope; surgical changes only. |
| Hot-reload of `failure_threshold` from config | Constant is fine; if tuning is ever needed it can be exposed later. |
| Option 3 (invert KG-vs-static-config precedence) | Captured in ADR-040, deferred. See Risks section. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `terraphim_orchestrator::dispatcher` | Uses `AgentDefinition`; adding a field affects serde-load paths | Low (additive `#[serde(default)]`) |
| `terraphim_orchestrator::control_plane::output_parser` | Source of "you've hit your session limit" classification | Low (read-only) |
| `terraphim_orchestrator::provider_budget::canonical_quota_key` | Used to attribute quota to `claude-code` from `sonnet` | None (no change) |
| `terraphim_orchestrator::kg_router` | Provides the routing decision; filter already canonical-aware | Low (no signature change planned) |
| Tests in `crates/terraphim_orchestrator/src/lib.rs` (many) | Build `AgentDef` instances inline | Medium -- the file already has many `fallback_provider: None,` initialisers; new field needs defaults added everywhere |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `chrono` (date parsing) | workspace pin | None (already used in `parse_reset_time`) | -- |
| `serde` | workspace pin | None | -- |

No new crates required.

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| New `bypass_kg_routing` flag is forgotten in some respawn paths | Med | High (silent regression to current bug) | Centralise spawn-respawn construction in a helper; unit test that fallback respawn sets the flag |
| Aggressive "block immediately on one quota" causes false blocks if a transient network error is misclassified as quota | Low | Med (provider blocked for full cooldown) | Only block on `ExitClass::RateLimit` (already pattern-matched on "session limit"); fall back to existing 5-failure breaker for other failure modes |
| Claude Code error message changes again | Med (Anthropic ships UI changes) | Med (parser silently fails) | Add a structured warn-log when `ExitClass::RateLimit` is set but `parse_reset_time` returns None; capture the unknown line in `terraphim-agent learn` so we notice |
| Tests that exercise the KG routing override may break if we change semantics | Med | Med | Read existing tests in `lib.rs` (5+ inline AgentDef constructors); update with `bypass_kg_routing: false` default |
| Stale `implementation-swarm-A` zombie process blocks worktree slot | Cert | Low | Out of scope; document the `kill 2743444` as an operator step in the rollout note |

### Open Questions

1. **What's the exact stderr string Claude Code emits on session-limit?** Need to capture from bigbox journal. The classifier already matches "you've hit your session limit"; need the line that contains "resets" to build the parser test fixture. (Investigation: `journalctl -u adf-orchestrator | rg -i 'resets' -B 2 -A 2 | head -50` on bigbox.)
2. **Does `ProviderRateLimitWindow::block_until` correctly survive cooldown semantics?** Confirmed: `lib.rs:358-376`, time-based, no manual unblock required.
3. **Should `bypass_kg_routing` also skip the keyword-routing engine fallback at `lib.rs:1970-1990`?** Probably yes -- if the operator picked a CLI/model, no router should override. To be confirmed in design phase.
4. **Are there other respawn sites that build a `fallback_def` and call `spawn_agent`?** Need to grep for `fallback_def`, `with_fallback_provider`, and any other code that mutates def fields pre-spawn.
5. **Does the existing `unhealthy_providers()` mid-spawn filter at `lib.rs:1920-1944` need to also consult `provider_rate_limits.blocked_providers()` AFTER our parse fix?** Reading the code: it already does (line 1921 extends `unhealthy` with blocked providers). So the parse fix alone closes the loop here.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| Claude Code's session-limit message contains the substring "resets" somewhere | The quota-detect code at `lib.rs:6505-6510` already searches stderr for any line containing "resets ", and the journal shows quota detection working, so the string IS present | Med -- if it's just "5h limit hit, retry later" the parser can't find the window | No -- need to grep bigbox journal |
| One quota exit means provider is fully blocked until reset | True for Anthropic Claude Code: session limit is a hard block per Anthropic's docs | Low -- worst case we set a too-long block and miss some uptime | Yes (Anthropic docs + observed behaviour) |
| `unhealthy_providers()` returning `claude-code` is sufficient to make `first_healthy_route` skip `anthropic` | Confirmed: `kg_router.rs:54-63` uses `canonical_quota_key(&r.provider)` to compare, and `canonical_quota_key("anthropic") == "claude-code"` | Low | Yes (code read) |
| Adding `bypass_kg_routing: bool` to `AgentDefinition` is additive | `#[serde(default)]` covers TOML files lacking the field | Low | Yes (serde behaviour) |
| Tests for `parse_reset_time` are easy to add | Pure function, no I/O | Low | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Fix only the parser (Option 2 alone) | Smaller change. Once parser works, `block_until` populates and `unhealthy_providers()` includes claude-code at next tick, so KG router's existing filter picks kimi. | Rejected as sufficient: it still relies on the parser handling every future Claude message variant. Combining with the bypass flag is belt-and-braces and gives an escape hatch when stderr formats drift. |
| Fix only the bypass flag (Option 1 alone) | Even smaller change. Once respawn sets `bypass_kg_routing=true`, fallback uses operator-chosen CLI/model verbatim. | Rejected as sufficient: leaves the broken loop intact for the first quota cycle (cron fires fresh spawn -> KG re-routes -> hits quota -> fallback works). With parser fix, the very next cron tick also skips Claude. |
| Lower `failure_threshold` from 5 to 1 globally | One-line change. | Rejected: would also flip transient failures (network blip, single timeout) into long blocks. Quota is structurally different from flakiness; needs a dedicated fast-path, not a globally lowered threshold. |
| Make KG routing read `def.cli_tool`/`def.model` and skip when set | Closest to "operator intent wins". | Rejected for now; that's Option 3 and goes in ADR-040. Risk is regressing healthy-state agents that intentionally rely on KG selection. |

## Research Findings

### Key Insights

1. **The KG router has no "stop overriding" signal.** Its only input is `def.task`. Operator config (`cli_tool`, `model`, `fallback_*`) and runtime state (just-failed provider) reach it only via the `unhealthy_providers()` filter -- which has a 5-failure deadband. A single bit on `AgentDefinition` to short-circuit the router closes this gap with minimal surface area.
2. **`parse_reset_time` is the load-bearing parser for quota recovery.** Without a reset-time, `ProviderRateLimitWindow` is empty and the only path to "claude-code is unhealthy" is the slow 5-failure breaker. Fixing the parser collapses the gap from 5 cron cycles to 1.
3. **`canonical_quota_key` is already correct.** `anthropic` and `claude-code` collapse to the same key. We do not need to touch attribution.
4. **The quota-fallback respawn already has the right intent in its code comments** ("respawning with configured fallback provider"). It just lacks the signalling to make the inner `spawn_agent` honour it.
5. **`first_healthy_route` has a latent "all routes unhealthy" silent fallthrough.** When the closure at `lib.rs:1922-1947` calls `first_healthy_route` and gets `None`, the outer `Some(decision)` still returns the unhealthy primary. Cosmetic issue in healthy-state operation, but if we're touching this block we should explicitly return `None` to fall through to `def.model` -- this is in scope for the design phase.

### Relevant Prior Art

- **Hystrix / resilience4j circuit breakers**: standard pattern of (a) fast trip on hard errors, (b) gradual trip on soft errors. Our hard error (quota) deserves fast trip.
- **Netflix Concurrency Limits**: separation of "rejection class" (hard limit vs transient) maps to our `ExitClass::RateLimit` vs `ExitClass::ModelError` distinction. Confirms the design instinct of treating them differently.
- **Anthropic Claude Code CLI documentation**: subscription session limits are time-window-based, not retryable -- structurally matches an `Instant`-bounded block list, not a probe-based breaker.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Capture Claude Code session-limit stderr verbatim from bigbox journal | Build parser test fixture from real text | 15 min |
| Grep for all `fallback_def` and pre-spawn `def.clone()` sites | Ensure `bypass_kg_routing` is set in every respawn path | 20 min |
| Audit inline `AgentDef` test constructors | Plan the additive-field migration without breaking tests | 30 min |

## Recommendations

### Proceed/No-Proceed

**Proceed** with Option 2+1 combined. Defer Option 3 with an ADR.

### Scope Recommendations

The implementation plan should be three small, independently-testable steps:

1. **Extend `parse_reset_time`** to handle: `"resets <time><am|pm>"`, `"resets in <N>h"`, `"resets in <N>m"`. Add a `warn!` log at the quota-detect site when `ExitClass::RateLimit` is set but `parse_reset_time` returns None (early-warning for future format drift).
2. **Add `bypass_kg_routing: bool` to `AgentDefinition`** (default false). At `lib.rs:1917`, skip the KG block when `def.bypass_kg_routing` is true. At `lib.rs:6829`, set `fallback_def.bypass_kg_routing = true` before `spawn_agent`.
3. **Fast-trip on `ExitClass::RateLimit`**: at the quota-detect site, additionally call `provider_rate_limits.block_until(provider, Instant::now() + Duration::from_secs(900))` when `parse_reset_time` returns None. 15 min is a conservative floor: long enough to skip the next cron tick, short enough that a misclassification doesn't waste a full day. This is the safety net for parse failures.

Each step is independently shippable; together they close the loop.

### Risk Mitigation Recommendations

- **Add a regression test** that simulates: quota exit -> next spawn must NOT spawn against the failed provider. Use a fake `KgRouter` that always returns anthropic/sonnet to lock the invariant.
- **Document the rollout** as: (a) merge PR, (b) restart `adf-orchestrator.service`, (c) `kill 2743444` (stale swarm-A zombie), (d) watch journal for one cron cycle.
- **Capture the exact Claude error string** before changing the parser so test fixtures match reality.

## Next Steps

If approved:

1. Capture the Claude Code session-limit stderr line from bigbox journal (see "Technical Spikes Needed").
2. Run the disciplined-design skill against this research doc to produce the implementation plan, including: file diffs, test additions, the three-step commit sequence.
3. Author ADR-040 documenting why Option 3 (invert KG-vs-static precedence) was deferred.
4. Implement under disciplined-implementation skill, one commit per step.

## Appendix

### Reference Materials

- Journal evidence (bigbox `sudo journalctl -u adf-orchestrator --since "20 hours ago"`):
  - 2026-05-22T00:05:19 -- compliance-watchdog: `model selected via KG tier routing concept=implementation_tier provider=anthropic model=sonnet`
  - 2026-05-22T00:05:49 -- `quota exit detected; recording provider failure and blocking provider=claude-code model=Some("sonnet")`
  - 2026-05-22T00:05:49 -- `no healthy KG route available, agent exits permanently`
  - 2026-05-22T00:05:49 -- `KG routing failed, respawning with configured fallback provider=opencode fallback_model=kimi-for-coding/k2p6`
  - 2026-05-22T00:05:49 -- `model selected via KG tier routing ... provider=anthropic model=sonnet confidence=0.5` (the bug: re-routes back to claude)
- Routing taxonomy: `docs/taxonomy/routing_scenarios/adf/implementation_tier.md` (4 routes: anthropic/sonnet, kimi, openai, zai)
- Per-agent config sample (current):
  ```toml
  # /opt/ai-dark-factory/conf.d/terraphim.toml, compliance-watchdog
  cli_tool = "/home/alex/.bun/bin/opencode"
  fallback_model = "kimi-for-coding/k2p6"
  # observed: KG router ignores cli_tool and forces claude/sonnet
  ```

### Code Snippets (verbatim, for design-phase context)

```rust
// crates/terraphim_orchestrator/src/kg_router.rs:54-63 -- the filter that works correctly
// once unhealthy_providers() actually contains "claude-code"
pub fn first_healthy_route(&self, unhealthy_providers: &[String]) -> Option<&RouteDirective> {
    let unhealthy_canonical: HashSet<&str> = unhealthy_providers
        .iter()
        .map(|p| crate::provider_budget::canonical_quota_key(p))
        .collect();
    self.fallback_routes.iter().find(|r| {
        let route_canonical = crate::provider_budget::canonical_quota_key(&r.provider);
        !unhealthy_canonical.contains(route_canonical)
    })
}
```

```rust
// crates/terraphim_orchestrator/src/provider_probe.rs:60-75 -- the deadband to soften (or sidestep)
cb_config: CircuitBreakerConfig {
    failure_threshold: 5,           // 5 consecutive failures before opening
    cooldown: Duration::from_secs(300),
    success_threshold: 1,
},
```
