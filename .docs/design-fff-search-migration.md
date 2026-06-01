# Implementation Plan: Migrate Terraphim to Published fff-search 0.8.4

**Status**: Review
**Research Doc**: `.docs/research-fff-search-migration.md`
**Author**: AI Agent
**Date**: 2026-05-28
**Estimated Effort**: 6-8 hours

## Overview

### Summary

Replace the git dependency on `fff-search` (`feat/external-scorer` branch) with the published crates.io version `0.8.4`. Published `fff-search` has no custom scoring hook, so Terraphim will preserve role-configured ranking by widening candidate retrieval, applying Terraphim-owned ranking boosts (KG or BM25F), then applying final pagination after re-ranking.

### Approach

Use **candidate post-processing**, not naive page post-processing:

1. Ask fff-search for a wider candidate set when Terraphim ranking is enabled.
2. Resolve file paths using `file.relative_path(&picker)` because `FFFStringStorage` is implemented for `&FilePicker`.
3. Apply the active Terraphim ranking strategy to mutate `Score.total`.
4. KG ranking uses path/thesaurus concept matches.
5. BM25F ranking uses the configured weighted-field relevance model where available.
6. Re-sort by boosted score.
7. Apply Terraphim-level offset/limit after boosting.
8. For grep, sort candidate files before calling grep so `GrepMatch.file_index` remains valid.

### Scope

**In Scope:**
- Update all Terraphim `fff-search` dependencies to `fff-search = "0.8.4"`.
- Refactor `KgPathScorer` from `ExternalScorer` trait implementation to path-string scorer.
- Add shared result-ranking helpers in `terraphim_file_search`.
- Preserve KG and BM25F ranking semantics across fuzzy search, indexing, MCP tools, and grep workflows.
- Make `terraphim_file_search` publishable by adding versioned path dependencies and adding it to the publish order.
- Restore crates.io publishing as a blocking release step once dry-run publishing succeeds.

**Out of Scope:**
- Forking, patching, or vendoring `fff-search`.
- Introducing unsafe code to access fff-search internals.
- Performance optimisation beyond correctness-preserving candidate limits and tests.
- Redesigning non-fff-search indexing/search behaviour.

**Avoid At All Cost:**
- Hard-coding KG as the only Terraphim ranking mode.
- Post-processing only the already-returned page.
- Re-sorting `GrepResult.files` after grep without remapping `GrepMatch.file_index`.
- Constructing fake `FileItem` values for unit tests using the new arena-based API.
- Publishing crates before `cargo publish --dry-run` succeeds for the whole publish chain.

## Architecture

### Component Diagram

```text
FilePicker (fff-search 0.8.4)
  - owns arena-based file paths
  - returns SearchResult / GrepResult
          |
          v
terraphim_file_search::result_ranking
  - resolve paths with &FilePicker
  - apply active Terraphim ranker (KG or BM25F)
  - sort fuzzy candidates before final pagination
  - sort grep input files before grep execution
          |
          v
Consumers
  - terraphim_middleware::FffIndexer
  - terraphim_mcp_server::McpService
  - terraphim_grep::HybridSearcher
```

### Fuzzy Search Data Flow

When Terraphim ranking is disabled:

```text
caller pagination -> fff-search fuzzy_search -> return as-is
```

When Terraphim ranking is enabled:

```text
caller offset/limit
  -> widen fff-search pagination to candidate_limit
  -> fff-search fuzzy_search
  -> resolve paths using file.relative_path(&picker)
  -> add active ranking boost to Score.total
  -> sort by boosted Score.total descending
  -> apply caller offset/limit
  -> return final page
```

### Grep Data Flow

```text
FilePicker collect_files/get_files
  -> clone/sort candidate file refs by active ranking score before grep
  -> call grep_search / picker.grep equivalent with ordered candidates
  -> do not re-sort GrepResult.files afterwards
  -> GrepMatch.file_index remains correct
```

