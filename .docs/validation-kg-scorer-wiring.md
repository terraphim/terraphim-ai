# Validation Report: KG Scorer Wiring in search_haystacks

**Status**: Validated
**Date**: 2026-05-25
**Feature**: Wire KgPathScorer from RoleGraph.thesaurus into FffIndexer for CLI/desktop search path

## Executive Summary

The implementation successfully wires the knowledge-graph path scorer into the `search_haystacks()` dispatcher for CLI/desktop search, closing the gap where only the MCP server path applied KG boosting. All acceptance criteria met, no regressions, NFRs validated.

## Acceptance Criteria

| AC# | Criterion | Evidence | Status |
|-----|-----------|----------|--------|
| AC-1 | TerraphimGraph role + thesaurus => scorer injected | `test_search_haystacks_injects_kg_scorer_for_terraphim_graph_role` | PASS |
| AC-2 | TitleScorer role => no scorer injected | `test_search_haystacks_no_scorer_for_title_scorer_role` | PASS |
| AC-3 | Empty thesaurus => graceful degradation | `test_search_haystacks_empty_thesaurus_no_scorer` | PASS |
| AC-4 | Scorer data matches RoleGraph | `test_search_haystacks_kg_scorer_preserves_thesaurus_data` | PASS |
| AC-5 | No regressions | All 19 tests PASS | PASS |
| AC-6 | Log reflects actual extensions | `fff.rs:236-241` | PASS |
| AC-7 | Stateful path (cache bypass) | `test_fff_with_kg_scorer_uses_stateful_path` | PASS |
| AC-8 | State preserved | `test_fff_with_kg_scorer_state_is_not_discarded` | PASS |
| AC-9 | KG scoring changes paginated result set | `test_search_haystacks_kg_scorer_boosts_priority_file_above_page_limit` | PASS |

## NFR Validation

| NFR | Target | Evidence | Status |
|-----|--------|----------|--------|
| Latency | No overhead for non-TG roles | `is_some_and()` O(1) check, scorer only built for TerraphimGraph | PASS |
| Memory | No extra allocation for non-TG | `Option<Arc<KgPathScorer>>` is `None` for non-TG | PASS |
| Graceful degradation | Empty thesaurus = default behaviour | Test confirms | PASS |
| Backward compatibility | Existing tests pass | 19/19 PASS | PASS |

## E2E System Test Scenarios

| ID | Workflow | Result | Status |
|----|----------|--------|--------|
| E2E-1 | TerraphimGraph + thesaurus => scorer injected | PASS | PASS |
| E2E-2 | TitleScorer => no scorer | PASS | PASS |
| E2E-3 | Empty thesaurus => graceful fallback | PASS | PASS |
| E2E-4 | Scorer data provenance matches RoleGraph | PASS | PASS |
| E2E-5 | KG scorer changes which file appears within page_limit: 200 | PASS | PASS |

## Defect Register

| ID | Description | Severity | Status |
|----|-------------|----------|--------|
| (none) | | | |

## Gate Checklist

- [x] All acceptance criteria met (9/9)
- [x] NFRs validated (4/4)
- [x] E2E scenarios pass (5/5)
- [x] No regressions in existing tests
- [x] UBS scan: 0 critical findings
- [x] Clippy: 0 warnings
- [x] Format: clean
- [x] Full workspace builds
