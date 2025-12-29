# Release Process Documentation

## Overview

Terraphim AI uses an automated release pipeline that builds, signs, notarizes, and publishes binaries across multiple platforms.

## Release Types

| Tag Format | Trigger | Artifacts |
|------------|---------|-----------|
| `v*` | Main release | All platforms + Homebrew update |
| `terraphim_server-v*` | Server-only | Server binaries only |
| `terraphim-ai-desktop-v*` | Desktop-only | Desktop apps only |
| `terraphim_agent-v*` | TUI-only | TUI binaries only |

## Prerequisites

### Required Credentials (stored in 1Password)

1. **Apple Developer** (`TerraphimPlatform` vault)
   - `apple.developer.certificate` - Developer ID Application certificate
     - Fields: `base64` (certificate), `password` (export password)
   - `apple.developer.credentials` - Apple ID and notarization credentials
     - Fields: `username` (Apple ID), `APPLE_TEAM_ID`, `APPLE_APP_SPECIFIC_PASSWORD`

2. **GitHub** (`TerraphimPlatform` vault)
   - `homebrew-tap-token` - GitHub PAT with `repo` scope
     - Field: `token`

3. **GitHub Secrets**
   - `OP_SERVICE_ACCOUNT_TOKEN` - 1Password service account token
   - `DOCKERHUB_USERNAME` - Docker Hub username (optional)

### Required Infrastructure

- **Self-hosted macOS Runners**:
  - `[self-hosted, macOS, X64]` - Intel Mac for x86_64 builds
  - `[self-hosted, macOS, ARM64]` - M-series Mac for arm64 builds + signing

## Release Steps

### 1. Create Release Tag

```bash
# For a full release
git tag -a v1.2.3 -m "Release v1.2.3: Description"
git push origin v1.2.3

# For a specific component
git tag -a terraphim_server-v1.2.3 -m "Server v1.2.3: Description"
git push origin terraphim_server-v1.2.3
```

### 2. Automated Pipeline Execution

The `release-comprehensive.yml` workflow automatically:

1. **Builds Binaries** (parallel)
   - Linux: x86_64-gnu, x86_64-musl, aarch64-musl, armv7-musl
   - macOS: x86_64 (Intel), aarch64 (Apple Silicon)
   - Windows: x86_64-msvc

2. **Creates Universal macOS Binaries**
   - Combines x86_64 + aarch64 using `lipo`
   - Produces single binary that runs on all Macs

3. **Signs and Notarizes macOS Binaries**
   - Signs with Developer ID Application certificate
   - Adds hardened runtime (`--options runtime`)
   - Submits to Apple for notarization
   - Waits for Apple approval (~2-10 minutes)
   - Verifies with `codesign --verify` and `spctl --assess`

4. **Builds Debian Packages**
   - `terraphim-server_*.deb`
   - `terraphim-agent_*.deb`
   - `terraphim-ai-desktop_*.deb`

5. **Builds Tauri Desktop Apps**
   - macOS: `.dmg` and `.app`
   - Linux: `.AppImage` and `.deb`
   - Windows: `.msi` and `.exe`

6. **Builds Docker Images**
   - Multi-arch: linux/amd64, linux/arm64, linux/arm/v7
   - Ubuntu 20.04 and 22.04 variants
   - Pushed to `ghcr.io/terraphim/terraphim-server`

7. **Creates GitHub Release**
   - Uploads all binaries with checksums
   - Generates release notes with asset descriptions
   - Marks pre-releases (alpha/beta/rc tags)

8. **Updates Homebrew Formulas** (for `v*` tags only)
   - Downloads checksums from release
   - Updates `terraphim/homebrew-terraphim` repository
   - Updates `terraphim-server.rb` and `terraphim-agent.rb`
   - Commits and pushes with automation message

## Workflow Jobs

```
build-binaries (matrix: 8 targets)
  ├── Linux: x86_64-gnu, x86_64-musl, aarch64-musl, armv7-musl
  ├── macOS: x86_64 (Intel runner), aarch64 (ARM runner)
  └── Windows: x86_64-msvc
  ↓
create-universal-macos
  └── Combines macOS x86_64 + aarch64 → universal binary
  ↓
sign-and-notarize-macos
  ├── Import certificate from 1Password
  ├── Sign with codesign --options runtime
  ├── Submit to Apple notarization service
  └── Verify signature and Gatekeeper acceptance
  ↓
create-release
  ├── Download all artifacts
  ├── Generate checksums
  └── Create GitHub Release with signed binaries
  ↓
update-homebrew (only for v* tags)
  ├── Clone terraphim/homebrew-terraphim
  ├── Update formula versions and SHA256 checksums
  └── Push to GitHub with homebrew-tap-token
```

