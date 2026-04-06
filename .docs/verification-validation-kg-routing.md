# V-Model Verification and Validation Report
# KG-Driven Model Routing (Gitea #400 / GitHub PR #761)

**Date**: 2026-04-06
**Branch**: `task/400-kg-driven-model-routing`
**Commits**: 4 (47622ad2, c2427630, 94694f69, 781ad57c)

---

## PHASE 4: VERIFICATION

Verification answers: "Did we build it right?" -- checking implementation against design.

### 4.1 Traceability Matrix

| ID | Design Requirement | Implementation File(s) | Test(s) | Status |
|----|-------------------|----------------------|---------|--------|
| REQ-1 | Load routing rules from markdown files with `route::`, `action::`, `synonyms::`, `priority::` directives | `crates/terraphim_automata/src/markdown_directives.rs` (action:: parsing, multi-route support) | `parses_multiple_routes_with_actions`, `action_without_route_warns` | PASS |
| REQ-2 | `action::` directive on RouteDirective type | `crates/terraphim_types/src/lib.rs` (RouteDirective.action, MarkdownDirectives.routes) | Compile-time verified, serde default confirmed | PASS |
| REQ-3 | Multi-route fallback chains (Vec<RouteDirective>) | `crates/terraphim_types/src/lib.rs`, `crates/terraphim_automata/src/markdown_directives.rs` | `parses_multiple_routes_with_actions` | PASS |
| REQ-4 | KG router loads taxonomy, builds thesaurus, routes via find_matches | `crates/terraphim_orchestrator/src/kg_router.rs` (KgRouter::load, route_agent) | `routes_to_primary_by_synonym_match`, `higher_priority_wins`, `no_match_returns_none`, `empty_dir_loads_with_zero_rules` | PASS |
| REQ-5 | Action template rendering with {{ model }} and {{ prompt }} substitution | `crates/terraphim_orchestrator/src/kg_router.rs` (KgRouteDecision::render_action) | `render_action_substitutes_placeholders` | PASS |
| REQ-6 | Health-aware fallback (skip unhealthy providers) | `crates/terraphim_orchestrator/src/kg_router.rs` (first_healthy_route) | `first_healthy_route_skips_unhealthy` | PASS |
| REQ-7 | Provider health tracking with circuit breakers | `crates/terraphim_orchestrator/src/provider_probe.rs` (ProviderHealthMap, CircuitBreaker reuse) | `new_health_map_is_stale`, `unknown_provider_is_healthy`, `record_failures_opens_circuit`, `record_success_keeps_healthy` | PASS |
| REQ-8 | Provider probing via CLI action templates | `crates/terraphim_orchestrator/src/provider_probe.rs` (probe_single, probe_all) | No unit test (async+process; integration-only) | PARTIAL |
| REQ-9 | spawn_agent() tries KG routing first, falls back to keyword RoutingEngine | `crates/terraphim_orchestrator/src/lib.rs` (spawn_agent model selection) | Covered by existing orchestrator tests (routing=None path) | PASS |
| REQ-10 | Hot-reload via mtime detection | `crates/terraphim_orchestrator/src/kg_router.rs` (reload_if_changed, dir_mtime) | `reload_picks_up_new_files` | PASS |
| REQ-11 | [routing] config section with taxonomy_path, probe_ttl_secs, probe_results_dir, probe_on_startup | `crates/terraphim_orchestrator/src/config.rs` (RoutingConfig) | Compile-time, serde defaults | PASS |
| REQ-12 | 10 taxonomy markdown files covering ADF routing scenarios | `docs/taxonomy/routing_scenarios/adf/*.md` (10 files) | Loaded by KG router tests with tempdir equivalents | PASS |
| REQ-13 | Backward compatibility (route field preserved, serde(default) on all new fields) | `crates/terraphim_types/src/lib.rs` | `parses_config_route_priority` (pre-existing test still passes) | PASS |

### 4.2 Test Results

