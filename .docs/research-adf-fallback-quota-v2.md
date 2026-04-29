# Research Document: ADF Fallback/Quota Bug (v2)

**Status**: Draft
**Author**: CTO Executive (research synthesis)
**Date**: 2026-04-29
**Supersedes**: `.docs/research-adf-fallback-quota-plan.md` (v1 planner findings)

## Executive Summary

The v1 implementation (4 files, ~333 insertions) by opencode agent k2p5 partially addressed the quota-to-fallback chain but introduced 7 defects and left 3 of the original 5 gaps unfixed. The integration test fails because the fallback agent uses the same name as the primary and exits before the test can observe it. A clean v2 implementation is needed that addresses the root cause -- agents without explicit `provider` field never record health failures, and no KG router fallback path exists.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Quota hits silently kill agents with no recovery -- production data loss |
| Leverages strengths? | Yes | ADF orchestrator is our own codebase; full control over routing/health |
| Meets real need? | Yes | Claude subscription quota hits are recurring; every hit wastes a full agent cycle |

**Proceed**: Yes (3/3)

## Problem Statement

### Description

When an ADF agent hits a subscription/quota limit (e.g., `"You've hit your limit - resets 2am Europe/Berlin"`), the orchestrator should:
1. Classify the exit as `RateLimit`
2. Record the provider health failure
3. Respawn the agent on the next healthy provider via KG routing

Currently none of these happen reliably because:
- The agent's `provider` field is `None` (KG routing sets `routed_model` but not `provider`)
- `poll_agent_exits` only triggers fallback on wall-clock timeout, not quota exit
- The agent-level `fallback_provider` field is not set for most agents (KG provides the fallback chain)

### Impact

Every Claude subscription quota hit silently kills the dispatched agent. The orchestrator logs the failure but does not recover. The next dispatch cycle re-selects the same unhealthy route.

### Success Criteria

1. Quota-text exits are classified as `RateLimit` regardless of exit code
2. Provider health records the failure even when `def.provider` is `None` (derive from routed model)
3. On `RateLimit`/`ModelError` exit, the orchestrator respawns using the next healthy KG route
4. If all routes are unhealthy, the agent exits cleanly (no infinite loop)
5. All existing tests continue passing; new tests cover each criterion

## Current State Analysis

### Existing Implementation (v1 diff, uncommitted)

4 files modified on bigbox `/home/alex/projects/terraphim/terraphim-ai`:

| File | Changes | Quality |
|------|---------|---------|
| `agent_run_record.rs` | +45 lines (quota patterns + 4 tests) | **Defective**: duplicate `modelerror` PatternDef block; quota patterns misclassified |
| `output_parser.rs` | +65 lines (12 new quota patterns + 7 tests) | **Acceptable**: patterns correct; `resets at`/`resets in` risk false positives |
| `telemetry.rs` | +29 lines (same 12 patterns + 1 test) | **Acceptable**: mirrors output_parser |
| `lib.rs` | +183/-14 lines (quota detection, exit override, respawn, integration test) | **Defective**: test fails; no provider derivation; no KG router fallback; name collision |

### v1 Defects Found (7 total)

| # | Defect | File | Severity | Root Cause |
|---|--------|------|----------|------------|
| D1 | Duplicate `modelerror` PatternDef block (two entries with same `concept_name`) | `agent_run_record.rs:310-318` | High | Agent appended a new block instead of editing the existing one |
| D2 | Quota patterns (`out of quota`, `quota exhausted`, `subscription quota`, `insufficient balance`) classified as `ModelError` instead of `RateLimit` | `agent_run_record.rs:280-283` | High | Agent added to wrong exit class |
| D3 | `test_quota_out_of_quota` asserts `ExitClass::ModelError` for a quota pattern | `agent_run_record.rs:829` | Medium | Follows from D2 |
| D4 | Missing `provider_key_from_routed_model` helper -- provider health never records failure for agents with `provider: None` | `lib.rs` | Critical | Plan called for it (Change 1) but agent skipped it |
| D5 | No KG router fallback path -- only `def.fallback_provider.is_some()` triggers respawn | `lib.rs:5679` | Critical | Plan called for it (Change 2) but agent implemented agent-level only |
| D6 | Integration test `test_quota_exit_triggers_fallback` fails -- fallback uses same name as primary and `sleep` exits immediately | `lib.rs:7665` | High | Name collision + test timing |
| D7 | `resets at`/`resets in` patterns risk false positives on non-quota text like "system resets its state" | `output_parser.rs`, `telemetry.rs` | Medium | Overly broad pattern |

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `poll_agent_exits()` | `lib.rs:5284-5720` | Drains agent output, classifies exits, handles fallback |
| `ExitClassifier` | `agent_run_record.rs` | Pattern-based exit classification |
| `EXIT_CLASS_PATTERNS` | `agent_run_record.rs:220-360` | Pattern definitions for each exit class |
| `parse_stderr_for_limit_errors()` | `output_parser.rs:203-240` | Text-based quota detection |
| `is_subscription_limit_error()` | `telemetry.rs:482-500` | Telemetry-level quota check |
| `spawn_agent()` | `lib.rs:1387-1830` | Agent spawn with KG routing and pre-check gates |
| `check_agent_timeouts()` | `lib.rs:5227-5280` | Wall-clock timeout with fallback respawn |
| `KgRouter` | `routing.rs` (external) | KG taxonomy-based route selection |
| `ProviderHealth` | `provider_health.rs` (external) | Tracks provider failure rates |
| `detect_quota_error()` | `lib.rs:6270-6285` | New helper scanning stdout/stderr for quota text |

