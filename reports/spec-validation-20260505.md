# Spec Validation Report

**Date**: 2026-05-05
**Agent**: spec-validator (Carthos, Domain Architect)
**Scope**: Validate all specification documents in `plans/` against actual crate implementations

---

## Specifications Evaluated

| Spec | Type | Status | Gaps |
|------|------|--------|------|
| `design-gitea82-correction-event.md` | Implementation Design | **PASS with gaps** | Missing 8 unit tests + 1 integration test |
| `design-gitea84-trigger-based-retrieval.md` | Implementation Design | **PASS with gaps** | Missing 4 integration tests for two-pass search |
| `d3-session-auto-capture-plan.md` | Implementation Design | **PASS** | None |
| `learning-correction-system-plan.md` | Research/Design Plan | **N/A** | Not a strict implementation spec |
| `design-single-agent-listener.md` | Operational Plan | **N/A** | No code changes required |

---

## Detailed Findings

### 1. `design-gitea82-correction-event.md` -- CorrectionEvent for Learning Capture

**Implementation Status**: Core functionality fully implemented

#### Implemented (matching spec):
- `CorrectionType` enum with all 7 variants, `Display`, and `FromStr` traits
- `CorrectionEvent` struct with `to_markdown()` and `from_markdown()` methods
- `capture_correction()` function with secret redaction
- `LearningEntry` enum unifying Learning, Correction, and Procedure
- `list_all_entries()` and `query_all_entries()` functions
- `query_all_entries_semantic()` for entity-based search
- CLI `Correction` subcommand in `terraphim-agent` main.rs
- `Correct` subcommand for adding corrections to existing learnings

#### Gaps:
- **MISSING: Unit tests** -- Spec requires 8 unit tests in `capture.rs`:
  - `test_correction_event_to_markdown`
  - `test_correction_event_roundtrip`
  - `test_capture_correction`
  - `test_correction_secret_redaction`
  - `test_list_all_entries_mixed`
  - `test_query_all_entries_finds_corrections`
  - `test_correction_type_roundtrip`
  - `test_learning_entry_summary`
- **MISSING: Integration test** -- Spec requires CLI integration test for `terraphim-agent learn correction`

**Files Verified**:
- `crates/terraphim_agent/src/learnings/capture.rs` (CorrectionEvent, capture_correction)
- `crates/terraphim_agent/src/learnings/mod.rs` (exports)
- `crates/terraphim_agent/src/main.rs` (CLI commands)

---

### 2. `design-gitea84-trigger-based-retrieval.md` -- Trigger-Based Contextual KG Retrieval

**Implementation Status**: Core functionality fully implemented

#### Implemented (matching spec):
- `trigger` and `pinned` fields added to `MarkdownDirectives` in `terraphim_types`
- `trigger::` and `pinned::` parsing in `terraphim_automata/src/markdown_directives.rs`
- `TriggerIndex` struct with TF-IDF implementation in `terraphim_rolegraph`
- `find_matching_node_ids_with_fallback()` two-pass search method
- `load_trigger_index()` method on `RoleGraph`
- `query_graph_with_trigger_fallback()` full query method
- `include_pinned` flag on `Search` CLI command
- `pinned` flag on `Graph` CLI command
- `SearchQuery` includes `include_pinned` field
- 15 unit tests for `TriggerIndex` in `crates/terraphim_rolegraph/tests/trigger_index_tests.rs`
- 5 unit tests for trigger/pinned directive parsing in `markdown_directives.rs`

#### Gaps:
- **MISSING: Integration tests** -- Spec requires 4 integration tests in `terraphim_rolegraph`:
  - `two_pass_aho_corasick_first` -- When Aho-Corasick finds matches, trigger index is not consulted
  - `two_pass_fallback_to_trigger` -- When Aho-Corasick finds nothing, trigger index returns results
  - `pinned_always_included` -- Pinned entries appear in results even when no match
  - `serializable_roundtrip_preserves_triggers` -- Serialise and deserialise preserves trigger data

