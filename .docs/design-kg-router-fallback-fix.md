# Implementation Plan: ADF KG-Router Fallback Fix

**Status**: Draft
**Research Doc**: `.docs/research-kg-router-fallback-fix.md`
**Author**: alex
**Date**: 2026-05-22
**Estimated Effort**: ~6 hours (3 steps × ~2 h each)

## Overview

### Summary

Close the quota-recovery loop in `terraphim_orchestrator` so that one Claude Code session-limit hit is sufficient to redirect subsequent spawns to the configured fallback provider (e.g. `opencode/kimi-for-coding/k2p6`) until the reset window expires, instead of looping back into the rate-limited provider for ~5 cron cycles.

### Approach

Three independent, surgical changes -- each one a shippable commit:

1. **Parser fix** -- teach `parse_reset_time` Claude Code's actual stderr formats (`"resets 11pm"`, `"resets in 4h"`, `"resets in 30m"`) and add a `warn!` log when classification is `RateLimit` but no reset window could be parsed (early-warning for future Anthropic format drift).
2. **Bypass flag** -- add `bypass_kg_routing: bool` to `AgentDefinition` (defaults `false`). `spawn_agent` skips the KG tier-routing block when set. The quota-exit fallback respawn sets the flag on the `fallback_def` so the operator-chosen `cli_tool`/`model` are honoured.
3. **Safety net** -- on `ExitClass::RateLimit` when `parse_reset_time` returns `None`, set a conservative 15-minute block via `provider_rate_limits.block_until` so we skip at least the next cron tick even if the parser can't extract the precise window.

### Scope

**In Scope:**
- `parse_reset_time` parser extension (lib.rs:384-430)
- `AgentDefinition.bypass_kg_routing` field (config.rs:689)
- `spawn_agent` KG block gating (lib.rs:1917-1964)
- Fallback respawn flag setting (lib.rs:6829-6841)
- `provider_rate_limits.block_until` safety floor (lib.rs:6493-6520)
- Unit tests for the parser, the bypass flag, and the safety floor
- Rollout: PR, `systemctl restart adf-orchestrator`, `kill 2743444` (stale swarm-A zombie)

**Out of Scope:**
- Re-ordering KG-vs-static-config precedence (Option 3 -- captured in ADR-040, deferred)
- Per-provider regex configuration for stderr patterns
- Probe-on-quota-detect for subscription providers
- Refactor of `spawn_agent` into a state machine
- Changes to `canonical_quota_key` or `first_healthy_route`
- Changes to the routing taxonomy (`docs/taxonomy/routing_scenarios/adf/`)
- Lowering `failure_threshold` globally

**Avoid At All Cost** (from 5/25 analysis):
- A generic "provider drivers" abstraction with per-provider hooks. Tempting but speculative -- two patterns work today.
- A new event bus / message channel for spawn lifecycle events. We have direct call sites; a bus would be ceremony without benefit.
- Hot-reload of the orchestrator config. Designed in, never used; would multiply the surface area of this fix.
- Telemetry/Prometheus counters for fallback hits. Useful eventually, but a journal grep is sufficient diagnostics today.
- Migrating `parse_reset_time` to `regex` crate. Plain `str::find` matches existing style; one-line dependency is not worth it for three patterns.

## Architecture

### Component Diagram (the spawn lifecycle around quota recovery)

```
+--------------------------------------------------------------+
|                      Orchestrator (lib.rs)                   |
|                                                              |
|  cron fires                                                  |
|     v                                                        |
|  spawn_agent(def)  --[1917]-->  KG tier router               |
|     |                            |                            |
|     |                            +-- if def.bypass_kg_routing |
|     |                            |     SKIP (NEW)             |
|     |                            +-- else: route_agent        |
|     |                                  + first_healthy_route  |
|     v                                                        |
|  child exits 1                                               |
|     v                                                        |
|  ExitClassifier  -->  ExitClass::RateLimit                   |
|     v                                                        |
|  [6493] quota-exit handler                                   |
|     +-- record_failure("claude-code")  [breaker +1]          |
|     +-- parse_reset_time(stderr)  --[NEW: more patterns]-->  |
|     |     |                                                  |
|     |     +-- Some(t) -> block_until("claude-code", t)       |
|     |     +-- None    -> warn!  +  block_until(now+15min)    |
|     |                                                (NEW)   |
|     v                                                        |
|  [6763] quota-exit fallback                                  |
|     +-- KG fallback respawn (existing)                       |
|     +-- configured-fallback respawn:                         |
|         fallback_def.bypass_kg_routing = true  (NEW)         |
|         spawn_agent(&fallback_def)  -->  honours cli/model   |
+--------------------------------------------------------------+
```

