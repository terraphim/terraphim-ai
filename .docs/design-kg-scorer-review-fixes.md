# Implementation Plan: KG Scorer Review Fixes

**Status**: Draft
**Research Doc**: `.docs/research-kg-scorer-review-fixes.md`
**Author**: OpenCode
**Date**: 2026-05-25
**Estimated Effort**: 1-2 hours

## Overview

### Summary

This plan fixes the two structural review P2 findings by strengthening behavioural test evidence for KG scorer injection and making scorer construction explicit and bounded to a single `search_haystacks()` invocation.

### Approach

Keep the production architecture unchanged: `search_haystacks()` remains the injection point and `FffIndexer::with_kg_scorer()` remains the extension point. Add a small helper that constructs an optional `Arc<KgPathScorer>` once per search, then add a deterministic test where KG sorting changes which files appear under the existing `page_limit`.

### Scope

**In Scope:**

- Add a helper in `crates/terraphim_middleware/src/indexer/mod.rs` to build a scorer for a `TerraphimGraph` role.
- Ensure the helper clones the thesaurus outside the rolegraph lock before constructing the scorer.
- Add or replace tests in `crates/terraphim_middleware/tests/fff_indexer.rs` to prove KG sorting affects results.
- Update verification/validation documents to reference the stronger test evidence.

**Out of Scope:**

- Process-wide scorer cache.
- New rolegraph revision tracking.
- Changing `IndexMiddleware` or `FffIndexer` public trait contracts.
- Making `GrepSearchOptions.page_limit` configurable.

**Avoid At All Cost:**

- Introducing global mutable cache without invalidation.
- Adding mocks for internal Terraphim components.
- Broad refactors in `ConfigState`, `RoleGraphSync`, or `KgPathScorer`.

## Architecture

### Component Diagram

```text
search_haystacks()
  -> build_kg_scorer_for_role(config_state, role_name, role)
       -> if role.relevance_function != TerraphimGraph: None
       -> if no RoleGraphSync: None
       -> lock RoleGraphSync, clone thesaurus, drop lock
       -> if thesaurus empty: None
       -> Arc<KgPathScorer>
  -> FffIndexer::default().with_kg_scorer(scorer)
  -> haystack loop
  -> FffIndexer::index()
  -> KG-sorted grep results
```

### Data Flow

```text
ConfigState + RoleName
  -> Role lookup
  -> Optional Thesaurus clone
  -> Optional Arc<KgPathScorer>
  -> FffIndexer instance
  -> sorted FileItem list
  -> grep_search limited page
  -> observable result set
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use per-search scorer helper | Explicitly limits construction to once per search and keeps invalidation simple. | Global cache, per-haystack construction. |
| Clone thesaurus inside a short lock, build scorer after lock | Minimises lock hold time. | Build scorer while holding lock. |
| Behavioural test via >200 matching files | Uses existing `page_limit` to make ordering observable. | Test-only flags, mocks, exposing internals. |
| Keep existing success tests but add stronger proof | Maintains regression coverage while closing evidence gap. | Remove all current tests. |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Global scorer cache | Needs invalidation on KG update or role change. | Stale scorer, subtle bugs. |
| Borrowed scorer over `RoleGraph.thesaurus` | Requires lifetime/API changes across async lock boundaries. | Complexity and lock contention. |
| Expose `FffIndexer` internals for assertions | Tests implementation details instead of behaviour. | Brittle tests. |
| Configurable grep page limit | Not necessary for this defect. | Expands public configuration surface. |

### Simplicity Check

**What if this could be easy?**

The easy version is to move the existing scorer-building block into a helper and add one test that constructs a temporary haystack where ordering matters. This avoids new abstractions, new dependencies, and new lifecycle problems.

**Senior Engineer Test**: This is not overcomplicated if we avoid global caching. A senior engineer would likely prefer the per-search helper until a real invalidation model exists.

**Nothing Speculative Checklist:**

- [x] No features the user did not request
- [x] No abstractions for unknown future needs
- [x] No speculative global cache
- [x] No new impossible error paths
- [x] No premature optimisation beyond avoiding obvious repeated construction

## File Changes

### New Files

| File | Purpose |
|------|---------|
| None | No new code files required. |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_middleware/src/indexer/mod.rs` | Extract scorer construction into a helper and use it once per search. |
| `crates/terraphim_middleware/tests/fff_indexer.rs` | Add deterministic behavioural test proving KG sorting changes the paginated result set. |
| `.docs/verification-kg-scorer-wiring.md` | Update traceability to reference the stronger behavioural test. |
| `.docs/validation-kg-scorer-wiring.md` | Update acceptance evidence to reference the stronger behavioural test. |

### Deleted Files

| File | Reason |
|------|--------|
| None | No deletion required. |

## API Design

### Internal Helper

Add an internal async helper in `crates/terraphim_middleware/src/indexer/mod.rs`:

