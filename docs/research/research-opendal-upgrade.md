# Research Document: OpenDAL Upgrade Analysis

**Status**: Review
**Author**: Terraphim AI
**Date**: 2026-03-11
**Scope**: Investigate upgrading opendal to eliminate transitive security advisories

---

## Executive Summary

**Current State**: opendal v0.54.1 with sled v0.34.7 backend brings in:
- `fxhash` v0.2.1 (RUSTSEC-2025-0057)
- `instant` v0.1.13 (RUSTSEC-2024-0384)

**Finding**: OpenDAL 0.55 still includes sled service with same dependency chain. The advisories are in transitive dependencies (sled → parking_lot → instant/fxhash) and cannot be eliminated without:
1. Sled upgrading its dependencies (unlikely - sled is in maintenance mode)
2. OpenDAL removing sled support (not planned)
3. Using non-sled backends only

**Recommendation**: Accept risk and document, or disable sled feature if not used.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Security debt elimination |
| Leverages strengths? | Yes | Dependency management |
| Meets real need? | Partial | Advisories are in transitive deps, not our code |

**Proceed**: Partial - investigate alternative backends

---

## Current Dependency Chain

```
opendal v0.54.1 (current)
└── sled v0.34.7
    ├── fxhash v0.2.1 (RUSTSEC-2025-0057) ← advisory
    └── parking_lot v0.11.2
        └── instant v0.1.13 (RUSTSEC-2024-0384) ← advisory
```

## OpenDAL 0.55 Analysis

From changelog (2025-11-11):
- `refactor: migrate sled service from adapter::kv to impl Access directly`
- **Sled service still exists** - just implemented differently
- No indication that sled or its dependencies were removed

### Breaking Changes in 0.55
1. `Scheme` enum → string-based (major API change)
2. Removed deprecated `Operator::from_map` / `via_map`
3. Migrated from chrono to jiff
4. MSRV bumped to 1.85, edition 2024

## Backend Analysis

### What backends do we currently use?
From `terraphim_persistence/Cargo.toml`:
- `services-memory` (always enabled)
- `services-sqlite` (optional)
- `services-dashmap` (optional)
- `services-s3` (optional)
- `services-redis` (optional)
- `services-redb` (optional)
- `services-ipfs` (optional)

**No explicit sled feature** - sled is pulled in as a transitive dependency of opendal core.

### Where is sled used in opendal?
- Sled is a **default-enabled service** in opendal
- Even if we don't use it, it's compiled in

---

## Options for Resolution

| Option | Effort | Risk | Impact |
|--------|--------|------|--------|
| Upgrade to opendal 0.55 | Medium | High (breaking changes) | Low (sled still present) |
| Disable sled in opendal | Medium | Medium | High (if we can patch) |
| Use alternative backend | Low | Low | Medium (sqlite/dashmap/redis) |
| Accept risk | None | Low | Document advisories |

---

## Recommendation

**Short-term**: Accept the transitive dependency risk. The advisories are:
- `instant`: Unmaintained but functional (used by parking_lot)
- `fxhash`: Unmaintained but functional (used by sled)

Neither affects our code directly - we use `std::time::Instant` and no fxhash.

**Long-term**:
1. Monitor sled/parking_lot for updates
2. Consider switching to `redb` or `sqlite` backend if persistence needs change
3. Open issue upstream with sled to update dependencies

---

## Code Locations

### OpenDAL Usage
| File | Purpose |
|------|---------|
| `crates/terraphim_persistence/src/lib.rs` | Core persistence with Operator |
| `crates/terraphim_persistence/src/settings.rs` | Device settings persistence |
| `crates/terraphim_service/src/lib.rs` | Service layer integration |
| `crates/terraphim_config/src/lib.rs` | Config persistence |

### Backends Used
Based on Cargo.toml features:
- **Default**: memory, sqlite, dashmap
- **Optional**: s3, redis, redb, ipfs

---

## Upgrade Path (if needed)

If upgrading opendal becomes necessary:

1. **Bump version** in 4 Cargo.toml files
2. **Fix API changes**:
   - `Scheme::Memory` → `"memory"` (string-based)
   - Remove `from_map` usage
3. **Test all backends**
4. **Verify no regressions**

**Estimated effort**: 2-4 hours

---

## Appendix

### Verification Commands
```bash
# Check sled dependency
cargo tree -i sled

# Check opendal features
cargo metadata --format-version 1 | jq '.packages[] | select(.name == "opendal") | .features'

# Check opendal usage
grep -r "use opendal" --include="*.rs" crates/
```

### OpenDAL Changelog
- v0.55.0 (2025-11-11): https://github.com/apache/opendal/releases/tag/v0.55.0
- Key: sled service migrated but still present