### Data Flow (post-fix, quota-hit case)

```
cron fires implementation-swarm-B at t=T
  -> spawn_agent(def)
     KG route -> anthropic/sonnet (unhealthy list empty, first run)
     spawn claude --model sonnet
  -> claude exits 1, stderr "you've hit your session limit, resets 11pm"
  -> ExitClass::RateLimit
     record_failure("claude-code")           [breaker 1/5, not yet open]
     parse_reset_time(...) -> Some(T+4h)     [NEW: parses "resets 11pm"]
     block_until("claude-code", T+4h)
  -> quota-exit fallback chain:
     local_unhealthy = ["claude-code"]       [via block list, not breaker]
     KG fallback: first_healthy_route -> Some(kimi)
     respawn with cli=opencode, model=kimi-for-coding/k2p5, bypass_kg_routing=true
     spawn_agent honours fields, skips KG re-routing      [NEW]
  -> kimi cycle runs to completion
cron fires implementation-swarm-B at t=T+30min
  -> spawn_agent(def)  [original def, no bypass]
     KG route -> route_agent + first_healthy_route(["claude-code"]) -> kimi
     spawn opencode --model kimi-for-coding/k2p5
  -> normal kimi run
...quota window expires at T+4h, claude-code re-enters healthy set...
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|-----------------------|
| Add a per-`AgentDefinition` `bypass_kg_routing` bool | One bit, default false, additive serde -- minimum surface area | (a) Pass a `force_provider` Option through `spawn_agent` args -- changes the function signature at many call sites; (b) Make `def.provider`+`def.model` set together imply "no routing" -- magical and easy to forget |
| Conservative 15 min safety block when parser fails | Long enough to skip one cron cycle (active window has 30 min between firings); short enough to not waste a whole day on a misclassification | (a) Block until breaker opens via 5 failures (status quo, demonstrated broken); (b) Block for full default 5h (too punishing on misclassification) |
| Keep `failure_threshold: 5` unchanged globally | Quota is structurally different from flakiness; treat it separately rather than globally lowering the threshold | Lowering to 1 would flip transient network errors into long blocks |
| `parse_reset_time` stays a plain function, no regex crate | Three patterns are easy to express with `str::find` + char-iter; matches existing code style; no new dependency | `regex` crate adds compile-time cost and a runtime dependency for trivial parsing |
| Warn-log when `RateLimit` + `parse_reset_time` returns None | Early-warning that Anthropic shipped a new format; lets us add a pattern before the safety net masks the drift | Silent fallback (status quo) -- no signal that the parser is going stale |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Invert KG-vs-static-config precedence (Option 3) | Behaviour change for all healthy-state agents; needs ADR + broader review | Regressions in agents that *want* tier routing |
| Add Prometheus counters for fallback events | Diagnostics work today via journal grep | Yak-shaving the metrics pipeline before the fix lands |
| Refactor `spawn_agent` into a state machine | Surgical changes are sufficient; refactor is a separate piece of work | Scope explosion; review cost; bigger blast radius |
| `bypass_kg_routing` as a `RoutingMode` enum (Auto/ForceConfig/ForceKg) | Two states cover today's need; three states are speculative | Premature flexibility |
| Per-provider stderr signature TOML | One provider needs a custom format today | Configuration burden out of proportion to value |

### Simplicity Check

**What if this could be easy?**

The simplest possible design is: when the fallback path builds its `fallback_def`, set one extra field; `spawn_agent` checks that field at the top of its KG block; the parser learns three new substrings. That is what this plan does. Total code change is well under 200 lines including tests.

**Senior Engineer Test**: A senior engineer would call this *under*-complicated and ask "are you sure that's enough?". The research doc's evidence (journal traces, code reads, canonical-key analysis) is the answer: yes, the existing `first_healthy_route` filter and per-agent `fallback_provider` machinery already do the heavy lifting -- we just have to stop the KG router from undoing them.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later" (`bypass_kg_routing` is a single bool, not an enum or trait)
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimisation

## File Changes

### New Files

None.

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_orchestrator/src/lib.rs` | Extend `parse_reset_time` patterns (lines 384-430); add `warn!` and `block_until(now+15min)` on parser-None in quota-detect (lines 6493-6520); add `bypass_kg_routing` short-circuit in `spawn_agent` (lines 1917-1964); set `bypass_kg_routing = true` in fallback respawn (lines 6829-6841); update inline-test `AgentDefinition` constructors to include the new field (~15 sites listed below) |
| `crates/terraphim_orchestrator/src/config.rs` | Add `bypass_kg_routing: bool` field with `#[serde(default)]` to `AgentDefinition` (line 689) |

