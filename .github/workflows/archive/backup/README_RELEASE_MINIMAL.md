# GitHub Actions: Minimal Release Workflow

**Workflow File**: `.github/workflows/release-minimal.yml`

## Purpose

Automatically build and release `terraphim-repl` and `terraphim-cli` binaries when version tags are pushed.

## Trigger

### Automatic (Tag Push)
```bash
git tag -a v1.0.1 -m "Release v1.0.1"
git push origin v1.0.1
```

### Manual (Workflow Dispatch)
1. Go to Actions tab
2. Select "Release Minimal Binaries"
3. Click "Run workflow"
4. Enter version (e.g., "1.0.1")

## What It Does

### Job 1: Build Binaries (build-minimal-binaries)

Builds binaries for **5 platforms** in parallel:

| Platform | Target | Method |
|----------|--------|--------|
| Linux x86_64 | x86_64-unknown-linux-musl | cross (static) |
| Linux ARM64 | aarch64-unknown-linux-musl | cross (static) |
| macOS Intel | x86_64-apple-darwin | native |
| macOS Apple Silicon | aarch64-apple-darwin | native |
| Windows | x86_64-pc-windows-msvc | native |

**Artifacts Created**:
- `terraphim-repl-<target>[.exe]`
- `terraphim-cli-<target>[.exe]`
- `SHA256SUMS` per platform

**Build Time**: ~10-15 minutes (matrix runs in parallel)

### Job 2: Create GitHub Release (create-release)

After all binaries build successfully:

1. Downloads all artifacts
2. Consolidates SHA256 checksums
3. Generates release notes (from `RELEASE_NOTES_v<version>.md` or git commits)
4. Creates GitHub release with:
   - Tag: `v<version>`
   - Title: "Terraphim v<version>"
   - All binaries attached
   - SHA256SUMS.txt for verification

**Permissions**: Requires `contents: write`

### Job 3: Update Homebrew Formulas (update-homebrew-formulas)

After release creation:

1. Downloads Linux x86_64 binaries
2. Calculates SHA256 checksums
3. Updates `homebrew-formulas/terraphim-repl.rb`:
   - Version number
   - Download URL
   - SHA256 checksum
4. Updates `homebrew-formulas/terraphim-cli.rb` similarly
5. Commits changes back to repository

**Result**: Homebrew formulas always have correct checksums!

### Job 4: Publish to crates.io (publish-to-crates-io)

If `CARGO_REGISTRY_TOKEN` secret is set:

1. Checks if already published (avoids errors)
2. Publishes `terraphim-repl` to crates.io
3. Publishes `terraphim-cli` to crates.io
4. Skips if already published

**Optional**: Only runs if token is configured

## Configuration

### Required Secrets

```bash
# Default - automatically available
GITHUB_TOKEN  # For creating releases

# Optional - for crates.io publishing
CARGO_REGISTRY_TOKEN  # Get from 1Password or crates.io
```

### Add CARGO_REGISTRY_TOKEN (Optional)

```bash
# Get token from 1Password
op read "op://TerraphimPlatform/crates.io.token/token"

# Or get from crates.io
# Visit https://crates.io/settings/tokens
# Create new token with "publish-update" scope

# Add to GitHub:
# Settings → Secrets and variables → Actions → New repository secret
# Name: CARGO_REGISTRY_TOKEN
# Value: <paste token>
```

## Usage

### Release v1.0.1 Example

```bash
# 1. Update versions in Cargo.toml files
sed -i 's/version = "1.0.0"/version = "1.0.1"/' crates/terraphim_repl/Cargo.toml
sed -i 's/version = "1.0.0"/version = "1.0.1"/' crates/terraphim_cli/Cargo.toml

# 2. Update CHANGELOGs
# Edit crates/terraphim_repl/CHANGELOG.md
# Edit crates/terraphim_cli/CHANGELOG.md

# 3. Create release notes (optional but recommended)
cat > RELEASE_NOTES_v1.0.1.md <<EOF
# Terraphim v1.0.1

## Changes
- Bug fixes and improvements

## Installation
\`\`\`bash
cargo install terraphim-repl terraphim-cli
\`\`\`
EOF

# 4. Commit changes
git add .
git commit -m "Prepare v1.0.1 release"
git push

# 5. Create and push tag
git tag -a v1.0.1 -m "Release v1.0.1"
git push origin v1.0.1

# 6. Wait for workflow to complete (~15 minutes)
# Check: https://github.com/terraphim/terraphim-ai/actions

# 7. Verify release created
gh release view v1.0.1
```

## Outputs

After successful run:

### GitHub Release
- URL: `https://github.com/terraphim/terraphim-ai/releases/tag/v<version>`
- **10 binaries** attached (2 binaries × 5 platforms)
- **SHA256SUMS.txt** for verification
- Release notes from file or auto-generated

### crates.io (if token set)
- `terraphim-repl` v<version> published
- `terraphim-cli` v<version> published

### Homebrew Formulas
- Updated with correct version and checksums
- Committed back to repository

## Troubleshooting

### Build Fails for Specific Target

