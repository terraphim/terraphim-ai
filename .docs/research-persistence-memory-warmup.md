# Research Document: Persistence Layer Memory Warm-up and Default Settings Cleanup

**Status**: Draft
**Author**: Terraphim AI
**Date**: 2026-01-23
**Reviewers**: Alex

## Executive Summary

This research investigates two related issues:
1. Memory-only profiles included in default user settings (should be test-only)
2. Lack of warm-up/population mechanism when fast in-memory services need data from slower persistent services

The current persistence layer uses a fallback mechanism that loads from slower services when fast services fail, but **does not copy the loaded data back to the fast service**, resulting in repeated slow loads.

## Problem Statement

### Description

**Problem 1: Memory Profile in Default Settings**
The `terraphim-private` repository's `settings.toml` includes `[profiles.memory]` which should only be used in tests. Memory services lose data on process restart, causing user configuration (like role selections) to be lost.

**Problem 2: Missing Cache Warm-up**
When a memory/dashmap service is configured as the fastest operator alongside a persistent service (sqlite/s3), the system:
1. Tries the fast memory service first (fails - no data)
2. Falls back to the slow persistent service (succeeds)
3. Returns the data but **does NOT cache it in the fast service**
4. Next request repeats steps 1-3

### Impact

- **Users**: Role selections lost between CLI invocations
- **Performance**: Repeated slow loads from persistent storage
- **Confusion**: Inconsistent behavior between test and production configurations

### Success Criteria

1. Default user settings use only persistent storage (SQLite/redb)
2. Test configurations retain memory-only capability
3. When data is loaded from a slow service, it gets cached in the fast service
4. Subsequent loads are served from the fast cache

## Current State Analysis

### Existing Implementation

**Storage Profile Types (by speed)**:
| Type | Speed | Persistence | Use Case |
|------|-------|-------------|----------|
| memory | ~1ms | None | Tests only |
| dashmap | ~2ms | None (in-memory) | Cache layer |
| redb | ~5ms | Durable | Production |
| sqlite | ~10ms | Durable | Production |
| rocksdb | ~15ms | Durable | Production |
| s3 | ~100ms+ | Durable | Cloud storage |

**Speed Determination**: `crates/terraphim_persistence/src/settings.rs:169-184`
- Each profile is benchmarked by writing/reading 1MB test file
- Profiles sorted by speed, fastest becomes `fastest_op`

**Fallback Mechanism**: `crates/terraphim_persistence/src/lib.rs:268-365`
- `load_from_operator()` tries `fastest_op` first
- On failure, iterates through all profiles by speed
- Returns first successful load
- **GAP**: Does not write successful fallback data to faster operators

### Code Locations

| Component | Location | Purpose |
|-----------|----------|---------|
| DeviceStorage | `crates/terraphim_persistence/src/lib.rs:36-97` | Storage singleton |
| Persistable trait | `crates/terraphim_persistence/src/lib.rs:200-411` | Save/load interface |
| Profile parsing | `crates/terraphim_persistence/src/settings.rs` | OpenDAL operator creation |
| Memory utils | `crates/terraphim_persistence/src/memory.rs` | Memory-only helpers |
| Default settings | `crates/terraphim_settings/default/settings.toml` | User-facing config |
| Test settings | `crates/terraphim_settings/test_settings/settings.toml` | Test config |
| Private settings | `/home/alex/projects/terraphim/terraphim-private/crates/terraphim_settings/default/settings.toml` | Has memory profile |

### Data Flow

**Current Flow (with fallback)**:
```
1. load() called
2. fastest_op.read(key) - FAILS (no data in memory)
3. for each slower_op:
     slower_op.read(key) - SUCCESS
     return data
4. Next load() - repeats steps 1-3
```

**Desired Flow (with warm-up)**:
```
1. load() called
2. fastest_op.read(key) - FAILS (no data in memory)
3. for each slower_op:
     slower_op.read(key) - SUCCESS
     fastest_op.write(key, data)  <-- NEW: cache to fast service
     return data
4. Next load() - fastest_op.read(key) - SUCCESS (from cache)
```

### Integration Points

- `terraphim_config::Config::load()` - Uses Persistable
- `terraphim_types::Document` - Persistable impl
- `terraphim_types::Thesaurus` - Persistable impl
- `terraphim_persistence::conversation::Conversation` - Persistable impl

## Constraints

### Technical Constraints

- **Static singleton**: `DEVICE_STORAGE` is `AsyncOnceCell` - once initialized, profiles are fixed
- **No profile type metadata**: Profile maps don't distinguish "cache" vs "persistence" layers
- **Test isolation**: Tests use `init_memory_only()` which precludes multi-profile testing

### Business Constraints

- Must maintain backward compatibility with existing configurations
- Tests must remain fast (memory-only)
- Production must be persistent (SQLite/redb)

### Non-Functional Requirements

| Requirement | Target | Current |
|-------------|--------|---------|
| First load latency | <500ms | ~100ms (sqlite) to ~500ms+ (s3) |
| Subsequent loads | <10ms | ~100ms (repeated slow loads) |
| Data persistence | 100% | 100% (for persistent profiles) |
| Test isolation | Full | Full |

