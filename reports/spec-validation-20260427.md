# Specification Validation Report: terraphim-agent F1.1

**Date**: 2026-04-27
**Validator**: Carthos (Domain Architect)
**Issue**: #851 — F1.1 populate concepts_matched and wildcard_fallback in robot-mode search envelope
**Specification Reference**: `docs/specifications/terraphim-agent-session-search-spec.md` — F1.1 Structured Output (lines 47-93)

---

## Executive Summary

**VERDICT: FAIL** — Specification not fully implemented. One of two search emission paths correctly populates `concepts_matched`, but the second path still hardcodes empty values.

**Gap Severity**: Medium. The multi-term search path is correct; the single-term path is incomplete.

**Scope**: 1 regression test regression + 1 incomplete implementation at one site.

---

## Specification Requirements (F1.1)

From `docs/specifications/terraphim-agent-session-search-spec.md`:

1. **Robot-mode structured output** with schema: success, meta (command, elapsed_ms, timestamp, version), data, errors
2. **SearchResultsData** containing:
   - `results: Vec<SearchResultItem>`
   - `total_matches: usize`
   - `concepts_matched: Vec<String>` — matched knowledge-graph concepts
   - `wildcard_fallback: bool` — whether search widened from original query

3. **Behaviour**:
   - `concepts_matched` populated from upstream automata/rolegraph
   - `wildcard_fallback = true` when search widens (zero initial results)
   - Both fields machine-readable in JSON output

---

## Current Implementation Status

### ✅ PASS: Multi-Term Search Path (line 1879)

**Location**: `crates/terraphim_agent/src/main.rs:1879-1884`

```rust
let concepts = service.extract_concepts_from_query(&role_name, &query).await;
let data = SearchResultsData {
    results: items,
    total_matches: total,
    concepts_matched: concepts,                // ✅ Correctly populated
    wildcard_fallback: result_count == 0,      // ✅ Correct logic
};
```

**Status**: Specification-compliant.
- Extracts concepts using upstream service
- Sets wildcard_fallback when `result_count == 0`
- Avoids double-work by reusing existing automata

### ❌ FAIL: Single-Term Search Path (line 3827)

**Location**: `crates/terraphim_agent/src/main.rs:3824-3829`

```rust
let data = SearchResultsData {
    results: items,
    total_matches: total,
    concepts_matched: vec![],          // ❌ Hardcoded empty
    wildcard_fallback: res_count == 0, // ✅ Correct logic
};
```

**Status**: Partial non-compliance.
- `concepts_matched` still hardcoded to `vec![]`
- `wildcard_fallback` correctly computed
- **Gap**: No concept extraction call on this path

---

## Code Location Map

| Component | File | Line(s) | Status |
|-----------|------|---------|--------|
| SearchResultsData struct | `src/robot/schema.rs` | 304–316 | ✅ Correct shape |
| Multi-term emission | `src/main.rs` | 1879–1884 | ✅ Spec-compliant |
| Single-term emission | `src/main.rs` | 3824–3829 | ❌ Incomplete |
| Concept extraction | `src/main.rs` | 1878 | ✅ Available for reuse |
| Tests | `tests/phase1_robot_mode_tests.rs` | 152–178 | ⚠️ Partial coverage |

---

## Test Coverage Analysis

**File**: `tests/phase1_robot_mode_tests.rs`

### Tests Present (13 total)

- ✅ `test_search_results_data_concepts_matched_serialised()` (line 152–164)
  - Validates serialization when `concepts_matched` contains values
  - **Evidence**: `assert!(json.contains("rust"));`

- ✅ `test_search_results_data_wildcard_fallback_when_empty()` (line 167–178)
  - Validates serialization when empty with wildcard
  - **Evidence**: `assert!(json.contains("wildcard_fallback"));`

### Tests Missing

- ❌ **Integration test**: Actual search query that populates concepts_matched
  - Current tests are unit-level only (struct serialization)
  - No end-to-end verification of concept extraction in search path

- ❌ **Regression test**: Single-term search path output validation
  - No verification that single-term search populates concepts correctly

---

## Acceptance Criteria Verification

From issue #851:

| Criterion | Status | Evidence |
|-----------|--------|----------|
| `concepts_matched` populated from upstream at both sites | ⚠️ PARTIAL | Multi-term: ✅ line 1879. Single-term: ❌ line 3827 |
| `wildcard_fallback` set correctly (true when widened) | ✅ PASS | Both paths: `res_count == 0` logic correct |
| Two regression tests covering concepts + wildcard | ⚠️ PARTIAL | Tests exist but are unit-level, not integration |
| `cargo test` passes | ✅ PASS | All 13 tests pass |
| `cargo clippy -D warnings` passes | ✅ PASS | No warnings |
| `cargo fmt --check` passes | ✅ PASS | No formatting issues |
| PR scope ≤500 LOC | ✅ UNKNOWN | Not yet submitted |