| Crate | Tests Run | Passed | Failed | Ignored |
|-------|-----------|--------|--------|---------|
| terraphim_automata | 90 | 90 | 0 | 0 |
| terraphim_types | 62 | 62 | 0 | 0 |
| terraphim_orchestrator | 374 | 374 | 0 | 0 |
| **Total** | **526** | **526** | **0** | **0** |

New tests added: 13 (7 kg_router + 4 provider_probe + 2 markdown_directives)

### 4.3 Code Quality

| Check | Result |
|-------|--------|
| `cargo fmt --check` | PASS -- no formatting issues |
| `cargo clippy -D warnings` | **1 WARNING** (see defect D-1 below) |
| `cargo check --workspace` | PASS -- full workspace compiles clean |
| Unsafe code | None |
| Unwrap in production code | None (all unwrap() calls are in test code only) |

### 4.4 Defect List

#### D-1: Clippy warning in provider_probe.rs (Origin: Phase 3 -- Implementation)

**Severity**: Low
**Location**: `crates/terraphim_orchestrator/src/provider_probe.rs:203`
**Description**: `std::io::Error::new(std::io::ErrorKind::Other, e)` should use the idiomatic `std::io::Error::other(e)` form.
**Fix**: Replace with `std::io::Error::other(e)` (one-line change).
**Impact**: Clippy lint failure only; no functional impact.

#### D-2: probe_on_startup config never read (Origin: Phase 2 -- Design gap)

**Severity**: Medium
**Location**: `crates/terraphim_orchestrator/src/config.rs:109` and `src/lib.rs`
**Description**: The `probe_on_startup` field is declared in `RoutingConfig` and defaults to `true`, but it is never checked during orchestrator initialisation. The `probe_all()` method is never called from any orchestrator lifecycle method. Similarly, `save_results()` is never called.
**Impact**: Provider probing is configured but never executes. Circuit breakers start empty and are never populated from probes. They can only be populated via `record_failure`/`record_success` -- which are also never called (see D-3).
**Fix**: Wire `probe_all()` into orchestrator startup (when `probe_on_startup` is true) and/or into the reconciliation tick (when TTL expires).

#### D-3: ExitClassifier feedback not wired to circuit breakers (Origin: Phase 2 -- Design gap)

**Severity**: Medium
**Location**: `crates/terraphim_orchestrator/src/lib.rs` (agent completion handler, ~line 2375)
**Description**: When the ExitClassifier classifies an agent exit as `ModelError` or similar, `provider_health.record_failure()` is never called. Conversely, successful exits never call `record_success()`. This means circuit breakers remain in their initial state (all providers healthy) and never trip even if a provider is consistently failing.
**Impact**: The health-aware fallback logic (`first_healthy_route`, `unhealthy_providers()`) is structurally present but inert -- it will always return the primary route because no provider is ever marked unhealthy.
**Fix**: In the agent completion handler, after `exit_classifier.classify()`, call `provider_health.record_failure(provider)` for `ModelError`/`RateLimited` classifications and `record_success(provider)` for `Success`/`CompletedWithDiff`.

#### D-4: reload_if_changed() never called in reconciliation loop (Origin: Phase 2 -- Design gap)

**Severity**: Low
**Location**: `crates/terraphim_orchestrator/src/kg_router.rs:249` and `src/lib.rs`
**Description**: The `reload_if_changed()` method is implemented and tested, but never called from the orchestrator's tick/reconciliation loop. Hot-reload is dead code.
**Impact**: Changes to taxonomy markdown files at runtime will not take effect until the orchestrator is restarted.
**Fix**: Call `kg_router.reload_if_changed()` in the orchestrator's tick method (perhaps gated behind a tick modulo to avoid checking every second).

#### D-5: split_whitespace() for command execution breaks quoted arguments (Origin: Phase 3 -- Implementation)

**Severity**: Low (probing only; not used for production agent dispatch)
**Location**: `crates/terraphim_orchestrator/src/provider_probe.rs:259`
**Description**: `action.split_whitespace()` does not handle shell quoting. An action template like `claude -p "hello world"` would be split into 4 args: `claude`, `-p`, `"hello`, `world"` -- which would fail. The probe uses the fixed prompt `"echo hello"` (no spaces after substitution in the test case), so this is not triggered today.
**Impact**: If action templates contain multi-word arguments (e.g., a test prompt with spaces), probing will fail.
**Fix**: Use `shell-words` crate or spawn via `sh -c "action"` for proper shell parsing.

