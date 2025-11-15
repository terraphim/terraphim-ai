# Local CI Testing Documentation

This guide explains how to run comprehensive CI checks locally before committing changes to the Terraphim AI repository.

## Overview

The local CI testing scripts mirror the GitHub Actions workflows, allowing you to validate changes locally and reduce failed commits. All scripts are located in the `scripts/` directory and can be run from the project root.

## Available Scripts

### Core CI Check Scripts

| Script | Purpose | Mirrors CI Job |
|--------|---------|---------------|
| `ci-check-format.sh` | Code formatting and linting | `lint-and-format` |
| `ci-check-frontend.sh` | Frontend build and test validation | `build-frontend` |
| `ci-check-rust.sh [target]` | Rust build and cross-compilation | `build-rust` |
| `ci-check-tests.sh` | Unit, integration, and documentation tests | `test-suite` |
| `ci-check-desktop.sh` | Desktop application E2E tests | `test-desktop` |

### Integration Scripts

| Script | Purpose | When to Use |
|--------|---------|-------------|
| `ci-run-all.sh` | Run all CI checks in sequence | Pre-merge validation |
| `ci-quick-check.sh` | Fast subset for pre-commit validation | Before each commit |
| `ci-pr-validation.sh [pr-number]` | Full PR validation with detailed reporting | PR preparation |

## Quick Start

### 1. Pre-commit Validation (Fast)

```bash
# Quick checks that run in < 2 minutes
./scripts/ci-quick-check.sh
```

### 2. Full Local Validation

```bash
# Run all CI checks (mirrors GitHub Actions)
./scripts/ci-run-all.sh
```

### 3. PR Validation with Report

```bash
# Generate detailed validation report
./scripts/ci-pr-validation.sh

# With specific PR number (fetches PR info)
./scripts/ci-pr-validation.sh 123
```

## Individual Script Usage

### Format Check

```bash
# Check code formatting and run clippy
./scripts/ci-check-format.sh
```

**What it does:**
- Installs system dependencies (clang, libssl-dev, etc.)
- Sets up Rust toolchain (version 1.87.0)
- Runs `cargo fmt --check`
- Runs `cargo clippy` with all features

**Time:** ~2-3 minutes

### Frontend Check

```bash
# Build and test frontend
./scripts/ci-check-frontend.sh
```

**What it does:**
- Installs Node.js system dependencies (libcairo2-dev, etc.)
- Sets Node.js options for compatibility
- Installs frontend dependencies with yarn
- Runs frontend tests (continues on error)
- Builds frontend application

**Time:** ~5-8 minutes

### Rust Build Check

```bash
# Build for default target
./scripts/ci-check-rust.sh

# Build for specific target
./scripts/ci-check-rust.sh aarch64-unknown-linux-gnu
./scripts/ci-check-rust.sh x86_64-unknown-linux-musl
```

**What it does:**
- Installs system dependencies for cross-compilation
- Sets up cross-compilation toolchain (if needed)
- Builds main binaries (terraphim_server, terraphim_mcp_server, terraphim_agent)
- Tests that binaries can run with `--version`

**Time:** ~8-15 minutes (varies by target)

### Test Suite Check

```bash
# Run all Rust tests
./scripts/ci-check-tests.sh
```

**What it does:**
- Installs system dependencies for testing
- Creates library symlinks for webkit compatibility
- Runs unit tests (`cargo test --workspace --lib`)
- Runs integration tests (`cargo test --workspace --test '*'`)
- Runs documentation tests (`cargo test --workspace --doc`)

**Time:** ~5-10 minutes

### Desktop Test Check

```bash
# Run desktop application tests
./scripts/ci-check-desktop.sh
```

**What it does:**
- Installs system dependencies for desktop testing
- Installs Playwright browsers
- Runs frontend unit tests
- Runs E2E tests with Playwright
- Runs config wizard specific tests

**Time:** ~10-15 minutes

## Environment Variables