If the available fff-search grep API does not accept an explicitly ordered candidate list in one consumer, that consumer must either keep current grep ordering or remap every `GrepMatch.file_index` after reordering. The preferred design is pre-grep candidate ordering.

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Widen candidates before Terraphim re-ranking | fff-search paginates internally, so page-only boosting loses off-page KG/BM25F hits | Boost returned page only |
| Put ranking logic in `terraphim_file_search::result_ranking` | Prevents divergent implementations across middleware, MCP, and grep | Inline helper copies in each consumer |
| Sort grep candidates before grep | Preserves `GrepMatch.file_index` integrity | Re-sort `GrepResult.files` after grep |
| Unit-test `score_path(&str)` directly | Avoids fake arena/FileItem construction | Use `FileItem::new_raw` fixtures |
| Publish `terraphim_file_search` | `terraphim_middleware` depends on it, so crates.io publish chain needs it | Keep `publish = false` |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Patch fff-search scoring internals | Not official crates.io dependency path | Permanent fork burden |
| Keep git dependency with non-blocking publish | Repeats the current failure mode | Release workflow remains misleading |
| Broad adapter layer for both 0.5 and 0.8 APIs | Only 0.8.4 is the target | Extra abstraction with no current value |
| Synthetic `FileItem` unit fixtures | 0.8.4 path storage is arena-based | Brittle tests tied to internals |
| KG-only helper API | BM25F is also a valid ranking mode | Bakes in the wrong domain assumption |

### Simplicity Check

The simplest correct approach is not “post-process the page”; it is “post-process enough candidates with the active Terraphim ranker”. The implementation remains small: one path/candidate scorer boundary, one fuzzy result helper, one grep candidate ordering helper, and targeted consumer rewiring.

**Nothing Speculative Checklist:**
- [x] No support for multiple fff-search versions.
- [x] No custom trait abstraction unless required by tests.
- [x] No unsafe access to fff-search arenas.
- [x] No reimplementation of fff-search scoring.
- [x] No behavioural changes outside search ranking and publishing.

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_file_search/src/result_ranking.rs` | Shared fuzzy result ranking and grep candidate ordering helpers for KG/BM25F |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_file_search/Cargo.toml` | Use `fff-search = { version = "0.8.4" }`; remove `publish = false`; add versions to path dependencies |
| `crates/terraphim_file_search/src/lib.rs` | Export `result_ranking`; remove `external_scorer` export |
| `crates/terraphim_file_search/src/kg_scorer.rs` | Replace `ExternalScorer` impl with `score_path(&self, path: &str) -> i32` |
| `crates/terraphim_file_search/src/watcher.rs` | Rewrite tests to avoid `FileItem::new_raw` fixtures |
| `crates/terraphim_file_search/benches/kg_scoring.rs` | Benchmark `score_path` and/or real FilePicker scanned temp files |
| `crates/terraphim_middleware/Cargo.toml` | Use `fff-search = { version = "0.8.4" }` |
| `crates/terraphim_middleware/src/indexer/fff.rs` | Use shared helpers; widen fuzzy candidates before active ranking pagination; rank-sort grep inputs before grep |
| `crates/terraphim_grep/Cargo.toml` | Use `fff-search = { version = "0.8.4", optional = true }` |
| `crates/terraphim_grep/src/hybrid_searcher.rs` | Replace direct `file.relative_path` field use with `file.relative_path(&picker)`; use shared helpers where applicable |
| `crates/terraphim_mcp_server/Cargo.toml` | Use `fff-search = { version = "0.8.4" }` |
| `crates/terraphim_mcp_server/src/lib.rs` | Use shared helpers for fuzzy result ranking and grep candidate ordering |
| `scripts/publish-crates.sh` | Add `terraphim_file_search` before `terraphim_middleware` |
| `.github/workflows/release-comprehensive.yml` | Remove `continue-on-error: true` from crates.io publish job after dry-runs pass |

### Deleted Files

| File | Reason |
|------|--------|
| `crates/terraphim_file_search/src/external_scorer.rs` | Published fff-search does not expose `ExternalScorer`; Terraphim no longer depends on that trait |

## API Design

### `KgPathScorer`

```rust
impl KgPathScorer {
    /// Calculate the KG boost for a resolved relative path.
    pub fn score_path(&self, relative_path: &str) -> i32;
}
```

KG remains a concrete scorer because it already has a path-based implementation. BM25F must be wired through the ranking boundary without assuming it is path-based if the existing implementation scores materialised documents or weighted fields.

`score_path` contains the current matching logic from `ExternalScorer::score`, but accepts a resolved path string instead of `&FileItem`.

### Ranking Strategy Boundary

