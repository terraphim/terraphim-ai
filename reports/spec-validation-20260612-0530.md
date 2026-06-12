# Spec Validation Report: 2026-06-12 05:30 CEST (Cron)

**Agent**: spec-validator (Carthos, Domain Architect)
**HEAD**: 62ac458158
**Date**: 2026-06-12 05:30 CEST
**Run type**: Cron (no mention context)

## Verdict: CONDITIONAL PASS

417 tests pass in workspace run; 1 flaky test (`test_terraphim_graph_search_comprehensive`) fails under concurrency but **passes in isolation** (pre-existing serial-ordering issue, not a regression); 14 ignored; clippy clean. Six plans carry forward unchanged. **P1 #2415** (hot-path validate() bypass) and **P2 #2495** (doc lie) remain open. **P2-1** (ValidationResult.strictness hardcoded Normal) is RESOLVED by d3076a144a.

---

## Tests

```
Workspace run: 417 PASS, 1 flaky, 14 ignored
Isolation run (-p terraphim_server test_terraphim_graph_search_comprehensive): 1 PASS
Clippy: clean (1 pre-existing tokio-tungstenite patch advisory, non-blocking)
```

### Flaky Test: test_terraphim_graph_search_comprehensive

- **File**: `terraphim_server/tests/terraphim_graph_search_test.rs:188`
- **Failure message**: "RoleGraph should have nodes after document indexing"
- **Root cause**: `#[serial]` test with shared `DeviceStorage` state interferes with concurrent workspace test run; passes cleanly in isolation
- **Severity**: P3 pre-existing — not a new regression at this HEAD
- **Status**: git blame shows this file has not changed since `9424eef721` (fix polyrepo publish, unrelated)

---

## Plans Validated (6 total)

| Plan | Title | Status | Notes |
|------|-------|--------|-------|
| design-gitea82-correction-event.md | CorrectionEvent for Learning Capture | PASS | Carry-forward |
| d3-session-auto-capture-plan.md | Session-Based Auto-Capture | PASS | Carry-forward |
| design-gitea84-trigger-based-retrieval.md | Trigger-Based KG Retrieval | PASS | Carry-forward |
| design-single-agent-listener.md | Single Gitea Listener | N/A | Operational plan |
| learning-correction-system-plan.md | Learning and Correction System | PARTIAL | Phases C, F–H deferred |
| research-single-agent-listener.md | Research Document | N/A | Research only |

No plan changes since 2026-06-11.

---

## P1-1 — Issue #2415 hot-path validate() regression (UNCHANGED)

**Status**: OPEN — unchanged at HEAD 62ac458158

The KG validation gate (`executor.validate()`) is absent from all four hot-path call sites:

| File | Function | Lines | Status |
|------|----------|-------|--------|
| `crates/terraphim_rlm/src/rlm.rs` | `execute_code()` | 310–335 | validate_session() only, no executor.validate() |
| `crates/terraphim_rlm/src/rlm.rs` | `execute_command()` | 366–391 | validate_session() only, no executor.validate() |
| `crates/terraphim_rlm/src/query_loop.rs` | `Command::Run` | 383–409 | executor.execute_command() called directly |
| `crates/terraphim_rlm/src/query_loop.rs` | `Command::Code` | 411–437 | executor.execute_code() called directly |

The executor-level `validate()` implementations (local.rs, docker.rs, firecracker.rs) are all correct. The gap is at the orchestrator layer.

**ACs failing**: AC1–AC5 of #2415 remain unmet.

---

## P2-1 — ValidationResult.strictness hardcoded Normal (RESOLVED)

**Status**: RESOLVED by `d3076a144a`

All three executors now propagate `validator.config().strictness` into `ValidationResult`:
- `executor/local.rs`: `strictness: validator.config().strictness` ✅
- `executor/docker.rs`: `strictness: validator.config().strictness` ✅
- `executor/firecracker.rs`: `strictness: validator.config().strictness` ✅

---

## P2-2 — Issue #2495 doc lie in config.rs (UNCHANGED)

**Status**: OPEN — unchanged at HEAD 62ac458158

`config.rs:356–357`:
```rust
/// Returns `true` for `Normal` and `Strict`, matching the hot-path
/// behaviour in `query_loop.rs` and `rlm.rs`.
```

This comment asserts that `blocks_unknown()` "matches the hot-path behaviour", but the hot-path in `query_loop.rs` and `rlm.rs` does **not** call `executor.validate()` at all (P1 #2415). The function is not on any hot-path. The doc is a lie until #2415 is fixed.

**Note**: #2494 is a duplicate of #2495 (carry-forward from memory).

---

## P2-3 — Phase C entity annotation unimplemented (UNCHANGED)

`annotate_with_entities()` / `--semantic` flag not present. Explicitly deferred in the plan. Carry-forward.

---

## Summary

| Severity | Finding | Issue | Status |
|----------|---------|-------|--------|
| P1 | validate() gate absent from all 4 execute hot-paths | #2415 | OPEN |
| P2 | Doc lie: blocks_unknown() claims hot-path match | #2495 | OPEN |
| P2 | Phase C entity annotation deferred | carry-forward | DEFERRED |
| P2 | ValidationResult.strictness hardcoded Normal | — | **RESOLVED** (d3076a144a) |
| P3 | Flaky test (serial-ordering under workspace run) | — | Pre-existing |
