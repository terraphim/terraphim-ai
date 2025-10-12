# Terraphim Browser Extensions - Build Guide

This document provides comprehensive instructions for building and packaging the Terraphim browser extensions for release.

## Overview

The Terraphim project includes two browser extensions:

1. **TerraphimAIParseExtension** - Advanced text processing with WASM-based automata
2. **TerraphimAIContext** - Quick context search and lookup functionality

## Prerequisites

### Required Tools

- **Rust** (latest stable) - For WASM compilation
- **wasm-pack** - For building WebAssembly packages
- **Node.js & npm** - For JavaScript dependencies
- **Python 3** - For JSON validation in build scripts
- **zip** - For creating distribution packages

### Installation Commands

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install system dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install nodejs npm python3 zip

# Verify installations
rustc --version
wasm-pack --version
node --version
python3 --version
zip --version
```

## Security Setup

### Pre-commit Hooks

The build system includes comprehensive security checks to prevent accidental credential commits:

```bash
# Install pre-commit hooks (recommended)
./scripts/install-pre-commit-hook.sh

# Manual security check
./scripts/check-api-keys.sh
```

### Credential Management

**⚠️ IMPORTANT**: Never hardcode API credentials in source files.

- Use the extension options page for credential configuration
- All credentials are stored securely using Chrome's storage.sync API
- Environment templates are provided for development setup

## Build Process

### Quick Start

```bash
# Build both extensions
./scripts/build-browser-extensions.sh

# Package for Chrome Web Store
./scripts/package-browser-extensions.sh
```

### Detailed Steps

#### 1. Environment Setup

```bash
# Copy environment template (optional, for development)
cp .env.template .env
# Edit .env with your development credentials (DO NOT COMMIT)
```

#### 2. Build Extensions

The build script performs the following operations:

- **Security Validation**: Scans for hardcoded credentials
- **WASM Compilation**: Builds the automata WASM module
- **Dependency Updates**: Updates and validates all dependencies
- **Manifest Validation**: Ensures valid extension manifests
- **File Cleanup**: Removes development artifacts

```bash
./scripts/build-browser-extensions.sh
```

#### 3. Package for Distribution

The packaging script creates Chrome Web Store ready packages:

- **Chrome Packages**: `*-chrome.zip` for store submission
- **Source Archives**: `*-source.zip` for review if requested
- **Release Notes**: Comprehensive documentation
- **Validation**: Automatic package integrity checks

```bash
./scripts/package-browser-extensions.sh
```

## Build Output

After successful build and packaging, you'll find:

```
dist/browser-extensions/
├── TerraphimAIParseExtension-v1.0.0-chrome.zip     # Chrome Web Store package
├── TerraphimAIParseExtension-v1.0.0-source.zip     # Source code archive
├── TerraphimAIContext-v0.0.2-chrome.zip            # Chrome Web Store package
├── TerraphimAIContext-v0.0.2-source.zip            # Source code archive
└── RELEASE_NOTES.md                                 # Comprehensive release notes
```

## Manual Build Steps

If you need to build components individually:

### WASM Module (TerraphimAIParseExtension only)

```bash
cd browser_extensions/TerraphimAIParseExtension/wasm

# Clean previous build
rm -rf pkg/

# Build WASM package
wasm-pack build --target web --out-dir pkg

# Verify output
ls -la pkg/
```

### Extension Validation

```bash
# Validate manifest files
python3 -m json.tool browser_extensions/TerraphimAIParseExtension/manifest.json
python3 -m json.tool browser_extensions/TerraphimAIContext/manifest.json

# Security check
./scripts/check-api-keys.sh
```

## Development Workflow

### Local Testing

1. **Build Extensions**: Run `./scripts/build-browser-extensions.sh`
2. **Load in Chrome**:
   - Navigate to `chrome://extensions/`
   - Enable "Developer mode"
   - Click "Load unpacked"
   - Select `browser_extensions/TerraphimAIParseExtension` or `TerraphimAIContext`
3. **Configure Credentials**: Use the extension options page
4. **Test Functionality**: Verify all features work as expected

### Release Preparation

1. **Version Updates**: Update version numbers in `manifest.json` files
2. **Security Review**: Run security checks and verify no hardcoded credentials
3. **Build & Package**: Execute full build and packaging pipeline
4. **Package Validation**: Test the generated `.zip` packages
5. **Documentation Update**: Update release notes and documentation

## Troubleshooting

### Common Issues

#### WASM Build Failures

```bash
# Update Rust toolchain
rustup update

# Add WASM target
rustup target add wasm32-unknown-unknown

# Update wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

#### Dependency Issues

```bash
# Clean and rebuild WASM
cd browser_extensions/TerraphimAIParseExtension/wasm
rm -rf pkg/ Cargo.lock
wasm-pack build --target web --out-dir pkg
```

#### Security Check Failures

```bash
# Run manual credential scan
grep -r -i -E "(api_key|apikey|secret|token).*[=:]\s*['\"][a-zA-Z0-9_-]{10,}['\"]" browser_extensions/

# Fix any hardcoded credentials found
# Use extension options page instead
```

#### Package Validation Errors

```bash
# Check ZIP integrity
zip -T dist/browser-extensions/*.zip

# Validate manifest extraction
unzip -l dist/browser-extensions/TerraphimAIParseExtension-v1.0.0-chrome.zip | grep manifest.json
```

### Debug Mode

Enable verbose output for troubleshooting:

```bash
# Run build with debug output
DEBUG=1 ./scripts/build-browser-extensions.sh

# Check specific component
cd browser_extensions/TerraphimAIParseExtension/wasm
RUST_LOG=debug wasm-pack build --target web --out-dir pkg
```

## Configuration Management

### API Endpoints

Extensions support multiple API configuration modes:

- **Auto-discovery**: Automatically finds local Terraphim servers
- **Development**: Uses localhost endpoints with fallback ports
- **Production**: Uses terraphim.cloud endpoints
- **Custom**: User-specified server URLs

### Credential Storage

- **Chrome Storage Sync**: Encrypted, synchronized across devices
- **No Local Files**: Credentials never stored in local files
- **Options Page**: Secure credential configuration interface

## Performance Optimization

### WASM Optimization

The WASM build is configured for optimal size:

```toml
[profile.release]
opt-level = "s"  # Optimize for size
```

### Package Size Limits

Chrome Web Store limits:
- **Maximum package size**: 50MB
- **Current package sizes**:
  - TerraphimAIParseExtension: ~260KB
  - TerraphimAIContext: ~10KB

## Continuous Integration

The build system is designed for CI/CD integration:

```yaml
# Example GitHub Actions workflow
- name: Build Extensions
  run: ./scripts/build-browser-extensions.sh

- name: Package Extensions
  run: ./scripts/package-browser-extensions.sh

- name: Upload Artifacts
  uses: actions/upload-artifact@v3
  with:
    name: browser-extensions
    path: dist/browser-extensions/
```

## Support

For build issues or questions:

1. Check this documentation first
2. Review error messages and logs
3. Verify all prerequisites are installed
4. Run security checks and validate configuration
5. Create an issue in the main repository with:
   - Build command used
   - Complete error output
   - System information (OS, tool versions)
   - Steps to reproduce

---

**Security Reminder**: Always review the generated packages before distribution and never commit actual API credentials to version control.