### 4.5 Verification Decision

**Result: CONDITIONAL GO**

The core KG routing logic (REQ-1 through REQ-7, REQ-9 through REQ-13) is correctly implemented and well-tested. The 526 tests in affected crates all pass. Backward compatibility is preserved via `#[serde(default)]` on all new fields.

However, defects D-2, D-3, and D-4 represent wiring gaps where designed functionality (probing, circuit breaker feedback, hot-reload) is implemented at the module level but not connected to the orchestrator lifecycle. These are design-level gaps (not implementation bugs) that reduce the feature to "KG-based routing with static health assumptions" rather than "KG-based routing with dynamic health adaptation."

**Recommendation**: Merge as-is for the routing foundation, and create follow-up issues for D-2/D-3/D-4 wiring.

---

## PHASE 5: VALIDATION

Validation answers: "Did we solve the right problem?" -- checking solution against original requirements.

### 5.1 Original Requirements

The problem statement was: **Replace static model assignments in the ADF orchestrator with dynamic KG-driven routing using markdown files.**

Sub-requirements:
1. On startup, probe all providers for availability and speed
2. Use Aho-Corasick pattern matching against agent task descriptions to select the best provider+model
3. During operation, adapt via circuit breakers and ExitClassifier feedback

### 5.2 System Test Results

| Requirement | Validation Evidence | Verdict |
|-------------|-------------------|---------|
| **Replace static model assignments** | `spawn_agent()` now tries KG routing first via `route_agent()`, falling back to keyword RoutingEngine. Explicit `model` field in agent config still takes priority. | PASS |
| **Markdown-based routing rules** | 10 taxonomy files in `docs/taxonomy/routing_scenarios/adf/` cover all ADF agent scenarios with priorities 30-80. Format reuses existing terraphim directive system. | PASS |
| **Aho-Corasick matching** | `KgRouter::route_agent()` calls `terraphim_automata::find_matches()` with thesaurus built from synonyms. 123 synonym patterns across 10 rules. | PASS |
| **Priority-based selection** | `higher_priority_wins` test confirms multi-match resolution. Priority range 30 (cost_fallback) to 80 (reasoning). | PASS |
| **Multi-route fallback chains** | Each rule has 2 routes. `first_healthy_route()` filters by unhealthy provider list. | PASS |
| **Provider probing on startup** | Code is present (`probe_all`) but NOT wired to startup. | FAIL (D-2) |
| **Circuit breaker adaptation** | Code is present (`ProviderHealthMap`) but NOT fed by ExitClassifier. | FAIL (D-3) |
| **Hot-reload of routing rules** | Code is present (`reload_if_changed`) but NOT called in tick loop. | FAIL (D-4) |

### 5.3 Acceptance Criteria Assessment

| Criterion | Met? | Evidence |
|-----------|------|----------|
| KG routing selects correct model for security tasks | Yes | synonym "security audit" maps to security_audit.md (priority 60, kimi primary) |
| KG routing selects correct model for code review | Yes | synonym "code review" maps to code_review.md (priority 70, anthropic/opus primary) |
| KG routing selects correct model for implementation | Yes | synonym "implement" maps to implementation.md (priority 50, kimi primary) |
| Fallback works when primary provider is unhealthy | Structurally yes, but circuit breakers are never populated (D-3) | `first_healthy_route_skips_unhealthy` test passes; production path is inert |
| Backward compatible with existing configs | Yes | `routing: None` path tested; `route` field preserved; `serde(default)` on all new fields |
| Existing tests unaffected | Yes | 526/526 pass in affected crates |
| Workspace compiles clean | Yes | `cargo check --workspace` passes |

### 5.4 NFR Compliance

