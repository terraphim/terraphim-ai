# Atomic Server Haystack Integration Tests

This document describes the comprehensive Playwright tests for atomic server haystack integration in Terraphim.

## Overview

The atomic server haystack integration tests validate:

1. **Atomic Server Connectivity** - Connection to atomic-server instances
2. **Document Creation** - Creating searchable documents in atomic server
3. **Search Functionality** - Searching through atomic server haystacks
4. **Dual Haystack Integration** - Combined Atomic + Ripgrep haystack searches
5. **Configuration Management** - Dynamic configuration updates
6. **Error Handling** - Graceful degradation when atomic server is unavailable

## Test Files

- `atomic-server-haystack.spec.ts` - Main test suite for atomic server integration
- `run-atomic-haystack-tests.sh` - Setup script with atomic server management

## Prerequisites

### Required Software

1. **atomic-server** - Install with `cargo install atomic-server`
2. **Terraphim server binary** - Build with `cargo build --release`
3. **yarn** - Frontend package manager
4. **Playwright** - Install with `yarn install` then `yarn playwright install`

### Environment Setup

The tests automatically:
- Start an atomic server instance on port 9883
- Create test documents in atomic server
- Start Terraphim server with atomic configuration
- Run comprehensive integration tests
- Clean up all resources after completion

## Running Tests

### Quick Start

```bash
# Run all atomic server haystack tests with setup
yarn test:atomic

# CI mode (headless, verbose reporting)
yarn test:atomic:ci

# Run only the test file (assumes servers already running)
yarn test:atomic:only

# Run both atomic and rolegraph search tests
yarn test:haystack
```

### Manual Setup

If you want to run tests manually:

```bash
# 1. Start atomic server
atomic-server --port 9883 --data-dir /tmp/atomic_test --allow-origin "*"

# 2. Start Terraphim server
cd ../terraphim_server
cargo run --release

# 3. Run tests
yarn playwright test tests/e2e/atomic-server-haystack.spec.ts
```

### CI Integration

For continuous integration:

```bash
CI=true yarn test:atomic:ci
```

This runs tests in:
- Headless mode
- Single worker (no parallelization)
- 2 retries on failure
- Extended timeouts (120s)
- Comprehensive reporting (GitHub, HTML, JSON)

## Test Structure

### Test Suites

1. **Atomic Server Haystack Integration**
   - Server connectivity verification
   - Document search via API
   - UI search functionality

2. **Dual Haystack Integration**
   - Combined Atomic + Ripgrep searches
   - Source differentiation (ATOMIC: vs regular docs)
   - Graceful degradation testing

3. **Configuration Tests**
   - Dynamic configuration updates
   - Authentication validation
   - URL configuration validation

### Test Data

The tests create standardized test documents:

```json
[
  {
    "title": "ATOMIC: Terraphim User Guide",
    "body": "Comprehensive guide for using Terraphim with atomic server integration...",
    "class": "Article"
  },
  {
    "title": "ATOMIC: Search Features", 
    "body": "Advanced search capabilities in Terraphim using atomic server backend...",
    "class": "Article"
  },
  {
    "title": "ATOMIC: Configuration & Roles",
    "body": "Configuration guide for atomic server integration in Terraphim roles...",
    "class": "Article"
  }
]
```

## Configuration Examples

### Atomic-Only Configuration

```json
{
  "id": "Desktop",
  "roles": {
    "Atomic Test": {
      "haystacks": [
        {
          "location": "http://localhost:9883",
          "service": "Atomic",
          "read_only": true,
          "atomic_server_secret": "test_secret_123"
        }
      ]
    }
  }
}
```

### Dual Haystack Configuration

```json
{
  "id": "Desktop", 
  "roles": {
    "Test Engineer": {
      "haystacks": [
        {
          "location": "http://localhost:9883",
          "service": "Atomic",
          "read_only": true,
          "atomic_server_secret": "test_secret_123"
        },
        {
          "location": "./docs/src",
          "service": "Ripgrep",
          "read_only": true,
          "atomic_server_secret": null
        }
      ]
    }
  }
}
```

## Expected Results

### Successful Test Run

```
🎯 Starting Atomic Server Haystack Integration Tests
==================================================
✅ Prerequisites check completed
✅ Atomic server started successfully
✅ Test documents created in atomic server  
✅ Terraphim server started successfully
✅ All atomic server haystack tests passed! 🎉
```

### Test Metrics

- **Test Count**: ~15 comprehensive integration tests
- **Duration**: 2-5 minutes (including setup/teardown)
- **Coverage**: Server connectivity, document CRUD, search functionality, error handling
- **Validation**: API responses, UI behavior, configuration persistence

## Troubleshooting

### Common Issues

1. **Port conflicts**: Ensure ports 8000 and 9883 are available
2. **Binary not found**: Build Terraphim with `cargo build --release`
3. **atomic-server missing**: Install with `cargo install atomic-server`
4. **Timeout issues**: Increase timeout values in CI environments

### Debug Mode

Add environment variables for detailed logging:

```bash
RUST_LOG=debug yarn test:atomic
```

### Test Artifacts

Check `test-results/` directory for:
- HTML test reports
- Screenshots of failures
- Video recordings (CI mode)
- JSON test results

## Integration with Existing Tests

These tests complement the existing rolegraph search validation tests:

- **rolegraph-search-validation.spec.ts** - Tests ripgrep haystack functionality
- **atomic-server-haystack.spec.ts** - Tests atomic server haystack functionality  
- **Combined testing** - Use `yarn test:haystack` to run both test suites

This ensures comprehensive validation of all haystack types and search functionality in Terraphim. 