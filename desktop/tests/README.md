# Novel Editor Autocomplete Tests

This directory contains comprehensive Playwright tests for the Novel Editor autocomplete functionality in Terraphim AI.

## Overview

The autocomplete test suite validates the complete Novel Editor autocomplete workflow, including:
- TipTap integration with custom Terraphim extension
- MCP server backend communication
- Keyboard navigation and accessibility
- Performance benchmarks
- Visual regression testing
- Error handling and fallback scenarios

## Test Structure

```
tests/
├── e2e/
│   ├── novel-autocomplete.spec.ts          # Main test specification
│   └── helpers/
│       └── autocomplete-helpers.ts         # Test helper functions
├── fixtures/
│   └── autocomplete-fixtures.ts            # Test data and mock responses
├── global-setup-autocomplete.ts            # Pre-test setup and MCP server startup
├── global-teardown-autocomplete.ts         # Post-test cleanup and artifact collection
├── playwright-autocomplete.config.ts       # Autocomplete-specific Playwright config
└── README.md                              # This file
```

## Prerequisites

### System Requirements
- Node.js (v18 or higher)
- Rust toolchain with Cargo
- Git
- curl (for health checks)

### Project Setup
1. Install frontend dependencies:
   ```bash
   cd desktop
   yarn install
   ```

2. Build MCP server:
   ```bash
   cd crates/terraphim_mcp_server
   cargo build --release
   ```

3. Install Playwright browsers:
   ```bash
   cd desktop
   npx playwright install
   ```

## Running Tests

### Quick Start
```bash
# From the desktop directory
cd desktop

# Run all autocomplete tests
npx playwright test --config=tests/playwright-autocomplete.config.ts

# Run with UI mode for debugging
npx playwright test --config=tests/playwright-autocomplete.config.ts --ui

# Run specific test project (Chrome only)
npx playwright test --config=tests/playwright-autocomplete.config.ts --project=autocomplete-chrome
```

### Test Projects Available

1. **autocomplete-chrome** - Primary testing in Chrome
2. **autocomplete-firefox** - Cross-browser testing in Firefox
3. **autocomplete-safari** - WebKit testing (macOS only)
4. **autocomplete-mobile-chrome** - Mobile touch interaction testing
5. **autocomplete-performance** - Performance benchmarking tests
6. **autocomplete-visual** - Visual regression testing

### Environment Variables

Control test behavior with these environment variables:

```bash
# Server configuration
export MCP_SERVER_PORT=8001              # MCP server port (default: 8001)
export SKIP_MCP_SERVER=true              # Skip MCP server startup

# Test scope
export TEST_SAFARI=true                  # Enable Safari tests in CI
export TEST_MOBILE=true                  # Enable mobile tests
export DEBUG=true                        # Enable debug logging

# CI/CD settings
export CI=true                          # Enable CI-specific timeouts and settings
```

### Advanced Usage

```bash
# Run only performance tests
npx playwright test --config=tests/playwright-autocomplete.config.ts --grep="@performance"

# Run only visual regression tests
npx playwright test --config=tests/playwright-autocomplete.config.ts --grep="@visual"

# Run with custom retry count
npx playwright test --config=tests/playwright-autocomplete.config.ts --retries=5

# Run with specific timeout
npx playwright test --config=tests/playwright-autocomplete.config.ts --timeout=60000
```

## Test Architecture

### Global Setup Process
The `global-setup-autocomplete.ts` handles:
1. Environment validation (Node.js, Cargo, Git, curl)
2. Test directory creation
3. MCP server startup and health verification
4. Functional test of autocomplete endpoint

### Global Teardown Process
The `global-teardown-autocomplete.ts` handles:
1. MCP server shutdown with graceful termination
2. Test artifact collection and archiving
3. Test report generation with pass/fail statistics
4. Temporary file cleanup (CI environments only)

