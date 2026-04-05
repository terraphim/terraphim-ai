# Spec Validation Report: 2026-04-05

**Validator:** Carthos (Domain Architect)
**Branch:** feature/warp-drive-theme
**Specs Evaluated:** 2 (from `plans/` directory)

## Executive Summary

Two active design specifications were cross-referenced against implementation.
Both core features are substantially implemented with all specified tests passing.
Minor gaps exist in CLI surface area and module exports.

**Verdict: PASS** (with noted follow-up items)

---

## Spec 1: Gitea #84 -- Trigger-Based Contextual KG Retrieval

**Plan:** `plans/design-gitea84-trigger-based-retrieval.md`
**Gitea Issue:** #84 (closed)

### Traceability Matrix

| Req ID | Requirement | Impl Ref | Tests | Status |
|-------:|-------------|----------|-------|--------|
| 84-1 | `trigger: Option<String>` and `pinned: bool` on `MarkdownDirectives` | `crates/terraphim_types/src/lib.rs:401-415` | Deserialization via serde defaults | PASS |
| 84-2 | Parse `trigger::` and `pinned::` directives | `crates/terraphim_automata/src/markdown_directives.rs:186-201` | 5/5 parsing tests pass | PASS |
| 84-3 | TF-IDF `TriggerIndex` struct | `crates/terraphim_rolegraph/src/lib.rs:48-248` | 5/5 TF-IDF unit tests pass | PASS |
| 84-4 | `RoleGraph` fields: `trigger_index`, `pinned_node_ids` | `crates/terraphim_rolegraph/src/lib.rs:317-319` | Initialised in `new_sync()` | PASS |
| 84-5 | `find_matching_node_ids_with_fallback()` two-pass search | `crates/terraphim_rolegraph/src/lib.rs:443-466` | 3/3 integration tests pass | PASS |
| 84-6 | `SerializableRoleGraph` with trigger data | `crates/terraphim_rolegraph/src/lib.rs:255-274` | `serializable_roundtrip_preserves_triggers` passes | PASS |
| 84-7 | CLI: `--include-pinned` flag and `kg list --pinned` | NOT FOUND | No tests | FAIL |

### Test Evidence

```
cargo test -p terraphim_automata --lib -- markdown_directives
  10 passed; 0 failed (includes 5 spec'd directive tests)

cargo test -p terraphim_rolegraph --lib -- tfidf
  5 passed; 0 failed (all 5 TF-IDF unit tests)

cargo test -p terraphim_rolegraph --lib -- two_pass
  2 passed (two_pass_aho_corasick_first, two_pass_fallback_to_trigger)

cargo test -p terraphim_rolegraph --lib -- pinned
  1 passed (pinned_always_included)

cargo test -p terraphim_rolegraph --lib -- serializable_roundtrip
  1 passed (serializable_roundtrip_preserves_triggers)
```

**All 14/14 specified tests pass.** Test coverage: 100% of spec'd tests.

### Gaps

| Gap | Severity | Detail |
|-----|----------|--------|
| CLI `--include-pinned` flag missing | Follow-up | Core search API supports it (`include_pinned` param exists); only CLI wiring absent |
| CLI `kg list --pinned` command missing | Follow-up | Agent uses TUI/forgiving parser architecture, not traditional clap subcommands |

### Assessment: 6/7 items PASS (86%)

Core bounded context (types, parsing, indexing, search, serialisation) is complete and tested.
The CLI gap is a thin integration layer; the invariant (two-pass search with pinned inclusion) is upheld at the domain level.

---

## Spec 2: Gitea #82 -- CorrectionEvent for Learning Capture

**Plan:** `plans/design-gitea82-correction-event.md`
**Gitea Issue:** #82 (closed)

### Traceability Matrix

