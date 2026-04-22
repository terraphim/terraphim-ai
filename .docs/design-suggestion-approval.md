# Implementation Plan: Suggestion Approval Workflow and Notification (Phase 4)

**Status**: Draft
**Research Doc**: `.docs/research-suggestion-approval.md`
**Author**: opencode (AI agent)
**Date**: 2026-04-22
**Gitea Issue**: #85 (part of Epic #81)
**Estimated Effort**: 1.5 days

## Overview

### Summary

Add a suggestion approval workflow that surfaces captured corrections as pending suggestions, lets users batch approve/reject by confidence threshold, tracks metrics in JSONL, and provides session-end and daily-sweep integration points.

### Approach

Layer a `SuggestionStatus` field on top of the existing `SharedLearning` type and `SharedLearningStore`. Add new CLI subcommands under `learn suggest`. No new storage backend, no new dependencies.

### Scope

**In Scope:**
1. `SuggestionStatus` enum (Pending/Approved/Rejected) on `SharedLearning`
2. `learn suggest` CLI subcommand with batch operations
3. `suggestion-metrics.jsonl` metrics tracking
4. Session-end prompt integration via `learn suggest --session-end`

**Out of Scope:**
- Interactive TUI for review
- Real-time notification (websocket/push)
- LLM-based auto-approval
- Morning brief template (document the command only)

**Avoid At All Cost:**
- New storage backend or database
- New external crate dependencies
- Modifying `TrustLevel` enum (it works for its purpose)
- Complex state machine transitions (keep it simple: pending -> approved/rejected)

## Architecture

### Component Diagram

```
CLI (main.rs)
  |
  +-- LearnSub::Suggest (new)
        |
        +-- SuggestSub (new enum)
              |
              +-- run_suggest_command() (new)
                    |
                    +-- SharedLearningStore (existing)
                    |     +-- suggest()
                    |     +-- promote_to_l3()  (= approve)
                    |     +-- new: reject()
                    |     +-- new: list_pending()
                    |     +-- new: list_by_status()
                    |
                    +-- SuggestionMetrics (new)
                          +-- append to suggestion-metrics.jsonl
```

### Data Flow

```
capture_correction() → SharedLearningStore.store_with_dedup() → L1 entry (status=Pending)
                                                                                  |
learn suggest list  ← SharedLearningStore.list_by_status(Pending) ←--------------+
learn suggest show  ← SharedLearningStore.get(id)                  ←-- show details
learn suggest approve ID → promote_to_l3() + metrics.append(Approved) → wiki sync
learn suggest reject  ID → reject() + metrics.append(Rejected)
learn suggest approve-all --min-confidence 0.8 → batch approve + metrics
learn suggest reject-all --max-confidence 0.3  → batch reject + metrics
learn suggest --session-end → count pending + show top suggestion
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| `SuggestionStatus` as field on `SharedLearning` | Doesn't break TrustLevel semantics; orthogonal concern | Extending TrustLevel with Rejected variant |
| Use BM25 score as confidence | Already computed by `suggest()`; no new scoring needed | Separate confidence model |
| JSONL for metrics | Append-only, no schema migration, easy to parse | SQLite table |
| `promote_to_l3()` = approve | L3 is "Human-Approved" per existing semantics; perfect fit | New approve() method that duplicates promote_to_l3 |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Interactive `--interactive` mode | Requires TUI dependencies; CLI flags are sufficient | Scope creep, complexity |
| Suggestion expiry/rotation | Premature optimisation; low volume | Maintenance burden |
| Gitea issue integration for review | Wiki sync already exists; issue creation is overkill | Tight coupling to Gitea API |
| Fuzzy matching on approve/reject | Exact ID match is safer; BM25 already handles similarity | Incorrect operations |

### Simplicity Check

**What if this could be easy?**

The simplest design: `SuggestionStatus` field on `SharedLearning`, three new store methods (`list_pending`, `list_by_status`, `reject`), and a `SuggestSub` CLI enum with 5 subcommands. No new traits, no new modules, no new files except for metrics writer.

**Senior Engineer Test**: This is a thin layer on existing infrastructure. The only new concept is the status field and the metrics file. Passes.

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
| `crates/terraphim_agent/src/learnings/suggest.rs` | Suggestion metrics writer |

### Modified Files

| File | Changes |
|------|---------|
| `terraphim_types/src/shared_learning.rs` | Add `SuggestionStatus` enum, add `suggestion_status` field to `SharedLearning`, add `rejection_reason` field |
| `crates/terraphim_agent/src/shared_learning/store.rs` | Add `list_pending()`, `list_by_status()`, `reject()`, `approve()` methods |
| `crates/terraphim_agent/src/learnings/mod.rs` | Add `pub mod suggest;` |
| `crates/terraphim_agent/src/main.rs` | Add `SuggestSub` enum, extend `LearnSub` with `Suggest` variant, add `run_suggest_command()` |

### Deleted Files

None.

## API Design

### Public Types

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for SuggestionStatus { ... }
impl std::str::FromStr for SuggestionStatus { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetricsEntry {
    pub id: String,
    pub status: SuggestionStatus,
    pub confidence: f64,
    pub timestamp: DateTime<Utc>,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SuggestionMetrics {
    pub metrics_path: PathBuf,
}
```