### Deleted Files

None.

### Inline test constructors needing the new field

From the existing grep (`rg "fallback_provider: None" crates/terraphim_orchestrator/src/lib.rs`):
- lib.rs:8031, 8059, 8380, 8499, 8706, 8796, 9053, 9223, 9319, 9615, 10016, 10359, 10654, 10936, 11230

Each gets `bypass_kg_routing: false,` added. (`#[serde(default)]` covers TOML, but Rust constructors must be explicit.)

## API Design

### New Field on `AgentDefinition`

```rust
// crates/terraphim_orchestrator/src/config.rs, inside AgentDefinition (after line 777)

/// If true, `spawn_agent` honours the explicit `cli_tool` and `model` on this
/// definition and skips KG tier-routing override. Set by the quota-exit fallback
/// respawn so that the operator-chosen fallback provider is not overridden by
/// the same tier-routing rule that selected the now-blocked primary.
///
/// Default `false` preserves existing behaviour for normal (non-fallback) spawns.
#[serde(default)]
pub bypass_kg_routing: bool,
```

### Modified Function Signatures

`parse_reset_time` keeps the same signature; its body grows to handle more cases:

```rust
// crates/terraphim_orchestrator/src/lib.rs:384
fn parse_reset_time(quota_line: &str) -> Option<Instant>;
```

`spawn_agent` keeps its signature; one new branch at the top of the KG block:

```rust
// crates/terraphim_orchestrator/src/lib.rs:1917
// (existing async fn signature unchanged)
// NEW: at the top of the `else if supports_model_flag` block:
if def.bypass_kg_routing {
    info!(agent = %def.name, "bypassing KG tier routing per agent definition");
    // fall through to the existing `def.model` static-config branch
} else {
    // ... existing KG routing block ...
}
```

### Helper for the safety-net floor

```rust
// crates/terraphim_orchestrator/src/lib.rs (private helper, near parse_reset_time)

/// Fallback block duration used when `parse_reset_time` can't extract a precise
/// window from the provider's stderr. Conservative enough to skip the next cron
/// firing (~30 min active-window cadence); short enough that a misclassified
/// non-quota error doesn't disable the provider for the day.
const DEFAULT_RATE_LIMIT_BLOCK: Duration = Duration::from_secs(900); // 15 min
```

Used at the quota-detect site:

```rust
// crates/terraphim_orchestrator/src/lib.rs:6511-6518 -- modified
if let Some(reset_time) = parse_reset_time(quota_line) {
    info!(provider = %provider_key, "blocking provider until rate-limit window expires");
    self.provider_rate_limits.block_until(provider_key, reset_time);
} else {
    warn!(
        provider = %provider_key,
        quota_line = %quota_line,
        "rate-limit detected but reset window could not be parsed; applying conservative {}s block",
        DEFAULT_RATE_LIMIT_BLOCK.as_secs(),
    );
    self.provider_rate_limits
        .block_until(provider_key, Instant::now() + DEFAULT_RATE_LIMIT_BLOCK);
}
```

### Fallback respawn flag (THREE sites, not one)

Spike (`rg -n "fallback_def" crates/terraphim_orchestrator/src/`) confirms three sites build a `fallback_def` and call `spawn_agent`. All three need `bypass_kg_routing = true`:

