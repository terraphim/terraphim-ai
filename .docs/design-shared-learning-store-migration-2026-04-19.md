# Implementation Plan: Shared Learning Store Markdown Backend Migration

**Status**: Approved
**Research Doc**: `.docs/research-shared-learning-store-migration-2026-04-19.md`
**Author**: Terraphim AI Agent
**Date**: 2026-04-19
**Estimated Effort**: 0.5-1 day

## Overview

### Summary

This plan makes `SharedLearningStore` durable by replacing its current in-memory-only persistence path with `MarkdownLearningStore`, while preserving the existing store API, BM25 deduplication, and trust-promotion behaviour.

### Approach

Keep `SharedLearningStore` as the high-level API and in-memory index. Remove the unused `DeviceStorage` dependency from `store.rs`, extend `MarkdownLearningStore` so it can round-trip the full `SharedLearning` payload, and load all persisted markdown learnings during `open()`, collapsing canonical and shared copies into one in-memory record per `learning.id`.

### Scope

**In Scope:**
- Expand markdown frontmatter coverage to preserve full `SharedLearning` fidelity
- Add de-duplicated markdown loading for `SharedLearningStore::open()`
- Replace fake persistence in `store.rs` with markdown-backed persistence
- Align default learning paths with `ProjectDirs` guidance
- Update unit and integration tests to verify restart durability

**Out of Scope:**
- Rewriting ranking or deduplication logic
- Changing CLI command shapes
- Auto-publishing promoted learnings to `shared/`
- Refactoring injector behaviour
- Introducing a new database or queue

**Avoid At All Cost** (from 5/25 analysis):
- Keeping partial markdown serialisation that drops metadata
- Loading canonical and shared copies without deterministic de-duplication
- Introducing another storage abstraction layer without a concrete need
- Broad refactors to CLI, injector, or wiki sync in the same change

## Architecture

### Component Diagram

```text
SharedLearningStore
  |- config: StoreConfig
  |- backend: MarkdownLearningStore
  `- index: RwLock<HashMap<String, SharedLearning>>

open()
  -> backend.list_all()
  -> de-duplicate by learning.id
  -> populate index

insert()/merge()/promote()/record_application()
  -> mutate index
  -> backend.save(&learning)

Injector / shared publication
  -> continue using shared markdown files separately
```

### Data Flow

```text
Process start
  -> SharedLearningStore::open(config)
  -> MarkdownLearningStore::list_all()
  -> de-duplicate by learning.id, preferring non-shared path
  -> index populated from markdown files

Mutation
  -> SharedLearningStore mutates learning in memory
  -> MarkdownLearningStore::save(&learning)

Process restart
  -> open() reloads same markdown file
  -> learning state is preserved
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Keep `SharedLearningStore` as the public store type | Preserves CLI and test callers, and keeps BM25 logic in one place | Replacing callers with `MarkdownLearningStore` directly |
| Use direct markdown backend rather than `terraphim_persistence` for this step | Existing markdown backend already works; current `DeviceStorage` path is effectively dead code | Adding another abstraction layer before solving durability |
| Make markdown frontmatter fully represent `SharedLearning` | Prevents silent state loss on restart | Persisting only a subset of fields |
| Load both canonical and shared markdown files, then de-duplicate by `learning.id` | Matches approved direction while keeping one logical record in the store index | Excluding `shared/` entirely |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Backend trait for one implementation | No immediate need; adds ceremony | More code paths to maintain |
| Auto-save promoted L2/L3 learnings to `shared/` during this step | Changes behaviour and confuses canonical persistence with distribution | Duplicate records, harder rollback |
| Full rewrite around orchestrator persistence patterns | Useful reference, but too broad here | Delays the actual durability fix |

### Simplicity Check

**What if this could be easy?**

It can. The easiest correct solution is:

1. One canonical markdown file per learning.
2. One de-duplicated store index loaded at startup.
3. One save path for mutations.
4. Shared publication handled separately.

No database, no new trait stack, no behavioural rewrite.

