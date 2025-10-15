# CI System Maintenance Guidelines

This document provides guidelines for maintaining and updating the Terraphim AI CI system, including both local testing scripts and GitHub Actions workflows.

## Overview

The CI system consists of:
- **8 Local CI Scripts** in `scripts/` directory
- **2 GitHub Actions Workflows** in `.github/workflows/`
- **Pre-commit Hook** system
- **Documentation** and usage guides

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    CI System Architecture                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Developer's Machine                                        │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Git Commit    │───▶│    Pre-commit Hook              │ │
│  └─────────────────┘    │  (ci-quick-check.sh)           │ │
│                         └─────────────────────────────────┘ │
│                                   │                         │
│                                   ▼                         │
│                         ┌─────────────────────────────────┐ │
│                         │   Quick Check Script            │ │
│                         │ (format, clippy, basic build)   │ │
│                         └─────────────────────────────────┘ │
│                                                             │
│  GitHub Actions                                            │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   Push/PR       │───▶│    ci-native.yml                 │ │
│  └─────────────────┘    │  ┌─────────────────────────────┐ │ │
│                         │  │   Core CI Scripts            │ │ │
│                         │  │ (format, frontend, rust,     │ │ │
│                         │  │  tests, desktop)             │ │ │
│                         │  └─────────────────────────────┘ │ │
│                         └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Version Management

### Rust Toolchain
- **Current Version:** 1.87.0
- **Location:** Defined in both `ci-native.yml` and individual scripts
- **Update Process:**
  1. Update version in `.github/workflows/ci-native.yml` (line ~79 and ~184)
  2. Update version in `scripts/ci-check-format.sh` (line ~35)
  3. Update version in `scripts/ci-check-tests.sh` (line ~30)
  4. Update version in `scripts/ci-check-rust.sh` (line ~25)
  5. Test with `./scripts/validate-ci-updates.sh`

### Node.js Version
- **Current Version:** 18
- **Location:** Defined in `frontend-build.yml` and `ci-check-frontend.sh`
- **Update Process:**
  1. Update version in `.github/workflows/frontend-build.yml` (line ~33)
  2. Update version in `scripts/ci-check-frontend.sh` (line ~25)
  3. Test with `./scripts/ci-check-frontend.sh`

## Script Maintenance

### When to Update Scripts

1. **Dependency Updates:** When Rust, Node.js, or system dependencies need updates
2. **New CI Requirements:** When adding new checks or modifying existing ones
3. **Performance Improvements:** When scripts can be optimized
4. **Bug Fixes:** When scripts have issues or edge cases

### Script Update Process

1. **Identify Changes Required**
   ```bash
   # Check current script behavior
   ./scripts/ci-check-format.sh
   ```

2. **Make Changes**
   ```bash
   # Edit script with your preferred editor
   vim scripts/ci-check-format.sh
   ```

3. **Test Changes**
   ```bash
   # Test individual script
   ./scripts/ci-check-format.sh

   # Test integration
   ./scripts/ci-quick-check.sh
   ```

4. **Validate Entire System**
   ```bash
   # Run comprehensive validation
   ./scripts/validate-ci-updates.sh
   ```

5. **Update Documentation**
   - Update `scripts/README-CI-LOCAL.md` if behavior changes
   - Update `scripts/ci-usage-guide.md` if commands change
   - Update this document if architecture changes

### Script Development Guidelines

#### Error Handling
```bash
# Use set -e for strict error handling
set -e

# Provide meaningful error messages
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found${NC}"
    exit 1
fi

# Use consistent color coding
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'
```

#### Progress Reporting
```bash
# Use consistent progress indicators
echo -e "${BLUE}🔍 Running cargo fmt check...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}  ✅ cargo fmt check passed${NC}"
else
    echo -e "${RED}  ❌ cargo fmt check failed${NC}"
    exit 1
fi
```

#### Timing and Performance
```bash
# Track script execution time
START_TIME=$(date +%s)
# ... script logic ...
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
echo "Completed in ${DURATION}s"
```

## GitHub Actions Maintenance

### Workflow Update Process

1. **Syntax Validation**
   ```bash
   # Check workflow syntax
   act -W .github/workflows/ci-native.yml --list
   ```

2. **Local Testing**
   ```bash
   # Test workflow locally (if act is installed)
   act -W .github/workflows/ci-native.yml -j lint-and-format
   ```

3. **Script Integration**
   - Ensure workflows call scripts, not inline commands
   - Maintain 1:1 mapping between jobs and scripts
   - Use consistent environment variables

4. **Validation**
   ```bash
   # Run comprehensive validation
   ./scripts/validate-ci-updates.sh
   ```

### Workflow Best Practices

#### Job Dependencies
```yaml
# Correct dependency chain (matches ci-native.yml)
jobs:
  lint-and-format:
    # No dependencies (can run in parallel)

  build-frontend:
    needs: setup
    # Depends on setup job

  build-rust:
    needs: [setup, build-frontend]
    # Depends on both setup and frontend build
```

