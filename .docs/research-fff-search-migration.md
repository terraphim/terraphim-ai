# Research Document: Migrate Terraphim to Published fff-search 0.8.4

**Status**: Completed
**Author**: AI Agent
**Date**: 2026-05-28

## Executive Summary

Terraphim currently depends on a git branch (`feat/external-scorer`) of `fff-search` that provides a custom `ExternalScorer` trait and a `FileItem` with simple `String` fields. The published `fff-search 0.8.4` on crates.io uses a fundamentally different architecture: arena-based string storage, no custom scoring hooks, and a `FileItem` where paths are accessed via methods requiring an arena reference.

This research document analyses the API differences and determines that migration is feasible through a **Terraphim-owned ranking post-processing strategy**: instead of injecting scoring into fff-search's pipeline, Terraphim will receive candidates from fff-search and then re-rank them using the role-configured ranking function, currently including KG and BM25F modes.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Unblocks crates.io publishing, removes git dependency fragility |
| Leverages strengths? | Yes | Post-processing approach is simpler than the current trait-based integration |
| Meets real need? | Yes | Multiple attempts to fix crates.io publishing have failed due to this dependency |

**Proceed**: Yes

## Problem Statement

### Description
Terraphim cannot publish to crates.io because several crates (`terraphim_middleware`, `terraphim_grep`) depend on `fff-search` via a git branch. The published versions of `fff-search` (0.8.x) have a different API that is not directly compatible.

### Impact
- crates.io publishing is blocked for `terraphim_middleware`, `terraphim_service`, `terraphim_grep`
- The release workflow has a permanently failing "Publish Rust crates" step
- Users cannot `cargo install terraphim` from crates.io

### Success Criteria
- All publishable Terraphim crates use only published crates.io dependencies
- `cargo publish --dry-run` succeeds for all crates in the publish list
- Terraphim's role-configured ranking continues to work, including KG and BM25F ranking modes
- No regression in search quality or performance

## Current State Analysis

### Current fff-search Usage (Git Branch)

The git branch provides:
- `FileItem` with public `String` fields: `relative_path`, `file_name`, `path`
- `ExternalScorer` trait: `fn score(&self, file: &FileItem) -> i32`
- `FileItem::new_raw(path, relative_path, file_name, size, modified, git_status, is_binary)`
- Direct integration: scorer is passed into fff-search's search pipeline

### Terraphim Ranking Requirement

Terraphim ranking is role-dependent. The fff-search migration must not assume KG is the only relevance model:

- **KG ranking** boosts candidates using role thesaurus / graph concept matches.
- **BM25F ranking** scores candidates using weighted field relevance and must remain available where configured.

Therefore, the migration boundary should be phrased as **candidate retrieval by fff-search followed by Terraphim ranking**, not as "KG post-processing" only. KG is one ranking strategy. BM25F is another. The shared helper should dispatch through an internal Terraphim ranking abstraction over candidate metadata (`relative_path`, and optionally `title`/`body` once a document is materialised) rather than embedding KG-only logic into each consumer.

### Current Integration Points

| Component | Location | Purpose |
|-----------|----------|---------|
| `KgPathScorer` | `crates/terraphim_file_search/src/kg_scorer.rs` | Implements `ExternalScorer` using KG concept matching |
| `ExternalScorer` trait | `crates/terraphim_file_search/src/external_scorer.rs` | Re-implementation of git branch trait |
| `FffIndexer` | `crates/terraphim_middleware/src/indexer/fff.rs` | Uses `KgPathScorer` with `FilePicker` for haystack indexing |
| `McpService` | `crates/terraphim_mcp_server/src/lib.rs` | Uses `FilePicker` + `KgPathScorer` for MCP tools |
| `HybridSearcher` | `crates/terraphim_grep/src/hybrid_searcher.rs` | Uses `FilePicker` for code search with Terraphim ranking |

### Data Flow (Current)
1. Build `FilePicker` with optional `ExternalScorer` + `SharedFrecency`
2. Call `picker.fuzzy_search()` or `picker.grep()`
3. fff-search internally calls `ExternalScorer::score()` for each result
4. Results returned with combined scores

## Published fff-search 0.8.4 Analysis

### Key API Differences

| Feature | Git Branch (0.5.1) | Published (0.8.4) |
|---------|-------------------|-------------------|
| `FileItem.relative_path` | Public `String` field | Method: `relative_path(&self, arena)` |
| `FileItem.file_name` | Public `String` field | Method: `file_name(&self, arena)` |
| `FileItem.path` | Public `PathBuf` field | Method (via arena) |
| `FileItem::new_raw` | 7 parameters including paths | 5 parameters: `filename_start`, `size`, `modified`, `git_status`, `is_binary` |
| `ExternalScorer` trait | Yes | **No** |
| Scoring hooks | Trait-based injection | **None** |
| Result mutability | Returns owned structs | Returns owned structs with **public mutable fields** |
| String storage | Individual `String` fields | Arena-based `ChunkedString` |

