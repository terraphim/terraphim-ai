# Research Document: KG Scorer Review Fixes

**Status**: Draft
**Author**: OpenCode
**Date**: 2026-05-25
**Reviewers**: Human maintainer

## Executive Summary

The structural review identified two P2 issues in the latest KG scorer wiring changes: tests prove `search_haystacks()` still returns results but do not prove KG sorting changes observable behaviour, and `search_haystacks()` clones the full role thesaurus when constructing a `KgPathScorer`. The research confirms both issues are valid but bounded: neither is a correctness blocker, and both can be fixed with small, local changes in middleware tests and scorer construction.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energising? | Yes | This directly improves confidence in KG-aware file ranking, a core Terraphim capability. |
| Leverages strengths? | Yes | The codebase already has `RoleGraph`, `KgPathScorer`, and `FffIndexer` extension points. |
| Meets real need? | Yes | The review identified specific evidence and performance gaps before merge. |

**Proceed**: Yes - 3/3 YES.

## Problem Statement

### Description

The current implementation wires `KgPathScorer` into `search_haystacks()` for `TerraphimGraph` roles. The tests added during verification check success and non-empty output, but a broken implementation that skipped scorer injection would still pass. The implementation also clones `RoleGraph.thesaurus` to build a scorer, which may be wasteful if done repeatedly for multiple `Ripgrep` haystacks or future call paths.

### Impact

Weak behavioural proof can let regressions pass undetected. Repeated thesaurus cloning can add avoidable latency and memory churn for large knowledge graphs, especially when a role has multiple filesystem haystacks.

### Success Criteria

- A test fails if KG path scoring is removed from the `search_haystacks()` path.
- The test demonstrates ordering or pagination effects, not merely non-empty search output.
- `search_haystacks()` avoids rebuilding/cloning the scorer more than necessary within a single search invocation.
- Existing non-`TerraphimGraph` and empty-thesaurus behaviours remain unchanged.

## Current State Analysis

### Existing Implementation

`search_haystacks()` resolves the role, checks for `RelevanceFunction::TerraphimGraph`, then locks the corresponding `RoleGraphSync`, clones `rg.thesaurus`, constructs `KgPathScorer`, and injects it via `FffIndexer::with_kg_scorer()`.

`FffIndexer::index_inner()` sorts files by `scorer.score(file)` before running `grep_search()`. The grep options include `page_limit: 200`, so only a bounded number of matched files are returned.

Current tests create a rolegraph and assert that `search_haystacks()` returns non-empty results. They do not create a dataset where KG ordering changes which documents appear.

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| `search_haystacks()` | `crates/terraphim_middleware/src/indexer/mod.rs` | Dispatches haystack search and now constructs/injects KG scorer. |
| `FffIndexer::index_inner()` | `crates/terraphim_middleware/src/indexer/fff.rs` | Filters files, applies KG path sorting, runs `grep_search()`, builds documents. |
| `KgPathScorer` | `crates/terraphim_file_search/src/kg_scorer.rs` | Scores `FileItem.relative_path` using KG thesaurus term matches. |
| `RoleGraphSync` | `crates/terraphim_rolegraph/src/lib.rs` | Thread-safe holder for `RoleGraph`, whose `thesaurus` is public on lock guard. |
| `fff_indexer` tests | `crates/terraphim_middleware/tests/fff_indexer.rs` | Current integration tests for FFF indexing and new search_haystacks flow. |

### Data Flow

```text
SearchQuery
  -> search_haystacks(config_state, query)
  -> resolve RoleName and Role
  -> if TerraphimGraph, read RoleGraph.thesaurus
  -> KgPathScorer::new(thesaurus)
  -> FffIndexer::with_kg_scorer(scorer)
  -> FffIndexer::index_inner()
  -> sort files by scorer.score(file)
  -> grep_search(page_limit = 200)
  -> Index<Document>
```

### Integration Points

- `ConfigState.roles` provides `RoleGraphSync` values keyed by `RoleName`.
- `RoleGraph.thesaurus` is cloned to build `KgPathScorer`.
- `FffIndexer::with_kg_scorer()` makes indexer stateful and bypasses `cached_fff_index()`.
- `grep_search()` pagination makes file order observable when more than 200 matching files exist.

## Constraints

### Technical Constraints

- Keep `IndexMiddleware` unchanged.
- Do not add mocks in tests.
- Prefer minimal local changes over new shared state or global caches.
- Existing `KgPathScorer::new()` owns a `Thesaurus`; no borrowed scorer constructor exists.
- `FffIndexer` uses fixed `page_limit: 200`; tests can use this to prove ordering.

### Business Constraints