| Site | Trigger | Source of fallback choice |
|------|---------|---------------------------|
| `lib.rs:6233` | Wall-clock timeout fallback | `def.fallback_provider` / `def.fallback_model` |
| `lib.rs:6779` | Quota exit, KG-router picks a healthy fallback route | `KgRouteDecision::first_healthy_route` |
| `lib.rs:6828` | Quota exit, KG fallback unavailable, configured fallback | `def.fallback_provider` / `def.fallback_model` |

Setting the flag at all three sites prevents `spawn_agent`'s inner KG block from re-running and potentially overriding the explicit choice. For site 6779 specifically: the outer `first_healthy_route` already filtered for the current unhealthy set, but spawn_agent re-evaluates with a freshly-computed unhealthy set — if breaker state has drifted (e.g. a successful probe just landed), the inner filter could pick a different route from the outer. Setting bypass keeps the decision stable.

```rust
// At each of lib.rs:6233-6249, 6779-6800, 6828-6841 -- after the existing field assignments:
fallback_def.bypass_kg_routing = true;  // NEW: honour explicit cli_tool/model verbatim
```

## Test Strategy

All tests are real-implementation per project policy (no mocks). The orchestrator already has extensive inline `mod tests`; we extend it.

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `parse_reset_time_handles_pm_suffix` | `lib.rs` tests mod | `"resets 11pm"` -> Some, near 23:00 local interpreted as next reset |
| `parse_reset_time_handles_am_suffix` | `lib.rs` tests mod | `"resets 7am"` -> Some |
| `parse_reset_time_handles_h_abbreviation` | `lib.rs` tests mod | `"resets in 4h"` -> Some(+4h) |
| `parse_reset_time_handles_m_abbreviation` | `lib.rs` tests mod | `"resets in 30m"` -> Some(+30min) |
| `parse_reset_time_unknown_format_returns_none` | `lib.rs` tests mod | `"limit reached, try later"` -> None (no "resets" token) |
| `agent_def_bypass_kg_routing_defaults_false` | `config.rs` tests mod | TOML without the field deserialises to `false` |
| `agent_def_bypass_kg_routing_explicit_true` | `config.rs` tests mod | TOML `bypass_kg_routing = true` deserialises to `true` |
| `default_rate_limit_block_is_15_minutes` | `lib.rs` tests mod | Sanity check on `DEFAULT_RATE_LIMIT_BLOCK` constant |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `spawn_agent_with_bypass_skips_kg_routing` | `lib.rs` tests mod | Build a real `KgRouter` with a rule that would match the agent's task; build an `AgentDefinition` with `bypass_kg_routing=true, cli_tool=X, model=Y`; call into the routing-decision path; assert the chosen model is `Y`, not the KG primary |
| `spawn_agent_without_bypass_uses_kg_routing` | `lib.rs` tests mod | Same setup as above but `bypass_kg_routing=false`; assert the KG primary is chosen (regression guard for healthy-state behaviour) |
| `quota_exit_with_parseable_window_sets_block` | `lib.rs` tests mod | Construct quota line `"resets 11pm"`; drive through the quota-detect code path (call the helper directly if extracted, or via a minimal `handle_agent_exit`-style fixture); assert `provider_rate_limits.is_blocked("claude-code")` true |
| `quota_exit_with_unparseable_window_applies_safety_floor` | `lib.rs` tests mod | Quota line `"hit your session limit"` with no "resets" token; assert the 15-min fallback block is applied |
| `fallback_respawn_def_has_bypass_set` | `lib.rs` tests mod | Pure-function test: given a `def` with `fallback_provider=Some, fallback_model=Some`, the constructed `fallback_def` has `bypass_kg_routing == true` |

Note: where the existing code is too deeply embedded in `Orchestrator` to test directly, we extract the quota-line-to-block decision into a small private helper `decide_rate_limit_block(quota_line: &str) -> Duration` (or `Instant`) and test that. This is the only extraction the plan demands; no broader refactor.

### Existing tests to verify still pass

The 15+ inline `AgentDef` constructors mentioned above. After adding `bypass_kg_routing: false,` to each, `cargo test -p terraphim_orchestrator` must remain green.

### What we do not test

- We do not write an end-to-end test that spawns a real Claude process. The orchestrator tests already avoid this; our changes are at the routing-decision and exit-classification layers and are unit-testable.
- We do not test the bigbox journal output; that's verified by the rollout step.

