# Spec Validation Report: plans/ directory

**Date**: 2026-05-09
**Validator**: Carthos (Domain Architect)
**Scope**: All active specification documents in `plans/`

---

## Executive Summary

6 plan documents examined. 3 carry validatable spec items. Overall verdict: **PARTIAL PASS** — 1 spec fully matches implementation, 1 has a minor CLI surface deviation, 1 has a function-signature mismatch where architecture diverged from spec. 3 documents are research or operational and require no code validation.

---

## Specs Validated

### 1. `plans/design-gitea82-correction-event.md`

**Status in plan**: approved
**Gitea issue**: #82
**Verdict**: PASS

All 8 spec items exist in the implementation.

| Spec Item | Location | Status |
|-----------|----------|--------|
| `CorrectionType` enum (7 variants) | `crates/terraphim_agent/src/learnings/capture.rs:43-59` | PASS |
| `CorrectionEvent` struct (9 fields) | `capture.rs:501-521` | PASS |
| `LearningEntry` enum (`Learning`, `Correction`) | `capture.rs:1224-1229` | PASS |
| `capture_correction()` | `capture.rs:1022-1066` | PASS |
| `list_all_entries()` | `capture.rs:1298-1369` | PASS |
| `query_all_entries()` | `capture.rs:1372-1404` | PASS |
| `Correction` variant in `LearnSub` | `main.rs:~2000` | PASS |
| Handler for `learn correction` subcommand | `main.rs:3141-3167` | PASS |

**Extensions beyond spec** (non-blocking):
- `LearningEntry` has an additional `Procedure(CapturedProcedure)` variant
- `query_all_entries_semantic()` function added alongside the specified one

---

### 2. `plans/design-gitea84-trigger-based-retrieval.md`

**Status in plan**: design
**Gitea issue**: #84
**Verdict**: PASS (with minor deviation)

11 of 12 spec items fully implemented. 1 minor structural deviation.

| Spec Item | Location | Status |
|-----------|----------|--------|
| `MarkdownDirectives.trigger: Option<String>` | `crates/terraphim_types/src/lib.rs:517` | PASS |
| `MarkdownDirectives.pinned: bool` | `terraphim_types/src/lib.rs:519` | PASS |
| Parser for `trigger::` directive | `crates/terraphim_automata/src/markdown_directives.rs:235-244` | PASS |
| Parser for `pinned::` directive | `markdown_directives.rs:246-250` | PASS |
| `TriggerIndex` struct (4 fields) | `crates/terraphim_rolegraph/src/lib.rs:77-88` | PASS |
| `TriggerIndex::new(threshold)` | `rolegraph/src/lib.rs:94-103` | PASS |
| `TriggerIndex::build(triggers)` | `rolegraph/src/lib.rs:129-152` | PASS |
| `TriggerIndex::query(text)` | `rolegraph/src/lib.rs:155-210` | PASS |
| `find_matching_node_ids_with_fallback()` | `rolegraph/src/lib.rs:477-500` | PASS |
| `load_trigger_index()` | `rolegraph/src/lib.rs:504-514` | PASS |
| `query_graph_with_trigger_fallback()` | `rolegraph/src/lib.rs:744-842` | PASS |
| `kg list --pinned` CLI subcommand | `main.rs` | DEVIATION |

**Deviation detail** — `kg list --pinned`:
Spec described a `list` subcommand under `kg`. Implementation provides `kg --pinned` (i.e. `graph --pinned`) flag directly on the top-level command. The pinned filtering behaviour is identical; only the CLI surface differs. This is a cosmetic gap, not a functional one.

---

### 3. `plans/d3-session-auto-capture-plan.md`

**Status in plan**: work item
**Gitea issue**: D3 feature
**Verdict**: PARTIAL FAIL

The CLI entry point exists and the feature works end-to-end, but the core function signature diverged from the spec.

| Spec Item | Spec Signature | Actual | Status |
|-----------|---------------|--------|--------|
| `TRIVIAL_COMMANDS` constant | `&[&str]` | `procedure.rs:382` — exact match | PASS |
| `FromSession` variant in `ProcedureSub` | present | `main.rs:1180` (feature-gated `repl-sessions`) | PASS |
| Handler for `from-session` subcommand | present | `main.rs:3417` | PASS |
| `from_session(session_id, title, sessions_path) -> io::Result<CapturedProcedure>` | monolithic fn | **MISSING** — split into `extract_bash_commands_from_session(session) -> Vec<(String, i32)>` + `from_session_commands(commands, title) -> CapturedProcedure` | FAIL |

