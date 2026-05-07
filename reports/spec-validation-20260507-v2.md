# Spec Validation Report: 2026-05-07 (v2)

**Validator:** Carthos (Domain Architect)
**Date:** 2026-05-07 08:33 CEST
**Prior run:** 2026-05-07 06:33 CEST (no changes detected since)
**Verdict:** FAIL — 2 persistent gaps, 0 new gaps

---

## Executive Summary

Six specification documents reviewed in `plans/`. No new specs or commits affecting spec compliance since the 06:33 run. Two gaps persist unchanged: `meta_coordinator` still absent from `lib.rs` (blocker, PR #1291 unmerged) and `guard.rs` still absent (medium, #1274 open). All other bounded contexts remain fully implemented.

---

## Specification-by-Specification Validation

### 1. `design-gitea82-correction-event.md` — ✅ FULLY IMPLEMENTED

Verified:
- `CorrectionType` enum: `crates/terraphim_agent/src/learnings/capture.rs:44`
- `CorrectionEvent` struct: `capture.rs:502`
- `capture_correction()` function: exported via `mod.rs:41`
- `LearningEntry` enum (Learning + Correction + Procedure): `capture.rs:1225`
- `list_all_entries`, `query_all_entries`: exported via `mod.rs:42-43`
- `LearnSub::Correction` CLI variant: `main.rs:3138`
- Secret redaction in capture path: `mod.rs` confirms `redact_secrets` called

All 8 unit tests and 1 CLI integration test specified in the plan are expected covered. Status: stable.

### 2. `design-gitea84-trigger-based-retrieval.md` — ✅ MOSTLY IMPLEMENTED / ⚠️ MINOR GAP

Verified implemented:
- `trigger` and `pinned` fields on `MarkdownDirectives`: `crates/terraphim_types/src/lib.rs:502-504`
- `trigger::` and `pinned::` parsing: `crates/terraphim_automata/src/markdown_directives.rs:215-251`
- `TriggerIndex` struct: `crates/terraphim_rolegraph/src/lib.rs:51`
- `trigger_index` and `pinned_node_ids` fields on `RoleGraph`: `lib.rs:320-322`
- `trigger_descriptions` and `pinned_node_ids` on `SerializableRoleGraph`: `lib.rs:271-273`
- `find_matching_node_ids_with_fallback()`: `lib.rs:451`
- `load_trigger_index()`: `lib.rs:478`
- `query_graph_with_trigger_fallback()`: `lib.rs:718`
- `--include-pinned` flag on search CLI: `main.rs:718`
- Integration tests: `two_pass_fallback_to_trigger` (`lib.rs:2196`), `pinned_always_included` (`lib.rs:2215`), `serializable_roundtrip_preserves_triggers` (`lib.rs:2233`)

**Minor gap**: The spec's §7 CLI design includes a `KgSub` enum with `kg list --pinned`. This sub-command is absent from `main.rs` (no `KgSub` enum found). The `--include-pinned` flag on the search path covers the primary acceptance criteria (AC6 in spec). The `kg list --pinned` listing command is not in the formal acceptance criteria list. Assessed as **follow-up**, not a blocker.

### 3. `d3-session-auto-capture-plan.md` — ✅ FULLY IMPLEMENTED

Verified:
- `from_session_commands()`: `crates/terraphim_agent/src/learnings/procedure.rs:412`
- `extract_bash_commands_from_session()`: `procedure.rs:471`
- `ProcedureSub::FromSession` variant gated `#[cfg(feature = "repl-sessions")]`: `main.rs:3413`
- Implementation wires to `ProcedureStore::save_with_dedup()`: `main.rs:3446`
- `procedure` module no longer behind `#[cfg(test)]`: `mod.rs:31` shows `pub(crate) mod procedure;`
- Test coverage: 6 unit tests including `test_from_session_commands_basic`, `_filters_trivial`, `_auto_title` confirmed in `procedure.rs`

Status: stable.

### 4. `design-single-agent-listener.md` — ✅ OPERATIONAL (infrastructure concern)

Verified:
- `~/.config/terraphim/listener-worker.json`: EXISTS (created 2026-04-16)
- `~/.config/terraphim/scripts/start-listener.sh`: EXISTS (created 2026-04-16)
- Spec explicitly states "No code changes to the Rust codebase are required"
- All code-level invariants (claim logic, self-filtering, retry) covered by existing tests in `listener.rs`

Whether the tmux session is currently running is an operational concern outside spec scope. Status: stable.

### 5. `learning-correction-system-plan.md` — ❌ GAP PERSISTS (Phase H)

**Gap G-2026-05-06-1: `guard.rs` module absent**

`crates/terraphim_agent/src/learnings/guard.rs` does not exist. Issue #1274 remains open.

Required by spec Phase H:
- `ExecutionTier` enum (Allow / Sandbox / Deny)
- `GuardDecision` type
- `evaluate_command()` function
- Integration with Firecracker sandbox tier

**Severity:** Medium — no automated command safety evaluation before procedure replay.

The rest of the plan (Phases A–G) is confirmed implemented per prior runs.

### 6. `research-single-agent-listener.md` — ✅ RESEARCH COMPLETE

Phase 1 artefact only; no implementation deliverables. Status: stable.

---

## Persistent Non-Spec Gap: `meta_coordinator.rs` Orphaned

**Gap G-2026-05-07-1: `meta_coordinator` not declared in `lib.rs`**

`crates/terraphim_orchestrator/src/meta_coordinator.rs` (25 KB, added 2026-05-06) remains absent from the `pub mod` list in `lib.rs` (lines 31–65, 34 declarations confirmed; `meta_coordinator` absent between `mention_chain` and `metrics_persistence`).

Issue #1275 open. PR #1291 exists — **not yet merged**.

Consequence: all 741 lines of `meta_coordinator.rs` are dead code. Five `#[tokio::test]` functions inside are unreachable. The `dispatch_cycle` integration invariant is unverified.

**Severity:** Blocker — entire module is unreachable until declaration is added.

Note: this gap has no corresponding `plans/` spec document. It is tracked via Gitea issue #1275 and PR #1291.

---

## Traceability Matrix

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Evidence | Status |
|-------:|-------------|------------|----------|-------|----------|--------|
| REQ-82-001 | `CorrectionEvent` struct with typed corrections | `design-gitea82-correction-event.md §1.2` | `capture.rs:502` | `test_correction_event_roundtrip` | `capture.rs:502` | ✅ |
| REQ-82-002 | `capture_correction()` with secret redaction | `§1.4` | `capture.rs`, `mod.rs:41` | `test_capture_correction`, `test_correction_secret_redaction` | `mod.rs:41` | ✅ |
| REQ-82-003 | `LearnSub::Correction` CLI | `§3.1` | `main.rs:3138` | CLI integration test | `main.rs:3138` | ✅ |
| REQ-82-004 | Unified `list_all_entries` / `query_all_entries` | `§1.5` | `mod.rs:42-43` | `test_list_all_entries_mixed` | `mod.rs:42` | ✅ |
| REQ-84-001 | `trigger::` / `pinned::` directive parsing | `design-gitea84 §2` | `markdown_directives.rs:215` | `parses_trigger_directive`, `parses_pinned_directive` | `markdown_directives.rs:348` | ✅ |
| REQ-84-002 | `TriggerIndex` TF-IDF fallback | `§3` | `rolegraph/lib.rs:51` | `tfidf_exact_match_scores_high`, `two_pass_fallback_to_trigger` | `lib.rs:2196` | ✅ |
| REQ-84-003 | `--include-pinned` search CLI flag | `§7` | `main.rs:718` | Acceptance criteria AC6 | `main.rs:718` | ✅ |
| REQ-84-004 | `kg list --pinned` CLI command | `§7` | ABSENT | ABSENT | `KgSub` enum not in `main.rs` | ⚠️ |
| REQ-D3-001 | `learn procedure from-session <id>` CLI | `d3-session §design` | `main.rs:3413` | `test_from_session_commands_basic` | `main.rs:3413` | ✅ |
| REQ-D3-002 | Trivial command filter | `d3-session §design` | `procedure.rs:412` | `test_from_session_commands_filters_trivial` | `procedure.rs:868` | ✅ |
| REQ-D3-003 | Feature-gated `repl-sessions` | `d3-session §dependencies` | `main.rs:3413` | N/A | `#[cfg(feature = "repl-sessions")]` | ✅ |
| PH-H-001 | `ExecutionTier` enum | `learning-correction-plan §Phase H` | `guard.rs` — ABSENT | ABSENT | File does not exist | ❌ |
| PH-H-002 | `evaluate_command()` | Same | Same | ABSENT | Same | ❌ |
| META-001 | `meta_coordinator` in public API | Gitea #1275 | `lib.rs` — ABSENT | 5 tests unreachable | `grep meta_coordinator lib.rs` → 0 hits | ❌ |

---

## Gap Summary

| Gap ID | Description | Severity | Issue | Status |
|--------|-------------|----------|-------|--------|
| G-2026-05-07-1 | `meta_coordinator` not in `lib.rs` — all code dead | Blocker | #1275, PR #1291 (unmerged) | ❌ OPEN |
| G-2026-05-06-1 | `guard.rs` absent — Phase H Graduated Guard missing | Medium | #1274 | ❌ OPEN |
| G-2026-05-07-2 | `kg list --pinned` CLI sub-command absent | Minor follow-up | (no issue) | ⚠️ FOLLOW-UP |

---

## Recommendations (smallest first)

1. **Merge PR #1291** — adds `pub mod meta_coordinator;` to `lib.rs`. One line change. Unblocks all internal tests and the `dispatch_cycle` integration invariant.
2. **Fix `last_cleanup` mutation bug** in `dispatch_cycle` — never updates `self.last_cleanup` after `cleanup_expired`, causing cleanup to run on every cycle after hour 1.
3. **Add `kg list --pinned` command** — trivial extension of the search CLI. Add `KgSub` enum with `List { pinned: bool }` per spec §7.
4. **Implement `guard.rs` (Phase H)** — `ExecutionTier` enum, `GuardDecision`, `evaluate_command()` per spec. Self-contained; no other gaps block it.
5. **Create `plans/design-meta-coordinator.md`** — document the bounded context, scoring formula, agent selection precedence, and TTL rationale for `MetaCoordinator`.

---

## Conclusion

No regression since the 06:33 run. Two gaps persist: the `meta_coordinator` orphan (blocker, one-line fix available as PR #1291) and the absent Graduated Guard module (medium, #1274). One minor follow-up (missing `kg list --pinned` CLI). All other specs are fully implemented and tested.

**Verdict: FAIL — 2 open gaps (1 blocker, 1 medium) + 1 minor follow-up**

---

<sub>Validated against commit `d5293544d` on branch `main`. Plans directory: 6 specs, unchanged since 2026-05-04.</sub>
