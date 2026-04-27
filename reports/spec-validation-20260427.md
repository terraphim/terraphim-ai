# Specification Validation Report — Issue #672

**Date:** 2026-04-27 14:30 CEST
**Agent:** Carthos (spec-validator)
**Branch:** task/672-token-budget-management
**Run:** 2 (full re-validation)
**Issue:** #672
**Status:** FAIL — 2 acceptance criteria gaps remain; 1 new wiring fix present (uncommitted)

---

## Executive Summary

Branch `task/672-token-budget-management` implements Task 1.5 (Token Budget Management).
Core logic — `BudgetEngine`, field filtering, token estimation, result limiting — is
correctly implemented and tested. Since the 10:32 CEST run, four uncommitted changes
improve the branch: pagination and token-budget metadata are now wired to `ResponseMeta`
(previously computed but silently dropped), `ThesaurusResponse` is aligned to the actual
API shape, and exit-code test expectations are corrected.

Two acceptance criteria from issue #672 remain unmet:
`preview_original_length` is absent from `SearchResultItem`, and
`truncated_count` is absent from `Pagination`.

---

## Spec Source Hierarchy

| Source | Section | Authority |
|--------|---------|-----------|
| `docs/specifications/terraphim-agent-session-search-tasks.md` | Task 1.5, subtasks 1.5.1–1.5.4 | Primary spec |
| Gitea issue #672 | Acceptance criteria | Secondary (defines the PR contract) |

---

## Changes Since 10:32 CEST Run (Uncommitted)

| File | Change | Effect |
|------|--------|--------|
| `src/main.rs` | `meta.with_pagination(pagination)` + `meta.with_token_budget(tb)` | Fixes silent drop of budget metadata in offline path |
| `src/client.rs` | `ThesaurusResponse` restructured: `terms: Vec<ThesaurusEntry>` → `thesaurus: Option<HashMap<String, String>>` | Aligns struct to actual API response shape |
| `tests/offline_mode_tests.rs` | Expected exit code `1` → `6` (ErrorNetwork) | Aligns test to exit-code spec (issue #860) |
| `tests/unit_test.rs` | `ThesaurusResponse` deserialization test updated | Matches new struct shape |

---

## Acceptance Criteria Validation

### From Issue #672

| Criterion | Evidence | Status |
|-----------|----------|--------|
| `--max-tokens` flag on Search | `main.rs`: `max_tokens: Option<usize>` in `Command::Search`, wired to `BudgetEngine` | ✅ PASS |
| `--max-results` flag on Search | `main.rs`: `max_results: Option<usize>` → `effective_max_results` | ✅ PASS |
| `--max-content-length` flag on Search | `main.rs`: `max_content_length: Option<usize>` | ✅ PASS |
| Fields tagged `_truncated: true` when shortened | `preview_truncated: bool` in `SearchResultItem`, skip-serialised when false | ✅ PASS |
| Original length included when field shortened | No `preview_original_length` field in `SearchResultItem` | ❌ GAP |
| `--format json --max-tokens 1000` respects budget | `BudgetEngine::apply_token_budget` progressive loop | ✅ PASS |
| Pagination includes `has_more` | `Pagination.has_more` computed in `Pagination::new` | ✅ PASS |
| Pagination wired to response | `meta.with_pagination(pagination)` now present in offline path | ✅ PASS (new) |
| Pagination includes `truncated_count` | No `truncated_count` field in `Pagination` | ❌ GAP |
| `cargo test -p terraphim_agent` passes | 9 pass, 1 fail (`test_performance_and_limits` in `comprehensive_cli_tests.rs` — graph top-k exceeded 30s; file not on this branch) | ⚠️ PRE-EXISTING |
| `cargo clippy -p terraphim_agent -- -D warnings` passes | Clean on prior run | ✅ PASS |

### From Task 1.5 Spec

| Req ID | Requirement | Design Ref | Impl Ref | Tests | Status |
|--------|-------------|------------|----------|-------|--------|
| REQ-1.5.1 | Token estimation (~4 chars = 1 token) | tasks.md §1.5.1 | `output.rs:estimate_tokens` | `test_token_budget_metadata_populated` | ✅ |
| REQ-1.5.2 | Field filtering modes (Full/Summary/Minimal/Custom) | tasks.md §1.5.2 | `budget.rs:fields_for_mode` | `test_field_mode_*` (4 tests) | ✅ |
| REQ-1.5.3 | Content shortened with indicator | tasks.md §1.5.3 | `budget.rs:truncate_item` | `test_truncate_content_*` | ✅ (no `original_length`) |
| REQ-1.5.4 | Result limiting with pagination metadata | tasks.md §1.5.4 | `budget.rs:apply_max_results` + `main.rs:with_pagination` | `test_max_results_limits_count` | ✅ (no `truncated_count`) |

---

## Gaps

### Gap 1 (BLOCKER): `original_length` absent from `SearchResultItem`

Issue #672 specifies: "Fields tagged `_truncated: true` when truncated, **include original length**."

`SearchResultItem` (`robot/schema.rs:296`) has `preview_truncated: bool` but no
`preview_original_length: usize` (or equivalent). Consumers cannot determine how much
content was removed.

**Fix — add to `SearchResultItem` (`robot/schema.rs`):**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub preview_original_length: Option<usize>,
```
**Set in `budget.rs::truncate_item`:**
```rust
item.preview_original_length = Some(preview.len());
item.preview = Some(shortened);
item.preview_truncated = true;
```

---

### Gap 2 (BLOCKER): `truncated_count` absent from `Pagination`

Issue #672 specifies: "Pagination metadata includes `has_more`, `truncated_count`."

`Pagination` (`robot/schema.rs:136`) has `total`, `returned`, `offset`, `has_more` —
but no `truncated_count`. Consumers cannot distinguish how many results were omitted.

**Fix — add to `Pagination` (`robot/schema.rs`):**
```rust
pub truncated_count: usize,
```
**Populate in `Pagination::new`:**
```rust
truncated_count: total.saturating_sub(returned),
```

---

### Pre-existing failure (not a blocker for this branch)

`mcp_tool_index::tests::test_discovery_latency_benchmark` — 99ms vs 70ms threshold.
No files in this branch modified `mcp_tool_index.rs`. Tracked separately.

---

## Architecture Notes

**Wiring now correct:** `BudgetedResults.pagination` and `.token_budget` are extracted and
attached to `ResponseMeta` via `with_pagination` / `with_token_budget`. Previously these
were computed but dropped before output — the new `main.rs` change closes that loop.

**Latent risk (unchanged):** `budget_engine.apply()` → `Vec<serde_json::Value>` →
`filter_map(|v| serde_json::from_value(v).ok())` in `main.rs`. Silent deserialization
failures could mask field mismatches. Not a blocker for this PR but worth a follow-up
note when `preview_original_length` is added (it must appear in `KNOWN_FIELDS` in
`budget.rs:24` to survive field filtering).

---

## Verdict

**FAIL** on issue #672 acceptance criteria. Two gaps must be closed before merge:

1. Add `preview_original_length: Option<usize>` to `SearchResultItem`; set it in `truncate_item`; add `"preview_original_length"` to `KNOWN_FIELDS` in `budget.rs`
2. Add `truncated_count: usize` to `Pagination`; populate as `total.saturating_sub(returned)` in `Pagination::new`

Both fixes are < 15 LOC combined. All other acceptance criteria are met.