**Result**: 4/7 criteria PASS; 3/7 PARTIAL/FAIL

---

## Gap Details

### Gap 1: Incomplete Implementation (Single-Term Path)

**Severity**: Medium
**Impact**: Single-term searches report `concepts_matched: []` even when concepts are matched
**Scope**: 1 location in code (line 3827)
**Fix Complexity**: Low (copy concept extraction from multi-term path)

**Why This Matters**:
- Spec guarantees metadata about matched concepts for AI consumers
- Single-term path silently breaks that contract
- Affects queries like `--robot search "rust"` vs multi-term equivalents

### Gap 2: Integration Test Gap

**Severity**: Low-Medium
**Impact**: Regression undetectable at integration level
**Scope**: Test coverage
**Fix Complexity**: Low (add end-to-end test with mock data)

**Why This Matters**:
- Unit tests validate serialization shapes, not actual data flow
- Concept extraction service might fail silently; tests wouldn't catch it
- Acceptance criteria require "regression tests...cover...a query that matches at least one concept"

---

## Risks and Unknowns

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| Single-term path not exercised in typical usage | Low | Gap goes unnoticed in production | Add integration test |
| Concept extraction service unavailable | Low | Fallback to empty (current single-term) | Explicit error handling |
| User confusion: multi-term vs single-term behavior differs | Medium | Inconsistent API surface | Document expectation or unify |

---

## Recommendations

### Priority 1 (Blocking)

1. **Apply concept extraction to single-term path** (line 3827)
   - Copy `extract_concepts_from_query` call from multi-term path
   - Estimated effort: 5 minutes
   - Location: Insert before line 3824

### Priority 2 (Strong)

2. **Add integration test validating actual concept extraction**
   - Test that a real search emits matched concepts
   - Test wildcard_fallback correctly signals fallback search
   - Estimated effort: 30 minutes
   - Location: `tests/phase1_robot_mode_tests.rs` or new integration test file

### Priority 3 (Nice-to-have)

3. **Unify search paths**
   - Both multi-term and single-term paths now have identical logic
   - Consider extracting common SearchResultsData construction
   - Prevents future divergence
   - Estimated effort: 1 hour (refactor, not bug fix)

---

## References

- **Specification**: `docs/specifications/terraphim-agent-session-search-spec.md#f11-structured-output`
- **Issue**: https://git.terraphim.cloud/terraphim/terraphim-ai/issues/851
- **Upstream concept extraction**: `crates/terraphim-session-analyzer/src/kg/search.rs:80`
- **Schema definition**: `crates/terraphim_agent/src/robot/schema.rs:304-316`
- **Multi-term search path**: `crates/terraphim_agent/src/main.rs:1772-1884`
- **Single-term search path**: `crates/terraphim_agent/src/main.rs:1809-1892`

---

## Detailed Code Analysis

### Multi-Term Search (Correct Path)

```
Query parsing (line 1760–1771)
    ↓
Multi-term search with operators (line 1772–1809)
    ↓
Results collection (line 1817)
    ↓
Concept extraction (line 1878) ✅
    ↓
SearchResultsData construction (line 1879–1884) ✅
```

### Single-Term Search (Gap Path)

```
Query parsing (line 1760–1771)
    ↓
Single-term backward-compat path (line 1810–1815)
    ↓
Results collection (line 1817)
    ↓
[NO concept extraction] ❌
    ↓
SearchResultsData construction (line 3824–3829) ❌
    concepts_matched hardcoded to vec![]
```

### Why Paths Diverged

Line 1772–1809 handles structured multi-term query with logical operators (AND/OR). This path is newer and includes the concept extraction logic from Phase 1.

Line 1810–1815 is "backward compatibility" path for single-term queries. It was likely written before concept extraction was added to the service layer.

---

## Quality Gate Status

| Gate | Status | Reasoning |
|------|--------|-----------|
| **Code Quality** | ✅ PASS | Clippy, fmt clean; tests pass |
| **Spec Compliance** | ❌ FAIL | One of two paths incomplete |
| **Test Coverage** | ⚠️ PARTIAL | Unit tests pass; integration tests missing |
| **Acceptance Criteria** | ⚠️ PARTIAL | 4/7 criteria met |

**Recommendation**: Do NOT merge until Priority 1 gap is closed. Priority 2 recommended before closing issue.

---

## Conclusion

The specification for F1.1 Structured Output is **not fully met**. The multi-term search path correctly implements concept extraction and wildcard fallback signalling. The single-term search path, marked as "backward compatibility," still hardcodes empty concepts.

This is a **straightforward gap**: one missing function call in one location. The tests validate that the schema works; they do not validate that data flows through both code paths.

**Action Required**: Fix line 3827 to call `service.extract_concepts_from_query()`, add integration test, and resubmit for validation.
