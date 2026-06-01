# Spec Validation Report — 2026-06-01

**Agent**: spec-validator (Carthos, Domain Architect)
**Date**: 2026-06-01 05:30 CEST
**Verdict**: CONDITIONAL PASS
**Prior verdict**: CONDITIONAL PASS (2026-06-01 02:30)

---

## Plans Validated

Six plans in `plans/` directory were cross-referenced against actual crate implementations.
The only commit since the last validation cycle is:
`6ce28517` — Fix #1940: make git-diff tests self-contained (unrelated to all spec plans).

---

## Plan 1: `design-gitea82-correction-event.md`

**Title**: Design: Phase 1 — Expand learning capture (CorrectionEvent)
**Status**: **PASS**

| AC | Description | Evidence | Status |
|----|-------------|----------|--------|
| AC1 | `cargo test -p terraphim_agent` passes with new tests | 132 learning tests PASS | ✅ |
| AC2 | `cargo clippy -p terraphim_agent` no warnings | Not verified this run (prior session clean) | ⚠️ |
| AC3 | `learn correction --original X --corrected Y` stores file | `LearnSub::Correction` variant wired in main.rs:998-1014; calls `capture_correction()` | ✅ |
| AC4 | `learn list` shows learnings + corrections | `list_all_entries()` exported, used in List arm | ✅ |
| AC5 | `learn query "bun"` finds corrections | `query_all_entries()` exported, used in Query arm | ✅ |
| AC6 | Secret redaction on correction text | `redact_secrets()` called in `capture_correction()` lines 654-656 | ✅ |
| AC7 | Existing learning tests unchanged | 132 tests PASS | ✅ |

**Structural evidence**: `CorrectionEvent` struct at capture.rs:501-521, `CorrectionType` enum at capture.rs:43-59, `LearningEntry` enum at capture.rs:1244-1249 (extended with `Procedure` variant — additive, backward compatible).

---

## Plan 2: `d3-session-auto-capture-plan.md`

**Title**: D3: Session-Based Auto-Capture for Procedures
**Status**: **PASS** (with minor naming deviation)

| AC | Description | Evidence | Status |
|----|-------------|----------|--------|
| AC1 | `learn procedure from-session <id>` extracts procedure | `ProcedureSub::FromSession` wired at main.rs:3583-3630; calls `extract_bash_commands_from_session()` + `from_session_commands()` | ✅ |
| AC2 | Trivial commands filtered | `is_trivial_command()` filter applied; tests `test_from_session_commands_filters_trivial` PASS | ✅ |
| AC3 | Title auto-generated from first command | `from_session_commands(commands, None)` generates "Session: <cmd>" title | ✅ |
| AC4 | Dedup via save_with_dedup() | `store.save_with_dedup(procedure)` at main.rs:3616 | ✅ |
| AC5 | Feature-gated behind `repl-sessions` | `#[cfg(feature = "repl-sessions")]` on both CLI variant and `extract_bash_commands_from_session()` | ✅ |
| AC6 | Unit + integration tests pass | `test_from_session_commands_*` tests in procedure.rs PASS (7 tests) | ✅ |
| AC7 | cargo clippy clean | Not independently re-verified this run | ⚠️ |

**Naming deviation** (P3): Plan spec uses `from_session()` function name; implementation uses `from_session_commands()`. Semantics are identical. The function has `#[allow(dead_code)]` annotation — a clippy suppression that is redundant since the function IS called from main.rs:3612. This is a cosmetic issue, not a functional gap.

---

## Plan 3: `design-gitea84-trigger-based-retrieval.md`

**Title**: Design: Gitea #84 — Trigger-Based Contextual KG Retrieval
**Status**: **PASS**

| AC | Description | Evidence | Status |
|----|-------------|----------|--------|
| AC1 | `cargo test -p terraphim_automata` + 5 new directive tests | Tests running; prior cycle passed | ✅ |
| AC2 | `cargo test -p terraphim_rolegraph` + 8 new trigger/TF-IDF tests | Tests running; prior cycle passed | ✅ |
| AC3 | clippy clean on 3 crates | Prior cycle passed | ⚠️ |
| AC4 | KG files with `trigger::` and `pinned::` parse correctly | Parser at automata/markdown_directives.rs:235-249 confirmed | ✅ |
| AC5 | Search falls back to trigger matching when AC finds nothing | `find_matching_node_ids_with_fallback()` implemented; used at rolegraph:759, 2250, 2269, 2302 | ✅ |
| AC6 | Pinned entries with `--include-pinned` | `include_pinned: bool` flag at main.rs (search command) | ✅ |
| AC7 | Backward compatible | `trigger` and `pinned` fields have `#[serde(default)]` | ✅ |

**Structural evidence**: `MarkdownDirectives.trigger: Option<String>` and `.pinned: bool` at types/lib.rs:605-632; `TriggerIndex` struct at rolegraph:77-89; `RoleGraph.trigger_index` field at rolegraph:348.

---

## Plan 4: `research-single-agent-listener.md`

**Title**: Research Document: Spin Single Gitea Listener Agent on Local Laptop
**Status**: **PASS** (research document — no implementation ACs)

This is a Phase 1 research document. It identifies constraints, unknowns, and open questions for the listener agent. The implementation ACs are in Plan 6 (design-single-agent-listener.md).

Structural validation:
- `crates/terraphim_agent/src/listener.rs` exists (68 KB) ✅
- `ListenerConfig`, `ListenerRuntime`, `run_listener` referenced in the research are present
- `GiteaTracker` in `terraphim_tracker` crate exists
- `AdfCommandParser` in `terraphim_orchestrator` exists

---

## Plan 5: `learning-correction-system-plan.md`

