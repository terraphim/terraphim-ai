# GitHub Actions Release Workflow Fix Implementation Plan

## Problem Statement
The GitHub Actions release workflows are failing across multiple platforms (Linux, macOS, Windows) preventing successful builds and releases. The issues include:
- 1Password CLI installation failures on Windows
- Svelte/Vite build errors in the Tauri desktop app
- Docker multi-architecture build failures
- Cross-compilation issues for different architectures
- Missing or incorrect configuration for Debian package builds

## Current State Analysis

### Failed Workflows
1. **publish-tauri.yml** (Last run: 19119603861)
   - Windows: 1Password CLI installation fails with "No such file or directory"
   - Ubuntu/macOS: Svelte build fails with CSS identifier error in `node_modules/svelma/src/components/Tooltip.svelte`

2. **release-comprehensive.yml** (Last run: 19078632924)
   - Windows: PowerShell parser error during Rust installation
   - Ubuntu 22.04: yarn cache dependency resolution failure
   - Docker 24.04: Missing `/desktop/yarn.lock` file

### Project Structure
- **Main workspace**: `/Users/alex/projects/terraphim/terraphim-ai/Cargo.toml`
- **Server binary**: `terraphim_server` (path: `terraphim_server/`)
- **TUI binary**: `terraphim_tui` (path: `crates/terraphim_tui/`)
- **Desktop app**: `terraphim-ai-desktop` (path: `desktop/src-tauri/`)
- **Docker builds**: Multi-arch support for Ubuntu 18.04, 20.04, 22.04, 24.04

## Proposed Changes

### 1. Fix 1Password CLI Installation (publish-tauri.yml)
**File**: `.github/workflows/publish-tauri.yml`
```yaml
# Line 30-31: Fix the 1Password CLI action version
- name: Install 1Password CLI
  uses: 1password/install-cli-action@v1.1.0  # Use specific version
```

### 2. Fix Svelte Build Errors
**File**: `desktop/package.json`
- Update or patch the `svelma` dependency causing CSS parsing errors
- Consider using a fork or different UI library if the issue persists
- Add build error handling to continue despite warnings

### 3. Fix Docker Build (Dockerfile.multiarch)
**File**: `docker/Dockerfile.multiarch`
```dockerfile
# Line 14: Fix yarn.lock copy path
COPY desktop/package.json desktop/yarn.lock* ./
# Use optional copy or create empty file if missing
```

### 4. Add Cross-Compilation Configuration
**File**: Create `Cross.toml` in project root
```toml
[build]
default-target = "x86_64-unknown-linux-gnu"

[target.x86_64-unknown-linux-musl]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-musl:latest"

[target.aarch64-unknown-linux-musl]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-musl:latest"

[target.armv7-unknown-linux-musleabihf]
image = "ghcr.io/cross-rs/armv7-unknown-linux-musleabihf:latest"
```

### 5. Fix Windows Build Issues (release-comprehensive.yml)
**File**: `.github/workflows/release-comprehensive.yml`
```yaml
# Lines 83-88: Fix Windows artifact preparation
- name: Prepare artifacts (Windows)
  if: matrix.os == 'windows-latest'
  shell: bash  # Use bash instead of default PowerShell
  run: |
    mkdir -p artifacts
    cp target/${{ matrix.target }}/release/terraphim_server.exe artifacts/terraphim_server-${{ matrix.target }}.exe
    cp target/${{ matrix.target }}/release/terraphim-tui.exe artifacts/terraphim-tui-${{ matrix.target }}.exe
```

### 6. Update GitHub Actions Dependencies
**File**: All workflow files
- Update `actions/checkout@v5` → `actions/checkout@v4` (stable)
- Update `actions/upload-artifact@v5` → `actions/upload-artifact@v4` (stable)
- Update `actions/download-artifact@v4` (already correct)
- Replace deprecated `actions-rs/toolchain@v1` with `dtolnay/rust-toolchain@stable`

### 7. Fix Tauri Build Configuration (tauri-build.yml)
**File**: `.github/workflows/tauri-build.yml`
```yaml
# Line 63: Fix Ubuntu 20.04 webkit package
- platform: ubuntu-20.04
  webkit-package: "libwebkit2gtk-4.0-dev"  # Ensure correct package for Ubuntu 20.04
```

### 8. Add Workflow Testing Script
**File**: Create `scripts/test-workflows.sh`
```bash
#!/bin/bash
# Test workflows locally using act
act -j build-binaries -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest
act -j build-tauri -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:act-latest
```

## Implementation Steps

1. **Create a feature branch**
   ```bash
   git checkout -b fix/github-actions-release-workflows
   ```

2. **Apply all fixes listed above**
   - Update workflow files with correct action versions
   - Fix Docker build configuration
   - Add Cross.toml for cross-compilation
   - Fix Windows-specific issues

3. **Test workflows locally**
   ```bash
   # Install act if not present
   brew install act
   # Run test script
   ./scripts/test-workflows.sh
   ```

4. **Create pull request and test**
   ```bash
   gh pr create --title "Fix GitHub Actions release workflows for all platforms" \
                --body "Fixes build failures across Linux, macOS, Windows, and Docker"
   ```

5. **Monitor workflow runs on PR**
   ```bash
   gh run watch
   ```

## Success Criteria
- ✅ All platforms (Linux, macOS, Windows) build successfully
- ✅ Docker multi-arch images build for all Ubuntu versions
- ✅ Debian packages are created correctly
- ✅ Tauri desktop app builds and packages for all platforms
- ✅ Release artifacts are uploaded to GitHub Releases
- ✅ No workflow failures in the release pipeline

## Monitoring Commands
```bash
# Check workflow status
GH_PAGER=cat gh run list --limit=10

# View specific workflow logs
GH_PAGER=cat gh run view <run-id> --log-failed

# Watch ongoing runs
gh run watch
```

## Risk Mitigation
- Keep the existing workflows as backups (rename with .bak extension)
- Test changes incrementally on feature branch
- Use workflow dispatch for manual testing before merging
- Ensure all secrets are properly configured in repository settings