#### Script Usage
```yaml
# Use scripts instead of inline commands
- name: Run format and linting checks
  run: ./scripts/ci-check-format.sh
  # ✅ Good: Maintains consistency with local testing

# Avoid inline commands
- name: Run cargo fmt check
  run: cargo fmt --all -- --check
  # ❌ Bad: Creates inconsistency with local testing
```

#### Environment Variables
```yaml
# Use consistent environment variables
env:
  CARGO_TERM_COLOR: always
  # ✅ Good: Consistent across scripts and workflows

# Pass variables to scripts
- name: Build Rust project
  run: |
    export TARGET="${{ matrix.target }}"
    ./scripts/ci-check-rust.sh "$TARGET"
  # ✅ Good: Script respects environment variables
```

## Monthly Maintenance Tasks

### 1. Dependency Updates
```bash
# Check for Rust updates
rustup update

# Check for Node.js updates
nvm ls-remote

# Update system packages
sudo apt-get update && sudo apt-get upgrade
```

### 2. Performance Monitoring
```bash
# Time script execution
time ./scripts/ci-run-all.sh

# Check for slow jobs
./scripts/ci-pr-validation.sh
# Review timings in the generated report
```

### 3. Validation Checks
```bash
# Run comprehensive validation
./scripts/validate-ci-updates.sh

# Test GitHub Actions syntax
act -W .github/workflows/ci-native.yml --list
act -W .github/workflows/frontend-build.yml --list
```

### 4. Documentation Updates
- Review `README-CI-LOCAL.md` for accuracy
- Update `ci-usage-guide.md` with any new commands
- Update this document with any architectural changes

## Troubleshooting

### Common Issues and Solutions

#### Script Permission Issues
```bash
# Symptom: Permission denied
./scripts/ci-check-format.sh
# > bash: ./scripts/ci-check-format.sh: Permission denied

# Solution: Make executable
chmod +x scripts/ci-*.sh
```

#### Rust Version Mismatch
```bash
# Symptom: Version warnings during script execution
./scripts/ci-check-format.sh
# > Warning: Rust version mismatch. Expected: 1.87.0, Got: 1.86.0

# Solution: Update Rust version in all scripts and workflows
```

#### Workflow Syntax Errors
```bash
# Symptom: act reports syntax errors
act -W .github/workflows/ci-native.yml --list
# > Error: ...

# Solution: Validate YAML syntax and fix indentation
```

#### Integration Failures
```bash
# Symptom: Script works locally but fails in CI
# Solution: Check environment differences:
# 1. System dependencies
# 2. Environment variables
# 3. Working directory
# 4. User permissions
```

### Debug Mode

Enable debug output for troubleshooting:

```bash
# Set debug environment variable
export DEBUG=true
./scripts/ci-check-format.sh

# Or run with bash -x
bash -x scripts/ci-check-format.sh
```

## Rollback Procedures

### Emergency Rollback

If CI updates cause issues:

1. **Identify Broken Commit**
   ```bash
   git log --oneline -10
   ```

2. **Revert Changes**
   ```bash
   # Revert specific files
   git checkout HEAD~1 -- scripts/ci-*.sh
   git checkout HEAD~1 -- .github/workflows/*.yml

   # Or revert entire commit
   git revert HEAD
   ```

3. **Validate Rollback**
   ```bash
   ./scripts/validate-ci-updates.sh
   ```

4. **Test and Push**
   ```bash
   ./scripts/ci-quick-check.sh
   git add .
   git commit -m "Rollback CI changes"
   git push
   ```

### Partial Rollback

If only specific scripts need rollback:

```bash
# Revert individual script
git checkout HEAD~1 -- scripts/ci-check-format.sh

# Test specific script
./scripts/ci-check-format.sh

# Commit rollback
git add scripts/ci-check-format.sh
git commit -m "Rollback ci-check-format.sh"
git push
```

## Future Enhancements

### Planned Improvements

1. **Parallel Script Execution**
   - Run independent checks in parallel
   - Reduce total execution time

2. **Caching Improvements**
   - Better dependency caching
   - Incremental build support

3. **Enhanced Reporting**
   - HTML reports for PR validation
   - Performance trending

4. **Cross-Platform Support**
   - Windows support for local scripts
   - macOS improvements

### Contribution Guidelines

When contributing to the CI system:

1. **Follow Existing Patterns**
   - Use consistent naming conventions
   - Maintain color coding and progress reporting
   - Follow error handling patterns

2. **Test Thoroughly**
   - Test individual scripts
   - Test integration with `ci-run-all.sh`
   - Validate with `ci-pr-validation.sh`

3. **Update Documentation**
   - Update relevant sections in this document
   - Update usage guides if commands change
   - Add examples for new features

4. **Validate Changes**
   - Run `./scripts/validate-ci-updates.sh`
   - Test GitHub Actions syntax with `act`
   - Ensure all validations pass

---

**Last Updated:** $(date)
**Maintainer:** Terraphim AI Development Team
**Version:** 1.0.0