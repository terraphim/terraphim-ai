# Research Document: terraphim-agent Auto-Update Failure on macOS Apple Silicon

**Status**: Draft
**Author**: AI Agent (Claude Code)
**Date**: 2026-05-27
**Reviewers**: TBD

## Executive Summary

The terraphim-agent auto-update mechanism fails on Apple Silicon (aarch64) Macs because the GitHub release v1.20.1 does not contain the expected `aarch64-apple-darwin` platform assets. The updater correctly identifies the platform but cannot download the missing asset, resulting in a 404 error. Additionally, the version check reports the `terraphim_update` crate version (1.5.1) instead of the actual `terraphim_agent` binary version (1.17.0), causing confusion about what version is installed.

## Essential Questions Check

| Question | Answer | Evidence |
|----------|--------|----------|
| Energizing? | Yes | Auto-update is a critical UX feature for CLI tools |
| Leverages strengths? | Yes | We already have comprehensive CI/CD and release infrastructure |
| Meets real need? | Yes | Users on Apple Silicon cannot update via the built-in mechanism |

**Proceed**: Yes - all 3/3 YES

## Problem Statement

### Description
When running `terraphim-agent update` on an Apple Silicon Mac, the update fails with:
```
[ERROR] Update failed: Failed to download archive: Failed to download any asset. 
Last attempt 'aarch64-apple-darwin'. Error: HTTP request failed: 
https://github.com/terraphim/terraphim-ai/releases/download/v1.20.1/aarch64-apple-darwin: 
status code 404
```

### Impact
- **Who**: All macOS users on Apple Silicon (M1/M2/M3/M4) chips
- **How**: Cannot use the built-in `terraphim-agent update` command
- **Workaround**: Manual installation from source or downloading assets manually

### Success Criteria
1. `terraphim-agent update` succeeds on Apple Silicon Macs
2. `terraphim-agent check-update` reports the correct installed version
3. The update mechanism works across all supported platforms

## Current State Analysis

### Existing Implementation

#### 1. Release Build Process (`.github/workflows/release-comprehensive.yml`)
The CI workflow builds binaries for multiple platforms:

**Build matrix includes:**
- Linux: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `armv7-unknown-linux-musleabihf`
- macOS: `x86_64-apple-darwin` (self-hosted Intel Mac), `aarch64-apple-darwin` (macos-14 GitHub runner)
- Windows: `x86_64-pc-windows-msvc`

**Universal binary creation:**
The `create-universal-macos` job uses `lipo` to combine x86_64 and aarch64 binaries into universal binaries:
- `terraphim-agent-universal-apple-darwin`
- `terraphim_server-universal-apple-darwin`
- `terraphim-grep-universal-apple-darwin`

**Release assets preparation:**
Raw binaries are named: `terraphim-agent-{target}` (e.g., `terraphim-agent-x86_64-apple-darwin`)
Archives are named: `terraphim-agent-{VERSION}-{target}.tar.gz` (e.g., `terraphim-agent-1.20.1-x86_64-apple-darwin.tar.gz`)

#### 2. Update Client (`crates/terraphim_update/src/lib.rs`)

**Platform detection:**
```rust
fn get_target_triples_with_fallback() -> Result<Vec<String>> {
    let target = format!("{}-{}", ARCH, OS);
    let targets = match target.as_str() {
        "aarch64-macos" => vec!["aarch64-apple-darwin".to_string()],
        // ... other platforms
    };
}
```
This correctly maps Apple Silicon to `aarch64-apple-darwin`.

**Asset name generation:**
```rust
fn get_asset_names(bin_name: &str, target: &str, version: &str) -> Vec<String> {
    let version_clean = version.trim_start_matches('v');
    let archive_name = format!("{}-{}-{}.tar.gz", bin_name, version_clean, target);
    assets.push(archive_name);
    
    let raw_name = target.to_string();
    assets.push(raw_name);
}
```

For `terraphim-agent` on `aarch64-apple-darwin`, this generates:
1. `terraphim-agent-1.20.1-aarch64-apple-darwin.tar.gz`
2. `aarch64-apple-darwin` (raw binary fallback)

**Version detection:**
```rust
pub fn new(bin_name: impl Into<String>) -> Self {
    Self {
        current_version: cargo_crate_version!().to_string(),
        // ...
    }
}
```
This uses the version of the `terraphim_update` crate (1.5.1), NOT the version of the binary being updated (terraphim_agent 1.17.0).

#### 3. Actual Release Assets (v1.20.1)

