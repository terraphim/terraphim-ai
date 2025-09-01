# Atomic Server Haystack Integration Tests

This directory contains comprehensive Playwright tests for atomic server haystack integration with CI-friendly features.

## Overview

The atomic server integration tests validate:

- **Atomic Server Connectivity**: Connection, authentication, and credential validation
- **Terraphim Server Integration**: Configuration updates and role management
- **Haystack Search Functionality**: End-to-end search through atomic server backend
- **Dual Haystack Support**: Combined Atomic + Ripgrep haystack operations
- **Error Handling**: Graceful handling of various error conditions
- **CI Compatibility**: Optimized execution for continuous integration environments

## Test Files

### 1. `atomic-server-haystack.spec.ts` (Primary Integration Test)
**Comprehensive test suite covering the complete atomic haystack integration flow:**

- ✅ Atomic server connectivity and credential validation
- ✅ Terraphim server configuration with atomic roles
- ✅ Search functionality with multiple search terms
- ✅ Dual haystack (Atomic + Ripgrep) configuration and testing
- ✅ Error handling and edge cases
- ✅ CI-friendly features and performance validation

### 2. `atomic-connection.spec.ts` (Connection Testing)
**Focused on atomic server connection and basic integration:**

- ✅ Basic atomic server connectivity
- ✅ Environment variable validation
- ✅ Terraphim server startup with atomic configuration
- ✅ Simple search functionality testing

### 3. `atomic-haystack-search-validation.spec.ts` (Search Validation)
**Detailed search functionality validation:**

- ✅ Multiple search term testing
- ✅ Result structure validation
- ✅ Response time monitoring
- ✅ Document ranking verification

## Prerequisites

### 1. Environment Setup

Create `.env` file in project root:
```bash
ATOMIC_SERVER_URL="http://localhost:9883"
ATOMIC_SERVER_SECRET="<base64-encoded-agent-credentials>"
```

### 2. Atomic Server Running

Ensure atomic server is running:
```bash
../atomic-server/target/release/atomic-server --port 9883 --data-dir /path/to/data
```

### 3. Dependencies Installed

```bash
cd desktop
yarn install
```

## Running Tests

### Quick Commands (from desktop directory)

```bash
# Run all atomic haystack tests
yarn run test:atomic

# Run specific test files
yarn run test:atomic:only          # atomic-server-haystack.spec.ts
yarn run test:atomic:connection    # atomic-connection.spec.ts
yarn run test:atomic:search        # atomic-haystack-search-validation.spec.ts

# Run in CI mode
yarn run test:atomic:ci

# Run combined haystack tests
yarn run test:haystack
```

### Advanced Usage

```bash
# Using the test runner script directly
bash scripts/run-atomic-haystack-tests.sh

# CI mode with enhanced reporting
bash scripts/run-atomic-haystack-tests.sh --ci

# Run specific test
bash scripts/run-atomic-haystack-tests.sh --test=atomic-connection.spec.ts

# Show help
bash scripts/run-atomic-haystack-tests.sh --help
```

### Manual Playwright Execution

```bash
# All atomic tests with CI settings
CI=true playwright test tests/e2e/atomic-*.spec.ts --workers=1 --retries=3

# Specific test in debug mode
playwright test tests/e2e/atomic-server-haystack.spec.ts --debug

# Headed mode for visual debugging
playwright test tests/e2e/atomic-server-haystack.spec.ts --headed
```

## CI-Friendly Features

### Optimized for Continuous Integration

- **Headless Execution**: All tests run without UI in CI environments
- **Enhanced Timeouts**: Extended timeouts for CI stability (120s vs 60s)
- **Retry Logic**: Automatic retries on CI (3x vs 1x)
- **Memory-Only Storage**: Terraphim server uses memory storage for faster, isolated tests
- **Comprehensive Reporting**: JSON, HTML, and GitHub-formatted reports
- **Sequential Execution**: Single worker to avoid resource conflicts
- **Graceful Error Handling**: Proper cleanup and informative error messages

### Environment Detection

Tests automatically detect CI environment via `CI=true` and adjust:

```typescript
const isCI = Boolean(process.env.CI);
if (isCI) {
  // Use CI-optimized settings
  actionTimeout: 60000,
  navigationTimeout: 60000,
  retries: 3,
  workers: 1
}
```

### Timeout Configuration