### Data Flow (Current, Broken)

```
Agent exits with quota text
    |
    v
poll_agent_exits()
    |-- drain stdout/stderr
    |-- detect_quota_error() -> Some(line) [WORKS]
    |-- exit_classifier.classify() -> RateLimit [WORKS for most patterns]
    |-- override to RateLimit if quota text found [WORKS]
    |-- record AgentRunRecord [WORKS]
    |
    v  (after removing from active_agents)
    |-- if is_quota_exit && def.fallback_provider.is_some() [FAILS: fallback_provider is None]
    |   |-- spawn fallback_def [NEVER REACHED]
    |-- else: handle_agent_exit() [HIT: agent just dies]
    |
    v
provider health update? -> GUARD: if let Some(ref provider) = def.provider [SKIP: provider is None]
```

### Data Flow (Required)

```
Agent exits with quota text
    |
    v
poll_agent_exits()
    |-- drain stdout/stderr
    |-- detect_quota_error() -> Some(line)
    |-- exit_classifier.classify() -> RateLimit
    |-- override to RateLimit if quota text found
    |-- record AgentRunRecord
    |-- record provider health failure (derive provider from routed_model)
    |
    v  (after removing from active_agents)
    |-- if is_quota_exit:
    |   |-- if def.fallback_provider.is_some(): spawn agent-level fallback
    |   |-- else if kg_router available: respawn on next healthy KG route
    |   |-- else: handle_agent_exit() (no recovery path)
    |
    v
provider health updated -> next reconcile() uses updated unhealthy set
```

## Constraints

### Technical Constraints
- **No external crate changes**: All fixes within `terraphim_orchestrator`
- **Rust edition 2021**: Standard toolchain on bigbox
- **Test infrastructure**: Integration tests use real process spawning (`tokio::process::Command`)
- **ADF orchestrator is live**: Changes must not break running agents; cargo check must pass before commit

### Business Constraints
- **Fix must be minimal**: ~97 lines as per original plan; v1 bloated to 333
- **Must work without agent config changes**: Most agents have no `fallback_provider`; KG routing must provide the chain
- **Must not introduce infinite respawn loops**: Provider health + unhealthy set guards this

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Quota detection coverage | >95% of known provider messages | ~80% (missing "insufficient balance" etc.) |
| Fallback latency | <5s from exit to respawn | N/A (not working) |
| False positive rate | <1% | Unknown (broad patterns risk FPs) |

## Vital Few (Essentialism)

### Essential Constraints (Max 3)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Provider derivation from routed model | Without this, health tracking is blind for KG-routed agents | G1: `def.provider` is `None` for all KG-routed agents |
| KG router fallback on quota exit | Without this, no recovery when agent-level fallback is absent | G2: Only timeout triggers fallback; most agents have no `fallback_provider` |
| Correct pattern classification | Quota patterns must be `RateLimit` not `ModelError` to trigger the right path | D2: "out of quota" classifies as `ModelError`, never triggers fallback |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| `error_signatures` config for anthropic (G3) | Nice-to-have; `detect_quota_error` already catches the text. Can add later. |
| Reconciliation ordering verification (G4) | Already correct; `poll_agent_exits` runs before `check_schedules` |
| Documentation of KG fallback design (G5) | No code change; can do separately |
| `resets at`/`resets in` pattern refinement | Acceptable risk; the `test_parse_quota_false_positive` test guards against "resets its state" |

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| `KgRouter::route_agent()` | Must return fallback routes | Low: already returns `KgRouteDecision` with multiple routes |
| `ProviderHealth::record_failure()` | Must accept derived provider key | Low: takes `&str` |
| `ProviderHealth::unhealthy_providers()` | Must reflect newly recorded failures | Low: returns snapshot |
| `KgRouteDecision::first_healthy_route()` | Must skip unhealthy providers | Low: already implemented |

