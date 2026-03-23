# Implementation Plan: Persistence Layer Memory Warm-up and Default Settings Cleanup

**Status**: Implemented
**Research Doc**: `.docs/research-persistence-memory-warmup.md`
**Author**: Terraphim AI
**Date**: 2026-01-23
**Implementation Date**: 2026-01-23
**Estimated Effort**: 4-6 hours

---

## Implementation Summary

**Completed 2026-01-23** - All core implementation tasks completed successfully.

### Implemented Features:
1. **Cache Write-back** (`lib.rs`): Automatic caching to fastest operator when loading from fallback
2. **Compression** (`compression.rs`): zstd compression for objects >1MB with magic header detection
3. **Schema Evolution Recovery**: Cache deletion and refetch on deserialization failure
4. **Tracing Spans**: Observability with `debug_span` for cache operations
5. **Same-operator Detection**: Pointer equality check to skip redundant cache writes

### Test Coverage:
- 5 unit tests in `compression.rs` - compression/decompression behavior
- 9 integration tests in `tests/persistence_warmup.rs` - cache warm-up scenarios
- All existing 33 persistence tests continue to pass

### Documentation:
- CLAUDE.md updated with "Persistence Layer Cache Warm-up" section
- Inline documentation added to `load_from_operator()` method

### Pending (Separate Repository):
- `terraphim-private` settings.toml still has `[profiles.memory]` enabled
- This is already disabled in `terraphim-ai` default settings

---

## Overview

### Summary
This plan implements two related improvements to the persistence layer:
1. Remove memory profile from user-facing default settings (keep for tests only)
2. Add cache write-back to `load_from_operator()` so data loaded from slow persistent services gets cached in fast memory services

### Approach
- Modify `load_from_operator()` to write back successfully loaded data to the fastest operator
- Use async fire-and-forget pattern to avoid blocking the load path
- Update terraphim-private settings to remove memory profile from defaults
- Retain `init_memory_only()` path for test isolation

### Scope
**In Scope:**
- Cache write-back in `load_from_operator()` method
- Settings cleanup in terraphim-private repository
- Unit tests for cache write-back behavior
- Documentation updates

**Out of Scope:**
- Profile classification (cache vs persistent) - future enhancement
- LRU eviction for memory caches - future enhancement
- Cache preloading at startup - future enhancement

## Architecture

### Component Diagram
```
                    ┌─────────────────────────────────────────┐
                    │           load_from_operator()          │
                    └─────────────────────────────────────────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    ▼                   ▼                   ▼
            ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
            │  fastest_op  │   │   profile_1  │   │   profile_N  │
            │  (memory)    │   │   (sqlite)   │   │   (s3)       │
            └──────────────┘   └──────────────┘   └──────────────┘
                    │                   │                   │
                    │                   │                   │
           ┌────────┴────────┐  ┌──────┴──────┐    ┌───────┴───────┐
           │ 1. Try read     │  │ 2. Fallback │    │ 3. Fallback   │
           │    MISS         │  │    SUCCESS  │    │    (if needed)│
           └─────────────────┘  └──────┬──────┘    └───────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │                  ▼                  │
                    │   ┌─────────────────────────────┐   │
                    │   │ NEW: Cache write-back       │   │
                    │   │ tokio::spawn(write to       │   │
                    │   │ fastest_op)                 │   │
                    │   └─────────────────────────────┘   │
                    └─────────────────────────────────────┘
```

### Data Flow
```
[load() called]
    -> [fastest_op.read(key)]
    -> MISS
    -> [iterate slower profiles]
    -> [profile_N.read(key)]
    -> SUCCESS
    -> [spawn: fastest_op.write(key, data)]  <-- NEW
    -> [return data]
```

### Key Design Decisions
| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Fire-and-forget cache write | Non-blocking, doesn't slow load path | Sync write (blocks), write-through (complex) |
| Use serde_json for serialization | Already used throughout codebase | bincode (incompatible with existing data) |
| Log at debug level on cache write failure | Non-critical operation, avoid log noise | warn level (too noisy), silent (no debugging) |
| Keep memory profile for tests only | Tests need isolation and speed | Remove completely (breaks tests) |

## File Changes

