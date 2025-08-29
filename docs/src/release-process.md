# Release Process for Terraphim AI

This document describes the complete release process for Terraphim AI, including all three main components: server, TUI, and desktop application.

## Overview

Terraphim AI uses an automated release process powered by:
- **release-plz**: Automated semantic versioning and changelog generation
- **GitHub Actions**: Multi-platform builds and artifact generation
- **cargo-deb**: Debian package generation
- **Earthly**: Consistent Docker image builds
- **Tauri**: Cross-platform desktop app packaging

## Release Components

### Main Binaries
1. **terraphim_server**: HTTP API server for backend operations
2. **terraphim-tui**: Terminal User Interface with REPL capabilities
3. **terraphim-ai-desktop**: Tauri-based desktop application

### Package Formats
- **Binary releases**: Native binaries for Linux, macOS, and Windows
- **Debian packages**: `.deb` files for Ubuntu/Debian systems
- **Desktop installers**: Platform-specific installers (.dmg, .msi, .AppImage)
- **Docker images**: Multi-architecture container images
- **Homebrew formula**: macOS/Linux package manager integration

## Automated Release Workflow

### 1. Development Phase
- Developers commit using [Conventional Commits](https://www.conventionalcommits.org/)
- release-plz monitors commits and creates/updates release PRs
- PR includes version bumps and changelog updates

### 2. Release Preparation
- Review the release PR created by release-plz
- Verify version bumps are appropriate
- Check changelog entries are accurate
- Test the builds locally if needed

### 3. Release Execution
- Merge the release PR to trigger the release
- release-plz creates git tags (e.g., `terraphim_server-v0.1.0`)
- GitHub Actions workflow triggers automatically
- Multi-platform builds execute in parallel

### 4. Artifact Generation
The release workflow creates:

#### Binary Artifacts
```
terraphim_server-linux-x64
terraphim_server-linux-arm64
terraphim_server-macos-x64
terraphim_server-macos-arm64
terraphim_server-windows.exe

terraphim-tui-linux-x64
terraphim-tui-linux-arm64
terraphim-tui-macos-x64
terraphim-tui-macos-arm64
terraphim-tui-windows.exe
```

#### Debian Packages
```
terraphim-server_0.1.0_amd64.deb
terraphim-server_0.1.0_arm64.deb
terraphim-tui_0.1.0_amd64.deb
terraphim-tui_0.1.0_arm64.deb
terraphim-ai-desktop_0.1.0_amd64.deb
terraphim-ai-desktop_0.1.0_arm64.deb
```

#### Desktop Applications
```
Terraphim-Desktop-0.1.0.dmg          (macOS)
Terraphim-Desktop-0.1.0.AppImage     (Linux)
Terraphim-Desktop-0.1.0.msi          (Windows)
```

#### Docker Images
```
ghcr.io/terraphim/terraphim-server:latest
ghcr.io/terraphim/terraphim-server:v0.1.0
```

### 5. Release Publication
- GitHub release created with all artifacts
- Docker images pushed to GitHub Container Registry
- Checksums generated for integrity verification

## Configuration Files

### Release-plz Configuration (`.release-plz.toml`)
```toml
[workspace]
release_always = false              # Only release via PR merges
dependencies_update = true          # Update dependencies
changelog_update = true             # Generate changelogs
git_release_enable = true           # Create GitHub releases
git_tag_enable = true              # Create git tags
semver_check = true                # Check for breaking changes

# Only release main binaries, not internal crates
[[package]]
name = "terraphim_server"
changelog_path = "./terraphim_server/CHANGELOG.md"

[[package]]
name = "terraphim-ai-desktop"
changelog_path = "./desktop/CHANGELOG.md"

[[package]]
name = "terraphim_tui"
changelog_path = "./crates/terraphim_tui/CHANGELOG.md"
```

### Debian Package Metadata
Each binary includes `[package.metadata.deb]` configuration for:
- Package descriptions and dependencies
- File installation paths
- License and maintainer information

## Manual Release Steps (if needed)

### Testing Debian Packages Locally
```bash
# Install cargo-deb
cargo install cargo-deb

# Build packages
cargo deb -p terraphim_server
cargo deb -p terraphim-ai-desktop  
cargo deb -p terraphim_tui

# Test installation
sudo dpkg -i target/debian/*.deb
```

### Building Multi-Platform Binaries
```bash
# Install cross-compilation tool
cargo install cross

# Build for different targets
cross build --release --target x86_64-unknown-linux-musl
cross build --release --target aarch64-unknown-linux-musl
cross build --release --target armv7-unknown-linux-musleabihf
```

### Docker Image Building
```bash
# Use Earthly for consistent builds
earthly +docker-all
```

## Version Management

### Conventional Commits
- `feat:` → Minor version bump
- `fix:` → Patch version bump  
- `feat!:` or `BREAKING CHANGE:` → Major version bump

### Multi-Package Versioning
- Each component (server, desktop, TUI) has independent versioning
- Tags follow pattern: `<package>-v<version>` (e.g., `terraphim_server-v0.2.0`)
- Internal crates follow workspace versioning

## Installation Methods

### End-User Installation
```bash
# Homebrew (macOS/Linux)
brew install terraphim/terraphim-ai/terraphim-ai

# Debian/Ubuntu
sudo dpkg -i terraphim-server_*.deb

# Docker
docker run ghcr.io/terraphim/terraphim-server:latest

# Direct binary download
wget https://github.com/terraphim/terraphim-ai/releases/latest/download/terraphim_server-linux-x64
chmod +x terraphim_server-linux-x64
./terraphim_server-linux-x64
```

## Troubleshooting

### Common Issues

#### Release PR Not Created
- Check conventional commit format
- Verify `.release-plz.toml` configuration
- Ensure changes affect releasable packages

#### Build Failures
- Check GitHub Actions workflow logs
- Verify dependencies are available for target platforms
- Test builds locally with same environment

#### Debian Package Issues
- Ensure `cargo-deb` metadata is correctly configured
- Verify file paths exist in assets configuration
- Check license file paths

#### Docker Build Issues
- Ensure Earthly configuration is correct
- Check for dependency conflicts in multi-arch builds
- Verify base images support target architectures

### Getting Help
- Check workflow logs in GitHub Actions
- Review release-plz documentation
- Test locally with `cargo build` and `cargo deb`

## Security Considerations

### Secrets Management
Required GitHub secrets:
- `GITHUB_TOKEN`: Automatically provided by GitHub
- `CARGO_REGISTRY_TOKEN`: For publishing to crates.io (if enabled)

### Artifact Integrity
- All releases include `checksums.txt` with SHA256 hashes
- Docker images are signed and pushed to ghcr.io
- Binary releases are created in isolated CI environments

### Dependency Security
- Automated dependency updates via release-plz
- Semver checks prevent accidental breaking changes
- Regular security audits via GitHub Dependabot

## Monitoring and Metrics

### Release Analytics
- Track download counts from GitHub releases
- Monitor Docker image pull statistics
- Review Homebrew installation metrics

### Quality Metrics
- Build success rates across platforms
- Test coverage maintenance
- Time from commit to release availability

This automated release process ensures consistent, reliable distribution of Terraphim AI across all supported platforms while maintaining security and quality standards.