### External Dependencies
| Dependency | Version | Risk |
|------------|---------|------|
| `terraphim_router` crate | workspace | Low: read-only usage |
| `terraphim_types` crate | workspace | Low: provides `AgentDefinition` |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| v1 diff reverts lose useful work (12 quota patterns) | High | Low | Copy patterns to v2 before reverting |
| Spawned fallback agent hits same quota (infinite loop) | Low | High | Clear `fallback_provider` on fallback_def; provider health excludes unhealthy |
| KG router returns no routes (empty taxonomy) | Low | Medium | Guard: if no healthy route, skip respawn, agent exits normally |
| `provider_key_from_routed_model` maps model to wrong provider | Medium | High | Explicit mapping table with tests for each known provider prefix |

### Open Questions
1. Should the fallback agent use the same name as primary, or append a suffix? -- **Recommend suffix `"-retry-N"` to avoid key collision in `active_agents`**
2. Should `provider_key_from_routed_model` also handle bare names like `sonnet`, `opus`? -- **Yes, map to `"anthropic"`**
3. What is the max retry count before giving up? -- **Recommend 3; after that, agent exits permanently**

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `KgRouteDecision` has a `first_healthy_route()` method | Seen in v1 plan; grep confirms | No KG fallback available | Yes |
| `active_agents` uses `def.name` as key | Grep line 5949: `contains_key(&def.name)` | Key collision if fallback uses same name | Yes |
| `sleep` without args exits immediately on Linux | Standard POSIX behaviour | Integration test fails | Yes |
| `poll_agent_exits` runs before `check_schedules` in reconcile | Seen in v1 plan Change 4 | Provider health stale for same-tick dispatch | Partially |

## Research Findings

### Key Insights

1. **The v1 implementation solved the wrong problem**: It added quota detection (which mostly worked already via the `ExitClassifier`) but missed the actual gap: no fallback path when `def.fallback_provider` is `None` (which is the case for all KG-routed agents).

2. **Provider health recording is the critical missing link**: Without recording the failure against "anthropic", the unhealthy set is never updated, and the next dispatch picks the same route.

3. **The integration test failure is a design flaw, not a timing issue**: Using the same name for the fallback agent as the primary creates key collision. The fallback agent's name must be unique.

4. **12 new quota patterns are correct and reusable**: The `output_parser.rs` and `telemetry.rs` additions are sound. Only `agent_run_record.rs` has the duplicate block and misclassification.

5. **`detect_quota_error()` helper is a good safety net**: Catches quota text that the exit classifier misses (e.g., exit code 0 with partial output).

### Relevant Prior Art
- Wall-clock timeout fallback (lines 5227-5280): Existing pattern for fallback respawn with `def.fallback_provider`
- Spawner-level fallback (lines 1725-1750): `with_fallback_provider` on `SpawnRequest`
- Safety agent restart: Different mechanism (auto-restart regardless of exit class)

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| `provider_key_from_routed_model` mapping | Verify all known model prefix/name to provider mappings | 30 min |
| `KgRouteDecision.first_healthy_route()` API | Confirm method signature and return type | 15 min |

## Recommendations

### Proceed/No-Proceed
**Proceed**. The fix is well-understood, minimal (~80 lines net new), and addresses a production-affecting bug.

### Scope Recommendations
- **Keep**: Quota pattern additions (output_parser, telemetry), `detect_quota_error()` helper, exit class override, provider derivation, KG router fallback
- **Revert**: Duplicate `modelerror` block in agent_run_record.rs, misclassified quota patterns, broken integration test
- **Defer**: `error_signatures` config (G3), documentation (G5), pattern refinement for `resets at`/`resets in`

### Risk Mitigation Recommendations
- Implement provider derivation and KG fallback in separate commits
- Add retry count limit (max 3) to prevent runaway respawn
- Use unique agent names (`{name}-retry-{N}`) for fallback agents
- Test each change incrementally with `cargo test -p terraphim_orchestrator --lib -- quota`

## Next Steps

If approved:
1. Revert v1 diff on bigbox (4 files)
2. Re-apply the 12 correct quota patterns (output_parser, telemetry)
3. Fix `agent_run_record.rs` (remove duplicate block, correct classification)
4. Implement `provider_key_from_routed_model` (lib.rs)
5. Implement KG router fallback path in `poll_agent_exits` (lib.rs)
6. Write new integration test with proper naming and timing
7. Run full test suite
8. Commit and push to PR #1081 branch
