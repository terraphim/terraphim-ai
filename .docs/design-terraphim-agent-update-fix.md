# Implementation Plan: Fix terraphim-agent Auto-Update on Apple Silicon

**Status**: Draft
**Research Doc**: `.docs/research-terraphim-agent-update-failure.md`
**Author**: AI Agent (Claude Code)
**Date**: 2026-05-27
**Estimated Effort**: 1 day

## Overview

### Summary
Fix the terraphim-agent auto-update mechanism to work correctly on Apple Silicon (aarch64) Macs by:
1. Correcting version detection to report the binary version instead of the update library version
2. Adding fallback to universal binaries when platform-specific assets are missing
3. Improving CI workflow reliability for macOS asset generation

### Approach
Use a three-pronged approach:
1. **Code fix**: Pass the correct version from the binary to the update library
2. **Resilience fix**: Add universal binary fallback for macOS platforms
3. **CI fix**: Add verification steps to ensure all assets are present before release creation

### Scope

**In Scope:**
- Fix version detection in `terraphim_update` crate
- Add universal binary fallback in asset download logic
- Add CI verification for release assets
- Update all binaries that use `terraphim_update` to pass correct version

**Out of Scope:**
- Rewriting the entire release process (use `cargo-dist`)
- Fixing non-macOS platform issues
- Adding new update features (delta updates, rollback UI)
- Windows-specific improvements

**Avoid At All Cost:**
- Breaking existing x86_64 macOS updates
- Changing asset naming convention (would break older versions)
- Adding complexity without clear benefit
- Rewriting the CI workflow from scratch

## Architecture

### Component Diagram

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  terraphim-agent │────▶│  terraphim_update │────▶│  GitHub Releases │
│  (binary)        │     │  (library)        │     │  (assets)        │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                         │
        │  Pass correct version   │  Try platform-specific asset
        │  (1.17.0)               │  (aarch64-apple-darwin)
        │                         │
        │                         ▼
        │                ┌──────────────────┐
        │                │  Fallback:       │
        │                │  universal-apple-│
        │                │  darwin          │
        │                └──────────────────┘
```

### Data Flow

```
[User: terraphim-agent update]
  │
  ▼
[terraphim_agent::main] passes env!("CARGO_PKG_VERSION") = "1.17.0"
  │
  ▼
[terraphim_update::UpdaterConfig::with_version("1.17.0")]
  │
  ▼
[terraphim_update::check_update]
  ├── Compare: 1.17.0 vs latest (e.g., 1.20.1)
  ├── If update available:
  │     └── [download_release_archive]
  │           ├── Try: terraphim-agent-1.20.1-aarch64-apple-darwin.tar.gz
  │           ├── Try: terraphim-agent-1.20.1-aarch64-apple-darwin.zip
  │           ├── Fallback: terraphim-agent-universal-apple-darwin
  │           └── Fallback: terraphim-agent-1.20.1-universal-apple-darwin.tar.gz
  │
  └── Return UpdateStatus
```

### Key Design Decisions

| Decision | Rationale | Alternatives Rejected |
|----------|-----------|----------------------|
| Pass version from binary instead of library | Binary version is what users care about | Hardcoding version in library |
| Add universal binary fallback | Apple Silicon can run universal binaries via Rosetta 2 | Only fixing CI (doesn't help past releases) |
| Keep existing asset naming | Backward compatibility with older updater versions | New naming convention |
| Minimal CI changes | Focus on verification, not restructuring | Full CI rewrite |

### Eliminated Options (Essentialism)

| Option Rejected | Why Rejected | Risk of Including |
|-----------------|--------------|-------------------|
| Use `cargo-dist` for releases | Too much change for a hotfix; can be future improvement | Scope creep, delay |
| Add delta/patch updates | Not needed for current issue; complex | Over-engineering |
| Create separate updater per binary | Duplicates code unnecessarily | Maintenance burden |
| Change to semantic version API | Would break existing releases | Compatibility issue |

### Simplicity Check

> "What if this could be easy?"

The simplest fix is:
1. Pass `env!("CARGO_PKG_VERSION")` from each binary to `UpdaterConfig::with_version()`
2. Add `universal-apple-darwin` to the target fallback list for macOS
3. Add a CI step that verifies all expected assets exist

This is minimal, focused, and solves the problem without over-engineering.

**Senior Engineer Test**: Would a senior engineer call this overcomplicated? 
No - it's a straightforward bug fix with a resilience improvement.

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
| `.github/scripts/verify-release-assets.sh` | CI script to verify all expected assets exist |

### Modified Files
| File | Changes |
|------|---------|
| `crates/terraphim_update/src/lib.rs` | Fix version detection, add universal binary fallback |
| `crates/terraphim_agent/src/main.rs` | Pass correct version to UpdaterConfig |
| `crates/terraphim_grep/src/main.rs` | Pass correct version to UpdaterConfig |
| `crates/terraphim_server/src/main.rs` | Pass correct version to UpdaterConfig (if applicable) |
| `.github/workflows/release-comprehensive.yml` | Add asset verification step |

### Deleted Files
None

## API Design

### Public Types (No changes to public API)
The existing `UpdaterConfig` API already supports custom versions via `with_version()`.

### Public Functions (Behavior changes)

**Current behavior:**
```rust
let config = UpdaterConfig::new("terraphim_agent");
// config.current_version = "1.5.1" (terraphim_update version)
```

**New behavior:**
```rust
let config = UpdaterConfig::new("terraphim_agent")
    .with_version(env!("CARGO_PKG_VERSION"));