Check the build logs for that matrix job. Common issues:
- **musl targets**: May need additional system libraries
- **macOS cross-compile**: Requires macOS runner
- **Windows**: May need Visual Studio components

**Solution**: Mark that target as `continue-on-error: true` in matrix

### Release Already Exists

Error: "Release v1.0.1 already exists"

**Solutions**:
1. Delete existing release: `gh release delete v1.0.1`
2. Use different tag: `v1.0.1-patch`
3. Set `draft: true` in workflow to create draft first

### Homebrew Formula Update Fails

**Cause**: Git push permissions or conflicts

**Solutions**:
1. Ensure `contents: write` permission
2. Check for conflicts in homebrew-formulas/
3. Manual update: Run `scripts/update-homebrew-checksums.sh`

### crates.io Publish Fails

Common errors:
- "crate already exists": Check if already published (handled by workflow)
- "authentication failed": Verify CARGO_REGISTRY_TOKEN secret
- "verification failed": May need `--no-verify` flag (already added)

## Testing the Workflow

### Test with Pre-release Tag

```bash
# Create test release
git tag -a v1.0.1-rc.1 -m "Release candidate 1"
git push origin v1.0.1-rc.1

# Workflow runs...

# Check artifacts
gh release view v1.0.1-rc.1

# Clean up test
gh release delete v1.0.1-rc.1 --yes
git tag -d v1.0.1-rc.1
git push origin :refs/tags/v1.0.1-rc.1
```

### Local Testing (act)

```bash
# Test with nektos/act
act -W .github/workflows/release-minimal.yml -j build-minimal-binaries --matrix target:x86_64-unknown-linux-musl
```

## Maintenance

### Update Build Matrix

To add new platform (e.g., Linux RISC-V):

```yaml
- os: ubuntu-22.04
  target: riscv64gc-unknown-linux-gnu
  use_cross: true
  binary_suffix: ''
```

### Update Formula Logic

Edit the `update-homebrew-formulas` job's sed commands to handle new formula patterns.

## Integration with Existing Workflows

### Relationship to Other Workflows

| Workflow | Purpose | Relationship |
|----------|---------|--------------|
| `release-comprehensive.yml` | Full server/desktop release | Separate - for complete releases |
| `release-minimal.yml` | **This workflow** - REPL/CLI only | New - for minimal toolkit |
| `release.yml` | release-plz automation | Complementary - handles versioning |
| `ci-native.yml` | CI testing | Pre-requisite - must pass before release |

### When to Use Each

- **release-minimal.yml**: For terraphim-repl/cli releases (v1.0.x)
- **release-comprehensive.yml**: For full platform releases (server + desktop)
- **release.yml**: For automated version bumps via release-plz

## Best Practices

### Before Tagging

1. ✅ Run full test suite: `cargo test --workspace`
2. ✅ Run clippy: `cargo clippy --workspace`
3. ✅ Update CHANGELOGs
4. ✅ Create RELEASE_NOTES_v<version>.md
5. ✅ Update Cargo.toml versions
6. ✅ Commit all changes
7. ✅ Create annotated tag with clear message

### After Workflow Completes

1. ✅ Verify binaries in release: `gh release view v<version>`
2. ✅ Test installation: `cargo install terraphim-repl@<version>`
3. ✅ Test binary download works
4. ✅ Verify Homebrew formulas updated correctly
5. ✅ Check crates.io publication

## Example Complete Release Process

```bash
# Step 1: Prepare release
./scripts/prepare-release.sh 1.0.1

# Step 2: Review and commit
git diff
git add .
git commit -m "Prepare v1.0.1 release"
git push

# Step 3: Create and push tag
git tag -a v1.0.1 -m "Release v1.0.1: Bug fixes and improvements"
git push origin v1.0.1

# Step 4: Monitor workflow
gh workflow view "Release Minimal Binaries"
gh run watch

# Step 5: Verify release
gh release view v1.0.1

# Step 6: Test installation
cargo install terraphim-repl@1.0.1 --force
terraphim-repl --version

# Step 7: Announce
# Post to Discord, Twitter, etc.
```

## Monitoring

### Watch Workflow Progress

```bash
# List recent runs
gh run list --workflow=release-minimal.yml

# Watch specific run
gh run watch <run-id>

# View logs
gh run view <run-id> --log
```

### Check Artifacts

```bash
# List release assets
gh release view v1.0.1 --json assets

# Download for testing
gh release download v1.0.1 --pattern '*linux*'
```

## Security

### Secrets Management

- ✅ Use GitHub Secrets for sensitive tokens
- ✅ Use 1Password CLI for local testing
- ✅ Never commit tokens to repository
- ✅ Rotate tokens periodically

### Binary Verification

Users can verify binaries with SHA256SUMS:
```bash
# Download binary and checksum
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.0.1/terraphim-repl-linux-x86_64
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.0.1/SHA256SUMS.txt

# Verify
sha256sum --check SHA256SUMS.txt
```

---

**Workflow Status**: ✅ Created and ready to use!

**Next Release**: Just tag and push - workflow handles the rest!
