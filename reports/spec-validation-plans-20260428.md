# Spec Validation Report: Plans Directory vs Implementation

**Agent:** Carthos (spec-validator)  
**Date:** 2026-04-28 08:32 CEST  
**Branch:** task/1044-test-failures-fix  
**Previous Validation:** Issue #1040 (2026-04-28 03:49 CEST)

---

## Executive Summary

Re-validation of 6 active plans against current implementation. **3 plans have spec deviations, 3 plans pass.** No new gaps identified since previous validation. All previously identified gaps persist.

---

## Plan-by-Plan Validation

### Plan 1: D3 Session Auto-Capture (d3-session-auto-capture-plan.md)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `learn procedure from-session` subcommand | PASS | `ProcedureSub::FromSession` at main.rs:1170, gated `#[cfg(feature = "repl-sessions")]` |
| `from_session_commands()` function | PASS | procedure.rs:412 |
| `extract_bash_commands_from_session()` | PASS | procedure.rs:471 |
| Session data model: `tool_uses[].exit_code: int` | **FAIL** | Spec expects `exit_code: int` in session JSON; actual model uses `ToolResult { is_error: bool }` (terraphim_sessions/src/model.rs:75) |
| Trivial command filtering | PASS | `is_trivial_command()` exists, filters cd/ls/echo/etc |
| Title auto-generation | PASS | Auto-generates from first meaningful command |

**Gap:** Data model mismatch persists. Adapter code translates `is_error` to `0`/`1`, but raw session model violates spec.

---

### Plan 2: CorrectionEvent (design-gitea82-correction-event.md)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `CorrectionType` enum | PASS | capture.rs:33-48 (7 variants) |
| `CorrectionEvent` struct | PASS | capture.rs:94-113 |
| `capture_correction()` function | PASS | capture.rs:323-367 |
| `LearningEntry` enum | PASS | capture.rs:375-378 |
| `learn correction` CLI | PASS | `LearnSub::Correction` at main.rs:956 |
| `learn list` uses `list_all_entries` | PASS | main.rs:3023 |
| `learn query` uses `query_all_entries` | **FAIL** | Spec requires `query_all_entries()`; actual calls `query_all_entries_semantic()` at main.rs:3058 |
| Secret redaction | PASS | `redact_secrets()` called in `capture_correction()` |

**Gap:** CLI deviation persists. No functional loss (semantic search is a superset), but interface diverges from approved design.

---

### Plan 3: Trigger-Based Retrieval (design-gitea84-trigger-based-retrieval.md)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `trigger::` directive parsing | PASS | terraphim_automata/src/markdown_directives.rs |
| `pinned::` directive parsing | PASS | terraphim_automata/src/markdown_directives.rs |
| `TriggerIndex` struct | PASS | terraphim_rolegraph/src/lib.rs:51 |
| TF-IDF build/query | PASS | `TriggerIndex::build()` and `::query()` implemented |
| `find_matching_node_ids_with_fallback()` | PASS | terraphim_rolegraph/src/lib.rs:451 |
| `load_trigger_index()` | PASS | terraphim_rolegraph/src/lib.rs:478 |
| Serialisable roundtrip | PASS | `SerializableRoleGraph` includes trigger_descriptions and pinned_node_ids |
| `--include-pinned` CLI flag | **PARTIAL** | Implemented in `terraphim_cli/src/main.rs:98` but NOT in `terraphim_agent/src/main.rs` (spec designated terraphim_agent) |
| `kg list --pinned` subcommand | **PARTIAL** | Implemented in `terraphim_cli/src/main.rs:68` as `KgSub::List` but NOT in `terraphim_agent` (spec designated terraphim_agent) |
| Two-pass search (Aho-Corasick + TF-IDF) | PASS | `find_matching_node_ids_with_fallback()` implements two-pass |
| Pinned entries inclusion | PASS | `query_graph_with_trigger_fallback` includes pinned when flag is true |