### `SharedLearning` Extensions

```rust
pub struct SharedLearning {
    // ... existing fields ...
    pub suggestion_status: SuggestionStatus,
    pub rejection_reason: Option<String>,
    pub bm25_confidence: Option<f64>,
}
```

### Store Methods

```rust
impl SharedLearningStore {
    pub async fn list_pending(&self) -> Result<Vec<SharedLearning>, StoreError>;
    pub async fn list_by_status(&self, status: SuggestionStatus) -> Result<Vec<SharedLearning>, StoreError>;
    pub async fn approve(&self, id: &str) -> Result<(), StoreError>;
    pub async fn reject(&self, id: &str, reason: Option<&str>) -> Result<(), StoreError>;
}
```

### Metrics Methods

```rust
impl SuggestionMetrics {
    pub fn new(metrics_path: PathBuf) -> Self;
    pub fn append(&self, entry: SuggestionMetricsEntry) -> Result<(), std::io::Error>;
    pub fn read_recent(&self, limit: usize) -> Result<Vec<SuggestionMetricsEntry>, std::io::Error>;
    pub fn summary(&self) -> Result<SuggestionMetricsSummary, std::io::Error>;
}

pub struct SuggestionMetricsSummary {
    pub total: usize,
    pub approved: usize,
    pub rejected: usize,
    pub pending: usize,
    pub approval_rate: f64,
    pub avg_time_to_review: Option<chrono::Duration>,
}
```

### CLI Subcommands

```rust
#[derive(Subcommand, Debug)]
enum SuggestSub {
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        status: Option<String>,
    },
    Show {
        id: String,
    },
    Approve {
        id: String,
    },
    Reject {
        id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    ApproveAll {
        #[arg(long, default_value_t = 0.8)]
        min_confidence: f64,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    RejectAll {
        #[arg(long, default_value_t = 0.3)]
        max_confidence: f64,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    Metrics {
        #[arg(long)]
        since: Option<String>,
    },
    SessionEnd {
        #[arg(long)]
        context: Option<String>,
    },
}
```

### Error Types

Uses existing `StoreError` and `std::io::Error`. No new error types needed.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_suggestion_status_roundtrip` | `shared_learning.rs` | Display/FromStr roundtrip for SuggestionStatus |
| `test_shared_learning_default_status` | `shared_learning.rs` | New SharedLearning defaults to Pending |
| `test_approve_promotes_to_l3` | `store.rs` | Approve sets L3 + Approved status |
| `test_reject_sets_status` | `store.rs` | Reject sets Rejected status + optional reason |
| `test_list_pending_filters` | `store.rs` | list_pending only returns Pending entries |
| `test_list_by_status` | `store.rs` | list_by_status filters correctly |
| `test_metrics_append_and_read` | `suggest.rs` | Write and read metrics entries |
| `test_metrics_summary` | `suggest.rs` | Summary calculation with approval rate |
| `test_approve_all_dry_run` | `store.rs` | Dry run doesn't modify entries |
| `test_approve_all_threshold` | `store.rs` | Only entries above threshold approved |
| `test_reject_all_threshold` | `store.rs` | Only entries below threshold rejected |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_suggest_list_cli` | `main.rs` (test) | CLI suggest list outputs correctly |
| `test_suggest_approve_reject_flow` | `main.rs` (test) | Full approve/reject cycle via store |

## Implementation Steps