- **CI Environment**: 120s test timeout, 60s action timeout
- **Development**: 60s test timeout, 30s action timeout
- **Network Requests**: 15s timeout with AbortSignal for all fetch operations

## Test Architecture

### Server Management

Each test suite includes a `TerraphimServerManager` class that:

- ✅ Automatically builds Terraphim server if needed (release or debug)
- ✅ Starts server with memory-only storage for isolation
- ✅ Waits for server readiness with health checks
- ✅ Properly shuts down and cleans up after tests

### Configuration Management

Tests create temporary role configurations:

```json
{
  "roles": {
    "Atomic Haystack Tester": {
      "shortname": "AtomicTest",
      "haystacks": [{
        "location": "http://localhost:9883",
        "service": "Atomic",
        "atomic_server_secret": "<secret>"
      }]
    }
  }
}
```

### Search Testing Strategy

1. **Multiple Search Terms**: Tests various keywords (test, article, data, atomic)
2. **Result Validation**: Verifies document structure (id, title, url, rank)
3. **Success Metrics**: Expects at least 1 successful search per test
4. **Performance Monitoring**: Tracks response times and success rates

## Troubleshooting

### Common Issues

**1. "Atomic server not accessible"**
```bash
# Check if atomic server is running
curl http://localhost:9883

# Start atomic server if needed
../atomic-server/target/release/atomic-server --port 9883
```

**2. "ATOMIC_SERVER_SECRET environment variable is required"**
```bash
# Verify .env file exists in project root
cat ../.env

# Should contain:
# ATOMIC_SERVER_SECRET=eyJ...
```

**3. "Terraphim server binary not found"**
```bash
# Build Terraphim server
cd .. && cargo build --release

# Or use debug build
cd .. && cargo build
```

**4. Tests timeout in CI**
```bash
# Increase timeout for CI
CI=true playwright test tests/e2e/atomic-*.spec.ts --timeout=180000
```

### Debug Mode

For detailed debugging:

```bash
# Run with debug logs
DEBUG=pw:test playwright test tests/e2e/atomic-server-haystack.spec.ts

# Interactive debugging
playwright test tests/e2e/atomic-server-haystack.spec.ts --debug

# Save screenshots on failure
playwright test tests/e2e/atomic-server-haystack.spec.ts --screenshot=only-on-failure
```

### Test Reports

After test execution, reports are available:

- **HTML Report**: `desktop/playwright-report/index.html`
- **JSON Results**: `desktop/test-results/results.json`
- **Screenshots**: `desktop/test-results/` (on failures)

## Integration with Frontend

These tests validate the complete flow:

1. **Frontend Configuration**: Terraphim UI configuration updates
2. **Backend Processing**: Terraphim server role and haystack management
3. **Atomic Integration**: Communication with atomic server
4. **Search Results**: End-to-end search functionality
5. **Error Handling**: Graceful degradation and user feedback

## Memory and Performance

### Memory Usage Optimization

- **Memory-Only Storage**: No persistent storage during tests
- **Process Isolation**: Each test suite runs independent server instances
- **Cleanup**: Automatic cleanup of temporary files and processes

### Performance Expectations

- **Server Startup**: ~5-10 seconds
- **Configuration Update**: ~2-3 seconds
- **Search Response**: ~1-5 seconds per query
- **Total Test Duration**: ~30-60 seconds per test file

## Contributing

When adding new atomic server integration tests:

1. **Follow Naming Convention**: `atomic-<feature>.spec.ts`
2. **Use TerraphimServerManager**: For consistent server lifecycle
3. **Add CI-Friendly Features**: Proper timeouts and error handling
4. **Update Package.json**: Add corresponding npm/yarn scripts
5. **Document in README**: Update this documentation

### Test Template

```typescript
import { test, expect } from '@playwright/test';
// ... other imports

test.describe('Your Atomic Feature', () => {
  let terraphimServer: TerraphimServerManager;

  test.beforeAll(async () => {
    // Setup server and configuration
  });

  test.afterAll(async () => {
    // Cleanup
  });

  test('should validate your feature', async () => {
    // Test implementation with CI-friendly timeouts
  });
});
```

## Status

✅ **Production Ready**: All atomic haystack integration tests are comprehensive, stable, and CI-friendly.

The test suite provides complete validation of atomic server integration from frontend configuration through backend search functionality, ensuring robust operation in both development and production environments.