### New Files
None - all changes are modifications to existing files.

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_persistence/src/lib.rs` | Add cache write-back in `load_from_operator()` |
| `terraphim-private/.../settings.toml` | Remove `[profiles.memory]` section |

### Deleted Files
None.

## API Design

### No Public API Changes
This implementation is entirely internal. The `Persistable` trait interface remains unchanged:
- `load()` - Same signature, now with transparent caching
- `save()` - Unchanged
- `load_from_operator()` - Same signature, internal behavior change

### Internal Implementation Changes

```rust
/// Modified load_from_operator() with cache write-back
///
/// When data is successfully loaded from a fallback (slower) operator,
/// it is asynchronously written to the fastest operator for future access.
///
/// # Cache Write-back Behavior
/// - Non-blocking: Uses tokio::spawn for fire-and-forget
/// - Best-effort: Failures logged at debug level, don't affect load
/// - Idempotent: Safe to write same data multiple times
async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
where
    Self: Sized + Clone + Send + Sync + 'static,
{
    // ... existing code ...

    // NEW: Cache write-back after successful fallback load
    if let Ok(serialized) = serde_json::to_vec(&obj) {
        let fastest = fastest_op.clone();
        let k = key.to_string();
        tokio::spawn(async move {
            if let Err(e) = fastest.write(&k, serialized).await {
                log::debug!("Cache write-back failed for '{}': {}", k, e);
            } else {
                log::debug!("Cached '{}' to fastest operator", k);
            }
        });
    }
    // ... return data ...
}
```

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_cache_writeback_on_fallback` | `lib.rs` | Verify data is written to fast op after fallback load |
| `test_cache_writeback_failure_doesnt_block` | `lib.rs` | Ensure load succeeds even if cache write fails |
| `test_subsequent_load_uses_cache` | `lib.rs` | Verify second load hits cached data |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_memory_sqlite_warmup` | `tests/persistence_warmup.rs` | Full flow with memory + sqlite profiles |
| `test_dashmap_sqlite_warmup` | `tests/persistence_warmup.rs` | Full flow with dashmap + sqlite profiles |

### Test Implementation Approach
```rust
#[tokio::test]
async fn test_cache_writeback_on_fallback() {
    // Setup: Create multi-profile storage (memory + sqlite)
    // 1. Write test data to sqlite only
    // 2. Call load() - should fallback to sqlite
    // 3. Wait briefly for async cache write
    // 4. Call load() again - should hit memory cache
    // 5. Verify memory operator contains the data
}

#[tokio::test]
async fn test_cache_writeback_failure_doesnt_block() {
    // Setup: Create storage where fastest_op write fails
    // 1. Write test data to fallback profile
    // 2. Call load() - should succeed despite cache write failure
    // 3. Verify data is returned correctly
}
```

## Implementation Steps

### Step 1: Add Cache Write-back to load_from_operator()
**Files:** `crates/terraphim_persistence/src/lib.rs`
**Description:** Modify `load_from_operator()` to write successfully loaded data back to fastest_op
**Tests:** Unit tests for cache write-back behavior
**Estimated:** 2 hours

```rust
// Key code change in load_from_operator() at line 344-350
Ok(obj) => {
    log::info!(
        "Successfully loaded '{}' from fallback profile '{}'",
        key,
        profile_name
    );

    // NEW: Cache to fastest operator (non-blocking)
    if let Ok(serialized) = serde_json::to_vec(&obj) {
        let fastest = fastest_op.clone();
        let k = key.to_string();
        tokio::spawn(async move {
            match fastest.write(&k, serialized).await {
                Ok(_) => log::debug!("Cached '{}' to fastest operator", k),
                Err(e) => log::debug!("Cache write-back failed for '{}': {}", k, e),
            }
        });
    }

    return Ok(obj);
}
```

### Step 2: Add Trait Bounds for Cache Write-back
**Files:** `crates/terraphim_persistence/src/lib.rs`
**Description:** Update trait bounds on `load_from_operator()` to support async spawning
**Tests:** Ensure existing implementations compile
**Dependencies:** Step 1
**Estimated:** 30 minutes

```rust
// May need to add Clone + Send + Sync + 'static bounds
async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
where
    Self: Sized + Clone + Send + Sync + 'static,