**Senior Engineer Test**: This is a small persistence swap with explicit fidelity guarantees, not a storage-platform project.

**Nothing Speculative Checklist**:
- [x] No features the user did not request
- [x] No abstractions for hypothetical future backends
- [x] No extra sync mechanism
- [x] No new ranking logic
- [x] No premature optimisation

## File Changes

### New Files

None required.

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_agent/src/shared_learning/markdown_store.rs` | Expand frontmatter schema, expose path metadata needed for deterministic load-time de-duplication, align default path logic |
| `crates/terraphim_agent/src/shared_learning/store.rs` | Remove `DeviceStorage`/`SharedLearningRecord`, add markdown backend field, implement real `load_all()` and markdown-backed `persist()` |
| `crates/terraphim_agent/src/shared_learning/mod.rs` | Re-export any new config helpers if needed |
| `crates/terraphim_agent/Cargo.toml` | Add `directories` dependency only if no existing helper can be reused cleanly |
| `crates/terraphim_agent/tests/shared_learning_cli_tests.rs` | Move tests to temp-directory-backed durable store instances and add restart assertions |

### Deleted Files

None, but `store.rs` should delete dead `DeviceStorage`/`Persistable` glue inside the file if it becomes unused.

## API Design

### Public Types

```rust
#[derive(Debug, Clone)]
pub struct StoreConfig {
    pub similarity_threshold: f64,
    pub auto_promote_l2: bool,
    pub markdown: MarkdownStoreConfig,
}

#[derive(Debug, Clone)]
pub struct MarkdownStoreConfig {
    pub learnings_dir: PathBuf,
    pub shared_dir_name: String,
}
```

### Public Functions

```rust
impl SharedLearningStore {
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError>;
}

impl MarkdownLearningStore {
    pub fn with_config(config: MarkdownStoreConfig) -> Self;