### Arena-Based String Storage

The published `FileItem` stores paths in a shared arena:
```rust
pub struct FileItem {
    pub size: u64,
    pub modified: u64,
    pub access_frecency_score: i16,
    pub modification_frecency_score: i16,
    pub git_status: Option<git2::Status>,
    pub(crate) path: ChunkedString,
    pub(crate) parent_dir_index: u32,
    // ...
}

pub fn relative_path(&self, arena: impl FFFStringStorage) -> String
```

To get a path string, you need access to the arena that stores the strings. The `FilePicker` owns the arena.

### Scoring Pipeline (Published)

```rust
pub fn fuzzy_search<'q>(
    &self,
    query: &'q FFFQuery<'q>,
    query_tracker: Option<&QueryTracker>,
    options: FuzzySearchOptions<'q>,
) -> SearchResult<'_>

pub struct SearchResult<'a> {
    pub items: Vec<&'a FileItem>,
    pub scores: Vec<Score>,
    pub total_matched: usize,
    pub total_files: usize,
    pub location: Option<Location>,
}

pub struct Score {
    pub total: i32,
    pub base_score: i32,
    pub filename_bonus: i32,
    pub special_filename_bonus: i32,
    pub frecency_boost: i32,
    pub git_status_boost: i32,
    pub distance_penalty: i32,
    pub current_file_penalty: i32,
    pub combo_match_boost: i32,
    pub path_alignment_bonus: i32,
    pub exact_match: bool,
    pub match_type: &'static str,
}
```

**Critical finding**: `SearchResult` and `Score` have **all public fields**. You can freely mutate scores after receiving results.

### FilePicker Arena Access

The `FilePicker` owns the string arena. To resolve paths from `FileItem`s returned by search:
```rust
// FilePicker provides access to its arena
picker.fuzzy_search(...).items.iter()
    .map(|file| file.relative_path(picker))
```

But wait - `relative_path` takes `impl FFFStringStorage`, and `FilePicker` implements this trait.

## Constraints

### Technical Constraints
- **Arena dependency**: To get path strings from `FileItem`, we need the `FilePicker`'s arena
- **No scoring hooks**: Cannot inject KG or BM25F scoring into fff-search's pipeline
- **Lifetime management**: `SearchResult<'a>` borrows from `FilePicker`, so results must be processed before picker is dropped
- **FileItem immutability**: `FileItem` fields are not publicly mutable

### Business Constraints
- Must maintain existing search quality for KG and BM25F ranking behaviour
- No regression in MCP tool functionality
- Must pass existing tests

## Vital Few (Essential Constraints)

| Constraint | Why It's Vital | Evidence |
|------------|----------------|----------|
| Post-processing only | Published fff-search has no scoring hooks | Analysis of score.rs and file_picker.rs shows hardcoded pipeline |
| Arena access required | FileItem paths stored in FilePicker's arena | FileItem.relative_path() requires arena parameter |
| Must preserve role ranking semantics | Users rely on KG and BM25F ranking behaviour | Role-configured relevance functions and existing ranking tests |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance regression from post-processing | Medium | Medium | Benchmark before/after; optimize with parallel iteration |
| Arena lifetime issues with async code | Medium | High | Ensure FilePicker lives long enough; clone path strings early |
| FileItem::new_raw API changes break tests | High | Low | Only affects test code; update test helpers |

### Assumptions

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| FilePicker implements FFFStringStorage | Reading file_picker.rs source | Cannot resolve paths | Yes - FilePicker has arena methods |
| Score fields can be mutated in-place | All fields are pub | Cannot adjust ranking | Yes - verified in types.rs |
| SearchResult items/scores are parallel arrays | items[i] corresponds to scores[i] | Ranking corruption | Yes - this is how the crate works |

## Migration Strategy: Post-Processing

Since fff-search 0.8.4 returns mutable results with no hooks, the migration strategy is:

### Critical Addendum: Pagination and Grep Constraints

Further API review surfaced two constraints that must shape the implementation design:

1. `FilePicker::fuzzy_search()` paginates before returning `SearchResult`.
   Applying Terraphim ranking only to the returned page cannot promote KG- or BM25F-relevant files that fff-search ranked outside that page. To preserve existing Terraphim semantics, callers that use role ranking must request a wider candidate window from fff-search, apply the active ranking strategy to that candidate set, then apply the final Terraphim-level offset/limit after re-ranking.