### Test Helpers
The helper functions provide:
- **Editor Interaction**: `waitForEditor`, `typeInEditor`, `clearEditor`
- **Autocomplete Testing**: `triggerAutocomplete`, `waitForSuggestions`, `selectSuggestion`
- **Keyboard Navigation**: `navigateWithArrows`, `selectWithEnter`, `cancelWithEscape`
- **Server Communication**: `testMCPConnection`, `validateSuggestionStructure`
- **Performance Measurement**: `measureResponseTime`, `checkDebounceEffectiveness`

### Test Fixtures
Comprehensive test data including:
- **Expected suggestions** for common queries
- **Mock responses** for offline testing
- **Error scenarios** and fallback behavior
- **Performance benchmarks** and thresholds
- **Visual test scenarios** for regression testing
- **Keyboard navigation** test sequences

## Test Scenarios Covered

### Core Functionality
- Basic autocomplete trigger and display
- Suggestion filtering and ranking
- Text insertion and editor integration
- Debouncing and performance optimization

### User Interactions
- **Keyboard Navigation**: Arrow keys, Tab, Enter, Escape
- **Mouse Interaction**: Click selection, hover effects
- **Touch Interface**: Mobile gesture support (when enabled)

### Backend Integration
- **MCP Server**: Live connection testing and response validation
- **Error Handling**: Network failures, timeout scenarios
- **Fallback Behavior**: Graceful degradation when servers unavailable

### Performance Testing
- Response time measurements (< 500ms target)
- Debounce effectiveness validation
- Memory usage monitoring
- UI responsiveness under load

### Visual Regression
- Dropdown appearance consistency
- Theme compatibility testing
- Mobile responsive layout validation
- High contrast mode support

### Accessibility
- Screen reader compatibility
- Keyboard-only navigation
- Focus management and indicators
- ARIA attributes and roles

## Configuration Details

### Test Timeouts
- **Global timeout**: 10 minutes (local), 15 minutes (CI)
- **Test timeout**: 60 seconds (local), 2 minutes (CI)
- **Action timeout**: 30 seconds (local), 60 seconds (CI)
- **Expect timeout**: 15 seconds (local), 30 seconds (CI)

### Browser Settings
- **Chrome**: Optimized for MCP server CORS, background throttling disabled
- **Firefox**: Notifications and permissions disabled for clean testing
- **Safari**: Standard WebKit configuration, skipped in CI unless requested
- **Mobile**: Touch-enabled testing with Pixel 5 viewport

### Reporter Configuration
- **Local**: HTML reports with failure screenshots, list reporter with steps
- **CI**: GitHub reporter, HTML, JSON, and JUnit XML output formats

## Debugging Test Failures

### Common Issues and Solutions

1. **MCP Server Connection Failed**
   ```bash
   # Check if MCP server is running
   curl http://localhost:8001/message?sessionId=test

   # Check server logs
   cd crates/terraphim_mcp_server
   RUST_LOG=debug cargo run -- --sse --bind 127.0.0.1:8001
   ```

2. **Frontend Not Loading**
   ```bash
   # Start development server manually
   cd desktop
   yarn run dev

   # Check port availability
   lsof -i :5173
   ```

3. **Test Timeout Issues**
   ```bash
   # Run with extended timeout
   npx playwright test --timeout=120000

   # Enable debug logging
   DEBUG=pw:api npx playwright test
   ```

4. **Browser Installation Issues**
   ```bash
   # Reinstall Playwright browsers
   npx playwright install --force

   # Install system dependencies
   npx playwright install-deps
   ```

### Debug Mode
Enable detailed logging and debugging:

```bash
# Enable Playwright debug logging
export DEBUG=pw:*

# Enable browser debugging
npx playwright test --headed --debug

# Record test execution
npx playwright test --headed --record-video=on
```

### Test Report Analysis

After test execution, reports are available in:
- `test-results/autocomplete-report/` - HTML report with screenshots
- `test-results/autocomplete-results.json` - Machine-readable test results
- `test-results/autocomplete-junit.xml` - CI/CD compatible results
- `test-results/autocomplete-summary.json` - Quick test summary

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Novel Editor Autocomplete Tests

