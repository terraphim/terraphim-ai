# Implementation Plan: Post-Merge Cleanup (3 Items)

**Status**: Draft
**Research Doc**: `.docs/research-post-merge-cleanup.md`
**Date**: 2026-04-05
**Estimated Effort**: 3-4 hours

## Overview

### Summary
Fix the three remaining issues from the PR merge campaign: (1) refactor shared_learning to use terraphim_persistence, (2) eliminate the rustls-webpki CVE from the discord dependency chain, (3) decide on NormalizedTerm ID type.

### Approach
Execute in dependency order. Item 2 (CVE) first because it's the smallest and most urgent. Item 1 (shared_learning refactor) second because it's self-contained. Item 3 (NormalizedTerm) is a decision-only, no code changes.

### Scope

**In Scope:**
- Disable `discord` default feature in terraphim_tinyclaw to eliminate CVE vector
- Re-add RUSTSEC-2026-0049 to `deny.toml` ignore list (still present via serenity when discord feature enabled)
- Refactor `shared_learning/store.rs` to use `Persistable` trait instead of sqlx
- Feature-gate `shared_learning` module
- Remove `sqlx` dependency from `terraphim_agent/Cargo.toml`
- Document NormalizedTerm ID decision

**Out of Scope:**
- Serenity 0.13 / `next` branch migration (separate PR)
- Aho-Corasick mention.rs rewrite (separate issue)
- Gitea issues for follow-up work (can be filed after)

**Avoid At All Cost:**
- Changing NormalizedTerm.id from u64 to String (cascading breakage across 5+ crates)
- Adding sqlx as a workspace dependency
- Rewriting wiki_sync.rs (works fine as-is with gitea-robot CLI)

## Architecture

### Component Diagram

```
terraphim_agent
  +-- shared_learning/          (feature-gated: "shared-learning")
        mod.rs                  (unchanged)
        types.rs                (unchanged - SharedLearning, TrustLevel, etc.)
        store.rs                (REWRITTEN: Persistable + in-memory index)
        wiki_sync.rs            (unchanged)
```

### Data Flow

```
Learning Capture
    |
    v
SharedLearningStore::store_with_dedup()
    |-- BM25 similarity check (in-memory HashMap)
    |-- Serialize SharedLearning to JSON
    |-- Persistable::save() -> OpenDAL operator -> filesystem/memory
    v
SharedLearningStore::find_similar()
    |-- Scan Persistable records -> deserialize
    |-- BM25 score + rank
    v
Trust Promotion (L1 -> L2 -> L3)
    |-- Load record, mutate trust_level, save back
    v
Wiki Sync (L2+ only)
    |-- gitea-robot wiki create/update
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Use Persistable + JSON files instead of sqlx/SQLite | Matches existing pattern in `terraphim_usage`, no new deps, no SQL | sqlx (rejected: new dep, violates arch), rusqlite (rejected: persistence layer exists), sled (rejected: overkill) |
| In-memory BM25 index built on load | Simple, sufficient for expected dataset size (< 10K learnings) | Full-text search in SQLite (rejected: requires sqlx), tantivy (rejected: heavy dep) |
| Disable discord default feature | Eliminates CVE vector without breaking builds | Serenity `next` branch (rejected: breaking API changes), forking serenity (rejected: maintenance burden) |
| Keep NormalizedTerm.id as u64 | Changing it breaks 5+ crates; the revert was deliberate | String/UUID IDs (rejected by prior decision `9c8dd28f`) |

### Eliminated Options

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| sqlx for shared_learning | Violates project architecture; terraphim_persistence is the storage layer | Architecture drift, dep proliferation |
| Serenity next branch | Breaking API: EventHandler methods changed, ChannelId.say() removed | Build failures, wasted hours on API migration |
| UUID NormalizedTerm IDs | Cascading changes to automata, rolegraph, hooks, file_search, orchestrator | 5+ crate breakage, test churn |

### Simplicity Check

**What if this could be easy?**

For Item 1: Instead of a SQLite-backed store with BM25, use a simple `Persistable` implementation that stores each learning as a JSON file and loads them all into a `HashMap<String, SharedLearning>` on open. BM25 scoring runs against the in-memory map. This is exactly how `terraphim_usage` works.

For Item 2: Just remove `discord` from default features. Done.

For Item 3: Just document the decision. No code.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? No. We're replacing a 952-line SQLite store with a ~300-line Persistable+HashMap store. That's a net reduction.

**Nothing Speculative Checklist**:
- [x] No features the user didn't request
- [x] No abstractions "in case we need them later"
- [x] No flexibility "just in case"
- [x] No error handling for scenarios that cannot occur
- [x] No premature optimization

## File Changes

### Modified Files

| File | Changes |
|------|---------|
| `crates/terraphim_tinyclaw/Cargo.toml` | Remove `discord` from default features |
| `deny.toml` | Re-add RUSTSEC-2026-0049 to ignore list (still present when discord enabled) |
| `crates/terraphim_agent/Cargo.toml` | Remove sqlx comment, add `shared-learning` feature flag |
| `crates/terraphim_agent/src/lib.rs` | Feature-gate `shared_learning` with `#[cfg(feature = "shared-learning")]` |
| `crates/terraphim_agent/src/shared_learning/store.rs` | Rewrite: replace sqlx with Persistable + in-memory HashMap |