2. `GrepResult.matches[*].file_index` indexes into `GrepResult.files`.
   Re-sorting `GrepResult.files` after grep can corrupt match-to-file relationships unless every match index is remapped. The safer approach is to KG-sort the candidate file list before invoking grep, then leave `GrepResult.files` and `GrepMatch.file_index` unchanged after grep.

3. `FilePicker` implements `FFFStringStorage` for `&FilePicker`, so path resolution must use `file.relative_path(&picker)` or an equivalent borrowed picker reference.

4. Publishing `terraphim_file_search` requires adding version requirements to all path dependencies, not just removing `publish = false`.

### Step 1: Search → Get Results → Apply Terraphim Ranking

Instead of:
```rust
// OLD: scorer injected into pipeline
let results = picker.fuzzy_search(&query, qt, options); // scores include Terraphim ranking boost
```

Do:
```rust
// NEW: get candidates, then rank using the role-configured strategy
let mut results = picker.fuzzy_search(&query, qt, widened_options);

// Post-process: add role-specific boost to each score
for (file, score) in results.items.iter().zip(results.scores.iter_mut()) {
    let path = file.relative_path(&picker); // resolve via arena
    let boost = ranker.score_path(&path);
    score.total += boost;
}

// Re-sort by total score descending
let mut combined: Vec<_> = results.items.into_iter()
    .zip(results.scores.into_iter())
    .collect();
combined.sort_by(|a, b| b.1.total.cmp(&a.1.total));
```

The final consumer-facing page must be sliced after Terraphim re-ranking. When no Terraphim ranking strategy is active, existing fff-search pagination can be preserved.

### Step 2: Create PathResolver Helper

Since `FileItem::relative_path` requires an arena, create a helper trait or wrapper:

```rust
pub trait PathResolver {
    fn resolve_path(&self, file: &FileItem) -> String;
}

impl PathResolver for FilePicker {
    fn resolve_path(&self, file: &FileItem) -> String {
        file.relative_path(self)
    }
}
```

### Step 3: Update KgPathScorer

Change from trait-based to standalone scoring calculator:

```rust
// OLD
impl ExternalScorer for KgPathScorer {
    fn score(&self, file: &FileItem) -> i32 { ... }
}

// NEW
impl KgPathScorer {
    pub fn score_path(&self, path: &str) -> i32 {
        // same logic, but takes &str instead of &FileItem
        ...
    }
}
```

BM25F should expose the same path/candidate-oriented scoring boundary if it participates in file ranking, or an equivalent document-oriented scorer if the consumer has already materialised `Document` values.

## Recommendations

### Proceed
Yes. The post-processing strategy is viable and actually simpler than the current trait-based integration.

### Scope
- Update 4 crates: `terraphim_file_search`, `terraphim_middleware`, `terraphim_grep`, `terraphim_mcp_server`
- Remove `external_scorer.rs` (or keep as internal helper)
- Update all `Cargo.toml` files to use `fff-search = "0.8.4"`
- Update tests that use `FileItem::new_raw`
- Remove `continue-on-error: true` from crates.io publishing in CI

### Risk Mitigation
- Write comprehensive tests before changes (characterization tests)
- Benchmark search performance before/after
- Run full test suite after migration

## Appendix

### Reference: FileItem Methods in 0.8.4
```rust
impl FileItem {
    pub fn relative_path(&self, arena: impl FFFStringStorage) -> String
    pub fn file_name(&self, arena: impl FFFStringStorage) -> String
    pub fn new_raw(filename_start: u16, size: u64, modified: u64, git_status: Option<git2::Status>, is_binary: bool) -> Self
    pub(crate) fn update_frecency_scores(&mut self, tracker: &FrecencyTracker, mode: FFFMode)
}
```

### Reference: SearchResult Structure
```rust
pub struct SearchResult<'a> {
    pub items: Vec<&'a FileItem>,
    pub scores: Vec<Score>,
    pub total_matched: usize,
    pub total_files: usize,
    pub location: Option<Location>,
}
```

### Reference: Score Structure (all fields public)
```rust
pub struct Score {
    pub total: i32,
    pub base_score: i32,
    pub filename_bonus: i32,
    pub special_filename_bonus: i32,
    pub frecency_boost: i32,
    pub git_status_boost: i32,
    pub distance_penalty: i32,
    pub current_file_penalty: i32,
    pub combo_match_boost: i32,
    pub path_alignment_bonus: i32,
    pub exact_match: bool,
    pub match_type: &'static str,
}
```
