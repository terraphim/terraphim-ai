# Implementation Plan: Token Budget Management (Gitea #707)

**Status**: Draft
**Research Doc**: `.docs/research-707-token-budget.md`
**Author**: AI Agent
**Date**: 2026-04-22
**Estimated Effort**: ~400 lines, single PR

## Overview

### Summary

Create `budget.rs` module that wires existing token budget primitives (FieldMode, RobotConfig, TokenBudget, Pagination, truncate_content, estimate_tokens) into an operational pipeline for bounding robot mode output.

### Approach

Standalone `BudgetEngine` struct that takes `Vec<SearchResultItem>` + `RobotConfig`, applies field filtering, content truncation, result limiting, and token budget tracking, and returns a `BudgetedResults` struct with filtered data + populated Pagination + TokenBudget.

### Scope

**In Scope:**
- `budget.rs` module with `BudgetEngine` and `BudgetedResults`
- Field filtering logic for `SearchResultItem` based on `FieldMode`
- Content truncation pipeline with truncation indicators
- Progressive token-budget limiting (consume results until budget exhausted)
- Pagination and TokenBudget metadata population
- Comprehensive unit tests

**Out of Scope:**
- CLI flag wiring to REPL commands (Task 1.4 scope)
- tiktoken integration (YAGNI)
- Budget for non-search data types (generalise later)
- Streaming budget (JSONL)

**Avoid At All Cost:**
- Adding new dependencies
- Modifying existing `RobotConfig`, `RobotFormatter`, or schema types
- Coupling budget to REPL handler infrastructure
- Creating generic trait abstractions "in case we need them later"

## Architecture

### Component Diagram

```
robot/
  mod.rs          -- Add: pub mod budget;
  output.rs       -- Existing: RobotConfig, FieldMode, RobotFormatter (no changes)
  schema.rs       -- Existing: TokenBudget, Pagination, SearchResultItem (no changes)
  budget.rs       -- NEW: BudgetEngine, BudgetedResults, BudgetError
  exit_codes.rs   -- Existing (no changes)
  docs.rs         -- Existing (no changes)
```

### Data Flow

```
Vec<SearchResultItem> + RobotConfig
        ↓
BudgetEngine::apply()
        ↓
  1. truncate_content() on each item's preview (via RobotFormatter)
  2. filter_fields() on each item (based on FieldMode)
  3. limit results by max_results (hard cap)
  4. progressively consume until max_tokens budget exhausted
  5. construct Pagination + TokenBudget
        ↓
BudgetedResults {
  results: Vec<serde_json::Value>,  // field-filtered
  pagination: Pagination,
  token_budget: Option<TokenBudget>,
}
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Return `Vec<serde_json::Value>` from field filter | Simple, correct for JSON-first robot mode; no need for typed field removal | Re-constructing SearchResultItem (fragile, duplicates struct) |
| `BudgetEngine` takes `&RobotConfig` | Config already holds all budget params; no duplication | Separate BudgetParams struct (duplication) |
| Progressive token consumption per-item | Accurate budget enforcement | Batch estimate (inaccurate) |
| Separate budget step from formatting | Budget is about data reduction, formatting is about serialisation | Merging into RobotFormatter (violates SRP) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Generic `Budget<T>` trait | Over-engineering for single consumer | Maintenance burden, premature abstraction |
| tiktoken crate integration | 4-char heuristic is sufficient and already exists | Dependency, complexity |
| Modifying `SearchResultItem` to have Optional fields | Would break existing API | Consumer code changes |
| Budgeting at serialization level | Too late; can't track which items were included | Incorrect metadata |

### Simplicity Check

**What if this could be easy?** A single function that takes results + config and returns filtered results + metadata. No traits, no generics, no complex abstraction.

**Senior Engineer Test**: The design is a pure function with input data + config -> output data + metadata. No framework, no plugin system, no event bus. Passes.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### New Files

| File | Purpose |
|------|---------|
| `crates/terraphim_agent/src/robot/budget.rs` | Budget engine + field filtering + result limiting |

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/robot/mod.rs` | Add `pub mod budget;` + re-export `BudgetEngine`, `BudgetedResults` |
| `crates/terraphim_agent/src/lib.rs` | Add `BudgetEngine`, `BudgetedResults` to re-exports |

### Deleted Files

None.

## API Design

### Public Types

```rust
/// Result of applying budget constraints to search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetedResults {
    /// Field-filtered, truncated, and budget-limited results
    pub results: Vec<serde_json::Value>,
    /// Pagination metadata
    pub pagination: Pagination,
    /// Token budget info (None if no token limit was set)
    pub token_budget: Option<TokenBudget>,
}

/// Errors from budget application
#[derive(Debug, thiserror::Error)]
pub enum BudgetError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

### Public Functions

```rust
/// Budget engine for constraining robot mode output
pub struct BudgetEngine {
    config: RobotConfig,
    formatter: RobotFormatter,
}

impl BudgetEngine {
    /// Create engine from robot config
    pub fn new(config: RobotConfig) -> Self;

    /// Apply budget constraints to search results
    ///
    /// Pipeline:
    /// 1. Truncate content fields (preview) based on max_content_length
    /// 2. Filter fields based on FieldMode
    /// 3. Limit by max_results (hard cap)
    /// 4. Progressively consume results within max_tokens budget
    /// 5. Build pagination and token budget metadata
    pub fn apply(&self, results: &[SearchResultItem]) -> Result<BudgetedResults, BudgetError>;
}
```

### Private Functions

```rust
/// Filter fields from a SearchResultItem based on FieldMode
/// Returns serde_json::Value with only the requested fields
fn filter_fields(item: &SearchResultItem, mode: &FieldMode) -> serde_json::Value;