```rust
/// Candidate metadata available immediately after fff-search candidate retrieval.
pub struct FileRankCandidate<'a> {
    pub relative_path: &'a str,
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
}

/// Scores file/search candidates for Terraphim-specific ranking.
pub trait FileRanker {
    fn score_candidate(&self, candidate: &FileRankCandidate<'_>) -> i32;
}

pub struct KgFileRanker<'a>(&'a KgPathScorer);

pub struct Bm25fFileRanker<'a> {
    // Wraps the existing BM25F implementation once located.
    inner: &'a dyn ExistingBm25fScorer,
}
```

If existing BM25F scoring is document-content based rather than path based, implementation must populate `title`/`body` where documents are available and call the existing BM25F scorer there. The shared fuzzy helper may initially have only path metadata for file candidates, but the public helper API must remain ranking-strategy neutral and must not force BM25F into a KG-style path-only shape.

### `result_ranking.rs`

```rust
use fff_search::{FilePicker, PaginationArgs, Score, SearchResult};
use crate::result_ranking::FileRanker;

/// Minimum candidate limit used when Terraphim ranking must re-rank before final paging.
pub const DEFAULT_RANKING_CANDIDATE_LIMIT: usize = 1000;

/// Return the fff-search pagination to use for Terraphim-ranked candidate collection.
pub fn widened_pagination(final_offset: usize, final_limit: usize) -> PaginationArgs;

/// Apply Terraphim ranking boost to a fuzzy result set, sort, then apply final pagination.
pub fn rank_fuzzy_results<'a>(
    picker: &FilePicker,
    results: SearchResult<'a>,
    ranker: &dyn FileRanker,
    final_offset: usize,
    final_limit: usize,
) -> SearchResult<'a>;

/// Sort file refs by Terraphim ranking before grep so GrepMatch.file_index remains valid.
pub fn sort_files_by_rank<'a>(
    picker: &FilePicker,
    files: Vec<&'a fff_search::types::FileItem>,
    ranker: &dyn FileRanker,
) -> Vec<&'a fff_search::types::FileItem>;
```

Implementation notes:
- `widened_pagination` should request `offset = 0` and `limit = max(DEFAULT_RANKING_CANDIDATE_LIMIT, final_offset + final_limit)`.
- `rank_fuzzy_results` must preserve metadata: `total_matched`, `total_files`, and `location`.
- Sorting should be stable for equal boosted totals to preserve fff-search ordering where the active ranking strategy does not differentiate.
- Path resolution must call `file.relative_path(&picker)` because `FFFStringStorage` is implemented for `&FilePicker`.

### Consumer Pattern

