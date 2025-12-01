# Atomic Client Feature Flag Documentation

## Overview

The Terraphim AI project uses conditional compilation to control the inclusion of atomic client functionality through feature flags.

## Feature Flag Details

### Primary Feature Flag
- **Name**: `terraphim_atomic_client`
- **Purpose**: Controls compilation of atomic client and atomic haystack functionality
- **Default**: Disabled (not included in default features)

### Files Affected

The following files have been updated to use the correct feature flag:

#### Core Middleware
- `crates/terraphim_middleware/src/lib.rs`
- `crates/terraphim_middleware/src/haystack/mod.rs`
- `crates/terraphim_middleware/src/indexer/mod.rs`

#### Desktop Application
- `desktop/src-tauri/src/cmd.rs`

## Usage

### To Enable Atomic Client
```bash
cargo build --features terraphim_atomic_client
```

### To Disable Atomic Client (Default)
```bash
cargo build
# or
cargo build --no-default-features --features "default"
```

## Implementation Details

### Conditional Compilation
All atomic client code is now properly gated behind `#[cfg(feature = "terraphim_atomic_client")]`:

```rust
#[cfg(feature = "terraphim_atomic_client")]
pub use haystack::AtomicHaystackIndexer;

#[cfg(feature = "terraphim_atomic_client")]
let atomic = AtomicHaystackIndexer::default();
```

### Runtime Behavior
When the feature is not enabled:
- Atomic haystack types are excluded from compilation
- Runtime warnings are logged when atomic haystacks are encountered
- Graceful fallback to other haystack types

## Migration Notes

This change standardizes the feature flag naming convention across the codebase and ensures that atomic client functionality is properly optional and can be enabled on-demand by users who need it.

## Testing

The changes have been validated with:
- `cargo build --workspace --lib` - ✅ Pass
- `cargo clippy --workspace` - ✅ Pass
- Pre-commit checks - ✅ Pass

## Backward Compatibility

This change is fully backward compatible:
- Existing builds without the feature continue to work
- No breaking changes to public APIs
- Feature can be enabled/disabled without affecting core functionality
