# ADF Fallback/Quota Bug -- Implementation Plan

**Date**: 2026-04-29
**Status**: Draft
**Scope**: Minimal changes to fix five interlocking gaps in the quota-detection-to-fallback chain.

---

## 1. Root-Cause Analysis

### Symptom

The `implementation-swarm` agent (configured with `cli_tool = "claude"`, no `model`, no `fallback_provider`) was dispatched to Claude/sonnet via KG tier routing. Claude exited with text `"You've hit your limit - resets 2am Europe/Berlin"`. The orchestrator classified the exit, updated provider health, but **did not respawn the agent on the next KG fallback route** (kimi/k2p5). The agent simply died.

### Chain of Events

```
1. KG routing matches implementation_tier.md
   -> primary: anthropic/sonnet via claude CLI
   -> fallback routes: kimi/k2p5, openai/gpt-5.3-codex, zai/glm-5-turbo

2. spawn_agent() builds SpawnRequest with:
   - primary: claude CLI, model=sonnet
   - NO fallback_provider / fallback_model on agent def (None/None)
   -> spawner.spawn_with_fallback() has nothing to fall back to

3. Claude exits with "You've hit your limit" text
   -> exit code may be 0 (partial output returned)

4. poll_agent_exits() classifies:
   a) ExitClassifier: "hit your limit" pattern -> RateLimit (correct)
   b) detect_quota_error(): "hit your limit" -> Some(...) (correct)
   c) detected_exit_class = RateLimit (correct, overridden from Unknown if needed)

5. Provider health update:
   a) record_failure("anthropic") -- but def.provider is None for implementation-swarm!
      -> Guard: if let Some(ref provider) = def.provider { ... } -- SKIPPED
   b) error_signatures classify: no signatures configured for "anthropic" -> Unknown
   c) No force_exhaust() call because provider field is None

6. No fallback respawn happens because:
   - poll_agent_exits does NOT respawn on RateLimit exit (only timeout respawns)
   - def.fallback_provider is None -> no spawner-level fallback
   - Provider health was never updated -> next dispatch selects same route
```

### Five Distinct Gaps

| # | Gap | File | Line(s) | Severity |
|---|-----|------|---------|----------|
| G1 | `def.provider` is `None` for implementation-swarm, so provider health never records the failure | `lib.rs` | 5541 | Critical |
| G2 | `poll_agent_exits` never respawns on `RateLimit` / `ModelError` -- only wall-clock timeout triggers fallback respawn | `lib.rs` | 5230-5280 | Critical |
| G3 | No `error_signatures` configured for `anthropic` (claude-code CLI) in provider budget config, so `force_exhaust` is never called on quota hit | `orchestrator.example.toml` / `provider_budget.rs` | 5560-5570 | High |
| G4 | `KgRouteDecision.first_healthy_route()` is only called at spawn time; the routing engine's `unhealthy_providers` snapshot is stale within the same tick | `lib.rs` | 1545-1570, `routing.rs` | Medium |
| G5 | Agent definition has no `fallback_provider`/`fallback_model` despite KG routing offering 4 fallback routes in the taxonomy | `orchestrator.example.toml` | implementation-swarm block | Medium |

---

## 2. Implementation Plan

### Change 1: Derive effective provider from routed model (G1)

**File**: `crates/terraphim_orchestrator/src/lib.rs`
**Function**: `poll_agent_exits()` (~line 5541)

**Current code** (line 5541):
```rust
if let Some(ref provider) = def.provider {
    match record.exit_class {
        ExitClass::ModelError | ExitClass::RateLimit => {
            self.provider_health.record_failure(provider);
```

**Problem**: `def.provider` is `None` when the agent uses bare model names like `sonnet` through the claude CLI. The KG routing override sets `routed_model` but the agent definition's `provider` field is never populated.

**Fix**: Derive the effective provider from the routed model when `def.provider` is `None`. Map bare names (`sonnet`, `opus`, `haiku`) and provider-prefixed names (`kimi-for-coding/k2p5`) to their canonical provider id.

```rust
let effective_provider = def.provider.as_deref()
    .or_else(|| provider_key_from_routed_model(
        routed_model.as_deref().or(def.model.as_deref())
    ));

if let Some(provider) = effective_provider {
    match record.exit_class {
        ExitClass::ModelError | ExitClass::RateLimit => {
            self.provider_health.record_failure(provider);
```

**New helper function** (add to `lib.rs` or `provider_budget.rs`):