| NFR | Assessment |
|-----|------------|
| Performance | KG routing adds one Aho-Corasick match per agent spawn -- negligible cost (sub-millisecond for 123 patterns). Thesaurus is built once at startup. |
| Maintainability | Routing rules are plain markdown files editable by non-developers. New scenarios require only adding a new .md file. |
| Extensibility | Multi-route support enables any number of fallback providers per scenario. |
| Security | No unsafe code. CLI execution in `probe_single` is bounded by timeout (30s). Command paths come from taxonomy files (admin-controlled). |
| Backward compatibility | Full. All new fields use `serde(default)`. Existing `route` field preserved as alias for `routes[0]`. |

### 5.5 Coverage of ADF Agent Scenarios

| ADF Agent Type | Taxonomy Rule | Priority | Primary Provider | Fallback Provider |
|---------------|---------------|----------|-----------------|-------------------|
| Security Sentinel | security_audit.md | 60 | kimi/k2p5 | anthropic/claude-sonnet-4-6 |
| Quality Coordinator / Spec Validator | code_review.md | 70 | anthropic/claude-opus-4-6 | kimi/k2p5 |
| Implementation Swarm | implementation.md | 50 | kimi/k2p5 | anthropic/claude-sonnet-4-6 |
| Documentation Generator | documentation.md | 40 | minimax/m2.5-free | anthropic/claude-sonnet-4-6 |
| Meta-Coordinator | reasoning.md | 80 | anthropic/claude-opus-4-6 | anthropic/claude-haiku-4-5 |
| Test Guardian / Browser QA | testing.md | 55 | kimi/k2p5 | anthropic/claude-sonnet-4-6 |
| Log Analyst | log_analysis.md | 45 | kimi/k2p5 | openai/gpt-5-nano |
| Merge Coordinator | merge_review.md | 65 | kimi/k2p5 | anthropic/claude-sonnet-4-6 |
| Product Owner | product_planning.md | 60 | anthropic/claude-sonnet-4-6 | kimi/k2p5 |
| Budget/Batch Tasks | cost_fallback.md | 30 | openai/gpt-5-nano | minimax/m2.5-free |

### 5.6 Validation Decision

**Result: CONDITIONAL PASS**

The core requirement -- "replace static model assignments with KG-driven routing" -- is fully satisfied. The routing foundation is solid:

- Markdown-based rules are loaded correctly
- Aho-Corasick matching works against agent task descriptions
- Priority-based selection resolves multi-matches correctly
- Multi-route fallback chains are structurally present
- Backward compatibility is preserved

The three wiring gaps (D-2, D-3, D-4) mean that the dynamic adaptation part of the design is not yet operational. The system currently operates as "KG-driven routing with static health" rather than "KG-driven routing with dynamic health adaptation."

---

## FINAL GO/NO-GO

**Decision: GO for merge (with follow-up issues)**

### Rationale

1. **Core value delivered**: KG-driven model routing replaces static assignments. This is the primary requirement.
2. **No regressions**: 526/526 tests pass in affected crates. Workspace compiles clean.
3. **Backward compatible**: Existing configs with `routing: None` work unchanged.
4. **Well-structured for follow-up**: The wiring gaps (D-2/D-3/D-4) are clearly bounded and can be addressed independently.

### Required Before Merge

- **D-1**: Fix clippy warning (`std::io::Error::other()`) -- trivial one-line fix

### Recommended Follow-up Issues

- **Issue for D-2**: Wire `probe_all()` into orchestrator startup and tick cycle
- **Issue for D-3**: Connect ExitClassifier feedback to `ProviderHealthMap.record_success/record_failure`
- **Issue for D-4**: Call `reload_if_changed()` in orchestrator tick method
- **Issue for D-5**: Use `shell-words` or `sh -c` for proper command parsing in probes

### Defect Origin Summary

| Defect | Origin Phase | Severity |
|--------|-------------|----------|
| D-1 | Phase 3 (Implementation) | Low |
| D-2 | Phase 2 (Design) | Medium |
| D-3 | Phase 2 (Design) | Medium |
| D-4 | Phase 2 (Design) | Low |
| D-5 | Phase 3 (Implementation) | Low |

---

**Signed off by**: V-Model Testing Orchestrator
**Date**: 2026-04-06