/// Get the set of field names for a given mode
fn fields_for_mode(mode: &FieldMode) -> Vec<&'static str>;

/// Estimate total tokens for a collection of serialized values
fn estimate_total_tokens(items: &[serde_json::Value]) -> usize;
```

### Field Mapping

```rust
fn fields_for_mode(mode: &FieldMode) -> Vec<&'static str> {
    match mode {
        FieldMode::Full => vec![
            "rank", "id", "title", "url", "score",
            "preview", "source", "date", "preview_truncated"
        ],
        FieldMode::Summary => vec![
            "rank", "id", "title", "url", "score"
        ],
        FieldMode::Minimal => vec![
            "rank", "id", "title", "score"
        ],
        FieldMode::Custom(fields) => {
            // Validate against known fields, return intersection
            // Known fields: rank, id, title, url, score, preview, source, date
        }
    }
}
```

## Test Strategy

### Unit Tests

| Test | Purpose |
|------|---------|
| `test_field_mode_full_returns_all_fields` | Full mode includes all 9 fields |
| `test_field_mode_summary_excludes_preview` | Summary mode excludes preview, source, date |
| `test_field_mode_minimal_only_core` | Minimal mode returns rank, id, title, score |
| `test_field_mode_custom_selects_specified` | Custom mode returns only named fields |
| `test_field_mode_custom_ignores_unknown` | Custom mode silently ignores invalid field names |
| `test_truncate_content_marks_truncated` | Content exceeding max_content_length is truncated with indicator |
| `test_truncate_content_short_unchanged` | Content within limit passes through |
| `test_max_results_limits_count` | max_results caps result count |
| `test_max_tokens_progressive_budget` | Results consumed until token budget exhausted |
| `test_max_tokens_includes_partial_results` | Results within budget are included even if next result would overflow |
| `test_no_budget_returns_all` | No limits returns all results with all fields |
| `test_pagination_metadata_populated` | Pagination has correct total/returned/offset/has_more |
| `test_token_budget_metadata_populated` | TokenBudget tracks max_tokens and estimated_tokens |
| `test_token_budget_truncated_flag` | truncated flag is true when results were cut |
| `test_empty_results` | Empty input produces valid output with pagination total=0 |
| `test_custom_fields_includes_preview_truncated_when_preview` | If custom includes "preview", also includes "preview_truncated" |

### Integration Tests

| Test | Purpose |
|------|---------|
| `test_budget_with_robot_response` | BudgetedResults integrates with RobotResponse envelope |

## Implementation Steps

### Step 1: Create `budget.rs` with types and field filtering
**Files:** `crates/terraphim_agent/src/robot/budget.rs`
**Description:** Define `BudgetEngine`, `BudgetedResults`, `BudgetError`, `fields_for_mode()`, `filter_fields()`
**Tests:** Unit tests for all FieldMode variants + field filtering
**Estimated:** 1 hour

```rust
// Key code to write
pub struct BudgetEngine { config: RobotConfig, formatter: RobotFormatter }
pub struct BudgetedResults { results, pagination, token_budget }
fn fields_for_mode(mode: &FieldMode) -> Vec<&str>
fn filter_fields(item: &SearchResultItem, mode: &FieldMode) -> serde_json::Value
```

### Step 2: Implement budget application pipeline
**Files:** `crates/terraphim_agent/src/robot/budget.rs`
**Description:** Implement `BudgetEngine::apply()` with truncation, filtering, result limiting, progressive token consumption
**Tests:** Unit tests for budget scenarios (max_results, max_tokens, combined, no limits)
**Dependencies:** Step 1
**Estimated:** 1 hour

### Step 3: Wire into module exports
**Files:** `crates/terraphim_agent/src/robot/mod.rs`, `crates/terraphim_agent/src/lib.rs`
**Description:** Add `pub mod budget;` and re-export `BudgetEngine`, `BudgetedResults`
**Tests:** `cargo build --workspace` compiles; `cargo test --workspace` passes
**Dependencies:** Step 2
**Estimated:** 15 minutes

### Step 4: Integration tests + final verification
**Files:** `crates/terraphim_agent/src/robot/budget.rs` (test module)
**Description:** Integration test with `RobotResponse<BudgetedResults>` envelope; run `cargo test --workspace`, `cargo clippy`, `cargo fmt`
**Dependencies:** Step 3
**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Remove `budget.rs`
2. Revert `mod.rs` and `lib.rs` changes
3. No schema changes to revert (we don't modify existing types)

## Dependencies

### New Dependencies

None.

### Dependency Updates

None.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Budget application (100 results) | < 5ms | Unit test timing |
| Token estimation (per result) | < 50us | Trivial (len/4) |
| Field filtering (per result) | < 100us | serde_json serialize + key filter |

No benchmarks needed -- operations are O(n) with trivial per-item cost.

## Open Items

| Item | Status | Decision |
|------|--------|----------|
| Should BudgetEngine be clonable? | Deferred | Not needed now; add `#[derive(Clone)]` if required |
| Should `apply` take offset parameter? | Deferred | Pagination offset=0 for now; offset-based pagination is Task 1.4 scope |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