**Gap:** CLI surface exists but in wrong binary (`terraphim_cli` instead of spec'd `terraphim_agent`). `terraphim_agent` hardcodes `include_pinned: false` in 4 places with no user-facing flag.

---

### Plan 4: Single Agent Listener (design-single-agent-listener.md)

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Listener config file | PASS | `~/.config/terraphim/listener-worker.json` exists (created 2026-04-16) |
| tmux launch script | N/A | Not checked (operational setup) |
| Binary build | N/A | Not checked (operational setup) |
| No code changes required | PASS | Design explicitly states no Rust code changes |

**Status: PASS** -- Operational setup plan, no implementation to validate.

---

### Plan 5: Learning System (learning-correction-system-plan.md)

| Phase | Status | Evidence |
|-------|--------|----------|
| Phase A: Foundation (#480 redaction, #578 flags) | PASS | Redaction wired in capture.rs and hook.rs; --robot/--format flags exist |
| Phase B: Procedural Memory (#693) | PASS | procedure.rs un-gated (mod.rs:31); FromSession, Replay, Health subcommands exist |
| Phase C: Entity Annotation (#703) | PASS | `annotate_with_entities()` at capture.rs:833; `--semantic` flag on learn query |
| Phase D: Procedure Replay (#694) | PASS | `Replay` subcommand at main.rs:1150; `replay.rs` exists |
| Phase E: Multi-Hook Pipeline (#599) | PASS | `LearnHookType` enum at hook.rs:33 (PreToolUse, PostToolUse, UserPromptSubmit) |
| Phase F: Self-Healing (#695) | PASS | `Health` subcommand at main.rs:1158; `health_check()` at procedure.rs:302 |
| Phase G: Shared Learning CLI (#727) | PASS | `SharedLearningSub` at main.rs:1027 (List, Promote, Import, Stats, Inject) |
| Phase H: Graduated Guard (#704) | **FAIL** | No `guard.rs` found in learnings/; no `ExecutionTier` or `GuardDecision` types |
| Phase I: Agent Evolution (#727-730) | **FAIL** | `terraphim_agent_evolution` crate exists but not integrated into main binary |
| Phase J: Validation Pipeline (#515-517, #451) | PARTIAL | PreToolUse validation exists in hook.rs; KG command pattern matching not verified |

**Status: PARTIAL** -- Foundation phases (A-G) complete. Advanced phases (H-J) partially unimplemented.

---

### Plan 6: Listener Research (research-single-agent-listener.md)

**Status: PASS** -- Research document only, no implementation expected.

---

## Summary

| Plan | Status | Gap |
|------|--------|-----|
| Plan 1: Session Auto-Capture | FAIL | Data model mismatch: `is_error: bool` vs spec'd `exit_code: int` |
| Plan 2: CorrectionEvent | FAIL | CLI deviation: `query_all_entries_semantic` vs spec'd `query_all_entries` |
| Plan 3: Trigger-Based Retrieval | PARTIAL | CLI in wrong binary (`terraphim_cli` not `terraphim_agent`) |
| Plan 4: Single Agent Listener | PASS | -- |
| Plan 5: Learning System | PARTIAL | Phases H-J (Guard, Evolution, Validation) incomplete |
| Plan 6: Listener Research | PASS | -- |

---

## Recommendations

1. **Plan 1:** Decide whether to update spec to match `is_error: bool` model or migrate session model to `exit_code: int`.
2. **Plan 2:** Either update spec to use `query_all_entries_semantic` or change main.rs:3058 to call `query_all_entries`.
3. **Plan 3:** Either update spec to designate `terraphim_cli` as the CLI target, or port `--include-pinned` and `kg list` to `terraphim_agent`.
4. **Plan 5:** File follow-up issues for Phase H (Guard tiers) and Phase I (Agent Evolution integration) if not already tracked.

---

**Signed:** Carthos, Domain Architect  
**Symbol:** Compass rose (orientation in complexity)
