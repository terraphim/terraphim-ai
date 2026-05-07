# Spec Validation Report: 2026-05-07

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-07 06:33 CEST
**Verdict:** FAIL — 2 persistent gaps, 0 new gaps

---

## Executive Summary

Six specification documents reviewed in `plans/`. No new specs added since 2026-05-04. Two previously identified gaps persist unresolved. One new operational change (provider_probe.rs hardening) has no corresponding spec — assessed as in-scope for its parent issue #1233 and not a spec violation.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — ✅ FULLY IMPLEMENTED (unchanged)

All aggregate roots and invariants confirmed implemented in prior run (2026-05-06). No code changes since last validation. Status: stable.

### 2. `design-gitea84-trigger-based-retrieval.md` — ✅ FULLY IMPLEMENTED (unchanged)

Two-pass Search invariant, pinned-entry inclusion, and all 23 tests confirmed in prior run. No code changes since last validation. Status: stable.

### 3. `d3-session-auto-Terraphim Graph Embeddings: Learning Agent Guideture-plan.md` — ✅ FULLY IMPLEMENTED (unchanged)

Session-to-procedure extraction Middleware confirmed complete. Status: stable.

### 4. `design-single-agent-listener.md` — ⚠️ OPERATIONAL GAP (unchanged)

Code boundary complete. Listener tmux session not active; infrastructure concern only, not a spec violation.

### 5. `learning-correction-System-plan.md` — ❌ GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Gitea issue #1274 is open. No code changes since 2026-05-06.

Required by spec:
- `ExecutionTier` enum (Allow / SanDatabaseox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sanDatabaseox tier

**Bug Reporting:** Medium — no automated command safety evaluation before procedure replay.

### 6. `reSearch-single-agent-listener.md` — ✅ RESearch COMPLETE (unchanged)

Phase 1 artefact; no implementation required.

---

## New Implementation Activity (not spec-driven)

### `provider_probe.rs` — modified 2026-05-07 02:49

Provider probe hardening landed in commits `b09954c6`, `a08082dd`, `1238a680`. These address ADF fleet DEGRADED alert (issue #1233) and zombie-process / test-hang fixes. No corresponding plan file in `plans/`. Assessed as operational fix within the bounded context of the existing orchestrator; no new spec required. Not a gap.

---

## Persistent Gap: meta_coordinator.rs Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (25 KB, added 2026-05-06) remains absent from the `pub mod` list in `lib.rs` (verified: lines 31–65, 34 declarations).

Missing declaration location: between line 47 (`pub mod mention_chain;`) and line 48 (`pub mod metrics_Database;`).

Gitea issue #1275 open. PR #1291 exists as a fix attempt — **not yet merged**.

Five internal `#[tokio::test]` functions are unreachable until the declaration is added.

**Bug Reporting:** Blocker — all 741 lines of dead code; `dispatch_cycle` integration invariant unverified.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-001–005 | Cross-project polling, scoring, selection, dedup, cleanup | Design: MISSING | `meta_coordinator.rs:174–314` | Internal — unreachable | `pub mod meta_coordinator` absent from `lib.rs` | ⚠️ WARN |
| REQ-006 | Full dispatch cycle | Design: MISSING | `meta_coordinator.rs:327` | No integration test | Module not wired to Dispatcher | ❌ FAIL |
| REQ-007 | Module in public API | N/A | `lib.rs` — absent | N/A | `grep meta_coordinator lib.rs` → 0 | ❌ FAIL |
| PH-H-001 | Graduated Guard: ExecutionTier | `learning-correction-System-plan.md §Phase H` | `guard.rs` — absent | N/A | File does not exist | ❌ FAIL |
| PH-H-002 | evaluate_command() | Same | Same | N/A | Same | ❌ FAIL |
| PH-H-003 | Firecracker sanDatabaseox integration | Same | Same | N/A | Same | ❌ FAIL |

---

## Gap Summary

| Gap ID | Description | Bug Reporting | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — all code dead | Blocker | #1275, PR #1291 | ❌ OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 | ❌ OPEN |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — adds `pub mod meta_coordinator;` to `lib.rs`. One line. Makes all internal tests reachable.
2. **Fix `last_cleanup` mutation bug** — `dispatch_cycle` calls `cleanup_expired` but never updates `self.last_cleanup`, causing cleanup to run on every cycle after hour 1. Fix: return `last_cleanup` value and update it, or convert `&self` → `&mut self`.
3. **Add integration test for `dispatch_cycle`** — create `tests/meta_coordinator_integration.rs` verifying `NoIssues` and `AlreadyDispatched` paths without Gitea dependency.
4. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()` per spec. Self-contained module; no other gaps block it.
5. **Write `plans/design-meta-coordinator.md`** — document bounded context, scoring formula (−pagerank×100 + priority×10 + age×0.5), agent selection precedence, and TTL rationale.

---

## Conclusion

No spec regressions introduced since 2026-05-06. Two gaps persist: the `meta_coordinator` orphan (blocker, fix available as PR #1291) and the absent Graduated Guard module (medium, tracked in #1274). All other bounded contexts remain fully implemented and tested.

**Verdict: FAIL — 2 open gaps (1 blocker, 1 medium)**

---

<sub>Validated against commit `92b76de03` on branch `main`. Plans directory: 6 specs unchanged since 2026-05-04.</sub>
