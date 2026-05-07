# Spec Validation Report: 2026-05-07 (v4)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-07 18:10 CEST
**Prior run:** 2026-05-07 12:35 CEST (v3)
**HEAD commit:** `2526414d7`
**Verdict:** FAIL — 2 persistent spec gaps + 1 process discrepancy; probe fix landed outside spec boundary

---

## Executive Summary

Since v3 (commit `6c8364563`, 12:35 CEST), four commits landed on the working branch
`task/446-anthropic-probe-circuit-breaker-fix`:

| Commit | Summary |
|--------|---------|
| `f78c5c16e` | fix(orchestrator): exempt C1-blocked probes from circuit-breaker updates Refs #446 |
| `977390d1f` | docs: update CHANGELOG and generate doc gap report |
| `df5c3c79a` | docs: session handover for issue #446 probe fix |
| `2526414d7` | test(orchestrator): add integration-path test for no-template probe exemption Refs #446 |

The probe fix and its integration-path test (`no_template_probe_does_not_open_breaker`) are
correct and close the `REQ-003` traceability gap identified in the #446 requirements review.
However, no `plans/` spec document covers this change — it is an unspecified boundary crossing
(noted below, not counted as a blocking gap).

Both persistent spec gaps from v3 remain open. PR #1291 remains unmerged.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — FULLY IMPLEMENTED

No changes since v3. All 8 unit tests and CLI integration test confirmed.

- `CorrectionType` enum: `capture.rs:44`
- `CorrectionEvent` struct: `capture.rs:502`
- `capture_correction()`: `mod.rs:41`
- `LearningEntry` enum: `capture.rs:1225`
- `list_all_entries`, `query_all_entries`: `mod.rs:42-43`
- `LearnSub::Correction` CLI: `main.rs:3138`

Status: **stable**.

---

### 2. `design-gitea84-trigger-based-retrieval.md` — MOSTLY IMPLEMENTED / MINOR GAP

No changes since v3. All primary acceptance criteria implemented.

**Follow-up G-2026-05-07-2:** `kg list --pinned` CLI sub-command (`KgSub` enum) absent from
`main.rs`. Not in the formal acceptance criteria list.

Status: **stable (minor follow-up persists)**.

---

### 3. `d3-session-auto-capture-plan.md` — FULLY IMPLEMENTED

No changes since v3. All 6 unit tests confirmed, `#[cfg(feature = "repl-sessions")]` in place.

Status: **stable**.

---

### 4. `design-single-agent-listener.md` — OPERATIONAL

Infrastructure files unchanged. No code-level regression.

Status: **stable**.

---

### 5. `learning-correction-system-plan.md` — GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Issue #1274 remains open.

Confirmed via directory listing (18:09 CEST):
```
capture.rs  compile.rs  export_kg.rs  hook.rs  install.rs
mod.rs  procedure.rs  redaction.rs  replay.rs  suggest.rs
```
No `guard.rs`.

Required per spec Phase H:
- `ExecutionTier` enum (Allow / Sandbox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sandbox tier

**Severity:** Medium — no automated command safety evaluation before procedure replay.

Phases A–G confirmed implemented per prior runs. Status: **gap persists**.

---

### 6. `research-single-agent-listener.md` — RESEARCH COMPLETE

Phase 1 artefact only; no implementation deliverables. Status: **stable**.

---

## Persistent Non-Spec Gap: `meta_coordinator.rs` Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (741 lines, 5 `#[tokio::test]`
functions) remains absent from `pub mod` declarations in
`crates/terraphim_orchestrator/src/lib.rs`.

Verified at 18:09 CEST: `grep "meta_coordinator" crates/terraphim_orchestrator/src/lib.rs` → 0 matches.

`pub mod` list ends at: `mention`, `mention_chain`, `metrics_persistence`, `mode`, `nightwatch` —
`meta_coordinator` absent between `mention_chain` and `metrics_persistence`.