## Implementation Steps

### Step 1: Parser fix + warn log

**Files:** `crates/terraphim_orchestrator/src/lib.rs`
**Description:**
- Extend `parse_reset_time` (lines 384-430) with three new patterns: `"resets <N>pm"` / `"resets <N>am"`, `"resets in <N>h"`, `"resets in <N>m"`.
- Add a `warn!` at the quota-detect call site (lines 6511-6518) when `parse_reset_time` returns None.
- Add a `DEFAULT_RATE_LIMIT_BLOCK = Duration::from_secs(900)` constant near `parse_reset_time`.
- Apply the safety-net block in the None branch.

**Tests:**
- New unit tests: 5 parser cases listed above.
- New integration test: `quota_exit_with_unparseable_window_applies_safety_floor`.
- New integration test: `quota_exit_with_parseable_window_sets_block`.

**Verification commands:**
```bash
cargo test -p terraphim_orchestrator parse_reset_time
cargo test -p terraphim_orchestrator quota_exit
cargo clippy -p terraphim_orchestrator -- -D warnings
```

**Commit message:**
```
fix(orchestrator): extend parse_reset_time for Claude Code session-limit formats

parse_reset_time previously only matched "resets in N hour/minute" or
"resets at HH:MM utc". Claude Code's actual session-limit message uses
shorter forms like "resets 11pm" or "resets in 4h", which fell through
to None -- leaving ProviderRateLimitWindow empty and forcing the
orchestrator to wait for the 5-failure breaker before treating
claude-code as unhealthy. That window allowed the KG tier router to
re-pick anthropic/sonnet for ~5 cron cycles after a quota hit.

Adds patterns for "resets <N>am|pm", "resets in <N>h", "resets in <N>m"
and a 15-minute conservative safety floor (DEFAULT_RATE_LIMIT_BLOCK)
applied when classification is RateLimit but the reset window can't be
parsed. The safety floor logs a warn! line carrying the offending
quota_line so we notice future format drift.

Refs #<gitea-issue-id>
```

**Estimated:** 2 h.

### Step 2: bypass_kg_routing flag

**Files:** `crates/terraphim_orchestrator/src/config.rs`, `crates/terraphim_orchestrator/src/lib.rs`
**Description:**
- Add `bypass_kg_routing: bool` to `AgentDefinition` (config.rs:689, `#[serde(default)]`).
- In `spawn_agent` KG block (lib.rs:1917-1964), short-circuit to the static-config branch when `def.bypass_kg_routing` is true.
- In the fallback respawn (lib.rs:6828-6837), set `fallback_def.bypass_kg_routing = true` before `spawn_agent(&fallback_def)`.
- Update the 15+ inline test constructors to include `bypass_kg_routing: false,`.

**Tests:**
- New unit tests: `agent_def_bypass_kg_routing_defaults_false`, `agent_def_bypass_kg_routing_explicit_true`.
- New integration tests: `spawn_agent_with_bypass_skips_kg_routing`, `spawn_agent_without_bypass_uses_kg_routing`, `fallback_respawn_def_has_bypass_set`.
- Existing test suite must remain green.

**Verification commands:**
```bash
cargo test -p terraphim_orchestrator bypass_kg_routing
cargo test -p terraphim_orchestrator fallback_respawn
cargo test -p terraphim_orchestrator  # full crate run
cargo clippy -p terraphim_orchestrator -- -D warnings
```

**Dependencies:** Step 1 not strictly required, but landing Step 1 first means each PR/commit can be reviewed and reverted independently; if Step 2 ships alone and Step 1 is reverted, the safety floor is gone but the bypass still works on the configured fallback.

**Commit message:**
```
feat(orchestrator): add bypass_kg_routing flag to AgentDefinition

The quota-exit fallback respawn at lib.rs:6829 carefully builds a
fallback_def with cli_tool=opencode and model=kimi-for-coding/k2p6, then
calls spawn_agent(&fallback_def) -- which then ignores both fields
because spawn_agent unconditionally re-runs KG tier routing on the
unchanged task text, re-matches the same tier rule, and re-picks
anthropic/sonnet. The agent immediately fails on the just-blocked
provider.

This adds bypass_kg_routing: bool (default false, additive serde) to
AgentDefinition. spawn_agent honours it by skipping the KG block and
falling through to the static def.model branch. The fallback respawn
sets the flag on the fallback_def so the operator-chosen CLI and model
are used verbatim.

Default-false preserves existing behaviour for all normally-scheduled
spawns; only the fallback respawn path opts in today.

Refs #<gitea-issue-id>
```

