# Spec Validation Report

**Agent:** Carthos (Domain Architect / spec-validator)
**Date:** 2026-05-05
**Repo:** terraphim/terraphim-ai

## Methodology

1. Read all active specifications from `plans/` directory
2. Cross-reference specified types, functions, CLI commands, and test cases against actual crate implementations
3. Identify gaps between specification and code

## Specifications Reviewed

| Spec | Status | Issue |
|------|--------|-------|
| `design-gitea82-correction-event.md` | **PASS** | Gitea #82 |
| `design-gitea84-trigger-based-retrieval.md` | **FAIL** | Gitea #84 |
| `d3-session-auto-capture-plan.md` | **PASS** | #693 (D3) |
| `learning-correction-system-plan.md` | **PARTIAL** | #693, #599, #703, #694, #695, #727 |
| `design-single-agent-listener.md` | **N/A** | Operational (no code changes) |
| `research-single-agent-listener.md` | **N/A** | Research document |

---

## Detailed Findings

### 1. design-gitea82-correction-event.md -- PASS

All specified items are implemented and wired:

- `CorrectionType` enum with all 7 variants (`ToolPreference`, `CodePattern`, `Naming`, `WorkflowStep`, `FactCorrection`, `StylePreference`, `Other`) -- `crates/terraphim_agent/src/learnings/capture.rs:42`
- `CorrectionEvent` struct with all fields -- `capture.rs:502`
- `to_markdown()` / `from_markdown()` roundtrip -- `capture.rs:561`
- `capture_correction()` function with secret redaction -- `capture.rs:1022`
- `LearningEntry` enum unifying Learning, Correction, Procedure -- `capture.rs:1225`
- `list_all_entries()` loading learnings + corrections + procedures -- `capture.rs:1298`
- `query_all_entries()` and `query_all_entries_semantic()` -- `capture.rs:1372`, `capture.rs:1410`
- CLI `learn correction` subcommand -- `main.rs:3138`
- CLI `learn list` and `learn query` using unified functions -- `main.rs`
- 6+ unit tests covering markdown roundtrip, secret redaction, query, type parsing -- `capture.rs` test section

**Verdict:** Spec fully satisfied. No gaps.

---

### 2. design-gitea84-trigger-based-retrieval.md -- FAIL

The core data structures and algorithms are implemented, but production wiring is incomplete:

#### 2.1 Trigger Index Never Loaded in Production (Critical)

- `TriggerIndex` struct with TF-IDF cosine similarity -- `crates/terraphim_rolegraph/src/lib.rs:51`
- `load_trigger_index()` method -- `lib.rs:478`
- **GAP:** `load_trigger_index()` is **only called in unit tests** (`lib.rs:2205`, `2224`, `2243`). It is **never called** during RoleGraph construction in production code.
- RoleGraph is constructed in:
  - `crates/terraphim_config/src/lib.rs:983`, `1008`, `1044`
  - `crates/terraphim_service/src/lib.rs:196`, `257`, `359`, `451`
- None of these construction sites call `load_trigger_index()`.
- **Consequence:** The TF-IDF trigger fallback is **dead code** in production. Parsed `trigger::` and `pinned::` directives from KG markdown files are ignored after parsing.

#### 2.2 Auto-Route Path Misses Trigger Fallback

- `terraphim_service/src/auto_route.rs:44` calls `rg.find_matching_node_ids(query)` directly.
- **GAP:** It does not use `find_matching_node_ids_with_fallback()`, so auto-routing never benefits from trigger-based matching.

#### 2.3 CLI Command Name Mismatch

- Spec requests: `kg list --pinned`
- Implementation provides: `graph --pinned` (`main.rs:738`)
- **GAP:** Different command semantics. `graph` displays role graph concepts; `kg list` would list KG entries. The spec's command structure was not implemented.

#### 2.4 Implemented Correctly

- `MarkdownDirectives` has `trigger: Option<String>` and `pinned: bool` -- `terraphim_types/src/lib.rs:323`
- `markdown_directives.rs` parses `trigger::` and `pinned::` with 5 unit tests -- `terraphim_automata/src/markdown_directives.rs:215`
- `find_matching_node_ids_with_fallback()` implements two-pass search (Aho-Corasick + TF-IDF) -- `terraphim_rolegraph/src/lib.rs:455`
- `query_graph_with_trigger_fallback()` is wired in `terraphim_config/src/lib.rs:1155` (but only when `include_pinned` is true; index is still empty)
- `--include-pinned` flag exists on search command -- `main.rs:718`
- 8 unit tests for TriggerIndex and two-pass search -- `terraphim_rolegraph/src/lib.rs:2117`