- This is a review-fix plan for P2 findings, not a broad refactor.
- Work should remain safe to merge into the existing PR branch.

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| Behavioural proof | Test fails if scorer injection is removed | Tests only assert non-empty output |
| Per-search scorer construction | Avoid repeated clone/rebuild for multiple filesystem haystacks | One scorer built before haystack loop today, but no helper/caching boundary is explicit |
| Compatibility | Existing tests pass | Passing |

## Vital Few (Essentialism)

### Essential Constraints

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Observable KG ordering test | Without it, scorer wiring can regress silently. | Review P2 finding on weak tests. |
| Minimal scorer construction change | Avoids adding complex invalidation or lifecycle concerns. | Current scorer is local to one search invocation. |
| Preserve fallback semantics | Non-TG and empty-thesaurus roles must keep working. | Existing tests cover these paths. |

### Eliminated from Scope

| Eliminated Item | Why Eliminated |
|-----------------|----------------|
| Global cross-request scorer cache | Requires invalidation strategy for KG/thesaurus updates; not needed for P2 fix. |
| Refactoring `KgPathScorer` to borrow `Thesaurus` | Larger API change across crates for marginal benefit. |
| Making `page_limit` configurable | Not required to prove ordering; current fixed value is sufficient. |
| Reworking `ConfigState` ownership model | Too broad and unrelated to review findings. |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| `FffIndexer::with_kg_scorer()` | Existing injection point; keep using it. | Low |
| `RoleGraphSync::lock()` | Needed to read thesaurus. | Low, but avoid holding lock across expensive work. |
| `grep_search(page_limit = 200)` | Enables deterministic behavioural test. | Medium if upstream semantics change. |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| `fff-search` | Git branch `feat/external-scorer` | Medium, API is branch-based. | Keep tests focused at middleware boundary. |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Behavioural test may be brittle due filesystem ordering | Medium | Medium | Use numbered filenames and enough files to make pagination effect clear. |
| Large test fixture may slow tests | Low | Low | Create 201 small files in a temp directory; keep content tiny. |
| Per-search cache mistaken for global cache | Medium | Low | Name helper clearly and document no cross-request persistence. |

### Open Questions

1. Should there eventually be a process-wide scorer cache keyed by role and thesaurus revision? Deferred; requires a revision/invalidation model.
2. Should `FffIndexer` expose `page_limit` for tests? Deferred; use current fixed constant for now.

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| `grep_search()` respects input file order when selecting matches up to `page_limit`. | FFF grep receives `&files` after sort and returns paginated matches. | Behavioural test may not prove scoring. | Partially; to be verified by test. |
| `RoleGraph.thesaurus` clone is the intended source for path scorer. | Existing implementation and MCP scorer build use same thesaurus type. | Scorer may lag KG updates. | Yes for current design. |
| Multiple Ripgrep haystacks can exist for a role. | `Role.haystacks` is a vector and loop handles any count. | Per-call scorer caching matters less if uncommon. | Yes. |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Fix only tests | Addresses evidence gap but not clone concern. | Rejected as incomplete against review. |
| Add global scorer cache | Best runtime efficiency but needs invalidation. | Rejected as over-engineering. |
| Add per-search scorer helper/cache | Addresses repeated construction in one invocation with low complexity. | Chosen. |

## Research Findings

### Key Insights

1. A deterministic behavioural test can exploit `FffIndexer`'s fixed `page_limit: 200`: create more than 200 matching files and ensure a KG-scored file appears only when pre-sorted.
2. The current implementation already builds one scorer before iterating haystacks, but extracting a helper makes this explicit, testable, and prevents accidental per-haystack construction if the dispatch grows.
3. A global scorer cache is not justified until the system has a rolegraph revision or KG update signal.

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| None | Existing APIs are sufficient for the fix. | N/A |

## Recommendations

### Proceed/No-Proceed

Proceed. The fixes are local, low risk, and directly address review findings.

### Scope Recommendations

- Add one helper in `search_haystacks()` module to build an optional scorer for a role.
- Use that helper once per `search_haystacks()` invocation.
- Replace or supplement weak tests with a strong pagination/order test.
- Update verification and validation reports to reference the stronger evidence.

### Risk Mitigation Recommendations

- Keep global caching out of scope.
- Avoid new public APIs unless needed for tests; prefer `pub(crate)` helper if unit-tested in-module.
- Keep temp test files small and deterministic.

## Next Steps

If approved:

1. Implement the helper and keep scorer construction outside the haystack loop.
2. Add a behavioural test with 201+ matching files and KG-matching path priority.
3. Re-run `cargo test -p terraphim_middleware --test fff_indexer`, `cargo test -p terraphim_middleware --all-features`, `cargo clippy -p terraphim_middleware`, and UBS on changed Rust files.
