# Spec Validation Report: Issue #55

**Date**: 2026-06-01 06:57 CEST
**Agent**: spec-validator (Carthos, Domain Architect)
**Issue**: #55 — Migrate spawn_agent to use spawn_with_fallback
**Review trigger**: quality-coordinator review chain
**Verdict**: CONDITIONAL PASS

---

## Executive Summary

Issue #55 encompasses two distinct validation targets: (1) the original `spawn_with_fallback` migration (Wave 2 Task 2.1, closed 2026-03-26), and (2) a single-line test timing gate added by the current review cycle. Both items conform to project specifications and established patterns. One traceability gap exists: the referenced spec document `plans/adf-gap-remediation-plan.md` is absent from the plans directory.

---

## Requirements Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|--------|-------------|------------|----------|-------|--------|
| REQ-01 | `spawn_agent()` uses `spawn_with_fallback()` as sole spawn path | `plans/adf-gap-remediation-plan.md` Wave 2 Task 2.1 (ABSENT) | `crates/terraphim_orchestrator/src/lib.rs` — 4 call sites at lines 2478, 2825, 3028, 3342 | `reviewpr_dispatch_rejects_banned_provider` at lib.rs:10189 | ⚠️ PASS (traceability gap) |
| REQ-02 | `spawn_with_model()` no longer reachable in production path | Same as above | 0 occurrences in lib.rs (12,147 lines scanned) | Security-sentinel confirmed via source scan | ✅ PASS |
| REQ-03 | Defence-in-depth: two independent provider allow-list gates | config.rs changes | Gate 1 line ~2663 (pre-routing), Gate 2 line ~2738 (post-routing) | `reviewpr_dispatch_rejects_banned_provider` | ✅ PASS |
| REQ-04 | Performance assertions gated for debug builds | CLAUDE.md pattern (established in project) | `comprehensive_cli_tests.rs:602` — `#[cfg(not(debug_assertions))]` | 10+ existing usages in dos_prevention_test.rs, execution_mode_tests.rs, mcp_tool_index.rs | ✅ PASS |
| REQ-05 | Functional correctness assertion remains ungated | Same pattern | `assert_eq!(code, 0, ...)` at line 600 — ungated | Confirmed in diff | ✅ PASS |
| REQ-06 | All workspace tests pass | N/A | Workspace-wide | test-guardian: 2983 unit + 1296 integration (1 fixed by gate) | ✅ PASS (after fix) |

---

## Findings Detail

### Finding 1 — Test timing gate (current change)

**Severity**: None  
**Location**: `crates/terraphim_agent/tests/comprehensive_cli_tests.rs:602`

```diff
+    #[cfg(not(debug_assertions))]
     assert!(
         duration.as_secs() < 30,
         "Graph with large top-k should complete within 30 seconds"
     );
```

The gate is applied at assertion granularity (more precise than function-level gates used elsewhere). The functional `assert_eq!(code, 0)` immediately above is correctly ungated. The pattern is established in 10+ locations across the workspace. Debug builds run 5-20x slower than release; a 30-second threshold for graph traversal with top-k=100 is release-only by nature.

**Verdict: PASS** — Conforms to established project pattern.

---

### Finding 2 — spawn_with_fallback migration (original issue)

**Severity**: Resolved  
**Location**: `crates/terraphim_orchestrator/src/lib.rs`

The migration is complete: zero `spawn_with_model` calls remain. Four `spawn_with_fallback` call sites exist (lines 2478, 2825, 3028, 3342). Defence-in-depth is implemented with two independent provider allow-list gates at pre- and post-routing positions.

**Verdict: PASS** — Original issue is fully implemented and tested.

---

### Finding 3 — Missing spec document (traceability gap)

**Severity**: Follow-up (P3)  
**Location**: `plans/` directory

The plan referenced in the issue — `plans/adf-gap-remediation-plan.md` — is absent from the current plans directory. The directory contains:
- `d3-session-auto-capture-plan.md`
- `design-gitea82-correction-event.md`
- `design-gitea84-trigger-based-retrieval.md`
- `design-single-agent-listener.md`
- `learning-correction-system-plan.md`
- `research-single-agent-listener.md`

The plan was either deleted after completion or was never committed. Since the implementation is verified correct by code inspection and tests, this is a traceability gap only — not a functional defect.

**Verdict: FOLLOW-UP** — Create ADR or note absence in CHANGELOG.

---

### Finding 4 — Uncommitted orchestrator changes

**Severity**: Informational  
**Location**: Working tree (18 files, 604 insertions)

The working tree contains substantial uncommitted changes across orchestrator crates. Inspection of the lib.rs diff reveals these are primarily module-level documentation additions — not spawn path or functional changes. These are separate from issue #55's scope but should be committed or tracked.

**Verdict: INFORMATIONAL** — Out of scope for this issue. Recommend commit or tracking issue.

---

## Gap Summary

| Gap | Severity | Action |
|-----|----------|--------|
| `plans/adf-gap-remediation-plan.md` absent | ⚠️ P3 | Create brief ADR documenting the Wave 2 decision |
| Uncommitted orchestrator changes (604 ins.) | ℹ️ | Commit or create tracking issue |

---

## Verdict

**CONDITIONAL PASS**

The changes under review conform to project specifications and established patterns:
- `spawn_with_fallback` migration: complete and tested
- Test timing gate: follows established `#[cfg(not(debug_assertions))]` pattern correctly
- Conditions: resolve the missing spec plan traceability gap (P3)