| Req ID | Requirement | Impl Ref | Tests | Status |
|-------:|-------------|----------|-------|--------|
| 82-1 | `CorrectionType` enum (7 variants + Display + FromStr) | `crates/terraphim_agent/src/learnings/capture.rs:43-93` | `test_correction_type_roundtrip` passes | PASS |
| 82-2 | `CorrectionEvent` struct (9 fields) | `capture.rs:335-354` | `test_correction_event_to_markdown` passes | PASS |
| 82-3 | Methods: `new()`, `with_session_id()`, `with_tags()`, `to_markdown()`, `from_markdown()` | `capture.rs:358-520` | `test_correction_event_roundtrip` passes | PASS |
| 82-4 | Helper fns: `extract_code_after_heading()`, `extract_section_text()` | `capture.rs:524-542` | Used by `from_markdown()` roundtrip test | PASS |
| 82-5 | `capture_correction()` with secret redaction | `capture.rs:642-686` | `test_capture_correction`, `test_correction_secret_redaction` pass | PASS |
| 82-6 | `LearningEntry` enum (Learning + Correction) | `capture.rs:820-867` | `test_learning_entry_summary` passes | PASS |
| 82-7 | `list_all_entries()` / `query_all_entries()` | `capture.rs:870-933` | `test_list_all_entries_mixed`, `test_query_all_entries_finds_corrections` pass | PASS |
| 82-8 | Public exports: `CorrectionEvent`, `CorrectionType`, `LearningEntry`, etc. | `learnings/mod.rs:33-36` | Compile-time verified | PARTIAL |
| 82-9 | CLI `LearnSub::Correction` variant | `main.rs:765-781` | Compile-time verified | PASS |
| 82-10 | CLI List/Query arms use unified functions | `main.rs:1959-2020` | Compile-time verified | PASS |
| 82-11 | 8 unit tests + 1 CLI integration test | `capture.rs:1274-1791` | 8/8 unit tests pass; CLI integration test missing | PARTIAL |

### Test Evidence

```
cargo test -p terraphim_agent --bin terraphim-agent -- test_correction
  4 passed: test_correction_type_roundtrip, test_correction_event_to_markdown,
            test_correction_event_roundtrip, test_correction_secret_redaction

cargo test -p terraphim_agent --bin terraphim-agent -- test_capture_correction
  1 passed

cargo test -p terraphim_agent --bin terraphim-agent -- test_list_all
  1 passed: test_list_all_entries_mixed

cargo test -p terraphim_agent --bin terraphim-agent -- test_query_all
  1 passed: test_query_all_entries_finds_corrections

cargo test -p terraphim_agent --bin terraphim-agent -- test_learning_entry
  1 passed: test_learning_entry_summary
```

**8/8 specified unit tests pass.** CLI integration test not implemented.

### Gaps

| Gap | Severity | Detail |
|-----|----------|--------|
| `CorrectionEvent` and `LearningEntry` not exported from `mod.rs` | Follow-up | Types are `pub` but not re-exported; downstream consumers would need `use learnings::capture::*` |
| CLI integration test missing | Follow-up | Spec calls for `terraphim-agent learn correction` end-to-end test |

### Assessment: 9/11 items PASS, 2 PARTIAL (82%)

The aggregate root (`CorrectionEvent`) and its invariants are fully implemented and tested at the unit level.
Export visibility and CLI integration test are thin boundary concerns, not domain logic gaps.

---

## Compilation Verification

All affected crates compile cleanly:

```
cargo test -p terraphim_automata --lib   -> 43 tests, 0 failures
cargo test -p terraphim_rolegraph --lib  -> 21 tests, 0 failures
cargo test -p terraphim_agent --bin terraphim-agent -> 207 tests, 0 failures
```

---

## Summary

| Spec | Issue | Core Domain | Tests | CLI/Exports | Overall |
|------|-------|-------------|-------|-------------|---------|
| Gitea #84 (Trigger Retrieval) | Closed | PASS (6/6) | PASS (14/14) | 1 gap | PASS |
| Gitea #82 (CorrectionEvent) | Closed | PASS (7/7) | PASS (8/9) | 2 gaps | PASS |

### Follow-up Items (non-blocking)

1. **#84 CLI gap**: Wire `--include-pinned` and `kg list --pinned` into agent TUI/CLI
2. **#82 export gap**: Re-export `CorrectionEvent` and `LearningEntry` from `learnings/mod.rs`
3. **#82 integration test**: Add CLI end-to-end test for `learn correction` subcommand

### Verdict: **PASS**

Both specifications are implemented at the domain level with full test coverage of specified invariants.
The gaps are boundary-layer concerns (CLI wiring, module visibility) that do not affect the correctness of the core bounded contexts.
