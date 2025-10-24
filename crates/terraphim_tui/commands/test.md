---
name: test
description: Run test suites with various test runners
usage: "test [path] [--type <type>] [--coverage] [--watch]"
category: Development
version: "1.5.0"
risk_level: Low
execution_mode: Local
permissions:
  - read
  - execute
aliases:
  - run-tests
parameters:
  - name: path
    type: string
    required: false
    default_value: "."
    description: Path to test directory or specific test file
  - name: type
    type: string
    required: false
    allowed_values: ["unit", "integration", "e2e", "all"]
    default_value: "all"
    description: Type of tests to run
  - name: coverage
    type: boolean
    required: false
    default_value: false
    description: Generate code coverage report
  - name: watch
    type: boolean
    required: false
    default_value: false
    description: Watch for file changes and re-run tests
resource_limits:
  max_memory_mb: 512
  max_cpu_time: 300
timeout: 600
---

# Test Command

Run various types of tests with automatic test runner detection and comprehensive reporting.

## Test Types

### Unit Tests
- Fast individual function/component tests
- Mock external dependencies
- High coverage of code paths

### Integration Tests
- Component interaction tests
- Database and API integration
- Service layer validation

### End-to-End Tests
- Full application workflow tests
- Browser automation
- User scenario validation

## Examples

```bash
# Run all tests
test

# Run unit tests only
test --type unit

# Run tests with coverage
test --coverage

# Watch mode for development
test --watch

# Test specific directory
test src/services
```

## Test Runners

### Rust (cargo test)
- Automatic cargo detection
- Feature flag support
- Integration test discovery

### Node.js (npm, yarn)
- Package manager detection
- Test framework support (Jest, Mocha, etc.)
- Coverage reporting

### Python (pytest, unittest)
- Virtual environment support
- Coverage tools integration
- Test discovery

## Coverage Reports

- **HTML Reports**: Interactive coverage visualization
- **JSON Output**: Machine-readable coverage data
- **Thresholds**: Minimum coverage requirements
- **Exclusions**: Configurable exclusion patterns

## Watch Mode

Automatically re-runs tests when files change:
- Intelligent file watching
- Fast test re-execution
- Real-time feedback
- Development workflow optimization

## Configuration

Test behavior can be customized through:
- `test.config.json` - Global test settings
- `.testrc` - Project-specific configuration
- Environment variables - Runtime settings
- Command-line arguments - Per-execution overrides