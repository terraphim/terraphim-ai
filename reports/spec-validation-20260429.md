# Spec Validation Report -- 2026-04-29 (Re-validation)

**Validator**: Carthos (Domain Architect)
**Date**: 2026-04-29T08:37:23Z
**Commit**: 0541f2c59 (HEAD)
**Previous Validation**: 2026-04-29T06:40:45Z (issue #1066)
**Verdict**: FAIL

---

## Executive Summary

Five active specification documents were validated against the current Rust codebase at commit `0541f2c59`. A previous validation at 06:40 claimed PASS but missed several persistent gaps. This re-validation corrects that assessment.

**Fixed since last validation**: Three gaps resolved (trigger index now live, procedure.rs un-gated, shared learning CLI present).

**Remaining**: Five critical gaps and two medium gaps.

---

## Plans Validated

| # | Plan File | Mapped Issue | Overall Status |
|---|-----------|--------------|----------------|
| 1 | `design-gitea82-correction-event.md` | Gitea #82 | Partial |
| 2 | `design-gitea84-trigger-based-retrieval.md` | Gitea #84 | Implemented |
| 3 | `learning-correction-system-plan.md` | #480, #578, #693, #703, #694, #695, #599, #686, #704, #515-#517, #451, #727-#730 | Partial |
| 4 | `design-single-agent-listener.md` | Operational | Implemented |
| 5 | `d3-session-auto-capture-plan.md` | #693 | Implemented |

---

## Fixed Gaps (confirmed resolved)

| ID | Gap | Evidence |
|----|-----|----------|
| F1 | Trigger-Based KG Retrieval was dead code | `find_matching_node_ids_with_fallback()` called at `terraphim_rolegraph/src/lib.rs:731` inside `query_graph_with_operators()`. Two-pass search live. |
| F2 | `procedure.rs` gated behind `#[cfg(test)]` | `terraphim_agent/src/learnings/mod.rs:31` now reads `pub(crate) mod procedure;`. `ProcedureStore` exported at line 37. |
| F3 | `learn shared` CLI missing | `SharedLearningSub` exists at `terraphim_agent/src/main.rs:1033` (List, Promote, Import, Stats, Inject). Gated behind `feature = "shared-learning"`. |

---

## Critical Gaps (remaining)

### C1: Hook Success Capture Missing
- **Plan**: `learning-correction-system-plan.md` Phase B, #693
- **Spec**: `should_capture_success()`, `SessionCommandBuffer`, automatic procedure creation from successful multi-step Bash sequences.
- **Actual**: `should_capture()` at `terraphim_agent/src/learnings/hook.rs:385` returns `tool_name == "Bash" && exit_code != 0` ONLY. Success events are silently discarded.
- **Impact**: Procedural memory cannot auto-build from successful workflows. `from_session()` CLI is a manual workaround, not automatic.
- **Severity**: Blocker for #693 full completion.

### C2: Agent Evolution Unwired
- **Plan**: `learning-correction-system-plan.md` Phase I, #727-#730
- **Spec**: `terraphim_agent_evolution` crate wired into `terraphim_agent` binary with real LLM adapters.
- **Actual**: Zero references in `terraphim_agent/Cargo.toml` or `main.rs`. Crate still uses mock LLM adapters internally.
- **Impact**: Cross-run compounding, nightwatch stagnation detection, and haystack ServiceType extension are unreachable.
- **Severity**: Blocker for #727-#730.

### C3: Sandbox/Firecracker Guard Tier Missing
- **Plan**: `learning-correction-system-plan.md` Phase H, #704
- **Spec**: `learnings/guard.rs` with `ExecutionTier` (Allow, Sandbox, Deny), Firecracker integration for sandboxed execution.
- **Actual**: `guard_patterns.rs` has `GuardDecision::Sandbox` enum variant but only performs pattern matching (Allow/Block/Sandbox classification). No Firecracker integration. No graduated execution tier.
- **Impact**: Sandbox decision has no backend; commands classified as Sandbox receive no actual sandboxing.
- **Severity**: Medium-High.

### C4: CorrectionEvent Not Publicly Exported
- **Plan**: `design-gitea82-correction-event.md`
- **Spec**: `CorrectionEvent` and `LearningEntry` publicly available from `learnings` module.
- **Actual**: `CorrectionEvent` struct exists at `capture.rs:502` but is absent from `terraphim_agent/src/learnings/mod.rs` public re-exports (lines 40-48). `LearningEntry` also not exported.
- **Impact**: External consumers cannot import `CorrectionEvent` from the public API.
- **Severity**: Medium.

### C5: Integration Test for `learn correction` Missing
- **Plan**: `design-gitea82-correction-event.md`
- **Spec**: Binary-level integration test for `learn correction` subcommand.
- **Actual**: `tests/learn_no_service_tests.rs` covers `learn list`, `learn hook`, `learn capture`. No test for `learn correction`.
- **Impact**: CLI regression risk for correction capture path.
- **Severity**: Medium.

---

## Medium Gaps (remaining)

### M1: `kg list --pinned` CLI Naming Mismatch
- **Plan**: `design-gitea84-trigger-based-retrieval.md`
- **Spec**: `kg list --pinned`
- **Actual**: `graph --pinned` at `main.rs:738`. `Search` uses `--include-pinned` at line 717.
- **Impact**: Spec and CLI diverge on command naming.
- **Severity**: Low.

### M2: Entity Annotation Not Wired to Hook Pipeline
- **Plan**: `learning-correction-system-plan.md` Phase C, #703
- **Spec**: Auto-suggest from KG at capture time (original TODO at capture.rs line 609).
- **Actual**: `annotate_with_entities()` exists and is exported from `mod.rs` but is not called from `capture_failed_command()` or `capture_correction()`.
- **Impact**: Captured learnings lack KG entity annotations unless manually post-processed.
- **Severity**: Low-Medium.

---

## Implementation State Matrix

| Feature | Plan Ref | File + Line | Status | Test Coverage |
|---------|----------|-------------|--------|---------------|
| CorrectionType | #82 | capture.rs:44 | Found | Unit tests |
| CorrectionEvent | #82 | capture.rs:502 | Found | Unit tests |
| `learn correction` CLI | #82 | main.rs:962 | Found | No integration test |
| CorrectionEvent export | #82 | mod.rs | Missing | N/A |
| TriggerIndex | #84 | rolegraph/lib.rs:51 | Found | Unit tests |
| Two-pass search (live) | #84 | rolegraph/lib.rs:731 | Found | Integration tests |
| `learn procedure` CLI | Master | main.rs:1111 | Found | Integration tests |
| `learn shared` CLI | Master | main.rs:1033 | Found | Feature-gated |
| FromSession | #693 | procedure.rs:412 | Found | Unit tests |
| Hook success capture | #693 | hook.rs:385 | Missing | N/A |
| Agent evolution wiring | #727 | Cargo.toml/main.rs | Missing | N/A |
| Graduated guard tier | #704 | learnings/guard.rs | Missing | N/A |

---

## Recommendations

1. **Priority 1 (Blocker)**: Extend hook pipeline to capture successes. Add `SessionCommandBuffer` and automatic `CapturedProcedure` creation on successful Bash sequences.
2. **Priority 2 (Blocker)**: Either wire `terraphim_agent_evolution` into the main binary or formally deprecate #727-#730.
3. **Priority 3**: Add `CorrectionEvent` and `LearningEntry` to `learnings/mod.rs` public re-exports.
4. **Priority 4**: Add integration test for `learn correction` CLI in `tests/learn_no_service_tests.rs`.
5. **Priority 5**: Close or defer #704 -- basic `GuardDecision::Sandbox` exists but full Firecracker integration is large scope.

---

## Verdict

**FAIL**

Three previously identified gaps are confirmed resolved. Five critical gaps persist, with two additional medium gaps. The codebase has made substantial progress on Phases A, B (partial), C (partial), D, and G of the learning system master plan. Phases E, F, H, and I remain largely unimplemented.

---

*Report generated by Carthos (Domain Architect).*
*Theme-ID: spec-gap*