## Dependencies

### Internal Dependencies

| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_settings | Profile configuration source | Low |
| terraphim_types | Document/Thesaurus types | Low |
| opendal | Storage abstraction | Low |

### External Dependencies

| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| opendal | 0.54+ | Low | N/A |
| async-once-cell | 0.5 | Low | tokio::sync::OnceCell |

## Risks and Unknowns

### Known Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Write conflicts | Low | Medium | Use fastest_op for cache writes only |
| Memory bloat | Medium | Low | Implement LRU eviction in future |
| Test regression | Low | High | Maintain `init_memory_only()` path |

### Open Questions

1. **Q**: Should all data be cached or only specific types?
   - Recommendation: Cache all Persistable loads to maintain consistency

2. **Q**: Should cache writes be async/fire-and-forget?
   - Recommendation: Yes, to avoid blocking the load path

3. **Q**: How to handle cache invalidation?
   - Recommendation: No invalidation needed - cache is warm-up only, not source of truth

### Assumptions

1. Memory profile is only needed for tests - basis: user config shouldn't be lost on restart
2. Fallback reads are infrequent after warm-up - basis: most data loaded once at startup
3. Cache writes are safe - basis: data came from authoritative persistent source

## Research Findings

### Key Insights

1. **Public vs Private Settings Divergence**: `terraphim-ai` (public) uses SQLite-only, `terraphim-private` has memory profile in defaults - should align

2. **Existing Warm-up Pattern**: `terraphim_multi_agent::pool::warm_up_pool()` shows the pattern for pre-populating resources

3. **Fallback Already Works**: The `load_from_operator()` method successfully falls back, just doesn't cache

4. **Profile Classification Missing**: No way to mark profiles as "cache" vs "persistent" - needed for write-back

### Relevant Prior Art

- **terraphim_multi_agent pool warming**: Pre-creates agents in pool at startup
- **OpenDAL layer pattern**: Could add caching layer to operators

### Technical Spikes Needed

| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Profile classification | Add cache/persistent metadata to profiles | 1-2 hours |
| Write-back implementation | Modify `load_from_operator` to cache | 2-3 hours |
| Test updates | Add integration tests for warm-up | 1-2 hours |

## Recommendations

### Proceed/No-Proceed

**PROCEED** - Both issues are well-understood with clear solutions:

1. **Remove memory from default settings**: Simple config change
2. **Add cache write-back**: Modify `load_from_operator()` to write to fastest operator after successful fallback load

### Scope Recommendations

**Phase 1** (this PR):
- Remove `[profiles.memory]` from default settings
- Keep `init_memory_only()` for tests
- Add cache write-back to `load_from_operator()`

**Phase 2** (future):
- Add profile classification (cache vs persistent)
- Add LRU eviction for memory caches
- Add cache preloading at startup

### Risk Mitigation Recommendations

1. Add integration test for fallback + cache behavior
2. Keep test path unchanged (memory-only)
3. Make cache write-back async/non-blocking

## Next Steps

If approved:
1. Create design document with implementation details
2. Implement profile classification (optional field)
3. Modify `load_from_operator()` to write-back on fallback success
4. Update default settings to remove memory profile
5. Add integration tests
6. Update CLAUDE.md with new behavior

## Appendix

### Reference Materials

- OpenDAL Documentation: https://opendal.apache.org/
- Caching patterns: Write-through vs Write-back

### Code Snippets

**Current fallback (no cache)**:
```rust
// crates/terraphim_persistence/src/lib.rs:331-350
for (profile_name, (op, _speed)) in ops_vec {
    if let Some(result) = try_read_from_op::<Self>(op, key, Some(profile_name)).await {
        match result {
            Ok(obj) => {
                log::info!("Successfully loaded '{}' from fallback profile '{}'", key, profile_name);
                return Ok(obj);  // <-- Missing: cache to fastest_op
            }
            // ...
        }
    }
}
```

**Proposed cache write-back**:
```rust
for (profile_name, (op, _speed)) in ops_vec {
    if let Some(result) = try_read_from_op::<Self>(op, key, Some(profile_name)).await {
        match result {
            Ok(obj) => {
                log::info!("Successfully loaded '{}' from fallback profile '{}'", key, profile_name);
                // NEW: Cache to fastest operator (non-blocking)
                if let Ok(serialized) = serde_json::to_vec(&obj) {
                    let fastest = fastest_op.clone();
                    let k = key.to_string();
                    tokio::spawn(async move {
                        if let Err(e) = fastest.write(&k, serialized).await {
                            log::debug!("Failed to cache '{}': {}", k, e);
                        }
                    });
                }
                return Ok(obj);
            }
            // ...
        }
    }
}
```

### Settings Comparison

**terraphim-ai (public) - CORRECT**:
```toml
# Only SQLite for persistence
[profiles.sqlite]
type = "sqlite"
datadir = "${TERRAPHIM_DATA_PATH:-~/.terraphim}/sqlite"
```

**terraphim-private - NEEDS FIX**:
```toml
# Has memory profile in defaults (should be test-only)
[profiles.memory]
type = "memory"
```