Investigation of the actual GitHub release shows:
- `checksums.txt`
- `terraphim-agent-1.20.1-x86_64-apple-darwin.tar.gz`
- `terraphim-agent-x86_64-apple-darwin`
- `terraphim-cli-1.20.1-x86_64-apple-darwin.tar.gz`
- `terraphim-cli-x86_64-apple-darwin`
- `terraphim-grep-1.20.1-x86_64-apple-darwin.tar.gz`
- `terraphim-grep-x86_64-apple-darwin`
- `terraphim_server-1.20.1-x86_64-apple-darwin.tar.gz`
- `terraphim_server-x86_64-apple-darwin`

**Critical finding**: No `aarch64-apple-darwin` assets exist in the release!
No universal binaries exist either!

### Data Flow

1. User runs `terraphim-agent update`
2. Code calls `TerraphimUpdater::update()` with `bin_name = "terraphim_agent"`
3. Binary name is normalized: `terraphim_agent` -> `terraphim-agent`
4. Platform detected: `aarch64-macos` -> `aarch64-apple-darwin`
5. Asset names generated:
   - `terraphim-agent-1.20.1-aarch64-apple-darwin.tar.gz`
   - `aarch64-apple-darwin`
6. Download attempted: `https://github.com/terraphim/terraphim-ai/releases/download/v1.20.1/aarch64-apple-darwin`
7. **404 NOT FOUND** - asset doesn't exist

### Integration Points

- GitHub Releases API for version checking and asset downloads
- `self_update` crate (0.42) for GitHub backend interaction
- `zipsign-api` for signature verification

## Constraints

### Technical Constraints
- The `self_update` crate 0.42 is used for GitHub backend
- Release assets are created by GitHub Actions workflow
- macOS universal binaries require both x86_64 and aarch64 builds to succeed
- The `cargo_crate_version!()` macro returns the version of the crate where it's invoked

### Business Constraints
- Releases are created automatically via CI on git tag push
- Must maintain backward compatibility with existing asset naming
- Must support both Intel and Apple Silicon Macs

### Non-Functional Requirements
| Requirement | Target | Current |
|-------------|--------|---------|
| Update success rate | >99% | ~0% on Apple Silicon |
| Version accuracy | 100% | Incorrect (reports update lib version) |
| Platform coverage | All supported | Missing aarch64 macOS assets |

## Vital Few (Essentialism)

### Essential Constraints
1. **Apple Silicon assets must exist in releases**: Without them, the update mechanism cannot work
2. **Version must reflect the binary version**: Users need accurate version information
3. **Must not break existing x86_64 support**: Intel Mac users currently can update

### Eliminated from Scope
- Windows update mechanism issues (not reported)
- Linux ARM update issues (not reported)
- Signature verification enhancements (working correctly)
- Automatic rollback features (not related to current failure)
- Homebrew formula updates (separate concern)

## Dependencies

### Internal Dependencies
| Dependency | Impact | Risk |
|------------|--------|------|
| terraphim_agent | Binary being updated | High - must correctly report its version |
| terraphim_update | Update library | High - contains the buggy version detection |
| CI/CD workflows | Release creation | High - must produce all assets |

### External Dependencies
| Dependency | Version | Risk | Alternative |
|------------|---------|------|-------------|
| self_update | 0.42 | Medium | Could use GitHub API directly |
| GitHub Actions | N/A | Medium | Could use alternative CI |
| GitHub Releases | N/A | Low | Primary distribution channel |

## Risks and Unknowns

### Known Risks
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| aarch64 macOS build failing in CI | High | High | Check CI logs, fix build issues |
| Universal binary creation failing | Medium | High | Add CI verification steps |
| Version fix breaking other binaries | Low | Medium | Test all binaries after fix |
| Asset naming inconsistency | Medium | Medium | Standardize naming convention |

### Open Questions
1. Why did the v1.20.1 release not include aarch64 assets? - Check CI logs for the release
2. Are universal binaries being uploaded to releases? - Verify release-comprehensive.yml asset upload
3. Does the version issue affect all binaries or just terraphim-agent? - Affects all using terraphim_update

### Assumptions Explicitly Stated

| Assumption | Basis | Risk if Wrong | Verified? |
|------------|-------|---------------|-----------|
| CI workflow is the only release mechanism | Found release-comprehensive.yml | Low - manual releases are possible but unlikely | Partially |
| aarch64 macOS build should work | It's in the build matrix | High - may have build issues | No |
| Users want auto-update for Apple Silicon | Common platform for developers | Low | Yes |
| `cargo_crate_version!()` is the issue | Code inspection | Low - clear from code | Yes |

### Multiple Interpretations Considered