### New Files

| File | Purpose |
|------|---------|
| None | All changes to existing files |

### Deleted Files

| File | Reason |
|------|--------|
| None | No files deleted |

## API Design

### Public Types (store.rs rewrite)

```rust
use std::collections::HashMap;
use std::sync::RwLock;
use terraphim_persistence::{DeviceStorage, Persistable};
use crate::shared_learning::types::{SharedLearning, TrustLevel};

pub struct SharedLearningStore {
    storage: DeviceStorage,
    index: RwLock<HashMap<String, SharedLearning>>,
    config: StoreConfig,
}

impl SharedLearningStore {
    pub async fn open(config: StoreConfig) -> Result<Self, StoreError>;
    pub async fn store_with_dedup(&self, learning: SharedLearning) -> Result<SharedLearning, StoreError>;
    pub async fn find_similar(&self, query: &str, limit: usize) -> Result<Vec<(f64, SharedLearning)>, StoreError>;
    pub async fn get(&self, id: &str) -> Result<SharedLearning, StoreError>;
    pub async fn list_all(&self) -> Result<Vec<SharedLearning>, StoreError>;
    pub async fn list_by_trust_level(&self, level: TrustLevel) -> Result<Vec<SharedLearning>, StoreError>;
    pub async fn promote_to_l2(&self, id: &str) -> Result<(), StoreError>;
    pub async fn promote_to_l3(&self, id: &str) -> Result<(), StoreError>;
    pub async fn record_application(&self, id: &str, agent: &str, effective: bool) -> Result<(), StoreError>;
}
```

### Persistable Implementation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedLearningRecord {
    pub key: String,
    // Delegates to SharedLearning fields via flattening or composition
}