## Manual Testing

### Test Signing Locally

```bash
# Build a binary
cargo build --release --bin terraphim_server

# Test signing script
export RUNNER_TEMP=/tmp/signing-test
./scripts/sign-macos-binary.sh \
  "target/release/terraphim_server" \
  "$(op read 'op://TerraphimPlatform/apple.developer.credentials/username' --no-newline)" \
  "$(op read 'op://TerraphimPlatform/apple.developer.credentials/APPLE_TEAM_ID' --no-newline)" \
  "$(op read 'op://TerraphimPlatform/apple.developer.credentials/APPLE_APP_SPECIFIC_PASSWORD' --no-newline)" \
  "$(op read 'op://TerraphimPlatform/apple.developer.certificate/base64' --no-newline)" \
  "$(op read 'op://TerraphimPlatform/apple.developer.certificate/password' --no-newline)"

# Verify signature
codesign --verify --deep --strict --verbose=2 target/release/terraphim_server
spctl --assess --type execute --verbose target/release/terraphim_server
```

### Test Homebrew Installation

```bash
# Test tap
brew tap terraphim/terraphim

# Test installation
brew install terraphim-server
brew install terraphim-agent

# Verify binaries run
terraphim_server --version
terraphim-agent --version

# Verify signatures (macOS only)
codesign --verify --deep --strict $(which terraphim_server)
spctl --assess --type execute $(which terraphim_server)
```

## Troubleshooting

### Signing Failures

**Issue**: `security: SecKeychainCreate: A keychain with the same name already exists`
- **Solution**: Temporary keychain from previous run wasn't cleaned up
- **Fix**: `security delete-keychain /tmp/signing-test/signing.keychain-db`

**Issue**: `base64: invalid input`
- **Solution**: Base64 certificate in 1Password has newlines
- **Fix**: Regenerate with `base64 certificate.p12 | tr -d '\n'` and update 1Password

**Issue**: Notarization rejected
- **Solution**: Check notarization log
- **Fix**: `xcrun notarytool log <submission-id> --keychain-profile "..."`
- Common issues: Missing `--options runtime`, unsigned dependencies

### Homebrew Update Failures

**Issue**: `homebrew-tap-token not found in 1Password`
- **Solution**: Token not created or wrong vault/name
- **Fix**: Create GitHub PAT with `repo` scope, store in `TerraphimPlatform/homebrew-tap-token`

**Issue**: Formula update fails with authentication error
- **Solution**: GitHub PAT expired or insufficient permissions
- **Fix**: Regenerate PAT with `repo` scope, update in 1Password

### Release Workflow Failures

**Issue**: Workflow doesn't trigger on tag push
- **Solution**: Tag format doesn't match pattern
- **Fix**: Use `v*`, `terraphim_server-v*`, etc. format

**Issue**: Self-hosted runner offline
- **Solution**: macOS runner not available
- **Fix**: Check runner status, restart if needed

## Post-Release Checklist

- [ ] Verify GitHub Release created with all artifacts
- [ ] Check Docker images published to ghcr.io
- [ ] Test Homebrew installation on macOS
- [ ] Verify macOS binaries are signed and notarized
- [ ] Update CHANGELOG.md with release notes
- [ ] Announce release on Discord/Discourse
- [ ] Update documentation if needed

## Rollback

If a release needs to be rolled back:

1. **Delete the tag**:
   ```bash
   git tag -d v1.2.3
   git push origin :refs/tags/v1.2.3
   ```

2. **Delete the GitHub Release** (UI or CLI):
   ```bash
   gh release delete v1.2.3
   ```

3. **Revert Homebrew formulas** (if updated):
   ```bash
   cd ~/terraphim-homebrew-terraphim-checkout
   git revert HEAD
   git push origin main
   ```

## Security Notes

- All credentials stored in 1Password (never in Git)
- Apple Developer ID certificate has 5-year expiration
- GitHub PATs should be rotated annually
- Self-hosted runners must be secured (firewalled, monitored)
- Signed binaries ensure authenticity and prevent tampering

## References

- [Apple Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [GitHub Actions Self-Hosted Runners](https://docs.github.com/en/actions/hosting-your-own-runners)
- [Code Signing Guide](../.docs/guide-apple-developer-setup.md)