| Interpretation | Implications | Why Chosen/Rejected |
|----------------|--------------|---------------------|
| Missing assets due to CI failure | Fix CI, re-run release | Most likely - aarch64 build may have failed |
| Missing assets due to workflow bug | Fix workflow asset upload | Possible - need to verify upload logic |
| Missing assets due to manual release | Document proper release process | Unlikely - tags trigger automated workflow |
| Version issue is by design | Keep as-is | Rejected - users need accurate version info |

## Research Findings

### Key Insights

1. **The release workflow BUILDS for aarch64 but the assets don't appear in the release**
   - The `build-binaries` job includes `aarch64-apple-darwin` target
   - The `create-universal-macos` job creates universal binaries
   - But the actual release v1.20.1 only has x86_64 assets
   - **Hypothesis**: The aarch64 build or universal binary creation failed, but the release was still created because `if: always()` conditions

2. **The version detection bug is in the library design**
   - `terraphim_update` is a shared library used by multiple binaries
   - `cargo_crate_version!()` returns the version of the crate where the macro is invoked
   - Since the macro is in `terraphim_update`, it returns 1.5.1 (the library version)
   - It should return the version of the BINARY being updated, not the library

3. **The release asset upload logic may have gaps**
   - The `create-release` job downloads artifacts with pattern `binaries-*`
   - It should include `binaries-universal-apple-darwin` and `binaries-signed-universal-apple-darwin`
   - Need to verify if signed universal binaries are being uploaded

### Relevant Prior Art
- `cargo-dist` handles multi-platform releases with proper asset naming
- Many Rust projects use `self_update` with platform-specific asset names
- Universal binaries on macOS are standard practice (e.g., Homebrew, Rustup)

### Technical Spikes Needed
| Spike | Purpose | Estimated Effort |
|-------|---------|------------------|
| Check CI logs for v1.20.1 release | Determine why aarch64 assets are missing | 30 minutes |
| Verify universal binary upload | Check if universal binaries reach the release | 30 minutes |
| Test version detection fix | Ensure all binaries report correct version | 1 hour |

## Recommendations

### Proceed/No-Proceed
**PROCEED** - This is a critical user-facing bug that affects all Apple Silicon users.

### Scope Recommendations
1. Fix the missing aarch64 macOS assets in releases (CI/workflow fix)
2. Fix the version detection to report binary version, not library version
3. Add fallback to universal binaries when platform-specific assets are missing
4. Add CI verification that all expected assets exist before creating release

### Risk Mitigation Recommendations
1. Add a pre-release asset verification step
2. Test the update mechanism on both Intel and Apple Silicon Macs
3. Consider using `cargo-dist` for more reliable multi-platform releases

## Next Steps

If approved:
1. Investigate CI logs for v1.20.1 to understand why aarch64 assets are missing
2. Design the fix for version detection
3. Design the CI workflow improvements
4. Implement and test

## Appendix

### Reference Materials
- `.github/workflows/release-comprehensive.yml` - Release workflow
- `crates/terraphim_update/src/lib.rs` - Update library
- `crates/terraphim_update/Cargo.toml` - Update library manifest (version 1.5.1)
- `crates/terraphim_agent/Cargo.toml` - Agent manifest (version 1.17.0)

### Code Snippets

Version detection bug:
```rust
// In terraphim_update/src/lib.rs
pub fn new(bin_name: impl Into<String>) -> Self {
    Self {
        bin_name: bin_name.into(),
        repo_owner: "terraphim".to_string(),
        repo_name: "terraphim-ai".to_string(),
        current_version: cargo_crate_version!().to_string(), // BUG: returns 1.5.1 (terraphim_update version)
        show_progress: true,
    }
}
```

Platform detection (correct):
```rust
"aarch64-macos" => vec!["aarch64-apple-darwin".to_string()],
```

Asset naming (correct for aarch64):
```rust
let archive_name = format!("{}-{}-{}.tar.gz", bin_name, version_clean, target);
// Results in: terraphim-agent-1.20.1-aarch64-apple-darwin.tar.gz
```

### Actual vs Expected Release Assets

**Expected (based on workflow):**
- `terraphim-agent-1.20.1-aarch64-apple-darwin.tar.gz`
- `terraphim-agent-aarch64-apple-darwin`
- `terraphim-agent-universal-apple-darwin`
- (and similarly for other binaries)

**Actual (in v1.20.1 release):**
- `terraphim-agent-1.20.1-x86_64-apple-darwin.tar.gz`
- `terraphim-agent-x86_64-apple-darwin`
- (NO aarch64, NO universal binaries)