impl Persistable for SharedLearningRecord {
    fn new(key: String) -> Self { Self { key, .. } }
    async fn save(&self) -> terraphim_persistence::Result<()> { self.save_to_all().await }
    fn get_key(&self) -> String {
        format!("shared-learning/{}.json", &self.key)
    }
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),
    #[error("learning not found: {0}")]
    NotFound(String),
    #[error("BM25 calculation error: {0}")]
    Bm25(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
```

## Test Strategy

### Unit Tests (in store.rs)

| Test | Purpose |
|------|---------|
| `test_store_open` | Open store with memory-only persistence |
| `test_insert_and_get` | Store a learning, retrieve by ID |
| `test_store_with_dedup` | Similar learnings get merged |
| `test_list_by_trust_level` | Filter by L1/L2/L3 |
| `test_promote_to_l2` | L1 -> L2 promotion |
| `test_record_application` | Track usage, auto-promote |
| `test_find_similar` | BM25 similarity ranking |

### Integration Tests

| Test | Purpose |
|------|---------|
| `test_persistence_roundtrip` | Persist, reload, verify equality |

### Test Approach
- Use `DeviceStorage::init_memory_only()` for all tests (no filesystem)
- All existing tests in store.rs should pass with new implementation
- No mocks -- use real in-memory persistence backend

## Implementation Steps

### Step 1: Fix CVE -- Disable Discord Default Feature (15 min)

**Files:** `crates/terraphim_tinyclaw/Cargo.toml`, `deny.toml`

**Description:** Remove `discord` from default features in tinyclaw. Re-add RUSTSEC-2026-0049 to deny.toml ignore list with updated comment.

**Tests:** `cargo build --workspace` passes. `cargo tree -p terraphim_tinyclaw` no longer shows serenity by default.

**Verification:**
```bash
cargo build --workspace
cargo tree -p terraphim_tinyclaw | grep serenity  # should be empty
cargo tree -p terraphim_tinyclaw --features discord | grep serenity  # should show serenity
```

### Step 2: Feature-Gate shared_learning Module (10 min)

**Files:** `crates/terraphim_agent/Cargo.toml`, `crates/terraphim_agent/src/lib.rs`

**Description:** Add `shared-learning` feature flag. Gate the module in lib.rs. Remove sqlx comment from Cargo.toml.

**Changes:**
```toml
# Cargo.toml
[features]
shared-learning = ["dep:terraphim_persistence"]

[dependencies]
terraphim_persistence = { path = "../terraphim_persistence", optional = true }
```

```rust
// lib.rs
#[cfg(feature = "shared-learning")]
pub mod shared_learning;
```

**Tests:** `cargo build -p terraphim_agent` passes without `shared-learning` feature.

### Step 3: Rewrite store.rs with Persistable (60-90 min)

**Files:** `crates/terraphim_agent/src/shared_learning/store.rs`

**Description:** Replace sqlx-backed SQLite store with Persistable + in-memory HashMap.

**Key changes:**
1. Remove all `sqlx::*` imports and queries
2. Add `terraphim_persistence::{DeviceStorage, Persistable}` imports
3. Replace `Pool<Sqlite>` with `DeviceStorage` + `RwLock<HashMap<String, SharedLearning>>`
4. Implement `Persistable` for a `SharedLearningRecord` wrapper
5. Keep BM25 scoring logic (it's pure math, no DB dependency)
6. Keep all existing public method signatures
7. Remove `SharedLearningRow` sqlx struct
8. Add `cargo build -p terraphim_agent --features shared-learning` test

**Dependencies:** Steps 1 and 2

**Estimated:** 60-90 min

### Step 4: Build Verification and Tests (30 min)

**Description:** Full workspace build with shared-learning feature. Run all tests.

```bash
cargo build --workspace
cargo build -p terraphim_agent --features shared-learning
cargo test -p terraphim_agent --features shared-learning
```

**Dependencies:** Step 3

### Step 5: Document NormalizedTerm Decision (5 min)

**Files:** `.docs/research-post-merge-cleanup.md` (append decision)

**Description:** Add section documenting the NormalizedTerm.id = u64 decision with rationale.

## Rollback Plan

- Step 1: Re-add `discord` to default features
- Step 2: Remove feature flag, comment module back out
- Step 3: Revert store.rs to sqlx version (still in git history)
- Full: `git revert HEAD~N` to undo all changes

## Dependencies

### New Dependencies

| Crate | Version | Justification |
|-------|---------|---------------|
| `terraphim_persistence` | workspace | Already in workspace; used as optional dep |

### Removed Dependencies

| Crate | Reason |
|-------|--------|
| `sqlx` | Replaced by terraphim_persistence |

## Performance Considerations

### Expected Performance

| Metric | Target | Notes |
|--------|--------|-------|
| Store open (load all) | < 100ms for 10K learnings | JSON deserialize from filesystem |
| find_similar (BM25) | < 10ms for 10K learnings | In-memory scan |
| store_with_dedup | < 5ms | HashMap lookup + JSON serialize |

For the expected dataset size (< 10K learnings per agent), in-memory HashMap with linear BM25 scan is sufficient. If scale becomes an issue, add an inverted index later.

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Human approval received
