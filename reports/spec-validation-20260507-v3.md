# Spec Validation Report: 2026-05-07 (v3)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-07 12:35 CEST
**Prior run:** 2026-05-07 08:33 CEST (v2)
**Verdict:** FAIL — 2 persistent gaps + 1 process discrepancy

---

## Executive Summary

Six specification documents in `plans/` reviewed against HEAD `eaae3d806`. Since the v2 run (08:33 CEST), one commit was added (`eaae3d806`) containing only documentation files (spec validation reports). No code changes. All four previously passing specs remain fully implemented. Both open gaps persist unchanged.

**New finding this run:** Issue #1275 was closed between v2 and v3, but PR #1291 remains open (`state=open, merged=False`). The one-line fix (`pub mod meta_coordinator;`) has not landed on `main`. The issue closure is premature — the blocker gap is still present.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — FULLY IMPLEMENTED

No changes since v2. All 8 unit tests and CLI integration test remain covered.
- `CorrectionType` enum: `capture.rs:44`
- `CorrectionEvent` struct: `capture.rs:502`
- `capture_correction()`: `mod.rs:41`
- `LearningEntry` enum: `capture.rs:1225`
- `list_all_entries`, `query_all_entries`: `mod.rs:42-43`
- `LearnSub::Correction` CLI: `main.rs:3138`

Status: stable.

### 2. `design-gitea84-trigger-based-retrieval.md` — MOSTLY IMPLEMENTED / MINOR GAP

No changes since v2. All primary acceptance criteria implemented. Minor gap persists:

**Follow-up G-2026-05-07-2:** `kg list --pinned` CLI sub-command (`KgSub` enum) absent from `main.rs`. Not in the formal acceptance criteria list. Follow-up only.

Status: stable.

### 3. `d3-session-auto-capture-plan.md` — FULLY IMPLEMENTED

No changes since v2. All 6 unit tests confirmed, feature gate `#[cfg(feature = "repl-sessions")]` in place.

Status: stable.

### 4. `design-single-agent-listener.md` — OPERATIONAL

No code changes required by spec. Infrastructure files exist. No code-level regression.

Status: stable.

### 5. `learning-correction-system-plan.md` — GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Issue #1274 remains open.

Required per spec Phase H:
- `ExecutionTier` enum (Allow / Sandbox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sandbox tier

The learnings directory contains: `capture.rs`, `compile.rs`, `export_kg.rs`, `hook.rs`, `install.rs`, `mod.rs`, `procedure.rs`, `redaction.rs`, `replay.rs`, `suggest.rs`. No `guard.rs`.

**Severity:** Medium — no automated command safety evaluation before procedure replay.

Phases A–G confirmed implemented per prior runs.

### 6. `research-single-agent-listener.md` — RESEARCH COMPLETE

Phase 1 artefact only; no implementation deliverables. Status: stable.

---

## Persistent Non-Spec Gap: `meta_coordinator.rs` Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (25 KB, 741 lines, 5 `#[tokio::test]` functions) remains absent from the `pub mod` declarations in `crates/terraphim_orchestrator/src/lib.rs`.

Verified by direct grep: `grep -n "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → 0 matches.

The `pub mod` list runs: `mention`, `mention_chain`, `metrics_persistence`, `mode`, `nightwatch` — `meta_coordinator` absent between `mention_chain` and `metrics_persistence`.

**Process discrepancy:** Issue #1275 was closed (confirmed via Gitea API: `state=closed`), but PR #1291 (`Fix #1275: wire meta_coordinator module into lib.rs`) remains open and unmerged (`state=open, merged=False`). The issue was closed without the fix landing on `main`. The fix is a single line addition: `pub mod meta_coordinator;`.

**Severity:** Blocker — all 741 lines of `meta_coordinator.rs` are dead code. Five `#[tokio::test]` functions are unreachable. The `dispatch_cycle` integration invariant is unverified. The `last_cleanup` mutation bug (pre-existing, documented in PR #1291 report) remains unresolved.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82-001 | `CorrectionEvent` struct with typed corrections | `design-gitea82 §1.2` | `capture.rs:502` | `test_correction_event_roundtrip` | `capture.rs:502` | PASS |
| REQ-82-002 | `capture_correction()` with secret redaction | `§1.4` | `capture.rs`, `mod.rs:41` | `test_capture_correction`, `test_correction_secret_redaction` | `mod.rs:41` | PASS |
| REQ-82-003 | `LearnSub::Correction` CLI | `§3.1` | `main.rs:3138` | CLI integration test | `main.rs:3138` | PASS |
| REQ-82-004 | Unified `list_all_entries` / `query_all_entries` | `§1.5` | `mod.rs:42-43` | `test_list_all_entries_mixed` | `mod.rs:42` | PASS |
| REQ-84-001 | `trigger::` / `pinned::` directive parsing | `design-gitea84 §2` | `markdown_directives.rs:215` | `parses_trigger_directive`, `parses_pinned_directive` | `markdown_directives.rs:348` | PASS |
| REQ-84-002 | `TriggerIndex` TF-IDF fallback | `§3` | `rolegraph/lib.rs:51` | `two_pass_fallback_to_trigger` | `lib.rs:2196` | PASS |
| REQ-84-003 | `--include-pinned` search CLI flag | `§7` | `main.rs:718` | Acceptance criteria AC6 | `main.rs:718` | PASS |
| REQ-84-004 | `kg list --pinned` CLI command | `§7` | ABSENT | ABSENT | `KgSub` enum not in `main.rs` | FOLLOW-UP |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | PASS |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | PASS |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | PASS |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist | FAIL |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | FAIL |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → 0 hits | FAIL |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — dead code; issue #1275 closed but PR #1291 unmerged | Blocker | #1275 (closed prematurely), PR #1291 (open) | OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 (open) | OPEN |
| G-2026-05-07-2 | `kg list --pinned` CLI sub-command absent | Minor follow-up | (no issue) | FOLLOW-UP |
| PROCESS-001 | Issue #1275 closed without PR #1291 merged — process discrepancy | Process | #1275, PR #1291 | OPEN |

---

## Recommendations (smallest first)

1. **Reopen or note issue #1275** — it was closed without the fix landing. Either reopen or add a comment linking to PR #1291 so the gap is traceable.
2. **Merge PR #1291** — single line: `pub mod meta_coordinator;` in `lib.rs`. Unblocks all dead tests and the `dispatch_cycle` integration invariant.
3. **Fix `last_cleanup` mutation bug** — `dispatch_cycle` takes `&self` so `last_cleanup` is never updated; cleanup runs every cycle after hour 1. Wrap in `Arc<Mutex<Instant>>` or change to `&mut self`.
4. **Add `kg list --pinned` command** — trivial extension per spec §7; add `KgSub` enum with `List { pinned: bool }`.
5. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()`. Self-contained; no other gaps block it. Issue #1274 open.
6. **Create `plans/design-meta-coordinator.md`** — 741-line bounded context with no design artefact. Document scoring formula, agent selection precedence, TTL rationale.

---

## Conclusion

No code regression since v2. Two spec gaps persist (blocker + medium). One new process finding: issue #1275 was closed without the code fix merging — PR #1291 remains open on Gitea. The issue closure is misleading and should be corrected.

**Verdict: FAIL — 2 open spec gaps (1 blocker, 1 medium) + 1 process discrepancy + 1 minor follow-up**

---

<sub>Validated against commit `eaae3d806` on branch `main`. Plans directory: 6 specs, unchanged since 2026-05-04. Gitea API confirmed: PR #1291 `merged=False`, issue #1274 `state=open`.</sub>