```

### Step 3: Add Unit Tests
**Files:** `crates/terraphim_persistence/src/lib.rs`
**Description:** Add tests for cache write-back behavior
**Tests:** Self (test module)
**Dependencies:** Step 2
**Estimated:** 1 hour

### Step 4: Add Integration Tests
**Files:** `crates/terraphim_persistence/tests/persistence_warmup.rs`
**Description:** Full integration tests with multi-profile scenarios
**Tests:** Self
**Dependencies:** Step 3
**Estimated:** 1 hour

### Step 5: Update terraphim-private Settings
**Files:** `/home/alex/projects/terraphim/terraphim-private/crates/terraphim_settings/default/settings.toml`
**Description:** Remove `[profiles.memory]` from default settings
**Tests:** Manual verification of config loading
**Dependencies:** None (can be done in parallel)
**Estimated:** 15 minutes

### Step 6: Documentation Update
**Files:** `CLAUDE.md`, inline docs
**Description:** Document cache write-back behavior
**Tests:** None
**Dependencies:** Step 4
**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Revert the cache write-back code in `load_from_operator()`
2. The fire-and-forget pattern means no data corruption risk
3. System continues to work without cache warm-up (just slower)

No feature flag needed - the change is transparent and backward compatible.

## Migration

No migration required:
- Cache write-back is transparent to callers
- Existing data remains valid
- No schema changes

## Dependencies

### New Dependencies
None required.

### Dependency Updates
None required.

## Performance Considerations

### Expected Performance
| Metric | Target | Current |
|--------|--------|---------|
| First load latency | Same as current | ~100-500ms (depends on backend) |
| Subsequent loads | <10ms | ~100-500ms (repeated slow loads) |
| Cache write overhead | <1ms (async) | N/A (no caching) |

### Memory Impact
- Minimal: Only caches data that would have been loaded anyway
- No additional memory allocation beyond what's returned to caller
- Async spawn has ~100 bytes overhead per operation

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify Clone bound doesn't break existing impls | Pending | Implementation phase |

## Approval Checklist

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received

## Next Steps

After Phase 2.5 approval:
1. Proceed to implementation (Phase 3) using `disciplined-implementation` skill
2. Run full test suite
3. Update CLAUDE.md with new behavior

---

## Specification Interview Findings

**Interview Date**: 2026-01-23
**Dimensions Covered**: Concurrency, Failure Modes, Edge Cases, Scale/Performance, Integration Effects, Migration
**Convergence Status**: Complete (user confirmed coverage sufficient)

### Key Decisions from Interview

#### Concurrency & Race Conditions
- **Allow duplicate writes**: If two concurrent loads both miss cache and fallback, both can spawn cache writes. Data is idempotent, so last-write-wins is acceptable. Simplest approach.
- **Shutdown behavior**: Let spawned cache write tasks drop on process shutdown. Fire-and-forget is acceptable since data exists in persistent source.

#### Failure Modes & Recovery
- **No backpressure/circuit breaker**: Each cache write is independent. Failures are debug-logged only; no tracking or exponential backoff needed.
- **Schema evolution handling**: If cached data fails to deserialize (schema changed), catch the error, delete the cache entry, and retry from persistent source.

#### Edge Cases & Boundaries
- **Large object handling**: Compress objects over 1MB threshold using zstd before caching. Adds `zstd` dependency.
- **Same-operator skip**: When fastest_op IS the persistent storage (SQLite-only config), skip the redundant cache write using pointer equality check.

#### Cache Invalidation Strategy
- **Write-through on save**: When `save_to_all()` is called, the cache is updated as part of the parallel write to all profiles. This ensures cache consistency without explicit invalidation.

#### Data Safety
- **Cache everything**: All Persistable types get cached. Cache is purely acceleration; persistent source remains the source of truth.

#### Scale & Performance
- **Compression**: 1MB threshold with zstd algorithm for large objects
- **No size limits otherwise**: Memory is cheap; data came from persistent source anyway

#### Observability & Operations
- **Tracing spans**: Use `tracing::info_span!` with fields for cache operations (hit/miss/write_success/write_fail) rather than dedicated metrics crate
- **Debug logs**: Maintain existing debug-level logging for troubleshooting

#### Testing Strategy
- **Use init_memory_only()**: Tests continue using memory-only path for isolation. Multi-profile scenarios tested via integration tests.
- **Verify trait bounds at implementation**: Check that Document, Config, Thesaurus, Conversation satisfy Clone + Send + Sync + 'static bounds during implementation.

#### Migration & Compatibility
- **No migration warning needed**: Memory profile still works if configured; just removed from defaults. No user notification required.

### Implementation Scope Changes

Based on interview findings, the following changes are added to the implementation plan:

1. **New dependency**: Add `zstd` crate for compression
2. **Compression logic**: Add helper to compress/decompress objects > 1MB
3. **Same-op detection**: Add pointer equality check before cache write
4. **Schema recovery**: Wrap cache read in try/catch, delete and retry on deserialization failure
5. **Tracing spans**: Add spans for cache operations

### Updated Implementation Steps

| Step | Description | Added by Interview |
|------|-------------|-------------------|
| 1a | Add zstd dependency | Yes |
| 1b | Implement compression helper (>1MB threshold) | Yes |
| 1c | Add same-operator detection | Yes |
| 1d | Add schema evolution recovery | Yes |
| 1e | Add tracing spans for observability | Yes |

### Deferred Items
- **LRU eviction**: Deferred to future iteration (out of scope)
- **Profile classification**: Deferred (cache vs persistent metadata)
- **Cache preloading at startup**: Deferred to future iteration

### Interview Summary

The specification interview clarified several important edge cases and implementation details that weren't explicit in the original design. The most significant findings were:

1. **Compression requirement**: Large objects (Documents with full body content) could be several MB. The decision to use zstd compression at 1MB threshold adds a dependency but prevents memory bloat.

2. **Schema evolution handling**: A critical edge case where cached data fails to deserialize after type changes. The fail-and-refetch pattern ensures graceful recovery.

3. **Same-operator optimization**: When only SQLite is configured (no memory layer), the cache write would be redundant. Detecting this via pointer equality avoids unnecessary work.

4. **Observability**: The decision to use tracing spans rather than a dedicated metrics crate keeps the implementation simpler while still enabling production debugging.

The interview achieved convergence after 5 rounds of questions, covering 6 of the 10 specification dimensions. The remaining dimensions (Accessibility, Internationalization, Security/Privacy detailed review) were not applicable to this internal caching feature.
