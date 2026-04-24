# Design: Agent `LearningStore` Trait Implementation

**Date:** 2026-04-24
**Issue:** terraphim/terraphim-ai#813 (PR 4)
**Branch:** `task/813-agent-learning-store-impl`

## Goal

Implement `terraphim_types::shared_learning::LearningStore` (sync trait) on
`terraphim_agent::shared_learning::SharedLearningStore` (async markdown-backed
store), using **terraphim-graph hybrid scoring** for `query_relevant` instead of
the existing BM25 scorer.

## Architecture

```
terraphim_types::LearningStore (sync trait)
    |
    | impl (sync-to-async bridge via block_in_place)
    v
terraphim_agent::SharedLearningStore
    ├── MarkdownLearningStore (persistence backend)
    ├── RwLock<HashMap<String, SharedLearning>> (in-memory index)
    ├── RoleGraph (optional, for hybrid scoring)
    |   └── query_graph() → weighted mean of node/edge/doc rank
    └── Bm25Scorer (existing, fallback when no graph)
```

## Key Design Decisions

### 1. RoleGraph for query_relevant

`query_relevant(agent, context, min_trust, limit)` will:
1. Filter learnings by `min_trust` and `applicable_agents` (same as before)
2. If a `RoleGraph` is available and has `learning_document_ids`, use
   `query_graph(context)` to get hybrid-scored document IDs, then match those
   back to learnings by ID
3. If no graph or no graph results, fall back to existing BM25 scoring on the
   remaining learnings

The RoleGraph's `query_graph()` performs Aho-Corasick matching → node lookup →
edge traversal → document ranking with weighted mean of node rank, edge rank,
and document rank. This is exactly the hybrid scoring the user requested.

### 2. Sync-to-async bridge

The `LearningStore` trait is synchronous. The agent's `SharedLearningStore`
methods are async. We use `tokio::task::block_in_place` +
`tokio::runtime::Handle::current().block_on()` — the same pattern as the
orchestrator's implementation in `crates/terraphim_orchestrator/src/learning.rs`.

All `impl LearningStore` methods MUST use `#[cfg(feature = "shared-learning")]`
and tests MUST use `#[tokio::test(flavor = "multi_thread")]`.

### 3. Type conversion

Agent re-exports types from `terraphim_types` via
`crates/terraphim_agent/src/shared_learning/types.rs`:

```rust
pub use terraphim_types::shared_learning::*;
```

So `crate::shared_learning::types::SharedLearning` IS
`terraphim_types::shared_learning::SharedLearning`. The agent's own `TrustLevel`
is also re-exported. This means **no type conversion is needed** — the types
are identical at the binary level.

### 4. RoleGraph integration

`RoleGraph` is already depended on by the agent crate. We add an optional
`Arc<RoleGraph>` to `SharedLearningStore`:

```rust
pub struct SharedLearningStore {
    backend: MarkdownLearningStore,
    index: RwLock<HashMap<String, SharedLearning>>,
    config: StoreConfig,
    role_graph: Option<Arc<RoleGraphSync>>,  // NEW
}
```

Wait — `RoleGraphSync` wraps `Arc<Mutex<RoleGraph>>`. We could use that, but
the `LearningStore` trait methods are sync and `RoleGraphSync::lock()` is async.
Instead, we'll store a snapshot or use `RoleGraph` directly behind a sync lock.

Actually, looking more carefully: `query_graph` on `RoleGraph` is synchronous.
We can store an `Arc<std::sync::RwLock<RoleGraph>>` — but the existing code
uses `RoleGraphSync` with tokio Mutex. For simplicity, let's store a clone of
the `RoleGraph` data (it's Clone) behind a `std::sync::RwLock`:

```rust
role_graph: Option<std::sync::RwLock<RoleGraph>>,
```

Or even simpler: just store `Option<RoleGraph>` directly since query_graph takes
`&self`. But RoleGraph might be large. Let's use `Arc<std::sync::RwLock<RoleGraph>>`.

Actually, looking at the constructor — `SharedLearningStore::open()` creates the
store. We can add a `set_role_graph()` method called after construction.

### 5. Method mapping

| LearningStore trait method | SharedLearningStore async method |
|---|---|
| `insert(SharedLearning)` | `insert(SharedLearning)` — direct, returns id |
| `get(id)` | `get(id)` — direct |
| `query_relevant(agent, context, min_trust, limit)` | Filter + RoleGraph hybrid + BM25 fallback |
| `record_applied(id)` | `record_application(id, "unknown", false)` |
| `record_effective(id)` | `record_application(id, "unknown", true)` |
| `list_by_trust(min_trust)` | Filter `list_all()` by trust >= min_trust |
| `archive_stale(max_age_days)` | Filter + remove L0 learnings older than threshold |

### 6. Feature gating

The `impl LearningStore for SharedLearningStore` goes behind
`#[cfg(feature = "shared-learning")]` in `store.rs`.

## Files to modify

1. `crates/terraphim_agent/src/shared_learning/store.rs` — Add RoleGraph field,
   add `set_role_graph()`, add `impl LearningStore`
2. `crates/terraphim_agent/src/shared_learning/mod.rs` — Re-export `LearningStore`
3. `crates/terraphim_agent/Cargo.toml` — Ensure `terraphim_rolegraph` has
   `features = ["kg-integration"]` when `shared-learning` is enabled

## Test plan

1. Test `insert` + `get` via trait
2. Test `query_relevant` with no graph (BM25 fallback)
3. Test `query_relevant` with graph (hybrid scoring)
4. Test `record_applied` / `record_effective` via trait
5. Test `list_by_trust` filters correctly
6. Test `archive_stale` removes old L0 learnings
7. Test sync-to-async bridge works (multi_thread runtime)