### Step 1: Add `SuggestionStatus` to terraphim_types

**Files:** `crates/terraphim_types/src/shared_learning.rs`
**Description:** Add `SuggestionStatus` enum with Display/FromStr/Serialize/Deserialize. Add `suggestion_status`, `rejection_reason`, `bm25_confidence` fields to `SharedLearning`. Update `SharedLearning::new()` default to `Pending`. Ensure backward compatibility (missing field in JSON/YAML defaults to Pending).
**Tests:** `test_suggestion_status_roundtrip`, `test_shared_learning_default_status`
**Dependencies:** None
**Estimated:** 1h

### Step 2: Add store methods for approval workflow

**Files:** `crates/terraphim_agent/src/shared_learning/store.rs`
**Description:** Add `list_pending()`, `list_by_status()`, `approve()` (calls promote_to_l3 internally + sets status), `reject()`. Wire `bm25_confidence` field in `suggest()` return path.
**Tests:** `test_approve_promotes_to_l3`, `test_reject_sets_status`, `test_list_pending_filters`, `test_list_by_status`
**Dependencies:** Step 1
**Estimated:** 2h

### Step 3: Add suggestion metrics module

**Files:** New `crates/terraphim_agent/src/learnings/suggest.rs`, modify `crates/terraphim_agent/src/learnings/mod.rs`
**Description:** `SuggestionMetrics` struct with JSONL append/read/summary. `SuggestionMetricsEntry` and `SuggestionMetricsSummary` types. Default metrics path: `.terraphim/suggestion-metrics.jsonl`.
**Tests:** `test_metrics_append_and_read`, `test_metrics_summary`
**Dependencies:** Step 1
**Estimated:** 1.5h

### Step 4: Add CLI subcommands

**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Add `SuggestSub` enum, add `Suggest` variant to `LearnSub`, implement `run_suggest_command()`. Wire `learn suggest list/show/approve/reject/approve-all/reject-all/metrics/session-end`.
**Tests:** `test_suggest_list_cli`, `test_suggest_approve_reject_flow`
**Dependencies:** Steps 2, 3
**Estimated:** 2h

### Step 5: Session-end prompt integration

**Files:** `crates/terraphim_agent/src/main.rs`
**Description:** Implement `SessionEnd` variant: count pending suggestions, display one-line summary ("N suggestions pending, top: '...'"), optionally call `suggest()` with provided context to show most relevant suggestion.
**Tests:** Unit test with mock store showing session-end output
**Dependencies:** Step 4
**Estimated:** 1h

### Step 6: Wire batch operations

**Files:** `crates/terraphim_agent/src/main.rs`, `crates/terraphim_agent/src/shared_learning/store.rs`
**Description:** Implement `approve-all` and `reject-all`: iterate pending suggestions, filter by confidence threshold, approve/reject in batch, write metrics entries. `--dry-run` shows what would happen without modifying.
**Tests:** `test_approve_all_dry_run`, `test_approve_all_threshold`, `test_reject_all_threshold`
**Dependencies:** Steps 2, 4
**Estimated:** 1.5h

## Rollback Plan

All changes are behind the `shared-learning` feature gate. If issues arise:
1. Remove `SuggestSub` from `LearnSub` enum
2. Remove `suggestion_status` field from `SharedLearning` (backward compatible -- defaults to Pending)
3. Remove `learnings/suggest.rs` module

No data migration needed -- existing `SharedLearning` entries will default to `Pending` status.

## Dependencies

### New Dependencies

None. Uses existing workspace crates: `serde`, `serde_json`, `chrono`, `tokio`.

## Performance Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| `learn suggest list` | < 100ms | BM25 in-memory scan |
| `learn suggest approve-all` (100 items) | < 2s | Batch file writes |
| `learn suggest --session-end` | < 100ms | Single count + one suggest() call |
| Metrics JSONL append | < 5ms | Single line append |

## Open Items

| Item | Status | Notes |
|------|--------|-------|
| BM25 confidence score surfacing | Needs investigation | `suggest()` returns `Vec<SharedLearning>` but not scores; need to add score to `SharedLearning` or return tuples |
| Morning brief template integration | Deferred | Document `learn suggest list --status pending` command for template; no code needed |
| `bm25_confidence` field population | Design decision | Set when `suggest()` is called, or set on import? Propose: set during `learn shared import` |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
