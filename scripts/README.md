# Terraphim AI Release Scripts

This directory contains comprehensive automation scripts for managing Terraphim AI releases. These scripts streamline the entire release process from version bumping to package creation and GitHub release management.

## 🚀 Quick Start

### Interactive Mode (Recommended)

Run the release manager for an interactive experience:

```bash
./scripts/release-manager.sh
```

This provides a user-friendly menu for all release operations.

### Command Line Mode

For automation and CI/CD pipelines, use the command line interface:

```bash
# Create a new release
./scripts/release-manager.sh release 0.2.5

# Validate an existing release
./scripts/release-manager.sh validate 0.2.4

# Generate changelog
./scripts/release-manager.sh changelog v0.2.3 v0.2.4
```

## 📋 Available Scripts

### 1. `release-manager.sh` - Master Control Script

The main entry point for all release operations. Provides both interactive and command-line interfaces.

**Features:**
- Interactive menu system
- Command-line interface for automation
- Dependency checking
- Configuration guidance
- Status monitoring

**Usage:**
```bash
./scripts/release-manager.sh [COMMAND] [OPTIONS]

Commands:
  release VERSION         Create a new release
  validate VERSION       Validate existing release
  changelog [FROM] [TO]  Generate changelog
  docker                 Build Docker images
  packages VERSION       Build packages only
  tag VERSION            Create git tag
  github-release VERSION Create GitHub release
  cleanup                Clean build artifacts
  status                 Show release status
  config                 Show configuration setup
  help                   Show help
```

### 2. `release.sh` - Core Release Automation

Comprehensive release automation script that handles the complete release process.

**Features:**
- Version bumping across all files
- Package creation (Debian, Arch, RPM, macOS)
- Docker image building
- Git tagging
- GitHub release creation
- Dry-run mode for testing

**Usage:**
```bash
./scripts/release.sh [OPTIONS] <VERSION>

Options:
  -h, --help           Show help message
  -n, --dry-run        Show what would be done without executing
  -s, --skip-tests     Skip running tests
  -b, --skip-build     Skip building packages
  -p, --push           Push changes to remote repository
  -r, --remote REMOTE  Remote repository name (default: origin)
  --no-docker          Skip Docker image creation
  --windows            Include Windows installer (experimental)
  --beta               Mark release as beta/prerelease

Examples:
  ./scripts/release.sh 0.2.5                    # Basic release
  ./scripts/release.sh --dry-run 0.2.5         # Preview changes
  ./scripts/release.sh --push --skip-tests 0.2.5  # Quick release
  ./scripts/release.sh --beta 0.3.0-beta.1      # Beta release
```

### 3. `validate-release.sh` - Release Validation

Validates release artifacts and performs comprehensive testing.

**Features:**
- Local artifact validation
- Remote GitHub release validation
- Package integrity checking
- Installation testing
- Comprehensive test suite
- Validation reporting

**Usage:**
```bash
./scripts/validate-release.sh [OPTIONS] <VERSION>

Options:
  -h, --help           Show help message
  -q, --quick          Quick validation (skip time-consuming checks)
  -l, --local          Validate local build artifacts only
  -r, --remote         Validate remote GitHub release only
  --install-test       Test installation of packages (requires sudo)

Examples:
  ./scripts/validate-release.sh 0.2.5                    # Full validation
  ./scripts/validate-release.sh --quick 0.2.5           # Quick validation
  ./scripts/validate-release.sh --install-test 0.2.5    # With installation test
```

### 4. `changelog.sh` - Changelog Generation

Generates professional changelogs based on git commits and conventional commit format.

**Features:**
- Conventional commit parsing
- Multiple output formats (Markdown, JSON, Plain)
- Automatic tag detection
- Web-friendly formatting
- Category-based organization

**Usage:**
```bash
./scripts/changelog.sh [OPTIONS] [FROM_TAG] [TO_TAG]

Options:
  -h, --help           Show help message
  -o, --output FILE    Output to file instead of stdout
  -f, --format FORMAT  Output format: markdown (default), json, or plain
  -w, --web            Generate web-compatible changelog
  -s, --skip-merge     Skip merge commits
  -a, --all-commits    Include all commits (not just conventional)

Examples:
  ./scripts/changelog.sh                                    # Auto-detect tags
  ./scripts/changelog.sh v0.2.3 v0.2.4                      # Custom range
  ./scripts/changelog.sh --output CHANGELOG.md             # Output to file
  ./scripts/changelog.sh --format json v0.2.3              # JSON format
  ./scripts/changelog.sh --web v0.2.3                      # Web-friendly
```

## 🛠️ Requirements

### Required Tools
- **git**: Version control system
- **cargo**: Rust package manager
- **bash**: Shell environment (Bash 4.0+)

