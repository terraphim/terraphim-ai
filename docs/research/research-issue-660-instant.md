# Research Document: Replace instant with web-time

**Status**: Review
**Author**: Terraphim AI
**Date**: 2026-03-11
**Scope**: Issue #660 - Replace unmaintained `instant` crate

---

## Executive Summary

**Finding**: `instant` is NOT used directly in our codebase. It is ONLY a transitive dependency through:
```
instant v0.1.13
├── parking_lot v0.11.2
│   └── sled v0.34.7
│       └── opendal v0.54.1
└── parking_lot_core v0.8.6
    └── parking_lot v0.11.2 (*)
```

All code in the repository uses `std::time::Instant`, not `instant::Instant`. The `instant` crate is brought in transitively by `parking_lot` which is used by `sled` which is used by `opendal`.

---

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Removes RUSTSEC-2024-0384 advisory |
| Leverages strengths? | No | Cannot fix directly - transitive only |
| Meets real need? | Yes | Security debt elimination |

**Proceed**: Partial - Document findings, recommend upstream fix

---

## Problem Statement

### Current State
- `instant` v0.1.13 is in the dependency tree (RUSTSEC-2024-0384: unmaintained)
- Brought in by `parking_lot` → `sled` → `opendal`
- **No direct usage** in any crate code

### Direct Usage Analysis

All `Instant` usages in the codebase use `std::time::Instant`:
```rust
use std::time::Instant;  // ✓ Used everywhere

// NOT used:
use instant::Instant;     // ✗ Not found
```

Sample of verified usages:
- `terraphim_firecracker/src/manager.rs:96` - `std::time::Instant::now()`
- `crates/terraphim_service/src/rate_limiter.rs:32` - `std::time::Instant`
- `terraphim_server/tests/llm_chat_matrix_test.rs:140` - `Instant::now()`

All 200+ usages are standard library, not the `instant` crate.

---

## Dependency Chain

```
opendal v0.54.1
└── sled v0.34.7
    └── parking_lot v0.11.2
        ├── instant v0.1.13  ← unmaintained (RUSTSEC-2024-0384)
        └── parking_lot_core v0.8.6
            └── instant v0.1.13 (*)
```

**Key insight**: `parking_lot` 0.11.2 depends on `instant` for WASM support. Newer versions of `parking_lot` (0.12+) may not have this dependency.

---

## Options for Resolution

| Option | Effort | Impact | Recommendation |
|--------|--------|--------|----------------|
| Upgrade opendal | Medium | High | Best long-term solution |
| [patch] instant → web-time | Low | Low | Workaround, complex |
| Wait for upstream | None | None | Document and monitor |

---

## Recommendation

Since `instant` is NOT used directly:

1. **Document** that `instant` is transitive-only (like `fxhash`)
2. **Close #660** with explanation
3. **Create new issue**: "Upgrade opendal to eliminate transitive instant dependency"
4. **Reference**: This is blocked on opendal upgrade

---

## Appendix: Verification Commands

```bash
# Verify no direct usage of instant crate
grep -r "use instant" --include="*.rs" crates/ || echo "No direct instant usage"
grep -r "instant::" --include="*.rs" crates/ || echo "No instant:: usage"

# Show dependency tree
cargo tree -i instant

# Verify all Instant usages are std::time::Instant
grep -r "use std::time::Instant" --include="*.rs" crates/ | wc -l
# Result: 200+ usages, all std::time::Instant
```

---

## Related Issues

- #659 - fxhash (same situation: transitive only)
- Future issue: Upgrade opendal to remove both instant and fxhash transitive dependencies