```rust
fn provider_key_from_routed_model(model: Option<&str>) -> Option<String> {
    let model = model?;
    if let Some((prefix, _)) = model.split_once('/') {
        Some(prefix.to_string())
    } else if config::CLAUDE_CLI_BARE_MODELS.contains(&model)
        || config::ANTHROPIC_BARE_PROVIDERS.contains(&model)
    {
        Some("anthropic".to_string())
    } else {
        None
    }
}
```

**Tests**:
- Add test in `orchestrator_tests.rs`: agent with `provider: None` and `routed_model: Some("sonnet")` correctly records provider health failure on RateLimit exit.

---

### Change 2: Respawn on RateLimit exit using KG fallback route (G2)

**File**: `crates/terraphim_orchestrator/src/lib.rs`
**Function**: `poll_agent_exits()`, after exit handling (~line 5580)

**Current behaviour**: After classifying the exit and updating provider health, the function simply removes the agent from `active_agents`. Only Safety agents get auto-restarted. No-one respawns Core/Growth agents on quota hits.

**Fix**: After recording the `RateLimit` or `ModelError` exit, attempt a respawn using the next healthy KG fallback route. This mirrors the existing wall-clock timeout respawn logic (~line 5259) but triggers on quota instead of timeout.

Insert after the provider health / error_signatures block (~line 5580):

```rust
// --- Quota-triggered respawn with KG fallback route ---
let should_attempt_quota_fallback = matches!(
    record.exit_class,
    ExitClass::RateLimit | ExitClass::ModelError
) && def.layer != AgentLayer::Safety; // Safety has its own restart logic

if should_attempt_quota_fallback {
    if let Some(ref router) = self.kg_router {
        let unhealthy = self.provider_health.unhealthy_providers();
        if let Some(kg_decision) = router.route_agent(&def.task) {
            if let Some(healthy_route) = kg_decision.first_healthy_route(&unhealthy) {
                if healthy_route.provider != record.model_used.as_deref().unwrap_or("") {
                    info!(
                        agent = %name,
                        original_model = ?record.model_used,
                        fallback_provider = %healthy_route.provider,
                        fallback_model = %healthy_route.model,
                        "respawning with KG fallback route (primary hit quota)"
                    );
                    let mut fallback_def = def.clone();
                    fallback_def.model = Some(healthy_route.model.clone());
                    if let Some(ref action) = healthy_route.action {
                        if let Some(cli) = action.split_whitespace().next() {
                            fallback_def.cli_tool = cli.to_string();
                        }
                    }
                    fallback_def.provider = None;
                    fallback_def.fallback_provider = None;
                    fallback_def.fallback_model = None;
                    if let Err(e) = self.spawn_agent(&fallback_def).await {
                        error!(agent = %name, error = %e, "failed to respawn with KG fallback");
                    }
                    // Skip the normal Safety restart path below
                    continue;
                }
            }
        }
    }
}
```

**Guard against infinite respawn loop**: Record the respawn attempt in `AgentRunRecord.matched_patterns` (already pushed `"quota_limit_detected"`). The next spawn will select a different route because `anthropic` is now in `unhealthy_providers`. If ALL routes are exhausted, `first_healthy_route` returns `None` and the respawn is skipped -- the agent simply exits.

**Tests**:
- Add test: simulate RateLimit exit for implementation-swarm, verify the agent is respawned with a different model (kimi/k2p5 instead of anthropic/sonnet).
- Add test: all providers unhealthy -> no respawn attempted.
- Add test: Safety agents do NOT trigger the quota respawn (they use their own restart path).

---

### Change 3: Add claude-code error signatures to provider budget config (G3)

**File**: `orchestrator.example.toml` (and the live config)
**Also**: `crates/terraphim_orchestrator/src/provider_budget.rs`

**Current**: The `[[providers]]` list has no entry for `anthropic` / `claude-code`. This means `error_signatures::classify_lines()` returns `Unknown` and `force_exhaust` is never called on quota hits.

**Fix**: Add an `anthropic` provider budget config with throttle patterns for Claude-specific quota messages:

```toml
[[providers]]
id = "anthropic"
max_hour_cents = 500   # $5/hr subscription cap
max_day_cents = 5000   # $50/day cap

[providers.error_signatures]
throttle = [
    "hit your limit",
    "you've hit your limit",
    "plan limit",
    "usage limit",
    "subscription limit",
    "capacity limit",
    "spending limit",
    "rate limit",
    "429",
    "resets at",
    "resets in",
]
flake = ["timeout", "connection reset", "EOF"]
```

**Code impact**: `build_signature_map()` in `error_signatures.rs` already picks up `error_signatures` from `ProviderBudgetConfig` entries. No code change needed beyond the config.