**Estimated:** 3 h (most of it is the inline-constructor migration and integration tests).

### Step 3: Roll out

**Files:** none in this repo. Operator steps on bigbox.
**Description:** sequencing of the deployment.

1. `gtr create-pull` for steps 1+2 (one PR per commit, or one PR with two commits -- either is fine; reviewer's call).
2. `gtr merge-pull` after review.
3. SSH to bigbox: `sudo systemctl restart adf-orchestrator`.
4. SSH to bigbox: `kill 2743444` (the stale implementation-swarm-A zombie from May 21 17:00).
5. Watch one cron cycle: `sudo journalctl -u adf-orchestrator --since "5 minutes ago" -f | rg -i 'swarm|compliance|rate_limit|fallback|bypass'`.
6. Verify in the journal:
   - At least one `bypassing KG tier routing per agent definition` line on a fallback respawn.
   - No `model selected via KG tier routing ... provider=anthropic model=sonnet` line immediately after `KG routing failed, respawning with configured fallback provider`.
   - Subsequent cron cycle picks kimi/opencode without going through Claude first.
7. After ~1 h of clean cycles, comment on the Gitea issue with the rollout outcome and close.

**Verification commands (run on bigbox):**
```bash
# 1. service restart
sudo systemctl restart adf-orchestrator
sudo systemctl status adf-orchestrator --no-pager

# 2. kill zombie
ps -o pid,lstart,cmd -p 2743444 2>/dev/null && kill 2743444

# 3. journal watch
sudo journalctl -u adf-orchestrator --since "5 minutes ago" -f \
  | rg -i 'swarm|compliance|rate_limit|fallback|bypass|kg tier'
```

**Estimated:** 1 h (mostly watching).

## Rollback Plan

If issues are discovered after rollout:

1. `gtr revert` or `git revert` the merge commit(s).
2. `cargo build --release -p terraphim_orchestrator && cargo install --path crates/terraphim_orchestrator` (or equivalent deploy step matching the bigbox `/usr/local/bin/adf` install path).
3. `sudo systemctl restart adf-orchestrator`.

The changes are field-additive (`bypass_kg_routing` defaults to `false`) and parser-additive (new patterns don't affect existing matches), so revert is clean. No data migration. No config schema change. The TOML files do not require updating.

Granular rollback:
- Step 1 alone reverts the parser changes; current behaviour returns (slow recovery via 5-failure breaker).
- Step 2 alone reverts the bypass flag; safety floor remains so recovery latency stays bounded but the configured fallback can still be re-routed by KG.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Quota-recovery latency | ≤ 1 cron tick (~30 min) | Journal trace of next swarm spawn after quota event |
| Spawn-decision CPU cost | Unchanged | Existing log timings remain comparable |
| Parser micro-cost | < 1 µs per call | Negligible; called only on quota exit |

The `bypass_kg_routing` short-circuit removes work from the hot path (skips an Aho-Corasick scan of `def.task`) but only when the flag is set, which today is only the fallback respawn -- so steady-state perf is unchanged.

### Benchmarks to Add

None required; the changes are not in a perf-critical path.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Capture exact Claude Code session-limit stderr line from bigbox journal before finalising parser patterns | RESOLVED: not available in journal (raw stderr is classified and dropped); learn store empty. Mitigation: ship plausible patterns (am/pm/h/m) + warn-log on parse failure + 15-min safety floor. First parse miss surfaces the verbatim string in the journal for follow-up iteration. | alex |
| Confirm there are no other sites (besides lib.rs:6829) that build a `fallback_def` and call `spawn_agent`; if so they must also set `bypass_kg_routing=true` | RESOLVED: three sites total -- lib.rs:6233 (timeout fallback), 6779 (KG-router fallback respawn), 6828 (configured-fallback respawn). All three updated in Step 2. | alex |
| Authoring ADR-040 (Option 3 deferral) in parallel | Pending | alex |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
