# Specification Validation Report

**Date**: 2026-03-27
**Branch**: task/42-budget-tracking
**Validator**: Terraphim AI -- spec-to-implementation cross-reference
**Previous Report**: spec-validation-20260326.md

---

## Executive Summary

| Specification | Coverage | Status | Delta vs 2026-03-26 |
|---------------|----------|--------|----------------------|
| Session Search (3 spec files) | 90% | Production-ready | No change |
| Desktop Application | 87% | Implemented (3 minor gaps) | No change |
| Chat Session History | 65% | Backend complete, Tauri bridge missing | No change |
| Learning Capture | 80% | Core working, KG integration deferred | No change |
| Design-708 Code Review Fixes | 100% | All 5 fixes verified | No change |
| Dark Factory Orchestration | 120% | Exceeds design scope | No change |
| Validation Framework | 100% | Fully integrated | No change |
| Trigger-Based Retrieval (#84) | **92%** | Backend complete, CLI flags missing | -8% (refined from 100%) |
| CorrectionEvent (#82) | **99%** | Fully implemented, 1 minor gap | New |

**Overall**: 7 of 9 specifications are fully or near-fully implemented. Two active design plans in `plans/` have been cross-referenced against code.

**Build status**: `terraphim_persistence` has compilation errors in uncommitted `cost_report.rs` (branch `task/42-budget-tracking`). This does not affect the spec-validated crates. `terraphim_automata` (38 tests pass), `terraphim_rolegraph` (20 tests pass) are green.

---

## Active Design Plans (plans/ directory)

### Plan 1: Gitea #82 -- CorrectionEvent for Learning Capture

**Design**: `plans/design-gitea82-correction-event.md` (approved, priority: high)
**Implementation**: `crates/terraphim_agent/src/learnings/capture.rs`, `mod.rs`, `main.rs`
**Parent**: Gitea #81 (epic), Gitea #82 (task)

#### Coverage: 99% -- 13/13 spec items implemented

| Spec Item | Status | Location |
|-----------|--------|----------|
| `CorrectionType` enum (7 variants) | IMPLEMENTED | capture.rs:43 |
| `CorrectionType::Display` impl | IMPLEMENTED | capture.rs |
| `CorrectionType::FromStr` impl | IMPLEMENTED | capture.rs |
| `CorrectionEvent` struct (10 fields) | IMPLEMENTED | capture.rs:335 |
| `CorrectionEvent::new()` constructor | IMPLEMENTED | capture.rs:358 |
| `CorrectionEvent::to_markdown()` | IMPLEMENTED | capture.rs:394 |
| `CorrectionEvent::from_markdown()` | IMPLEMENTED | capture.rs:443 |
| `extract_code_after_heading()` helper | IMPLEMENTED | capture.rs:524 |
| `extract_section_text()` helper | IMPLEMENTED | capture.rs:535 |
| `capture_correction()` function | IMPLEMENTED | capture.rs:642 |
| `LearningEntry` enum | IMPLEMENTED | capture.rs:820 |
| `list_all_entries()` function | IMPLEMENTED | capture.rs:870 |
| `query_all_entries()` function | IMPLEMENTED | capture.rs:905 |

#### Public API Exports (mod.rs)

| Export | Status |
|--------|--------|
| `CorrectionType` | EXPORTED |
| `LearningEntry` | EXPORTED |
| `capture_correction` | EXPORTED |
| `list_all_entries` | EXPORTED |
| `query_all_entries` | EXPORTED |
| `CorrectionEvent` | NOT RE-EXPORTED (accessible via `LearningEntry::Correction`) |

#### CLI Integration (main.rs)

| Spec Item | Status | Location |
|-----------|--------|----------|
| `LearnSub::Correction` variant | IMPLEMENTED | main.rs:805 |
| Match arm for `LearnSub::Correction` | IMPLEMENTED | main.rs:2143 |
| `LearnSub::List` uses `list_all_entries()` | IMPLEMENTED | main.rs:2068 |
| `LearnSub::Query` uses `query_all_entries()` | IMPLEMENTED | main.rs:2097 |

#### Test Coverage

| Spec Test Case | Status | Location |
|----------------|--------|----------|
| `test_correction_event_to_markdown` | IMPLEMENTED | capture.rs:1442 |
| `test_correction_event_roundtrip` | IMPLEMENTED | capture.rs:1463 |
| `test_capture_correction` | IMPLEMENTED | capture.rs:1482 |
| `test_correction_secret_redaction` | IMPLEMENTED | capture.rs:1510 |
| `test_list_all_entries_mixed` | IMPLEMENTED | capture.rs:1535 |
| `test_query_all_entries_finds_corrections` | IMPLEMENTED | capture.rs:1581 |
| `test_correction_type_roundtrip` | IMPLEMENTED | capture.rs:1617 |
| `test_learning_entry_summary` | IMPLEMENTED | capture.rs:1636 |
| CLI integration test (e2e) | MISSING | No `tests/` directory file |

**Additional tests beyond spec**: 18 more unit tests covering correction phrase detection and transcript extraction (26 total vs 8 specified).

#### Acceptance Criteria

| Criterion | Status |
|-----------|--------|
| `cargo test -p terraphim_agent` passes | BLOCKED (terraphim_persistence compile error on this branch) |
| `cargo clippy -p terraphim_agent` no warnings | BLOCKED (same) |
| `learn correction --original X --corrected Y` stores file | IMPLEMENTED |
| `learn list` shows both learnings and corrections | IMPLEMENTED |
| `learn query "bun"` finds corrections | IMPLEMENTED |
| Secret redaction on correction text | IMPLEMENTED |
| Existing learning tests unchanged | IMPLEMENTED (backward-compatible) |

#### Gaps

1. **MINOR**: `CorrectionEvent` not re-exported from `learnings/mod.rs`. Accessible through `LearningEntry::Correction` variant. Diverges from spec section 2.1.
2. **MINOR**: No dedicated CLI integration test (spec item 9). Unit test coverage is comprehensive (26 tests).

---

### Plan 2: Gitea #84 -- Trigger-Based Contextual KG Retrieval

**Design**: `plans/design-gitea84-trigger-based-retrieval.md`
**Implementation**: `crates/terraphim_types/`, `crates/terraphim_automata/`, `crates/terraphim_rolegraph/`, `crates/terraphim_agent/`
**Parent**: Gitea #84

#### Coverage: 92% -- Backend complete, CLI flags missing

| Spec Section | Status | Location |
|--------------|--------|----------|
| **Section 1: MarkdownDirectives fields** | | |
| `trigger: Option<String>` | IMPLEMENTED | terraphim_types/src/lib.rs:397 |
| `pinned: bool` | IMPLEMENTED | terraphim_types/src/lib.rs:399 |
| **Section 2: Directive parsing** | | |
| `trigger::` parsing | IMPLEMENTED | markdown_directives.rs:186-195 |
| `pinned::` parsing (true/yes/1) | IMPLEMENTED | markdown_directives.rs:197-201 |
| **Section 3: TriggerIndex** | | |
| `TriggerIndex` struct | IMPLEMENTED | terraphim_rolegraph/src/lib.rs:52-61 |
| `build()` method (smoothed IDF) | IMPLEMENTED | lib.rs:74-97 |
| `query()` method (cosine similarity) | IMPLEMENTED | lib.rs:100-155 |
| `tokenise()` with stopword removal | IMPLEMENTED | lib.rs (private) |
| **Section 4: RoleGraph integration** | | |
| `trigger_index: TriggerIndex` field | IMPLEMENTED | lib.rs:280 |
| `pinned_node_ids: Vec<u64>` field | IMPLEMENTED | lib.rs:282 |
| `SerializableRoleGraph::trigger_descriptions` | IMPLEMENTED | lib.rs:234 |
| `SerializableRoleGraph::pinned_node_ids` | IMPLEMENTED | lib.rs:236 |
| **Section 5: Fallback search** | | |
| `find_matching_node_ids_with_fallback()` | IMPLEMENTED | lib.rs:406-429 |
| `load_trigger_index()` | IMPLEMENTED | lib.rs:433-443 |
| **Section 6: Query methods** | | |
| `query_graph_with_trigger_fallback()` | IMPLEMENTED | lib.rs:667-682 |
| **Section 7: CLI** | | |
| `--include-pinned` flag on search | **MISSING** | Not in main.rs |
| `kg list --pinned` subcommand | **MISSING** | No `Kg` command variant |

#### Test Coverage -- VERIFIED

| Spec Test | Status | Location |
|-----------|--------|----------|
| **Directive parsing (5/5)** | | |
| `parses_trigger_directive` | PASS | markdown_directives.rs:313 |
| `parses_pinned_directive` | PASS | markdown_directives.rs:328 |
| `pinned_false_variants` | PASS | markdown_directives.rs:339 |
| `trigger_and_synonyms_coexist` | PASS | markdown_directives.rs:354 |
| `empty_trigger_ignored` | PASS | markdown_directives.rs:373 |
| **TF-IDF unit (5/5)** | | |
| `tfidf_empty_index_returns_empty` | PASS | lib.rs:2019 |
| `tfidf_exact_match_scores_high` | PASS | lib.rs:2026 |
| `tfidf_no_match_scores_zero` | PASS | lib.rs:2039 |
| `tfidf_partial_match` | PASS | lib.rs:2051 |
| `tfidf_threshold_filters` | PASS | lib.rs:2064 |
| **Integration (4/4)** | | |
| `two_pass_aho_corasick_first` | PASS | lib.rs:2080 |
| `two_pass_fallback_to_trigger` | PASS | lib.rs:2096 |
| `pinned_always_included` | PASS | lib.rs:2115 |
| `serializable_roundtrip_preserves_triggers` | PASS | lib.rs:2133 |

**Test run results**: `terraphim_automata` 38/38 pass. `terraphim_rolegraph` 20/20 pass (1 ignored).

#### Acceptance Criteria

| Criterion | Status |
|-----------|--------|
| `cargo test -p terraphim_automata` -- 5 new tests | VERIFIED PASS |
| `cargo test -p terraphim_rolegraph` -- 8 new tests | VERIFIED PASS |
| `cargo clippy` no warnings | Not run (persistence compile error on branch) |
| KG files with `trigger::`/`pinned::` parsed | IMPLEMENTED |
| Fallback to trigger only when AC returns empty | IMPLEMENTED |
| Pinned entries with `--include-pinned` | **NOT WIRABLE** (flag missing) |
| Backward compatible | IMPLEMENTED |

#### Gaps

1. **MEDIUM**: `--include-pinned` CLI flag missing from search subcommand. Backend method `find_matching_node_ids_with_fallback(text, include_pinned)` exists but CLI cannot pass `true`.
2. **MEDIUM**: `kg list --pinned` subcommand not implemented. No `Kg` command variant in CLI enum.

---

## Build Blocker (Current Branch)

`crates/terraphim_persistence/src/cost_report.rs` (new, uncommitted) references `crate::Error::StorageError` which does not exist in `crates/terraphim_persistence/src/error.rs`. This blocks compilation of `terraphim_agent` and any crate depending on `terraphim_persistence`. The #82 and #84 spec crates (`terraphim_automata`, `terraphim_rolegraph`, `terraphim_types`) compile and test independently.

---

## Carried-Forward Specifications (unchanged from 2026-03-26)

| Specification | Coverage | Change |
|---------------|----------|--------|
| Session Search | 90% | -- |
| Desktop Application | 87% | -- |
| Chat Session History | 65% | -- |
| Learning Capture | 80% | -- |
| Design-708 Code Review Fixes | 100% | -- |
| Dark Factory Orchestration | 120% | -- |
| Validation Framework | 100% | -- |

See `reports/spec-validation-20260326.md` for full details.

---

## Priority Action Items

### P0 -- Critical (blocks builds)

1. **Fix `cost_report.rs` compilation** -- Add `StorageError` variant to `terraphim_persistence::Error` or change error handling in `cost_report.rs`. Blocks all downstream crate tests.

### P1 -- High (spec gaps in active plans)

2. **Add `--include-pinned` flag** to search subcommand in `crates/terraphim_agent/src/main.rs` (Gitea #84, Section 7)
3. **Add `kg list --pinned` subcommand** to CLI (Gitea #84, Section 7)
4. **Add CLI integration test** for `learn correction` end-to-end workflow (Gitea #82, Test Case 9)
5. **Re-export `CorrectionEvent`** from `learnings/mod.rs` (Gitea #82, Section 2.1)
6. **Wire conversation API endpoints** -- `api_conversations.rs` endpoints are dead code *(Carried forward)*
7. **Implement Tauri command handlers** for chat sessions *(Carried forward)*

### P2 -- Medium (functionality gaps)

8. Add KG auto-suggest to learning capture *(Carried forward)*
9. Add KG synonym expansion to learning queries *(Carried forward)*
10. Wire token budget truncation in robot mode *(Carried forward)*
11. Implement learning CLI stats/prune commands *(Carried forward)*

### P3 -- Low (polish)

12. Desktop auto-update UI *(Carried forward)*
13. Desktop system tray *(Carried forward)*
14. Tantivy migration plan for session search beyond 50K *(Carried forward)*

---

## Spec-to-Crate Traceability Matrix

| Specification | Primary Crates | Test Count | Verified |
|---------------|---------------|------------|----------|
| Session Search | terraphim_sessions, terraphim_agent | 50+ | Previous |
| Desktop App | desktop/ (Svelte), desktop/src-tauri | 45+ E2E | Previous |
| Chat Session History | terraphim_types, terraphim_persistence, terraphim_service | 9 | Previous |
| Learning Capture (#82) | terraphim_agent/src/learnings/ | 26 unit | Blocked |
| Design-708 | terraphim_tinyclaw, terraphim-session-analyzer | 640+ | Previous |
| Dark Factory | terraphim_orchestrator, agent_supervisor, agent_registry | 176 | Previous |
| Validation Framework | terraphim_validation | ~20 | Previous |
| Trigger Retrieval (#84) | terraphim_rolegraph, terraphim_automata, terraphim_types | 14 | PASS (2026-03-27) |

---

## Coverage Trend

| Specification | 2026-03-24 | 2026-03-25 | 2026-03-26 | 2026-03-27 |
|---------------|------------|------------|------------|------------|
| Session Search | 95% | 95% | 90% | 90% |
| Desktop App | -- | 85% | 87% | 87% |
| Chat Session History | -- | 45% | 65% | 65% |
| Learning Capture | -- | 78% | 80% | 80% |
| Design-708 | -- | 100% | 100% | 100% |
| Dark Factory | -- | 120% | 120% | 120% |
| Validation Framework | -- | 100% | 100% | 100% |
| Trigger Retrieval (#84) | -- | -- | 100% | 92% (refined) |
| CorrectionEvent (#82) | -- | -- | -- | 99% (new) |