on: [push, pull_request]

jobs:
  autocomplete-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install dependencies
        run: |
          cd desktop
          yarn install
          npx playwright install --with-deps

      - name: Build MCP server
        run: |
          cd crates/terraphim_mcp_server
          cargo build --release

      - name: Run autocomplete tests
        run: |
          cd desktop
          npx playwright test --config=tests/playwright-autocomplete.config.ts
        env:
          CI: true
          MCP_SERVER_PORT: 8001

      - uses: actions/upload-artifact@v3
        if: failure()
        with:
          name: playwright-report
          path: desktop/test-results/
```

### Jenkins Pipeline Example
```groovy
pipeline {
    agent any

    environment {
        CI = 'true'
        MCP_SERVER_PORT = '8001'
    }

    stages {
        stage('Setup') {
            steps {
                sh 'cd desktop && yarn install'
                sh 'npx playwright install --with-deps'
                sh 'cd crates/terraphim_mcp_server && cargo build --release'
            }
        }

        stage('Test') {
            steps {
                sh 'cd desktop && npx playwright test --config=tests/playwright-autocomplete.config.ts'
            }
            post {
                always {
                    publishHTML([
                        allowMissing: false,
                        alwaysLinkToLastBuild: true,
                        keepAll: true,
                        reportDir: 'desktop/test-results/autocomplete-report',
                        reportFiles: 'index.html',
                        reportName: 'Playwright Report'
                    ])
                }
            }
        }
    }
}
```

## Performance Benchmarks

### Response Time Targets
- **Excellent**: < 100ms
- **Good**: < 300ms
- **Acceptable**: < 500ms
- **Poor**: > 1000ms

### Debounce Settings
- **Minimum delay**: 200ms
- **Optimal delay**: 300ms
- **Maximum delay**: 600ms

### Suggestion Counts
- **Minimal**: 3 suggestions
- **Standard**: 8 suggestions
- **Maximum**: 12 suggestions

## Contributing

### Adding New Tests
1. Add test scenarios to `fixtures/autocomplete-fixtures.ts`
2. Implement test logic in `e2e/novel-autocomplete.spec.ts`
3. Update helper functions in `helpers/autocomplete-helpers.ts` if needed
4. Document any new configuration requirements

### Test Data Management
- Keep expected suggestions synchronized with actual MCP server responses
- Update mock responses when backend API changes
- Maintain visual regression baselines for UI changes

### Performance Testing
- Add new benchmarks to `PERFORMANCE_BENCHMARKS` in fixtures
- Use `@performance` tag for performance-specific tests
- Monitor and update thresholds based on baseline measurements

## Troubleshooting Guide

### Port Conflicts
If default port 8001 is occupied:
```bash
export MCP_SERVER_PORT=8002
npx playwright test --config=tests/playwright-autocomplete.config.ts
```

### Memory Issues
For large test suites on limited memory systems:
```bash
# Reduce parallel workers
npx playwright test --workers=1

# Skip memory-intensive projects
export TEST_MOBILE=false
export TEST_SAFARI=false
```

### Network Issues
For environments with limited network access:
```bash
# Skip MCP server startup
export SKIP_MCP_SERVER=true

# Use shorter timeouts
npx playwright test --timeout=30000
```

## Support

For issues with the autocomplete test suite:
1. Check the troubleshooting guide above
2. Review test output in `test-results/` directory
3. Enable debug logging with `DEBUG=true`
4. Consult the main Terraphim AI documentation
5. File issues in the project repository with test logs attached

## Related Documentation

- [Novel Editor Integration Guide](../AUTOCOMPLETE_DEMO.md)
- [MCP Server Documentation](../../crates/terraphim_mcp_server/README.md)
- [Testing Scripts README](../TESTING_SCRIPTS_README.md)
- [Playwright Configuration Reference](https://playwright.dev/docs/test-configuration)
