# Implementation Plan: Flush Compiled Thesaurus Cache After KG Markdown Edits

**Status**: Draft
**Research Doc**: `.docs/research-gitea-945.md`
**Author**: AI Agent
**Date**: 2026-04-26
**Estimated Effort**: 1-2 days

## Overview

### Summary
Implement lazy, on-demand cache invalidation for compiled thesauri by storing a content hash of source KG markdown files alongside the cached thesaurus. When a thesaurus is loaded, compare the stored hash against a freshly computed hash of the source files; on mismatch, invalidate the cache and rebuild. Also expose a manual `cache flush` CLI subcommand.

### Approach
**Option A — Content hash check (selected)**
Store a combined content hash of all `.md` files in the role's KG directory as a separate KV entry (`thesaurus_<role>_source_hash`). On every `ensure_thesaurus_loaded()`, compute the current hash and compare. If mismatched, delete the old cache entries and rebuild from markdown.

**Why not Option B (file watchers)?**
File watchers add a background thread, OS-specific complexity, and race conditions. The issue explicitly recommends Option A as simpler and sufficient.

### Scope

**In Scope:**
- Content hash computation for KG markdown source files
- Hash storage alongside cached thesaurus in KV store
- Hash comparison in the thesaurus load path
- Automatic cache invalidation and rebuild on hash mismatch
- Manual cache flush CLI subcommand (`terraphim-agent cache flush [--role ROLE]`)
- Per-role invalidation (only affected role is recompiled)
- Graceful fallback when cache is missing, corrupted, or DB is locked
- Regression test for stale-cache scenario
- Clear in-process `#[cached]` memoization when persistent cache is invalidated

**Out of Scope:**
- Real-time file watching (inotify/FSEvents)
- Incremental thesaurus updates (diffing)
- Cache TTL or time-based expiry
- Modifying the ripgrep-based builder or markdown parser

**Avoid At All Cost** (from 5/25 analysis):
- Adding a background thread or file watcher (overkill for this use case)
- Modifying the `Thesaurus` struct to include metadata (breaks serialization compatibility)
- Incremental/partial thesaurus updates (complexity not justified)
- Cryptographic hashing (SHA-256) when non-cryptographic is sufficient and faster

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     TerraphimService                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         ensure_thesaurus_loaded(role_name)          │   │
│  │                                                     │   │
│  │  1. Get role's KG path from ConfigState             │   │
│  │  2. Compute source_hash = hash_kg_dir(path)         │   │
│  │  3. Load cached_hash from KV store                  │   │
│  │  4. IF cached_hash != source_hash:                  │   │
│  │       - Delete thesaurus_<role>.json from cache    │   │
│  │       - Delete thesaurus_<role>_source_hash        │   │
│  │       - Clear in-process #[cached] memoization      │   │
│  │       - Rebuild from markdown (existing flow)       │   │
│  │       - Save new thesaurus + new hash to cache      │   │
│  │  5. ELSE: load thesaurus from cache (existing flow) │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         │                              │
         ▼                              ▼