// config.current_version = "1.17.0" (terraphim_agent version)
```

### Error Types (No changes)
Existing error handling is sufficient.

## Test Strategy

### Unit Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_version_from_binary` | `terraphim_update/src/lib.rs` | Verify correct version is used |
| `test_universal_fallback` | `terraphim_update/src/lib.rs` | Verify universal binary is tried |
| `test_macos_targets_include_universal` | `terraphim_update/src/lib.rs` | Verify target list includes universal |

### Integration Tests
| Test | Location | Purpose |
|------|----------|---------|
| `test_update_check_shows_correct_version` | `terraphim_agent/tests/` | Verify check-update shows binary version |
| `test_download_with_universal_fallback` | `terraphim_update/tests/` | Mock GitHub API, test fallback |

### Manual Tests
1. Run `terraphim-agent check-update` and verify it shows binary version
2. On Apple Silicon Mac, verify update mechanism tries correct assets
3. Verify x86_64 Mac updates still work

## Implementation Steps

### Step 1: Fix version detection in terraphim_update
**Files:** `crates/terraphim_update/src/lib.rs`
**Description:** 
- Modify `UpdaterConfig::new()` to accept an optional version parameter or document that `with_version()` should be used
- Add better documentation explaining the version behavior
- Add a warning if the default `cargo_crate_version!()` is used (since it's likely wrong)

**Key code changes:**
```rust
impl UpdaterConfig {
    /// Create a new updater config for Terraphim AI binaries
    /// 
    /// **IMPORTANT**: The default version is the version of the terraphim_update
    /// library, NOT the binary being updated. Always use `.with_version()` to
    /// pass the binary's version:
    /// 
    /// ```rust
    /// let config = UpdaterConfig::new("terraphim-agent")
    ///     .with_version(env!("CARGO_PKG_VERSION"));
    /// ```
    pub fn new(bin_name: impl Into<String>) -> Self {
        Self {
            bin_name: bin_name.into(),
            repo_owner: "terraphim".to_string(),
            repo_name: "terraphim-ai".to_string(),
            current_version: cargo_crate_version!().to_string(),
            show_progress: true,
        }
    }
    // ...
}
```

**Tests:** Add unit test verifying documentation example
**Estimated:** 30 minutes

### Step 2: Add universal binary fallback for macOS
**Files:** `crates/terraphim_update/src/lib.rs`
**Description:**
- Modify `get_target_triples_with_fallback()` to include `universal-apple-darwin` as a fallback for macOS platforms
- Modify `get_asset_names()` to also generate universal binary names

**Key code changes:**
```rust
fn get_target_triples_with_fallback() -> Result<Vec<String>> {
    use std::env::consts::{ARCH, OS};

    let target = format!("{}-{}", ARCH, OS);

    let targets = match target.as_str() {
        "x86_64-macos" => vec![
            "x86_64-apple-darwin".to_string(),
            "universal-apple-darwin".to_string(),
        ],
        "aarch64-macos" => vec![
            "aarch64-apple-darwin".to_string(),
            "universal-apple-darwin".to_string(),
        ],
        // ... other platforms
    };

    Ok(targets)
}