```rust
async fn kg_scorer_for_role(
    config_state: &ConfigState,
    role_name: &terraphim_types::RoleName,
    role: &terraphim_config::Role,
) -> Option<Arc<KgPathScorer>> {
    if role.relevance_function != RelevanceFunction::TerraphimGraph {
        return None;
    }

    let rg_sync = config_state.roles.get(role_name)?;
    let thesaurus = {
        let rg = rg_sync.lock().await;
        if rg.thesaurus.is_empty() {
            return None;
        }
        rg.thesaurus.clone()
    };

    Some(Arc::new(KgPathScorer::new(thesaurus)))
}
```

Use it from `search_haystacks()` after resolving `role`:

```rust
let kg_scorer = kg_scorer_for_role(&config_state, &search_query_role, role).await;
let mut fff = FffIndexer::default();
if let Some(scorer) = kg_scorer {
    fff = fff.with_kg_scorer(scorer);
}
```

### Public API

No public API changes.

## Test Strategy

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_search_haystacks_kg_scorer_changes_paginated_results` | `tests/fff_indexer.rs` | Prove KG sorting changes which document appears within `page_limit`. |
| Existing `test_search_haystacks_no_scorer_for_title_scorer_role` | `tests/fff_indexer.rs` | Confirm non-TG roles keep working without scorer. |
| Existing `test_search_haystacks_empty_thesaurus_no_scorer` | `tests/fff_indexer.rs` | Confirm empty thesaurus remains graceful. |

### Behavioural Test Shape

Create a temporary haystack with 201 Markdown files, each containing the same search term, and configure a `TerraphimGraph` role with a thesaurus term that matches only the last file's path.

Example data:

```text
temp/
  neutral-000.md  contains "needle"
  ...
  neutral-199.md  contains "needle"
  kg-priority/special-concept.md contains "needle"
```

Thesaurus contains `special-concept`. With KG scoring, `kg-priority/special-concept.md` is sorted before neutral files and included in results. Without KG scoring, it appears after 200 neutral matches and should be excluded by `page_limit: 200`.

Assertions:

```rust
let docs = search_haystacks(config_state, query).await.unwrap();
assert!(docs.values().any(|doc| doc.url.ends_with("kg-priority/special-concept.md")));
```

For control, run the same haystack with a `TitleScorer` role and assert the priority file is absent or, if filesystem ordering proves non-deterministic, assert that the TG result order differs and includes the priority file in a stronger position. Prefer absence assertion with deterministic creation order.

### Verification Commands

Run after implementation:

```bash
cargo fmt --check
cargo clippy -p terraphim_middleware
cargo test -p terraphim_middleware --test fff_indexer
cargo test -p terraphim_middleware --all-features
cargo check --workspace
UBS_MAX_DIR_SIZE_MB=0 ubs --only=rust crates/terraphim_middleware/src/indexer crates/terraphim_middleware/tests
```

## Implementation Steps

### Step 1: Extract scorer helper

**Files:** `crates/terraphim_middleware/src/indexer/mod.rs`

**Description:** Move `TerraphimGraph` check, rolegraph lock, thesaurus clone, and scorer construction into `kg_scorer_for_role()`.

**Tests:** Existing search_haystacks tests must still pass.

**Estimated:** 20 minutes

### Step 2: Use helper after role resolution

**Files:** `crates/terraphim_middleware/src/indexer/mod.rs`

**Description:** Resolve `role` first, then call `kg_scorer_for_role(&config_state, &search_query_role, role).await`, then build `FffIndexer` once.

**Tests:** `cargo test -p terraphim_middleware --test fff_indexer test_search_haystacks`.

**Estimated:** 15 minutes

### Step 3: Add behavioural pagination test

**Files:** `crates/terraphim_middleware/tests/fff_indexer.rs`

**Description:** Create 201 small Markdown files in a temp directory, inject a rolegraph whose thesaurus matches only the final file path, and assert the priority file appears for `TerraphimGraph` search.

**Tests:** New test should fail if `with_kg_scorer()` wiring is removed.

**Estimated:** 35 minutes

### Step 4: Update docs

**Files:** `.docs/verification-kg-scorer-wiring.md`, `.docs/validation-kg-scorer-wiring.md`

**Description:** Replace weak evidence claims with the new behavioural test evidence.

**Tests:** Documentation review only.

**Estimated:** 10 minutes

### Step 5: Run verification suite

**Files:** N/A

**Description:** Run formatting, clippy, tests, workspace check, and UBS.

**Estimated:** 20 minutes

## Rollback Plan

If the helper or behavioural test proves brittle:

1. Keep the existing production scorer wiring unchanged.
2. Revert only the helper extraction and new test.
3. Document the remaining P2 finding as accepted risk, or add a narrower unit test around `kg_scorer_for_role()`.

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| Non-TG role overhead | No scorer construction | Existing TitleScorer test + code inspection |
| TG multi-haystack scorer construction | One scorer per `search_haystacks()` invocation | Helper called once before haystack loop |
| Test runtime | < 1 second additional local time | `cargo test -p terraphim_middleware --test fff_indexer` |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Global scorer cache with invalidation | Deferred | Future design |
| Rolegraph revision tracking | Deferred | Future design |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