┌─────────────────┐            ┌─────────────────┐
│  Hash Compute   │            │   KV Store      │
│  (twox-hash)    │            │  (SQLite/etc.)  │
└─────────────────┘            └─────────────────┘
```

### Data Flow

```
User edits KG markdown file
  → Next replace_matches() call
  → TerraphimService::ensure_thesaurus_loaded()
  → hash_kg_dir() computes fresh source hash
  → load cached hash from KV
  → Hash mismatch detected
  → cache_invalidate_thesaurus(role_name) deletes old entries
  → cached::clear_cache() clears in-process memoization
  → Logseq::build() rebuilds from markdown
  → Thesaurus::save() stores new thesaurus
  → save_source_hash() stores new hash
  → RoleGraph::new() builds new automaton
  → replace_matches() returns updated mappings
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Store hash in separate KV key (`thesaurus_<role>_source_hash`) | Avoids modifying `Thesaurus` struct; works with all backends | Embedding hash in `Thesaurus` struct (breaks compat) |
| Use `twox-hash` (XxHash64) | Fast, non-cryptographic, pure Rust, minimal dependency | SHA-256 (slower, unnecessary); std DefaultHasher (unstable across versions); manual FNV (reinventing wheel) |
| Combined directory hash (not per-file) | Simpler implementation; only one key to manage | Per-file hash tracking (more granular but complex) |
| Hash check in `ensure_thesaurus_loaded()` | Centralised load path; all entry points benefit | Check in `Thesaurus::load()` (couples persistence to invalidation logic) |
| Clear `#[cached]` memoization on invalidation | Prevents in-process cache from masking file changes | Leave `#[cached]` alone (would cause stale data within same process) |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| File watcher (notify crate) | Not in vital few; adds thread + OS complexity | Background thread lifecycle, cross-platform bugs, resource leaks |
| Incremental/partial updates | Over-engineering; full rebuild is fast enough | Complex merge logic, inconsistent state, bugs |
| Cache TTL | Time-based expiry is unreliable for this use case | Would expire valid caches, causing unnecessary rebuilds |
| SHA-256 for hashing | Cryptographic strength not needed; 3-5x slower | Unnecessary CPU overhead on every thesaurus load |

### Simplicity Check

> "What if this could be easy?"

The simplest design: compute a hash of all markdown files when loading a thesaurus. If the hash differs from what we stored, rebuild. Store the hash in a second KV key. This is exactly what we're doing — no threads, no watchers, no partial updates.

**Senior Engineer Test**: A senior engineer would call this appropriately simple. The only complexity is clearing the `#[cached]` memoization, which is a one-line call.

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
| `crates/terraphim_automata/src/hash.rs` | KG directory hash computation using twox-hash |
| `crates/terraphim_persistence/src/hash_store.rs` | Generic hash storage/retrieval in KV store |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_automata/src/lib.rs` | Add `mod hash;`, export `hash_kg_dir`, add `clear_cached_index()` |
| `crates/terraphim_automata/src/builder.rs` | Add `cached` clear function for `index_inner` |
| `crates/terraphim_automata/Cargo.toml` | Add `twox-hash` dependency |
| `crates/terraphim_service/src/lib.rs` | Add hash check to `ensure_thesaurus_loaded()`; add `flush_thesaurus_cache()` |
| `crates/terraphim_service/src/lib.rs` | Add `invalidate_thesaurus_cache()` helper |
| `crates/terraphim_persistence/src/lib.rs` | Add `HashStore` trait or free functions for hash read/write |
| `crates/terraphim_agent/src/repl/commands.rs` | Add `CacheFlush` variant to `ReplCommand` |
| `crates/terraphim_agent/src/repl/handler.rs` | Handle `CacheFlush` command |
| `crates/terraphim_agent/src/main.rs` | Add `cache flush` CLI subcommand |
| `crates/terraphim_agent/src/service.rs` | Add `flush_cache()` method |
| `crates/terraphim_service/tests/thesaurus_persistence_test.rs` | Add regression test |

### Deleted Files
| File | Reason |
|------|--------|
| None | |

## API Design

### New Types and Functions

```rust
// crates/terraphim_automata/src/hash.rs

use std::path::Path;
use twox_hash::XxHash64;
use std::hash::Hasher;

/// Compute a combined content hash of all `.md` files in a directory.
///
/// Walks the directory recursively, reads each `.md` file, and combines
/// their individual hashes into a single directory hash. Files are processed
/// in sorted order to ensure deterministic results.
///
/// Returns a hex-encoded string representation of the combined hash.
pub fn hash_kg_dir(path: &Path) -> std::io::Result<String> {
    let mut hasher = XxHash64::default();
    let mut entries: Vec<_> = walkdir::Dir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();
    entries.sort();

    for path in entries {
        let content = std::fs::read(&path)?;
        hasher.write(&content);
    }

    Ok(format!("{:016x}", hasher.finish()))
}