### Optional Tools
- **GitHub CLI (gh)**: For GitHub release operations
- **docker**: For container image building
- **docker buildx**: For multi-architecture builds
- **npm**: For frontend package management
- **cargo-deb**: For Debian package creation
- **alien**: For RPM package conversion (Ubuntu/Debian)

### Installation

```bash
# Install GitHub CLI
# macOS: brew install gh
# Ubuntu: sudo apt install gh
# Other: https://cli.github.com/

# Install cargo-deb
cargo install cargo-deb

# Install alien (Ubuntu/Debian for RPM conversion)
sudo apt-get install alien

# Install Docker
# Follow instructions at https://docs.docker.com/get-docker/
```

## 📦 Package Types Created

### Linux Packages
- **Debian/Ubuntu**: `.deb` packages created with cargo-deb
- **Arch Linux**: `.tar.zst` packages with PKGBUILD metadata
- **RHEL/CentOS/Fedora**: `.rpm` packages converted from Debian packages

### macOS Packages
- **App Bundles**: Native `.app` packages with proper structure
- **Tar Archives**: Compressed `.tar.gz` archives for distribution

### Docker Images
- **Multi-architecture**: AMD64 and ARM64 support
- **Multi-stage**: Optimized builds with minimal runtime images
- **Production ready**: Health checks and proper configuration

## 🔄 Workflow Examples

### Standard Release Process

1. **Preparation**
   ```bash
   # Check current status
   ./scripts/release-manager.sh status

   # Ensure clean working directory
   git status
   ```

2. **Create Release**
   ```bash
   # Interactive release
   ./scripts/release-manager.sh release 0.2.5

   # Or command line
   ./scripts/release.sh --dry-run 0.2.5  # Preview
   ./scripts/release.sh 0.2.5            # Execute
   ```

3. **Validation**
   ```bash
   # Validate created release
   ./scripts/release-manager.sh validate 0.2.5
   ```

### Quick Patch Release

```bash
# Quick release without tests (for emergency patches)
./scripts/release.sh --skip-tests --push 0.2.4.1

# Validate quickly
./scripts/validate-release.sh --quick 0.2.4.1
```

### Beta Release

```bash
# Create beta/prerelease
./scripts/release.sh --beta --push 0.3.0-beta.1
```

### Docker-Only Release

```bash
# Build and push Docker images only
./scripts/release-manager.sh docker
```

## 📁 Directory Structure

```
scripts/
├── README.md                 # This file
├── release-manager.sh       # Master control script
├── release.sh               # Core release automation
├── validate-release.sh      # Release validation
├── changelog.sh             # Changelog generation
└── build-macos-bundles.sh   # macOS package building

release/
├── v0.2.3/                  # Previous releases
├── v0.2.4/                  # Current release
│   ├── install.sh          # Installation script
│   ├── docker-run.sh       # Docker script
│   ├── README.md           # Release notes
│   ├── *.deb               # Debian packages
│   ├── *.pkg.tar.zst       # Arch packages
│   ├── *.rpm               # RPM packages
│   └── *.tar.gz            # macOS packages
└── vVERSION/               # Future releases
```

## ⚙️ Configuration

### Git Configuration

Ensure your git user information is configured:

```bash
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

### GitHub Authentication

For GitHub operations, authenticate with GitHub CLI:

```bash
gh auth login
```

### Environment Variables (Optional)

```bash
export GITHUB_TOKEN="your_github_token"
export DOCKER_REGISTRY="your-registry.com"
export CARGO_TARGET_DIR="custom/target/dir"
```

## 🐛 Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   chmod +x scripts/*.sh
   ```

2. **GitHub CLI Not Authenticated**
   ```bash
   gh auth login
   ```

3. **Docker Permission Issues**
   ```bash
   sudo usermod -aG docker $USER
   # Log out and log back in
   ```

4. **Build Failures**
   ```bash
   # Clean build artifacts
   ./scripts/release-manager.sh cleanup

   # Check dependencies
   ./scripts/release-manager.sh config
   ```

5. **Tag Already Exists**
   ```bash
   # Delete existing tag (only if you're sure)
   git tag -d v0.2.5
   git push origin :refs/tags/v0.2.5
   ```

### Debug Mode

Run scripts with bash debugging:

```bash
bash -x ./scripts/release.sh 0.2.5
```

## 📝 Contributing

When modifying release scripts:

1. Test changes with `--dry-run` flag
2. Validate scripts with `bash -n`
3. Update documentation
4. Test with different scenarios
5. Maintain backward compatibility

## 📄 License

These scripts are part of the Terraphim AI project and follow the same license terms.

## 🆘 Support

For issues with the release scripts:

1. Check this README first
2. Run `./scripts/release-manager.sh config` for setup guidance
3. Check GitHub Issues for known problems
4. Create a new issue with detailed error information

---

*These scripts are designed to make releasing Terraphim AI as painless and reliable as possible. Happy releasing! 🚀*