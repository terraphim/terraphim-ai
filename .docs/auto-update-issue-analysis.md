# Auto-Update Issue #462 - Analysis

## Problem

Auto-update fails with 404 when downloading release assets.

## Root Cause

**Asset naming mismatch between CI and updater:**

### What CI Releases (Raw Binaries)
```
terraphim-agent-x86_64-unknown-linux-gnu
terraphim-agent-x86_64-apple-darwin
terraphim-agent-x86_64-pc-windows-msvc.exe
```

### What self_update Crate Expects (Archives)
```
terraphim-agent-1.5.2-x86_64-unknown-linux-gnu.tar.gz
terraphim-agent-1.5.2-x86_64-apple-darwin.tar.gz
terraphim-agent-1.5.2-x86_64-pc-windows-msvc.zip
```

### Differences
1. **Format**: CI releases raw binaries, self_update expects archives (tar.gz/zip)
2. **Version in filename**: CI omits version, self_update includes version
3. **Extension**: CI has no extension (Unix) or .exe (Windows), self_update expects .tar.gz/.zip

## Code Analysis

The updater code (`crates/terraphim_update/src/lib.rs`) already handles name normalization:
- Line 156: `bin_name.replace('_', "-")` - converts underscores to hyphens
- This normalization is applied in `check_update()`, `update()`, and other functions

**The code is correct.** The issue is the release asset format.

## Solutions

### Option 1: Update CI to Create Archives (Recommended)

Modify `.github/workflows/release-comprehensive.yml` to create tar.gz archives:

```yaml
- name: Prepare artifacts (Unix)
  if: matrix.os != 'windows-latest'
  run: |
    mkdir -p artifacts
    VERSION="${{ needs.verify-versions.outputs.version }}"
    
    # Create tar.gz archives instead of raw binaries
    if [ -f "target/${{ matrix.target }}/release/terraphim_server" ]; then
      tar -czf "artifacts/terraphim_server-${VERSION}-${{ matrix.target }}.tar.gz" \
        -C "target/${{ matrix.target }}/release" terraphim_server
    fi
    
    tar -czf "artifacts/terraphim-agent-${VERSION}-${{ matrix.target }}.tar.gz" \
      -C "target/${{ matrix.target }}/release" terraphim-agent
    
    tar -czf "artifacts/terraphim-cli-${VERSION}-${{ matrix.target }}.tar.gz" \
      -C "target/${{ matrix.target }}/release" terraphim-cli
```

### Option 2: Configure self_update for Raw Binaries

The self_update crate may support raw binaries with custom configuration:

```rust
// In the updater configuration
builder.target(target_triple);
// Don't use bin_name pattern, use custom logic
```

**Note**: This requires investigation of self_update crate capabilities.

### Option 3: Custom Download Logic

Implement custom asset download that matches CI naming:

```rust
fn download_raw_binary(&self, release: &Release) -> Result<()> {
    let asset_name = format!("{}-{}", self.bin_name, self.target);
    // Find asset by name pattern
    // Download and install directly
}
```

## Recommendation

**Implement Option 1 (Update CI)** because:
1. Follows Rust ecosystem conventions (tar.gz releases)
2. Enables compression (smaller downloads)
3. Works with self_update crate without code changes
4. Minimal CI changes required

## Immediate Workaround

Users can manually download and install:

```bash
# Download manually from releases page
curl -LO https://github.com/terraphim/terraphim-ai/releases/download/v1.5.2/terraphim-agent-x86_64-unknown-linux-gnu

# Install
chmod +x terraphim-agent-x86_64-unknown-linux-gnu
mv terraphim-agent-x86_64-unknown-linux-gnu ~/.cargo/bin/terraphim-agent
```

## Files to Modify

- `.github/workflows/release-comprehensive.yml` (lines 224-243)
  - Change artifact preparation to create tar.gz archives
  - Include version in archive names

## Verification

After CI changes:
1. Create test release
2. Run `terraphim-agent check-update`
3. Verify asset is found and can be downloaded
4. Test `terraphim-agent update` end-to-end
