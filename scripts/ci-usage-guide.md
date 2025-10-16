# CI Scripts Usage Guide

## Quick Reference

### Before Committing
```bash
./scripts/ci-quick-check.sh
```

### Before Pushing
```bash
./scripts/ci-run-all.sh
```

### Before Creating PR
```bash
./scripts/ci-pr-validation.sh
```

### Individual Checks
```bash
./scripts/ci-check-format.sh      # Code formatting
./scripts/ci-check-frontend.sh    # Frontend build
./scripts/ci-check-rust.sh        # Rust build
./scripts/ci-check-tests.sh       # Test suite
./scripts/ci-check-desktop.sh     # Desktop tests
```

## Environment Options

```bash
# Skip desktop tests (faster)
export SKIP_DESKTOP_TESTS=true
./scripts/ci-run-all.sh

# Use specific Rust target
export TARGET=aarch64-unknown-linux-gnu
./scripts/ci-check-rust.sh

# Skip build in quick check
export SKIP_BUILD=true
./scripts/ci-quick-check.sh
```

## Troubleshooting

```bash
# Fix formatting
cargo fmt

# Fix clippy warnings
cargo clippy --fix --allow-dirty --allow-staged

# Clean frontend build
cd desktop && rm -rf node_modules dist && yarn install --legacy-peer-deps

# Reinstall Playwright
cd desktop && npx playwright install --with-deps
```

**Full documentation:** See `scripts/README-CI-LOCAL.md`