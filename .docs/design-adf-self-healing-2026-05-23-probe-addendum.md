# Probe Design Addendum: Per-(CLI, provider, model) Health

**Status**: Draft addendum to `.docs/design-adf-self-healing-2026-05-23.md`
**Date**: 2026-05-23
**Trigger**: Z.AI investigation showed a provider can be **healthy via one CLI and broken via another**. Current probe is keyed by `(provider, model)`; it cannot represent this.

## Evidence

Direct invocation on bigbox 2026-05-23, same minute, same env:

| Route | Result |
|---|---|
| `opencode run -m zai-coding-plan/glm-5.1` | step_start only -> silence -> 60 s TIMEOUT |
| `pi-rust --provider zai-coding-plan --model glm-5.1 -p "ping"` | "Pong! 🏓" in ~3 s |

All four `zai-coding-plan/*` models reproduce the same divergence. Other providers (kimi, minimax) work through both CLIs.

## Problem

`provider_probe` keyed by `(provider, model)` cannot distinguish:
- `(zai-coding-plan, glm-5.1)` over opencode -> unhealthy
- `(zai-coding-plan, glm-5.1)` over pi-rust -> healthy

The KG router has two routes pointing at the same `(provider, model)` but with different `action::` templates. The probe currently marks the *provider/model* unhealthy and drops both routes. We lose the healthy one.

## Proposed Design

### Key change

Probe key becomes `(cli_binary_basename, provider, model)` -- a 3-tuple.

```rust
// Before
pub struct ProbeKey { provider: String, model: String }

// After
pub struct ProbeKey {
    cli: String,           // "opencode", "pi-rust", "claude" -- basename only
    provider: String,
    model: String,
}
```

### Action template parsing

`RouteDirective` (in `kg_router.rs`) already has the `action::` template. Extract the CLI basename at parse time:

```rust
impl RouteDirective {
    /// Returns the basename of the first whitespace-delimited token in
    /// the action template, e.g. "opencode" from
    /// "/home/alex/.bun/bin/opencode run -m {{ model }} ..."
    pub fn cli_basename(&self) -> Option<&str> {
        self.action.as_deref()
            .and_then(|a| a.split_whitespace().next())
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|f| f.to_str())
    }
}
```

### Probe execution

Probes use the **route's own action template** (not a separate code path). This is true black-box probing: probe IS the spawn:

1. Render the action with `prompt = "ping"`
2. Spawn the rendered command with a 30 s wall-clock cap (configurable)
3. Read stdout/stderr lines until exit
4. Classify outcome by content presence, not just exit code:
   - Healthy: at least one token-bearing event (e.g. opencode `type:"text"` line, or any non-empty stdout for stdout-based CLIs)
   - Truncated: exit 0 with no token content (the Z.AI-via-opencode case) -> mark unhealthy
   - Timeout: no exit within wall-clock cap -> unhealthy
   - Auth missing: stderr contains "No API key" / "ANTHROPIC_API_KEY not set" / etc -> unhealthy with `reason="auth"`
   - Endpoint error: exit non-zero with stderr matching network errors -> unhealthy with `reason="endpoint"`

### Route selection

`first_healthy_route` looks up each route by its 3-tuple:

```rust
let route_is_healthy = |r: &RouteDirective| -> bool {
    let key = ProbeKey {
        cli: r.cli_basename().unwrap_or("").to_string(),
        provider: r.provider.clone(),
        model: r.model.clone(),
    };
    probe_cache.is_healthy(&key)
};

decision.fallback_routes.iter().find(|r| route_is_healthy(r))
```

### Storage

- `HashMap<ProbeKey, ProbeOutcome>` -- bounded by `|tiers| x |routes per tier|` -- realistic worst case is ~30-50 entries
- TTL unchanged (1800 s default; new entries on miss)
- Persisted under `~/.terraphim/benchmark-results/` per existing config

## Self-healing properties

After this design:

| Scenario | Behaviour |
|---|---|
| opencode breaks for Z.AI (today) | `(opencode, zai-coding-plan, glm-5.1)` marked unhealthy; `(pi-rust, zai-coding-plan, glm-5.1)` stays healthy. KG router selects pi-rust route automatically. |
| pi-rust breaks for some model | Symmetric: opencode route selected. |
| Both CLIs break for same model | All routes for that `(provider, model)` are out of selection; KG router falls to the next route in priority order. |
| Anthropic itself rate-limits | `(claude, anthropic, sonnet)` and `(opencode, anthropic, sonnet)` both unhealthy; KG router falls back to next-priority route exactly as today. |

This is genuine **cross-CLI self-healing**: a broken CLI does not poison the model, and the orchestrator routes around it automatically on the next probe cycle.

## Migration

1. Probe cache file format: add a `cli` field. Old entries (without `cli`) treated as `cli = "opencode"` (the historical default) at load time, refreshed on next probe.
2. KG router unchanged externally; only `first_healthy_route` internal lookup changes.
3. No new TOML field on `AgentDefinition` -- the CLI is derived from the route's action template.

## Implementation steps (new Step 0)

Inserts **before** the existing 8-step plan; required for the per-CLI selection to work correctly post-deploy.

| # | Action | Files | Tests | Hours |
|---|---|---|---|---|
| 0a | Add `cli_basename()` to `RouteDirective`; add `ProbeKey` 3-tuple in `provider_probe.rs`; rewrite probe cache lookup keyed by tuple | `crates/terraphim_orchestrator/src/kg_router.rs`, `provider_probe.rs` | `route_cli_basename_extracts_opencode`, `route_cli_basename_extracts_pi_rust`, `probe_key_distinguishes_cli` | 2 |
| 0b | Rewrite `probe_provider` to use the route's action template + classify by content presence (not exit code alone) | `provider_probe.rs` | `probe_classifies_truncated_stream_as_unhealthy`, `probe_classifies_pong_as_healthy` | 2 |
| 0c | `first_healthy_route` looks up 3-tuple | `kg_router.rs` | `first_healthy_route_keeps_pi_rust_zai_when_opencode_zai_unhealthy` (the exact regression that this design fixes) | 1 |
| 0d | Probe cache file migration (load-time defaulting old `cli=null` to `opencode`) | `provider_probe.rs` | `legacy_probe_entry_migrates_to_opencode_cli` | 1 |

**Total**: 6 hours. Net delta to overall plan: +6 h (was 14 h, now 20 h). But: the Z.AI taxonomy fix landed (~30 min) means Step 1 is already partially done and the remaining "investigate Z.AI" item collapses to "monitor whether per-CLI probe re-enables opencode route when fixed upstream".

## Why this matters

Without this, the orchestrator's self-healing claim is partial: it can route around an unhealthy provider, but not around an unhealthy CLI for a healthy provider. The Z.AI case is the existence proof that this gap matters in production today.

With this, **the alternative-spawner pattern (pi-rust ↔ opencode) gains automatic failover** -- which was the whole point of adding pi-rust as a parallel CLI. The probe is what turns "two CLIs for the same model" from a theoretical option into operational redundancy.

## Out of scope

- Per-route circuit-breaker with state machine (Closed/Open/HalfOpen) -- existing `terraphim_spawner::health::CircuitBreaker` already handles that layer; we are extending its key, not its state machine
- Streaming-token-rate probe (e.g. mark unhealthy if < 5 tokens/sec) -- premature; binary healthy/unhealthy is enough for the Z.AI case
- Per-region probing -- single bigbox node, not relevant
