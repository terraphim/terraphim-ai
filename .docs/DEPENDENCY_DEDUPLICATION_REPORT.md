# Dependency Deduplication Report

## Executive Summary

Analysis of the Terraphim AI workspace reveals significant dependency duplication that contributes to excessive disk usage during compilation. This report identifies duplicate dependencies and provides recommendations for consolidation.

## Duplicate Dependencies Analysis

### Critical Duplicates (High Impact)

| Package | Versions | Est. Size Impact |
|---------|----------|------------------|
| **ahash** | 0.7.8, 0.8.12 | ~2-3 MB |
| **hashbrown** | 0.12.3, 0.14.5, 0.15.5, 0.16.1 | ~5-8 MB |
| **syn** | 1.0.109, 2.0.114 | ~8-12 MB |
| **thiserror** | 1.0.69, 2.0.17 | ~1-2 MB |
| **getrandom** | 0.2.16, 0.3.4 | ~1-2 MB |
| **rand** | 0.8.5, 0.9.2 | ~2-3 MB |
| **toml** | 0.5.11, 0.9.8 | ~2-3 MB |
| **phf** | 0.11.3, 0.13.1 | ~1-2 MB |

### Medium Impact Duplicates

| Package | Versions | Est. Size Impact |
|---------|----------|------------------|
| **console** | 0.15.11, 0.16.1 | ~1-2 MB |
| **indicatif** | 0.17.11, 0.18.3 | ~1-2 MB |
| **darling** | 0.20.11, 0.21.3 | ~2-3 MB |
| **derive_more** | 1.0.0, 2.0.1 | ~1-2 MB |
| **dirs** | 5.0.1, 6.0.0 | ~0.5-1 MB |
| **html5ever** | 0.27.0, 0.36.1 | ~3-5 MB |
| **quick-xml** | 0.37.5, 0.38.4 | ~1-2 MB |
| **parking_lot** | 0.11.2, 0.12.5 | ~1-2 MB |
| **lru** | 0.7.8, 0.16.3 | ~0.5-1 MB |
| **zip** | 2.4.2, 4.6.1 | ~2-3 MB |

### Root Cause Analysis

#### 1. **ahash & hashbrown Duplication**
```
ahash v0.7.8
└── hashbrown v0.12.3
    └── lru v0.7.8
        └── memoize v0.5.1
            └── terraphim_rolegraph

ahash v0.8.12
└── hashbrown v0.14.5
    └── dashmap v6.1.0
        └── opendal v0.54.1
```

**Issue:** `terraphim_rolegraph` uses `memoize` which depends on old `lru` 0.7.8, while the rest of the workspace uses newer hashbrown through opendal.

#### 2. **syn Duplication**
```
syn v1.0.109
├── many proc-macro crates

syn v2.0.114
├── newer proc-macro crates
```

**Issue:** Mix of proc-macro crates using syn v1 and v2. Common in large workspaces during migration periods.

#### 3. **thiserror Duplication**
```
thiserror v1.0.69
└── various crates

thiserror v2.0.17
└── newer crates
```

**Issue:** Similar to syn - workspace transitioning between major versions.

## Consolidation Recommendations

### Immediate Actions (Phase 2)

#### 1. Update `terraphim_rolegraph` Dependencies

**File:** `crates/terraphim_rolegraph/Cargo.toml`

```toml
[dependencies]
# Replace: memoize = "0.5.1"
# With a custom memoization or update to newer version
# Or use cached crate which is already in workspace

cached = { workspace = true }  # Use workspace version
```

**Expected Savings:** 3-5 MB

#### 2. Standardize on syn v2

Update crates using syn v1 to syn v2 where possible:

```bash
# Find crates using syn v1
cargo tree -p syn@1.0.109 --edges normal
```

**Expected Savings:** 8-12 MB

#### 3. Standardize on thiserror v2

```toml
[workspace.dependencies]
thiserror = "2.0"
```

Update all crates to use thiserror v2.

**Expected Savings:** 1-2 MB

### Medium-Term Actions (Phase 3)

#### 4. Consolidate rand and getrandom

```toml
[workspace.dependencies]
rand = "0.9"
getrandom = "0.3"
```

**Expected Savings:** 3-5 MB

#### 5. Consolidate toml

```toml
[workspace.dependencies]
toml = "0.9"
```

**Expected Savings:** 2-3 MB

#### 6. Update indicatif and console

```toml
[workspace.dependencies]
indicatif = "0.18"
console = "0.16"
```

**Expected Savings:** 2-3 MB

### Workspace-Level Dependency Management

#### Update `Cargo.toml` workspace dependencies:

```toml
[workspace.dependencies]
# Core dependencies - standardized versions
ahash = "0.8"
hashbrown = "0.15"
syn = { version = "2.0", features = ["full"] }
thiserror = "2.0"
rand = "0.9"
getrandom = "0.3"
toml = "0.9"
indicatif = "0.18"
console = "0.16"
phf = { version = "0.13", features = ["macros"] }
dirs = "6.0"
quick-xml = "0.38"
parking_lot = "0.12"
lru = "0.16"
zip = "4.6"
```

## Implementation Plan

### Phase 2A: High-Impact Updates (Week 1)

1. **terraphim_rolegraph**: Replace `memoize` with `cached`
2. **syn v2 migration**: Update proc-macro crates
3. **thiserror v2 migration**: Update error handling crates

**Expected Savings:** 12-20 MB

### Phase 2B: Medium-Impact Updates (Week 2)

1. **rand/getrandom**: Update random number generation
2. **toml**: Update configuration parsing
3. **indicatif/console**: Update progress indicators

**Expected Savings:** 7-11 MB

### Phase 2C: Low-Impact Updates (Week 3)

1. **phf**: Update perfect hash functions
2. **dirs**: Update directory handling
3. **quick-xml**: Update XML parsing
4. **parking_lot**: Update synchronization primitives

**Expected Savings:** 5-8 MB

## Total Expected Savings

| Phase | Expected Savings |
|-------|------------------|
| Phase 2A (High Impact) | 12-20 MB |
| Phase 2B (Medium Impact) | 7-11 MB |
| Phase 2C (Low Impact) | 5-8 MB |
| **Total** | **24-39 MB** |

## Additional Benefits

Beyond disk usage reduction:

1. **Faster Compilation:** Fewer duplicate crates to compile
2. **Smaller Binary Sizes:** Less duplicate code in final binaries
3. **Better Security:** Fewer versions to track for vulnerabilities
4. **Simpler Maintenance:** Single version to update
5. **Better Cache Utilization:** More cache hits in CI

## Monitoring

Track progress with:

```bash
# Before changes
cargo tree --duplicates > /tmp/duplicates-before.txt
wc -l /tmp/duplicates-before.txt

# After changes
cargo tree --duplicates > /tmp/duplicates-after.txt
wc -l /tmp/duplicates-after.txt
```

## Risk Assessment

| Action | Risk Level | Mitigation |
|--------|------------|------------|
| memoize → cached | Medium | Test memoization behavior thoroughly |
| syn v2 migration | Low | Compile-time checks catch most issues |
| thiserror v2 | Low | API-compatible upgrade |
| rand v0.9 | Medium | Test random-dependent code |
| toml v0.9 | Low | Configuration parsing tests |

## Notes

- Some duplicates (like `crossbeam-epoch`) are pulled in by dependencies and cannot be directly controlled
- The `terraphim_rolegraph` → `memoize` → `lru` 0.7.8 chain is the highest-impact fix
- Consider using `cargo-deny` to prevent future duplication