/// Clear the in-process cached memoization for `index_inner`.
///
/// This must be called when the persistent cache is invalidated,
/// otherwise the `cached` crate will return stale data.
pub fn clear_cached_index() {
    // The cached crate generates a function `INDEX_INNER_CACHE.clear()`
    // We need to expose this.
}
```

```rust
// crates/terraphim_persistence/src/hash_store.rs

use crate::{DeviceStorage, Error, Result};
use opendal::Operator;

/// Get the hash key for a role's thesaurus source.
fn hash_key(role_name: &str) -> String {
    let normalized = role_name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "_");
    format!("thesaurus_{}_source_hash", normalized)
}

/// Load the stored source hash for a role from the fastest operator.
pub async fn load_source_hash(role_name: &str) -> Result<Option<String>> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    match storage.fastest_op.read(&key).await {
        Ok(bs) => {
            let hash = String::from_utf8(bs.to_vec())
                .map_err(|e| Error::Config(format!("Invalid UTF-8 in hash: {}", e)))?;
            Ok(Some(hash))
        }
        Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Save the source hash for a role to all operators.
pub async fn save_source_hash(role_name: &str, hash: &str) -> Result<()> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    for (op, _time) in storage.ops.values() {
        op.write(&key, hash.to_string()).await?;
    }
    Ok(())
}

/// Delete the source hash and thesaurus cache for a role.
pub async fn delete_source_hash(role_name: &str) -> Result<()> {
    let storage = DeviceStorage::instance().await?;
    let key = hash_key(role_name);

    if let Err(e) = storage.fastest_op.delete(&key).await {
        log::debug!("Failed to delete hash key '{}': {}", key, e);
    }
    Ok(())
}
```

```rust
// crates/terraphim_service/src/lib.rs — additions to TerraphimService

impl TerraphimService {
    /// Invalidate the thesaurus cache for a role, forcing rebuild on next load.
    ///
    /// This deletes both the thesaurus JSON and the source hash from the
    /// persistent cache, and clears the in-process memoization.
    pub async fn invalidate_thesaurus_cache(&mut self,
        role_name: &RoleName,
    ) -> Result<()> {
        // 1. Delete persistent cache entries
        let thesaurus = Thesaurus::new(role_name.as_lowercase().to_string());
        let key = thesaurus.get_key();

        let storage = terraphim_persistence::DeviceStorage::instance().await
            .map_err(|e| ServiceError::Persistence(e))?;

        if let Err(e) = storage.fastest_op.delete(&key).await {
            log::debug!("Failed to delete thesaurus cache '{}': {}", key, e);
        }

        terraphim_persistence::hash_store::delete_source_hash(
            &role_name.as_lowercase()
        ).await.map_err(ServiceError::Persistence)?;

        // 2. Clear in-process memoization
        terraphim_automata::clear_cached_index();

        // 3. Remove from config_state roles (forces rebuild)
        self.config_state.roles.remove(role_name);

        log::info!("Invalidated thesaurus cache for role '{}'", role_name);
        Ok(())
    }

    /// Flush (invalidate) thesaurus cache for one or all roles.
    pub async fn flush_thesaurus_cache(
        &mut self,
        role_name: Option<&RoleName>,
    ) -> Result<usize> {
        let mut count = 0;

        if let Some(role) = role_name {
            self.invalidate_thesaurus_cache(role).await?;
            count = 1;
        } else {
            // Get all role names first to avoid borrow issues
            let roles: Vec<RoleName> = self.config_state.roles.keys().cloned().collect();
            for role in roles {
                self.invalidate_thesaurus_cache(&role).await?;
                count += 1;
            }
        }

        log::info!("Flushed thesaurus cache for {} role(s)", count);
        Ok(count)
    }
}
```

### Modified Function Signatures

```rust
// In TerraphimService::ensure_thesaurus_loaded() — add hash check