**Gap analysis**:
The implementation adopted a two-step architecture (extract then construct) rather than the spec's single monolithic function. The CLI subcommand wires these together correctly so end-to-end behaviour matches the spec's intent. However:

- Return type differs: spec `io::Result<CapturedProcedure>`, actual `CapturedProcedure` (no error propagation at function boundary)
- Parameter set differs: spec takes `sessions_path: &Path`, actual takes a pre-loaded `Session` object
- The function `from_session()` with the spec signature does not exist

This is an **architectural divergence**, not a broken feature. The feature works end-to-end, but the module boundary defined in the spec was not followed.

---

### 4. `plans/design-single-agent-listener.md`

**Status in plan**: operational
**Verdict**: NOT APPLICABLE (no code changes specified)

This plan describes config file creation and tmux setup only. Existing listener code (`terraphim-agent listen`) is the target binary. No Rust source changes were planned.

---

### 5. `plans/learning-correction-system-plan.md`

**Status in plan**: research + design roadmap
**Verdict**: NOT APPLICABLE (plan document, not a spec)

This is a 10-phase roadmap referencing issues #480, #578, #693-695, #703-704, #727-730. Individual phases should produce their own design specs before validation.

---

### 6. `plans/research-single-agent-listener.md`

**Status in plan**: research
**Verdict**: NOT APPLICABLE (research document, not a spec)

---

## Traceability Matrix

| Req ID | Plan | Spec Item | Impl Location | Status |
|--------|------|-----------|---------------|--------|
| REQ-82-01 | gitea82 | `CorrectionType` enum | `capture.rs:43` | PASS |
| REQ-82-02 | gitea82 | `CorrectionEvent` struct | `capture.rs:501` | PASS |
| REQ-82-03 | gitea82 | `LearningEntry` enum | `capture.rs:1224` | PASS |
| REQ-82-04 | gitea82 | `capture_correction()` | `capture.rs:1022` | PASS |
| REQ-82-05 | gitea82 | `list_all_entries()` | `capture.rs:1298` | PASS |
| REQ-82-06 | gitea82 | `query_all_entries()` | `capture.rs:1372` | PASS |
| REQ-82-07 | gitea82 | `LearnSub::Correction` CLI | `main.rs:~2000` | PASS |
| REQ-84-01 | gitea84 | `trigger` + `pinned` fields | `terraphim_types/lib.rs:517,519` | PASS |
| REQ-84-02 | gitea84 | `trigger::` + `pinned::` parser | `markdown_directives.rs:235-250` | PASS |
| REQ-84-03 | gitea84 | `TriggerIndex` struct + 3 methods | `rolegraph/lib.rs:77-210` | PASS |
| REQ-84-04 | gitea84 | `find_matching_node_ids_with_fallback` | `rolegraph/lib.rs:477` | PASS |
| REQ-84-05 | gitea84 | `kg list --pinned` CLI | `main.rs` | DEVIATION |
| REQ-D3-01 | d3 | `TRIVIAL_COMMANDS` | `procedure.rs:382` | PASS |
| REQ-D3-02 | d3 | `ProcedureSub::FromSession` | `main.rs:1180` | PASS |
| REQ-D3-03 | d3 | `from-session` CLI handler | `main.rs:3417` | PASS |
| REQ-D3-04 | d3 | `from_session(id, title, path)` | MISSING | FAIL |

---

## Gap Register

| ID | Severity | Description | Recommendation |
|----|----------|-------------|----------------|
| GAP-01 | FOLLOW-UP | `kg list --pinned` is `kg --pinned` in CLI — cosmetic CLI surface difference | Update spec to match implementation, or add alias |
| GAP-02 | FOLLOW-UP | `from_session()` signature diverged — two-step architecture adopted instead | Update `d3-session-auto-capture-plan.md` to document actual two-step API |

No blockers. Both gaps are spec-documentation misalignments, not broken features. End-to-end behaviour is correct in both cases.

---

## Verdict

**PARTIAL PASS**

- `design-gitea82-correction-event.md`: **PASS** — fully implemented, 8/8 items
- `design-gitea84-trigger-based-retrieval.md`: **PASS** — 11/12 items, cosmetic CLI deviation only
- `d3-session-auto-capture-plan.md`: **PARTIAL FAIL** — function signature diverged, end-to-end feature intact

Recommended action: update the two plan documents to reflect actual implementation. No code changes required to close these gaps.