fn get_asset_names(bin_name: &str, target: &str, version: &str) -> Vec<String> {
    let mut assets = Vec::new();
    let version_clean = version.trim_start_matches('v');
    
    // For universal binaries, the naming may differ
    if target == "universal-apple-darwin" {
        // Try versioned archive first
        assets.push(format!("{}-{}-{}.tar.gz", bin_name, version_clean, target));
        // Try raw universal binary
        assets.push(format!("{}-{}", bin_name, target));
    } else {
        // Existing logic for platform-specific assets
        let archive_name = format!("{}-{}-{}.tar.gz", bin_name, version_clean, target);
        assets.push(archive_name);
        
        let raw_name = if cfg!(windows) {
            format!("{}.exe", target)
        } else {
            target.to_string()
        };
        assets.push(raw_name);
    }
    
    // Also try zip for Windows
    if cfg!(windows) {
        let zip_name = format!("{}-{}-{}.zip", bin_name, version_clean, target);
        assets.push(zip_name);
    }

    assets
}
```

**Tests:** Add unit tests for new asset names and target fallbacks
**Estimated:** 1 hour

### Step 3: Update all binaries to pass correct version
**Files:** 
- `crates/terraphim_agent/src/main.rs`
- `crates/terraphim_grep/src/main.rs`
- `crates/terraphim_server/src/main.rs` (if it has update command)

**Description:** Find all places where `UpdaterConfig::new()` is called and add `.with_version(env!("CARGO_PKG_VERSION"))`

**Key code changes (example for terraphim_agent):**
```rust
// In the update command handler
let config = UpdaterConfig::new("terraphim_agent")
    .with_version(env!("CARGO_PKG_VERSION"));
let updater = TerraphimUpdater::new(config);
```

**Tests:** Verify each binary shows correct version in `check-update`
**Estimated:** 30 minutes

### Step 4: Add CI asset verification
**Files:** `.github/workflows/release-comprehensive.yml`
**Description:** Add a job that verifies all expected assets exist before creating the release

**Key additions:**
```yaml
  verify-release-assets:
    name: Verify release assets
    needs: [build-binaries, sign-and-notarize-macos, build-debian-packages]
    runs-on: ubuntu-latest
    if: always()
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          pattern: binaries-*
          path: all-binaries
          merge-multiple: true
        continue-on-error: true

      - name: Verify expected assets exist
        run: |
          # List all downloaded files
          echo "Downloaded artifacts:"
          find all-binaries -type f | sort
          
          # Define expected assets
          EXPECTED_ASSETS=(
            "terraphim-agent-x86_64-apple-darwin"
            "terraphim-agent-aarch64-apple-darwin"
            "terraphim-agent-universal-apple-darwin"
            "terraphim-grep-x86_64-apple-darwin"
            "terraphim-grep-aarch64-apple-darwin"
            "terraphim-grep-universal-apple-darwin"
            # ... add other expected assets
          )
          
          MISSING=0
          for asset in "${EXPECTED_ASSETS[@]}"; do
            if [ ! -f "all-binaries/$asset" ]; then
              echo "❌ Missing asset: $asset"
              MISSING=$((MISSING + 1))
            else
              echo "✅ Found asset: $asset"
            fi
          done
          
          if [ $MISSING -gt 0 ]; then
            echo "❌ $MISSING expected assets are missing!"
            echo "This may indicate a build failure. Check the build-binaries job."
            exit 1
          fi
          
          echo "✅ All expected assets found"
```

**Tests:** Test by running workflow on a test tag
**Estimated:** 1 hour

### Step 5: Add CI script for release asset verification
**Files:** `.github/scripts/verify-release-assets.sh`

**Description:** Create a standalone script that can be run locally or in CI to verify release assets

**Estimated:** 30 minutes

### Step 6: Test the fix
**Description:**
1. Build terraphim-agent locally
2. Run `terraphim-agent check-update` and verify it shows the correct version
3. Check that the update mechanism tries the correct asset URLs

**Estimated:** 30 minutes

## Rollback Plan

If issues discovered:
1. Revert the code changes in `terraphim_update`
2. Revert the binary changes
3. Users can still manually install from source

## Dependencies

### New Dependencies
None

### Dependency Updates
None

## Performance Considerations

### Expected Performance
No performance impact - the changes only affect:
- One additional version comparison (negligible)
- One additional asset URL to try (negligible)

## Open Items

| Item | Status | Owner |
|------|--------|-------|
| Verify all binaries use UpdaterConfig | Pending | Implementation |
| Check if terraphim-cli also needs fix | Pending | Implementation |
| Test on actual Apple Silicon hardware | Pending | QA |

## Approval

- [ ] Technical review complete
- [ ] Test strategy approved
- [ ] Performance targets agreed
- [ ] Human approval received
