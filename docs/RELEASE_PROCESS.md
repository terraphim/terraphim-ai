# Terraphim AI Release Process

This document describes the complete process for creating and publishing a new release of Terraphim AI.

## Prerequisites

- GitHub CLI (`gh`) installed and authenticated
- Rust toolchain with cargo-deb installed
- Docker and Docker Buildx installed
- Git access to the repository
- Pre-commit hooks installed (`./scripts/install-hooks.sh`)

## Release Process Steps

### 1. Update Version Numbers

1. Update version in `terraphim_server/Cargo.toml`
2. Update version in `crates/terraphim_tui/Cargo.toml`
3. Update any other references to the old version number

### 2. Create Release Tag

```bash
git tag -a v0.2.3 -m "Release v0.2.3"
git push origin v0.2.3
```

### 3. Create GitHub Release

```bash
gh release create v0.2.3 --title "Release v0.2.3" --notes "Release notes here"
```

### 4. Build Debian Packages

```bash
# Temporarily disable panic abort for building
sed -i 's/panic = "abort"/# panic = "abort"/' .cargo/config.toml

# Create LICENSE file for cargo-deb
cp LICENSE-Apache-2.0 LICENSE

# Build binaries
cargo build --release --package terraphim_server
cargo build --release --package terraphim_tui --features repl-full

# Create Debian packages
cargo deb --package terraphim_server
cargo deb --package terraphim_tui

# Restore panic abort
sed -i 's/# panic = "abort"/panic = "abort"/' .cargo/config.toml
```

### 5. Build Arch Linux Packages

```bash
# Create source tarball
git archive --format=tar.gz --prefix=terraphim-server-0.2.3/ v0.2.3 -o terraphim-server-0.2.3.tar.gz

# Create package structure
mkdir -p arch-packages/terraphim-server/usr/bin
mkdir -p arch-packages/terraphim-server/etc/terraphim-ai
mkdir -p arch-packages/terraphim-server/usr/share/doc/terraphim-server
mkdir -p arch-packages/terraphim-server/usr/share/licenses/terraphim-server

# Copy files
cp target/release/terraphim_server arch-packages/terraphim-server/usr/bin/
cp terraphim_server/default/*.json arch-packages/terraphim-server/etc/terraphim-ai/
cp README.md arch-packages/terraphim-server/usr/share/doc/terraphim-server/
cp LICENSE-Apache-2.0 arch-packages/terraphim-server/usr/share/licenses/terraphim-server/

# Create PKGINFO
cat > arch-packages/terraphim-server/.PKGINFO << EOF
pkgname = terraphim-server
pkgbase = terraphim-server
pkgver = 0.2.3-1
pkgdesc = Terraphim AI Server - Privacy-first AI assistant backend
url = https://terraphim.ai
builddate = $(date +%s)
packager = Terraphim Contributors <team@terraphim.ai>
size = 38865120
arch = x86_64
license = Apache-2.0
depend = glibc
depend = openssl
provides = terraphim-server
EOF

# Create package
cd arch-packages
tar -I 'zstd -19' -cf terraphim-server-0.2.3-1-x86_64.pkg.tar.zst terraphim-server/
cd ..
```

### 6. Create Installation Scripts

The installation scripts should already exist in `release/v0.2.3/`:
- `install.sh` - Automated source installation
- `docker-run.sh` - Docker deployment script

### 7. Upload Artifacts

```bash
# Create release directory
mkdir -p release/v0.2.3

# Copy all artifacts
cp target/debian/*.deb release/v0.2.3/
cp arch-packages/*.pkg.tar.zst release/v0.2.3/
cp release/v0.2.3/*.sh release/v0.2.3/
cp release/v0.2.3/*.md release/v0.2.3/

# Upload to GitHub
gh release upload v0.2.3 release/v0.2.3/*.deb release/v0.2.3/*.pkg.tar.zst release/v0.2.3/*.sh release/v0.2.3/*.md
```

### 8. Update Documentation

Update `README.md` with new release information and installation instructions.

### 9. Commit Changes

```bash
# Stage changes (excluding large binary files)
git add README.md release/v0.2.3/*.sh release/v0.2.3/*.md

# Commit with conventional format
git commit -m "docs: update documentation for v0.2.3 release"
```

## Automated Release Workflow (Future)

A GitHub Actions workflow should be created to automate this process:

### Workflow Steps:
1. Trigger on tag push (e.g., `v*.*.*`)
2. Build Debian packages using cargo-deb
3. Build Arch Linux packages
4. Create installation scripts
5. Upload all artifacts to GitHub release
6. Update documentation

### Workflow File: `.github/workflows/release.yml`

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install cargo-deb
        run: cargo install cargo-deb
      - name: Build packages
        run: |
          # Build steps...
      - name: Upload Release Assets
        uses: softprops/action-gh-release@v1
        with:
          files: |
            target/debian/*.deb
            arch-packages/*.pkg.tar.zst
            release/*/install.sh
            release/*/docker-run.sh
            release/*/README.md
```

## Release Checklist

- [ ] Version numbers updated in all Cargo.toml files
- [ ] Release tag created and pushed
- [ ] GitHub release created
- [ ] Debian packages built successfully
- [ ] Arch Linux packages built successfully
- [ ] Installation scripts created/updated
- [ ] README.md updated with new release info
- [ ] All artifacts uploaded to GitHub release
- [ ] Documentation updated
- [ ] Changes committed to repository
- [ ] Release tested on fresh system (optional but recommended)

## Troubleshooting

### Common Issues:

1. **Panic strategy conflicts**: Temporarily disable `panic = "abort"` in `.cargo/config.toml`
2. **Missing LICENSE file**: Copy `LICENSE-Apache-2.0` to `LICENSE` for cargo-deb
3. **Large file errors in pre-commit**: Don't commit binary packages, only infrastructure files
4. **Conventional commit format errors**: Keep commit message simple and follow format

### Dependencies for Future Improvements:

- **RPM packages**: Install `rpmbuild` or use `alien` to convert from .deb
- **Windows installer**: Set up cross-compilation toolchain
- **macOS app bundle**: Set up macOS build environment
- **Multi-arch Docker**: Fix html2md dependency issues

## Post-Release

1. Announce the release on community channels (Discourse, Discord)
2. Update website with new release information
3. Monitor for installation issues and bug reports
4. Plan next release based on user feedback and roadmap