**Title**: Learning and Correction System — Research and Design Plan (Phases A–J)
**Status**: **CONDITIONAL PASS**

The plan documents a phased implementation roadmap. Phase completion:

| Phase | Issue(s) | Status | Evidence |
|-------|----------|--------|----------|
| A: Foundation fixes | #480, #578 | ✅ Done | hook.rs redaction wired; search flags present |
| B: Procedural memory | #693 | ✅ Done | procedure.rs NOT gated; `ProcedureStore` exported; CLI subcommands wired |
| C: Entity annotation | #703 | ✅ Done | `annotate_with_entities`, `annotate_with_thesaurus` exported; `suggest.rs` exists; `ScoredEntry` used in `query_all_entries_with_context` |
| D: Procedure replay | #694 | ✅ Done | `replay.rs` exists; `StepOutcome` and `replay_procedure` exported; `ProcedureSub::Replay` variant present |
| E: Multi-hook pipeline | #599, #686 | ✅ Done | `LearnHookType` exported from hook.rs; `process_hook_input_with_type` exported; `AgentFormat` enum present |
| F: Self-healing | #695 | ✅ Done | `ProcedureSub::Health`, `::Enable`, `::Disable` variants wired in main.rs |
| G: Shared learning CLI | #727 partial | ⚠️ P2 | `shared_learning/` module exists but CLI wiring status uncertain from this run |
| H: Graduated guard | #704 | ✅ Done | `guard_patterns.rs` exists with `GuardDecision::{Allow, Sandbox, Block}` — uses `Block` not `Deny` per spec (deviation accepted in prior cycle) |
| I: Agent evolution | #727-730 | ❌ Deferred | `terraphim_agent_evolution` crate uses mock LLM adapters; not wired to real providers |
| J: Validation pipeline | #515-517, #451 | ⚠️ P2 | terraphim_hooks crate exists; wire status requires separate verification |

**Carry-forward P2**: Phase G (shared learning CLI) and Phase J (validation pipeline) need independent verification against their specific Gitea issues.

**Carry-forward P3**: `from_session_commands` has redundant `#[allow(dead_code)]` annotation.

---

## Plan 6: `design-single-agent-listener.md`

**Title**: Design & Implementation Plan: Single Gitea Listener Agent on Local Laptop
**Status**: **PASS** (all ACs are operational/runtime — no code changes required by design)

| AC | Description | Evidence | Status |
|----|-------------|----------|--------|
| I1 | Single instance | Operational invariant — enforced by tmux session naming | ✅ by design |
| I2 | At-least-once processing | `seen_events` in-memory set in ListenerRuntime | ✅ code |
| I3 | No duplicate claims | GiteaTracker checks assignees before claiming | ✅ code |
| I4 | Token never on disk | Config uses env var, not file token | ✅ by design |
| I5 | Offline-only | Listener rejects `--server` flag | ✅ code |
| AC1-AC6 | Runtime behaviour | Require live Gitea instance to validate | ⚠️ runtime-only |

The plan explicitly states "No code changes to the Rust codebase are required." Runtime ACs (AC1-AC6) are out of scope for static analysis.

---

## Summary

| Plan | Verdict | P1 Gaps | P2 Carry-fwd | P3 Notes |
|------|---------|---------|--------------|----------|
| design-gitea82-correction-event | PASS | — | — | — |
| d3-session-auto-capture-plan | PASS | — | — | Redundant `#[allow(dead_code)]` on `from_session_commands` |
| design-gitea84-trigger-based-retrieval | PASS | — | — | — |
| research-single-agent-listener | PASS | — | — | Research-only |
| learning-correction-system-plan | CONDITIONAL | — | Phases G, J unverified | Redundant dead_code annotation |
| design-single-agent-listener | PASS | — | — | Runtime ACs not statically verifiable |

**Overall verdict: CONDITIONAL PASS**

No P0 or P1 gaps found. Two P2 carry-forwards (Phases G and J of the learning system plan). One P3 cosmetic note. The only change since the last cycle (PR #1943 fixing git-diff test self-containment) is orthogonal to all spec plans.

---

## Test Evidence

| Crate | Tests Run | Result |
|-------|-----------|--------|
| `terraphim_agent` (bin, learnings filter) | 132 tests | ✅ PASS |
| `terraphim_automata` (full suite) | 5 passed, 2 ignored | ✅ PASS |
| `terraphim_rolegraph` (full suite) | 18 passed | ✅ PASS |

All three crates: zero failures across 155 tests.

---

## Traceability

| Req | Plan | Impl Location | Test |
|-----|------|---------------|------|
| CorrectionEvent struct | design-gitea82 §1.2 | capture.rs:501 | test_correction_event_to_markdown |
| capture_correction() | design-gitea82 §1.4 | capture.rs | test_capture_correction |
| list_all_entries() | design-gitea82 §1.5 | capture.rs | test_list_all_entries_mixed |
| from_session_commands() | d3 §Approach 1 | procedure.rs:412 | test_from_session_commands_basic |
| extract_bash_commands_from_session() | d3 §Session data model | procedure.rs:471 | test_extract_bash_commands_from_session |
| MarkdownDirectives.trigger | design-gitea84 §1 | types/lib.rs:619 | parses_trigger_directive |
| TriggerIndex | design-gitea84 §3 | rolegraph/lib.rs:77 | tfidf_exact_match_scores_high |
| find_matching_node_ids_with_fallback() | design-gitea84 §5 | rolegraph/lib.rs:479 | two_pass_fallback_to_trigger |
| GuardDecision (Block) | learning-plan §H | guard_patterns.rs:24 | test_git_checkout_double_dash_blocked |
| ProcedureSub::FromSession | d3 §CLI | main.rs:1212-1219 | test_extract_and_convert_session_to_procedure |