**Verdict:** Spec violated. Production wiring missing for trigger index population.

---

### 3. d3-session-auto-capture-plan.md -- PASS

All specified items are implemented:

- `learn procedure from-session <session-id>` CLI -- `main.rs:1180`
- `FromSession` variant in `ProcedureSub` -- `main.rs:1115`
- `extract_bash_commands_from_session()` and `from_session_commands()` -- `procedure.rs`
- Trivial command filtering (`cd`, `ls`, `echo`, etc.) -- `procedure.rs`
- Title auto-generation from first/last commands -- `procedure.rs`
- Dedup via `ProcedureStore::save_with_dedup()` -- `procedure.rs`

**Verdict:** Spec fully satisfied.

---

### 4. learning-correction-system-plan.md -- PARTIAL

This is a multi-phase plan. Status by phase:

| Phase | Issues | Status | Notes |
|-------|--------|--------|-------|
| A: Foundation | #480, #578 | **PASS** | Redaction wired in hook stdout passthrough (`hook.rs:124`). `--robot`/`--format` flags wired to search output (`main.rs:575`). |
| B: Procedural Memory | #693 | **PARTIAL** | `procedure.rs` un-gated (`mod.rs:31`). CLI subcommands all implemented. **GAP:** Success capture in hook pipeline NOT implemented. No `should_capture_success()`, `capture_success_from_hook()`, or `SessionCommandBuffer`. Hook only captures failures (`hook.rs:384`). |
| C: Entity Annotation | #703 | **PASS** | `annotate_with_entities()` exists (`capture.rs:833`). Used in capture pipeline (`capture.rs:982`) and semantic query (`capture.rs:1410`). |
| D: Procedure Replay | #694 | **PASS** | `replay.rs` exists with `replay_procedure()`, `StepOutcome`, `ReplayResult`. CLI `learn procedure replay` wired. |
| E: Multi-Hook Pipeline | #599, #686 | **PASS** | `LearnHookType` enum with `PreToolUse`, `PostToolUse`, `UserPromptSubmit` (`hook.rs:33`). `ImportanceScore` exists (`capture.rs:101`). |
| F: Self-Healing | #695 | **PASS** | `ProcedureHealthReport`, `health_check()`, `auto_disabled` flag (`procedure.rs:74`, `302`). CLI `learn procedure health` wired. |
| G: Shared Learning | #727 | **PASS** | `learn shared` CLI subcommands wired (`main.rs:3526`). |
| H: Graduated Guard | #704 | **NOT VALIDATED** | Out of scope for this validation cycle. |
| I: Agent Evolution | #727-#730 | **NOT VALIDATED** | Out of scope. |
| J: Validation Pipeline | #515-#517, #451 | **NOT VALIDATED** | Out of scope. |

**Key Gap:** Phase B success capture in hook pipeline is the only unimplemented item from the reviewed phases.

---

### 5. design-single-agent-listener.md -- N/A

This spec explicitly states: "**No code changes to the Rust codebase are required.**" It is an operational setup task.

- Config file (`listener-worker.json`) and launch script (`start-listener.sh`) exist per spec.
- Binary exists at `~/.cargo/bin/terraphim-agent`.
- **Operational gap:** tmux session `terraphim-worker` is not currently running. This is outside the scope of code validation.

---

## Summary

| Spec | Verdict | Critical Gaps |
|------|---------|---------------|
| design-gitea82 | PASS | None |
| design-gitea84 | **FAIL** | 1. `load_trigger_index()` never called in production 2. `auto_route.rs` misses fallback 3. Missing `kg list --pinned` |
| d3-session-auto-capture | PASS | None |
| learning-correction-system | **PARTIAL** | 1. Success capture in hook pipeline not implemented |
| design-single-agent-listener | N/A | Operational (tmux not running) |

**Overall Verdict: FAIL**

Specs `design-gitea84` and `learning-correction-system` (Phase B) have implementation gaps that violate their acceptance criteria.