All scripts respect the following environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `TARGET` | `x86_64-unknown-linux-gnu` | Rust build target |
| `SKIP_DESKTOP_TESTS` | `false` | Skip desktop tests in run-all.sh |
| `SKIP_BUILD` | `false` | Skip build in quick-check.sh |
| `SKIP_TESTS` | `false` | Skip tests in quick-check.sh |
| `GENERATE_REPORT` | `true` | Generate detailed report in pr-validation.sh |

### Example Usage

```bash
# Skip desktop tests for faster validation
export SKIP_DESKTOP_TESTS=true
./scripts/ci-run-all.sh

# Skip build in quick check
export SKIP_BUILD=true
./scripts/ci-quick-check.sh

# Use specific target
export TARGET=aarch64-unknown-linux-gnu
./scripts/ci-check-rust.sh
```

## Prerequisites

### System Dependencies

All scripts automatically install system dependencies, but you can install them manually:

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev
```

### Required Tools

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Node.js (using nvm recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Yarn
npm install -g yarn

# GitHub CLI (optional, for PR validation)
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt-get update
sudo apt-get install gh
```

## CI Workflow Mapping

The local scripts map 1:1 to GitHub Actions jobs:

| GitHub Actions Job | Local Script | Status |
|-------------------|--------------|--------|
| `lint-and-format` | `ci-check-format.sh` | ✅ Implemented |
| `build-frontend` | `ci-check-frontend.sh` | ✅ Implemented |
| `build-rust` | `ci-check-rust.sh` | ✅ Implemented |
| `test-suite` | `ci-check-tests.sh` | ✅ Implemented |
| `test-desktop` | `ci-check-desktop.sh` | ✅ Implemented |

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   chmod +x scripts/ci-*.sh
   ```

2. **Node.js Version Mismatch**
   ```bash
   # Use nvm to switch to Node.js 18
   nvm use 18
   ```

3. **Rust Version Mismatch**
   ```bash
   # Set specific Rust version
   rustup default 1.87.0
   ```

4. **Missing System Dependencies**
   - Scripts auto-install dependencies, but may require sudo access
   - Run with sudo if you encounter permission issues

5. **Frontend Build Fails**
   ```bash
   # Clean and reinstall
   cd desktop
   rm -rf node_modules dist
   yarn install --legacy-peer-deps
   ```

6. **Playwright Issues**
   ```bash
   # Reinstall Playwright browsers
   cd desktop
   npx playwright install --with-deps
   ```

### Debug Mode

To enable debug output for any script:

```bash
# Set debug environment variable
export DEBUG=true
./scripts/ci-check-format.sh

# Or run with bash -x
bash -x scripts/ci-check-format.sh
```

## Integration with Git Hooks

### Pre-commit Hook (Optional)

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Pre-commit hook for Terraphim AI
echo "Running pre-commit checks..."
./scripts/ci-quick-check.sh
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

### Pre-push Hook (Optional)

Create `.git/hooks/pre-push`:

```bash
#!/bin/bash
# Pre-push hook for Terraphim AI
echo "Running pre-push validation..."
./scripts/ci-run-all.sh
```

Make it executable:

```bash
chmod +x .git/hooks/pre-push
```

## Performance Tips

1. **Parallel Testing**: Run scripts in parallel for different components
2. **Caching**: Scripts use Cargo and Yarn caches automatically
3. **Selective Testing**: Use environment variables to skip unnecessary checks
4. **SSD Storage**: Significantly improves build times

## Continuous Integration

When you push changes, the same checks run in GitHub Actions. By running locally first:

- ✅ Reduce CI queue time
- ✅ Catch issues early
- ✅ Faster development iteration
- ✅ Less context switching between local and CI

## Support

If you encounter issues:

1. Check this documentation first
2. Review script output for specific error messages
3. Ensure all prerequisites are installed
4. Try running individual scripts to isolate issues
5. Check the GitHub Actions workflow logs for comparison

## Contributing

When adding new CI checks:

1. Create corresponding local script
2. Update this documentation
3. Maintain 1:1 mapping with GitHub Actions
4. Test scripts across different environments