PR #1291 (`Fix #1275: wire meta_coordinator module into lib.rs`) remains
`state=open, merged=False`. Issue #1275 was closed without the fix landing on `main`.

**Severity:** Blocker — all 741 lines of dead code; 5 `#[tokio::test]` functions unreachable;
`dispatch_cycle` integration invariant unverified; `last_cleanup` mutation bug unresolved
(tracked separately in issue #1301).

---

## New Observation: #446 Probe Fix Landed Without `plans/` Spec

Commits `f78c5c16e` and `2526414d7` implement the probe-circuit-breaker exemption and its
integration-path test. These are correct and address the root cause of issue #446.

However, no `plans/design-gitea446-*.md` or equivalent spec document exists for this change.
The work is bounded by the issue description and handover note
(`.docs/handover-2026-05-07-issue-446-probe-fix.md`), but lacks a formal design artefact.

**Classification:** Process observation only. The change is well-motivated and tested;
not counted as a spec gap. Recommend creating a `plans/` entry retroactively if the team
policy requires spec coverage for all non-trivial code changes.

The new `is_environment_error()` helper and the `no_template_probe_does_not_open_breaker`
test close the REQ-003 integration-path gap flagged in the #446 traceability review.
PR #1316 (`Fix #446: exempt C1-blocked probes from circuit-breaker updates`) is the upstream
Gitea tracking issue.

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
| REQ-446-001 | `is_environment_error()` covers all local-setup error kinds | Issue #446 / handover | `provider_probe.rs` | `is_environment_error_classifications` | `provider_probe.rs` | PASS |
| REQ-446-002 | No-template probes exempt from circuit-breaker | Issue #446 | `provider_probe.rs` | `no_template_probe_does_not_open_breaker` | `provider_probe.rs:862` | PASS |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — dead code; PR #1291 open/unmerged; issue #1275 closed prematurely | Blocker | #1275 (closed), PR #1291 (open) | OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 (open) | OPEN |
| G-2026-05-07-2 | `kg list --pinned` CLI sub-command absent | Minor follow-up | (no issue) | FOLLOW-UP |
| PROCESS-001 | Issue #1275 closed without PR #1291 merged | Process | #1275, PR #1291 | OPEN |
| OBS-446 | #446 probe fix landed without `plans/` spec document | Observation | #446 (open) | NOTED |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — single line: `pub mod meta_coordinator;` in `lib.rs`. Unblocks dead
   tests and `dispatch_cycle` integration invariant. Highest-priority action.
2. **Reopen or annotate issue #1275** — closed without the fix merging. Add comment linking
   PR #1291 so traceability is preserved.
3. **Fix `last_cleanup` mutation bug** (issue #1301) — `dispatch_cycle` takes `&self` so
   `last_cleanup` never updates; cleanup runs every cycle after hour 1. Wrap in
   `Arc<Mutex<Instant>>` or change to `&mut self`.
4. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`,
   `evaluate_command()`. Self-contained; no other gaps block it. Issue #1274 open.
5. **Add `kg list --pinned` command** — trivial extension per spec §7;
   add `KgSub::List { pinned: bool }`.
6. **Create retroactive `plans/design-gitea446-probe-circuit-breaker.md`** — if team policy
   requires spec coverage for all non-trivial code changes. Low urgency.

---

## Conclusion

No regression in existing spec coverage. The probe fix (issues #446, #1316) is correctly
implemented and fully tested. Two spec gaps remain: one blocker (`meta_coordinator` orphaned)
and one medium (`guard.rs` absent). The blocker is unblocked by a one-line PR that has been
open since at least v1 of today's reports.

**Verdict: FAIL — 2 open spec gaps (1 blocker, 1 medium) + 1 process discrepancy**

---

<sub>Validated against commit `2526414d7` on branch `task/446-anthropic-probe-circuit-breaker-fix`.
Plans directory: 6 specs, unchanged since 2026-05-04.
Gitea API confirmed: PR #1291 `merged=False`, issue #1274 `state=open`, issue #1316 `state=open`.</sub>