**Files Verified**:
- `crates/terraphim_types/src/lib.rs` (MarkdownDirectives)
- `crates/terraphim_automata/src/markdown_directives.rs` (parsing)
- `crates/terraphim_rolegraph/src/lib.rs` (TriggerIndex, RoleGraph)
- `crates/terraphim_agent/src/main.rs` (CLI flags)

---

### 3. `d3-session-auto-capture-plan.md` -- Session-Based Auto-Capture for Procedures

**Implementation Status**: Fully implemented

#### Implemented (matching spec):
- `FromSession` variant in `ProcedureSub` enum
- `from_session_commands()` function in `procedure.rs`
- `extract_bash_commands_from_session()` function
- Trivial command filtering (cd, ls, echo, etc.)
- Auto-title generation from first/last commands
- Dedup check via `ProcedureStore::save_with_dedup()`
- Feature-gated behind `repl-sessions`
- Unit tests in `procedure.rs` for all scenarios

#### Gaps:
- None identified

**Files Verified**:
- `crates/terraphim_agent/src/main.rs` (FromSession CLI)
- `crates/terraphim_agent/src/learnings/procedure.rs` (from_session_commands, tests)

---

### 4. `learning-correction-system-plan.md` -- Learning and Correction System

**Implementation Status**: Partially implemented (this is a research/design plan, not a strict spec)

#### Phase-by-Phase Assessment:

| Phase | Issues | Status | Notes |
|-------|--------|--------|-------|
| A (Foundation) | #480, #578 | Partial | Redaction wired; --robot/--format flags need verification |
| B (Procedural Memory) | #693 | Mostly Done | procedure.rs un-gated, CLI exists, hook success capture not verified |
| C (Entity Annotation) | #703 | Done | `annotate_with_entities()` implemented, semantic query exists |
| D (Replay) | #694 | Done | Replay command and engine implemented |
| E (Multi-Hook) | #599, #686 | Done | Hook types exist, LearnHookType enum implemented |
| F (Self-Healing) | #695 | Partial | Health command exists, auto-disable not verified |
| G (Shared Learning) | #727 | Partial | CLI behind feature flag, not wired by default |
| H (Graduated Guard) | #704 | Done | Guard module exists |
| I (Agent Evolution) | #727-730 | Not Done | Standalone crate, not integrated |
| J (Validation Pipeline) | #515-517, #451 | Partial | kg_validation module exists |

---

### 5. `design-single-agent-listener.md` -- Single Gitea Listener Agent

**Implementation Status**: Operational plan (no code changes required)

This specification describes operational setup (tmux, config files, token injection) rather than code changes. The underlying listener infrastructure exists in `terraphim_agent/src/listener.rs`. Validation of operational setup requires runtime environment checks outside the scope of code review.

---

## Summary

### Critical Gaps (Blockers)

1. **CorrectionEvent unit tests missing** -- 8 unit tests specified in `design-gitea82-correction-event.md` are not present in `crates/terraphim_agent/src/learnings/capture.rs`. The code is implemented but untested for:
   - Markdown roundtrip serialization
   - Secret redaction in corrections
   - Mixed learning/correction listing
   - Correction type parsing

2. **Trigger-based retrieval integration tests missing** -- 4 integration tests specified in `design-gitea84-trigger-based-retrieval.md` are not present. The two-pass search logic (Aho-Corasick + TF-IDF fallback) and pinned node inclusion are implemented but lack integration-level verification.

### Recommendations

1. **Add CorrectionEvent tests** to `crates/terraphim_agent/src/learnings/capture.rs` or a new `crates/terraphim_agent/src/learnings/capture_tests.rs`
2. **Add two-pass search integration tests** to `crates/terraphim_rolegraph/tests/trigger_index_tests.rs` or a new integration test file
3. **Close spec gap issue** after tests are added

### Verdict: **FAIL**

Two specifications (`design-gitea82-correction-event.md` and `design-gitea84-trigger-based-retrieval.md`) have implementation code that matches the design but lack the required test coverage specified in their acceptance criteria.