    /// List persisted learnings across canonical and shared locations.
    pub async fn list_all(&self) -> Result<Vec<SharedLearning>, MarkdownStoreError>;
}
```

### Error Types

```rust
#[derive(Error, Debug)]
pub enum StoreError {
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("learning not found: {0}")]
    NotFound(String),
    #[error("BM25 calculation error: {0}")]
    Bm25(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

No new public store error type is required if markdown backend errors are mapped into `StoreError::Persistence`.

## Test Strategy

### Unit Tests

| Test | Location | Purpose |
|------|----------|---------|
| `test_save_and_load_roundtrip_preserves_full_state` | `markdown_store.rs` | Ensure all `SharedLearning` metadata survives markdown round-trip |
| `test_list_all_supports_dedup_inputs` | `markdown_store.rs` | Ensure the backend can feed canonical and shared copies into deterministic store de-duplication |
| `test_open_loads_existing_markdown_learnings` | `store.rs` | Verify startup hydration from persisted files |
| `test_open_dedups_shared_and_canonical_copies` | `store.rs` | Ensure one logical record per `learning.id` after startup load |
| `test_persist_after_promotion_and_application` | `store.rs` | Ensure quality metrics and trust promotion survive reload |
| `test_sparse_old_frontmatter_still_loads` | `markdown_store.rs` | Backwards-compatible parsing for already-written markdown files |

### Integration Tests

| Test | Location | Purpose |
|------|----------|---------|
| `shared_list_with_trust_level_filter` | `shared_learning_cli_tests.rs` | Keep existing CLI behaviour intact with durable backend |
| `shared_promote_l1_to_l2` | `shared_learning_cli_tests.rs` | Ensure promotion still works end-to-end |
| `shared_store_survives_restart` | `shared_learning_cli_tests.rs` | Create store, insert data, reopen store, verify persistence |

### Property / Invariant Tests

No property tests are required for this step. The critical invariants are deterministic and best covered by round-trip tests.

## Implementation Steps

### Step 1: Expand Markdown Fidelity
**Files:** `crates/terraphim_agent/src/shared_learning/markdown_store.rs`
**Description:** Extend `LearningFrontmatter` and parsing logic so all persisted `SharedLearning` fields round-trip cleanly. Newly added fields should be optional/defaulted on read so older sparse files still load.
**Tests:** Add full-state round-trip test and sparse-frontmatter compatibility test.
**Estimated:** 2-3 hours

```rust
#[derive(Debug, Serialize, Deserialize)]
struct LearningFrontmatter {
    id: String,
    title: String,
    agent_id: String,
    captured_at: String,
    updated_at: String,
    promoted_at: Option<String>,
    trust_level: String,
    source: String,
    applicable_agents: Vec<String>,
    keywords: Vec<String>,
    verify_pattern: Option<String>,
    quality: QualityMetrics,
    original_command: Option<String>,
    error_context: Option<String>,
    correction: Option<String>,
    wiki_page_name: Option<String>,
}
```

### Step 2: Add Canonical Listing and Path Helper
**Files:** `crates/terraphim_agent/src/shared_learning/markdown_store.rs`, optionally `crates/terraphim_agent/Cargo.toml`
**Description:** Ensure backend loading exposes both canonical and shared markdown files, with enough path context for deterministic de-duplication in `SharedLearningStore::load_all()`. Update default path construction to follow `ProjectDirs::from("com", "aks", "terraphim").data_local_dir()` while preserving `TERRAPHIM_LEARNINGS_DIR` override.
**Tests:** Shared and canonical inputs are both discoverable; temp-dir config still works.
**Dependencies:** Step 1
**Estimated:** 1-2 hours

### Step 3: Refactor `SharedLearningStore` to Use Markdown Backend
**Files:** `crates/terraphim_agent/src/shared_learning/store.rs`
**Description:** Replace the `DeviceStorage` field with `MarkdownLearningStore`, remove dead `SharedLearningRecord` persistence glue, implement real `load_all()`, de-duplicate by `learning.id` while preferring non-shared paths, and persist mutations through markdown saves.
**Tests:** Existing store tests plus new startup hydration tests.
**Dependencies:** Steps 1-2
**Estimated:** 2-3 hours

```rust
pub struct SharedLearningStore {
    backend: MarkdownLearningStore,
    index: RwLock<HashMap<String, SharedLearning>>,
    config: StoreConfig,
}
```

### Step 4: Update Integration Tests for Durability
**Files:** `crates/terraphim_agent/tests/shared_learning_cli_tests.rs`
**Description:** Stop describing the store as in-memory. Use temp-directory-backed store configuration, reopen the store, and verify persisted state survives restart.
**Tests:** Integration suite with `shared-learning` feature.
**Dependencies:** Step 3
**Estimated:** 1-2 hours

## Rollback Plan

If migration introduces unexpected regressions:

1. Revert `store.rs` to the current in-memory-only implementation.
2. Keep the expanded markdown serialiser if it is independently correct and tested.
3. Preserve the new tests so the durability gap remains visible.

## Migration

### Data Migration

No migration from the old `DeviceStorage` path is required because the current implementation does not perform durable loads.

The only compatibility requirement is reading markdown files already created by the current `MarkdownLearningStore`, which use a sparse frontmatter schema.

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `directories` | Current compatible version | Only if `terraphim_agent` cannot reuse an existing helper for `ProjectDirs` cleanly |

### Dependency Updates

None expected.

## Performance Considerations

### Expected Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Store open | Acceptable for local CLI use with small-to-moderate learning set | Unit/integration tests |
| Mutation cost | One file write per logical update | Code inspection + tests |
| Query speed after open | Same as current in-memory behaviour | Existing store tests |

### Benchmarks to Add

No dedicated benchmark is required for this step.

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Decide whether to add `directories` directly or expose a shared helper | Pending | Human + implementer |
| Approved: load `shared/` too and de-duplicate by `learning.id`, preferring non-shared copies | Resolved | Human approval |

## Approval

- [x] Technical review complete
- [x] Test strategy defined
- [x] Simplicity check passed
- [x] Human approval received