pub async fn ensure_thesaurus_loaded(&mut self,
    role_name: &RoleName,
) -> Result<Thesaurus> {
    // ... existing logic ...

    // NEW: Check source hash before loading from cache
    if let Some(kg_path) = self.get_kg_path_for_role(role_name).await {
        match terraphim_automata::hash::hash_kg_dir(&kg_path) {
            Ok(current_hash) => {
                match terraphim_persistence::hash_store::load_source_hash(
                    &role_name.as_lowercase()
                ).await {
                    Ok(Some(cached_hash)) if cached_hash == current_hash => {
                        // Hash matches — proceed with normal cache load
                        log::debug!("Source hash matches for role '{}' — using cache", role_name);
                    }
                    Ok(Some(_)) => {
                        // Hash mismatch — invalidate and rebuild
                        log::info!("Source hash mismatch for role '{}' — rebuilding", role_name);
                        self.invalidate_thesaurus_cache(role_name).await?;
                    }
                    Ok(None) => {
                        // No cached hash — first load, will save after build
                        log::debug!("No cached hash for role '{}' — will save after build", role_name);
                    }
                    Err(e) => {
                        log::warn!("Failed to load source hash for role '{}': {:?}", role_name, e);
                        // Continue — will fall back to rebuild if cache load fails
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to compute source hash for role '{}': {:?}", role_name, e);
                // Continue — will fall back to existing behavior
            }
        }
    }

    // ... rest of existing logic ...

    // NEW: After successful build/load, save the source hash
    if let Some(kg_path) = self.get_kg_path_for_role(role_name).await {
        if let Ok(hash) = terraphim_automata::hash::hash_kg_dir(&kg_path) {
            if let Err(e) = terraphim_persistence::hash_store::save_source_hash(
                &role_name.as_lowercase(), &hash
            ).await {
                log::warn!("Failed to save source hash for role '{}': {:?}", role_name, e);
            }
        }
    }

    Ok(thesaurus)
}
```

### CLI Subcommand

```rust
// In terraphim-agent CLI (crates/terraphim_agent/src/main.rs)

#[derive(Subcommand)]
enum CacheCommands {
    /// Flush the compiled thesaurus cache
    Flush {
        /// Role to flush (flushes all roles if omitted)
        #[arg(long)]
        role: Option<String>,
    },
}

// In command handler:
Commands::Cache { command } => {
    match command {
        CacheCommands::Flush { role } => {
            let role_name = role.map(RoleName::from);
            let count = service.flush_thesaurus_cache(role_name.as_ref()).await?;
            println!("Flushed cache for {} role(s)", count);
        }
    }
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_hash_kg_dir_deterministic` | `crates/terraphim_automata/src/hash.rs` | Same directory produces same hash |
| `test_hash_kg_dir_changes_on_edit` | `crates/terraphim_automata/src/hash.rs` | Editing a file changes the hash |
| `test_hash_kg_dir_empty_dir` | `crates/terraphim_automata/src/hash.rs` | Empty directory returns consistent hash |
| `test_load_save_source_hash` | `crates/terraphim_persistence/src/hash_store.rs` | Round-trip hash storage |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_stale_cache_invalidation` | `crates/terraphim_service/tests/thesaurus_persistence_test.rs` | Build thesaurus, edit markdown, verify next load reflects changes |
| `test_cache_flush_command` | `crates/terraphim_agent/tests/` | CLI flush command removes cache and forces rebuild |
| `test_graceful_fallback_on_hash_error` | `crates/terraphim_service/tests/` | Hash computation failure doesn't crash; falls back to normal load |

### Regression Test (stale-cache scenario)
```rust
#[tokio::test]
async fn test_theaurus_cache_invalidation_on_kg_edit() {
    // 1. Set up temp directory with a KG markdown file
    // 2. Build and save thesaurus
    // 3. Verify replace_matches returns original mapping
    // 4. Edit the markdown file
    // 5. Call ensure_thesaurus_loaded() again
    // 6. Verify replace_matches returns updated mapping
    // 7. Verify new hash is saved to cache
}
```

## Implementation Steps

### Step 1: Hash computation module
**Files:** `crates/terraphim_automata/src/hash.rs`, `crates/terraphim_automata/src/lib.rs`, `crates/terraphim_automata/Cargo.toml`
**Description:** Add `twox-hash` dependency and implement `hash_kg_dir()`
**Tests:** Unit tests for determinism, change detection, empty dir
**Estimated:** 2 hours

### Step 2: Hash storage module
**Files:** `crates/terraphim_persistence/src/hash_store.rs`, `crates/terraphim_persistence/src/lib.rs`
**Description:** Implement `load_source_hash()`, `save_source_hash()`, `delete_source_hash()`
**Tests:** Unit tests for round-trip storage
**Estimated:** 2 hours

### Step 3: Integrate hash check into load path
**Files:** `crates/terraphim_service/src/lib.rs`
**Description:** Modify `ensure_thesaurus_loaded()` to check hash before loading; add `invalidate_thesaurus_cache()` and `flush_thesaurus_cache()`
**Tests:** Integration test for stale-cache scenario
**Estimated:** 4 hours

### Step 4: Clear in-process memoization
**Files:** `crates/terraphim_automata/src/builder.rs`, `crates/terraphim_automata/src/lib.rs`
**Description:** Expose a function to clear the `#[cached]` memoization for `index_inner()`
**Tests:** Verify cache is cleared after invalidation
**Estimated:** 1 hour

### Step 5: CLI subcommand
**Files:** `crates/terraphim_agent/src/main.rs`, `crates/terraphim_agent/src/repl/commands.rs`, `crates/terraphim_agent/src/repl/handler.rs`, `crates/terraphim_agent/src/service.rs`
**Description:** Add `cache flush [--role ROLE]` subcommand to both CLI and REPL
**Tests:** CLI integration test
**Estimated:** 2 hours

### Step 6: Documentation and final verification
**Files:** README updates, inline docs
**Description:** Document the cache invalidation behavior and CLI command
**Tests:** Run full test suite
**Estimated:** 1 hour

## Rollback Plan

If issues discovered:
1. Revert changes to `TerraphimService::ensure_thesaurus_loaded()` — the old logic remains intact as fallback
2. Remove hash check calls — system falls back to existing behavior (cache never invalidates)
3. Feature flag: Could add `#[cfg(feature = "thesaurus-cache-invalidation")]` if needed

## Migration

No database migration required. The new `thesaurus_<role>_source_hash` keys are created lazily on first save. Old thesaurus entries without hash metadata will be treated as "no cached hash" and will get a hash saved on next load.

## Dependencies

### New Dependencies
| Crate | Version | Justification |
|-------|---------|---------------|
| `twox-hash` | `^2.0` | Fast non-cryptographic hash for file content hashing |
| `walkdir` | `^2.0` | Directory traversal (likely already transitive dep) |

### Dependency Updates
| Crate | From | To | Reason |
|-------|------|-----|--------|
| None | — | — | — |

## Performance Considerations

### Expected Performance
| Metric | Target | Measurement |
|--------|--------|-------------|
| Hash computation | < 5ms for < 100 files | Benchmark `hash_kg_dir()` |
| Extra load latency | < 5ms per `ensure_thesaurus_loaded()` | Benchmark integration test |
| Rebuild penalty | Same as current cold start | No change to rebuild path |

### Benchmarks to Add
```rust
#[tokio::test]
async fn bench_hash_kg_dir() {
    let temp_dir = setup_test_kg_dir(50); // 50 markdown files
    let start = Instant::now();
    let _hash = terraphim_automata::hash::hash_kg_dir(&temp_dir).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(10), "Hash computation too slow: {:?}", elapsed);
}
```

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Confirm `twox-hash` version and feature flags | Pending | Implementer |
| Verify `cached` crate cache clearing API | Pending | Implementer |
| Decide if `terraphim-cli` also needs flush command | Pending | Human reviewer |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