```rust
let final_pagination = options.pagination;

if let Some(ranker) = active_ranker {
    options.pagination = widened_pagination(final_pagination.offset, final_pagination.limit);
    let candidates = picker.fuzzy_search(&query, query_tracker, options);
    let results = rank_fuzzy_results(
        &picker,
        candidates,
        ranker,
        final_pagination.offset,
        final_pagination.limit,
    );
} else {
    let results = picker.fuzzy_search(&query, query_tracker, options);
}
```

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_score_path_no_match` | `kg_scorer.rs` | No KG terms returns zero |
| `test_score_path_single_match` | `kg_scorer.rs` | One KG term returns configured weight |
| `test_score_path_caps_at_max_boost` | `kg_scorer.rs` | Multiple terms respect max boost |
| `test_widened_pagination_covers_final_page` | `result_ranking.rs` | Candidate window covers final offset/limit |
| `test_rank_fuzzy_results_sorts_before_final_pagination` | `result_ranking.rs` | Off-page KG/BM25F hit can move into final page |
| `test_sort_files_by_rank_is_stable_for_ties` | `result_ranking.rs` | Equal ranking scores preserve input ordering |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| Existing KG page-limit boost test | `crates/terraphim_middleware/tests/fff_indexer.rs` | Must still prove off-page KG-relevant files can be promoted |
| BM25F ranking regression test | relevant ranking tests | BM25F remains selectable and produces expected ordering |
| MCP fuzzy search ranking test | `crates/terraphim_mcp_server` tests | MCP output preserves active ranking mode ordering |
| Grep candidate ordering test | `terraphim_grep` or middleware tests | Grep results still point to correct files |

### Publishing Verification

Use the publishing script, not isolated spot checks:

```bash
./scripts/publish-crates.sh --version 1.20.2 --dry-run
```

Also verify the key newly publishable crate directly:

```bash
cargo publish --package terraphim_file_search --dry-run --allow-dirty
```

## Implementation Steps

### Step 1: Clean Current Working Changes

**Files:** Current working tree
**Description:** Inspect current diff from prior investigation and keep only intended documentation or code changes. Revert exploratory partial code changes before implementation begins if they are not part of the approved design.
**Tests:** `git diff --stat` review
**Estimated:** 15 minutes

### Step 2: Make `terraphim_file_search` Publishable

**Files:** `crates/terraphim_file_search/Cargo.toml`, `src/kg_scorer.rs`, `src/lib.rs`, `src/result_ranking.rs`
**Description:** Switch to `fff-search = "0.8.4"`; remove `publish = false`; add version requirements for path dependencies; remove `ExternalScorer`; add `score_path` and ranking-neutral shared helpers.
**Tests:** `cargo test -p terraphim_file_search`
**Dependencies:** Step 1
**Estimated:** 1.5 hours

### Step 3: Update `terraphim_middleware`

**Files:** `crates/terraphim_middleware/Cargo.toml`, `src/indexer/fff.rs`
**Description:** Use fff-search 0.8.4; replace trait-based scorer usage with shared fuzzy ranking and grep candidate ordering helpers; preserve KG and BM25F selection semantics.
**Tests:** `cargo test -p terraphim_middleware --test fff_indexer`
**Dependencies:** Step 2
**Estimated:** 1.5 hours

### Step 4: Update `terraphim_mcp_server`

**Files:** `crates/terraphim_mcp_server/Cargo.toml`, `src/lib.rs`
**Description:** Use fff-search 0.8.4; resolve paths with `file.relative_path(&picker)`; use shared helpers for active ranking mode fuzzy ranking and grep ordering.
**Tests:** `cargo test -p terraphim_mcp_server`
**Dependencies:** Step 2
**Estimated:** 1.5 hours

### Step 5: Update `terraphim_grep`

**Files:** `crates/terraphim_grep/Cargo.toml`, `src/hybrid_searcher.rs`
**Description:** Use fff-search 0.8.4; replace field-based path access; preserve grep match/file index semantics.
**Tests:** `cargo test -p terraphim_grep --features code-search`
**Dependencies:** Step 2
**Estimated:** 1 hour

### Step 6: Update CI Publish Flow

**Files:** `scripts/publish-crates.sh`, `.github/workflows/release-comprehensive.yml`
**Description:** Add `terraphim_file_search` to publish order; remove non-blocking crates.io publish only after dry-run succeeds.
**Tests:** `./scripts/publish-crates.sh --version 1.20.2 --dry-run`
**Dependencies:** Steps 2-5
**Estimated:** 45 minutes

### Step 7: Full Verification

**Description:** Run focused tests first, then workspace checks.
**Commands:**
- `cargo check --workspace`
- `cargo test -p terraphim_file_search`
- `cargo test -p terraphim_middleware --test fff_indexer`
- `cargo test -p terraphim_grep --features code-search`
- `cargo publish --package terraphim_file_search --dry-run --allow-dirty`
- `./scripts/publish-crates.sh --version 1.20.2 --dry-run`
**Estimated:** 1-2 hours

## Rollback Plan

If migration fails:

1. Restore git dependencies on `fff-search`.
2. Restore `external_scorer.rs` and the `ExternalScorer`-based implementation.
3. Remove `terraphim_file_search` from publish order.
4. Restore non-blocking crates.io publish until a narrower fix is ready.

## Dependencies

| Crate | From | To | Reason |
|-------|------|----|--------|
| `fff-search` | git branch `feat/external-scorer` | `0.8.4` | Official stable crates.io version |
| `terraphim_file_search` | private workspace crate | publishable crate | Required by `terraphim_middleware` publishing |

## Performance Considerations

| Operation | Impact | Mitigation |
|-----------|--------|------------|
| Wider candidate collection | More fff-search work when Terraphim ranking is enabled | Limit to `max(1000, offset + limit)` initially |
| Path resolution | Allocates one string per candidate | Resolve once per candidate in helper |
| Second sort | `O(n log n)` over candidate window | Stable sort over bounded candidate count |

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm exact grep API path for ordered candidates in each consumer | Pending during Step 3/4/5 | Implementer |
| Confirm `terraphim_file_search` package metadata passes crates.io validation | Pending Step 2 | Implementer |
| Locate and wire existing BM25F scorer API | Pending before Step 2 implementation | Implementer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Publishing strategy approved
- [ ] Human approval received

**Implementation must not start until this revised plan is approved.**