**Tests**:
- Add test in `error_signatures_tests.rs`: `"You've hit your limit - resets 2am Europe/Berlin"` classifies as `Throttle` against the `anthropic` provider signatures.
- Add test in `provider_gate_tests.rs`: after a throttle-classified exit, the anthropic provider budget is force-exhausted.

---

### Change 4: Refresh unhealthy_providers snapshot before each spawn (G4)

**File**: `crates/terraphim_orchestrator/src/lib.rs`
**Function**: `spawn_agent()` (~line 1495)

**Current**: The `RoutingDecisionEngine` is constructed with `self.provider_health.unhealthy_providers()` at spawn time. For the inline KG routing path (~line 1545), `unhealthy` is also fetched fresh. This is actually correct for each spawn.

**The real issue**: Between the agent exiting and the next dispatch tick, the provider health update happens AFTER the agent is removed from `active_agents`. If another dispatch happens in the same reconciliation tick, it uses the pre-update health map.

**Fix**: No code change needed here. The `poll_agent_exits()` call happens in `reconcile()` before `check_schedules()`, so the health map IS updated before the next dispatch cycle. Verify this ordering is correct:

```rust
// In run() / reconcile():
async fn reconcile(&mut self) {
    self.poll_agent_exits().await;     // Updates provider health
    self.check_agent_timeouts().await;
    self.check_schedules().await;       // Dispatches new agents (uses updated health)
    // ...
}
```

**Action**: Add a tracing span to confirm ordering, no structural change.

---

### Change 5: Document that KG fallback routes replace agent-level fallback_provider (G5)

**File**: `orchestrator.example.toml`

**Current**: The `implementation-swarm` agent definition has no `fallback_provider` / `fallback_model`. This is correct by design -- the KG taxonomy provides the fallback chain (`anthropic/sonnet -> kimi/k2p5 -> openai/gpt-5.3-codex -> zai/glm-5-turbo`).

**Fix**: Add a comment explaining the design intent. No code change.

```toml
# NOTE: No fallback_provider/fallback_model needed. KG tier routing
# (implementation_tier.md) provides a 4-route fallback chain.
# When the primary (anthropic/sonnet) hits quota, the orchestrator
# respawns on the next healthy KG route automatically.
```

---

## 3. Summary of All Changes

| Change | Files Modified | Lines Changed (est.) | Risk |
|--------|---------------|---------------------|------|
| C1: Derive effective provider | `lib.rs` | +25 | Low |
| C2: Quota-triggered respawn | `lib.rs` | +45 | Medium |
| C3: Claude error signatures | `orchestrator.example.toml` | +20 | Low |
| C4: Verify reconcile ordering | `lib.rs` | +3 (tracing) | None |
| C5: Document design intent | `orchestrator.example.toml` | +4 | None |
| **Total** | **2 files** | **~97 lines** | |

### New Test Functions

| Test | File | Validates |
|------|------|-----------|
| `provider_health_records_failure_with_routed_model_only` | `orchestrator_tests.rs` | G1 fix |
| `rate_limit_triggers_kg_fallback_respawn` | `orchestrator_tests.rs` | G2 fix |
| `all_providers_unhealthy_skips_respawn` | `orchestrator_tests.rs` | G2 edge case |
| `safety_agents_skip_quota_respawn` | `orchestrator_tests.rs` | G2 guard |
| `claude_quota_text_classifies_as_throttle` | `error_signatures_tests.rs` | G3 fix |
| `anthropic_force_exhaust_on_throttle` | `provider_gate_tests.rs` | G3 integration |

---

## 4. Dependency Order

```
C1 (derive provider)  -->  C2 (quota respawn)  -->  C3 (error sigs)
                                                         |
C4 (verify ordering) ------------------------------------+
C5 (documentation) --------------------------------------+
```

1. **C1 first** -- provider health must be recorded correctly before respawn logic can use it.
2. **C3 in parallel with C1** -- error signatures are config-only, no code dependency.
3. **C2 after C1** -- the respawn logic needs the effective provider to be correct.
4. **C4 and C5** -- no code dependencies, can be done last.

---

## 5. What This Plan Does NOT Change

- No changes to `KgRouter` or the taxonomy markdown files.
- No changes to `RoutingDecisionEngine` scoring logic.
- No changes to `ExitClassifier` (already correctly detects "hit your limit").
- No changes to `AgentSpawner` or `spawn_with_fallback`.
- No changes to the `FlowExecutor` (flows have their own step-level error handling).
