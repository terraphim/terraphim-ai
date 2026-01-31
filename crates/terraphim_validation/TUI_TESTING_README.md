# Terraphim AI TUI Testing Framework

A comprehensive testing suite for terraphim-ai TUI interface validation and release testing.

## Overview

This framework provides automated testing capabilities for the Terraphim AI Text User Interface (TUI), ensuring quality and reliability across different platforms and usage scenarios.

## Components

### 1. TUI Test Harness (`harness.rs`)
- **Main orchestration engine** for running comprehensive TUI tests
- **Command execution simulation** with timeout handling
- **Result aggregation and reporting**
- **Test suite configuration** and management

### 2. Mock Terminal (`mock_terminal.rs`)
- **Terminal emulation** for testing without real TUI dependencies
- **ANSI escape sequence parsing** and rendering
- **Cursor positioning and screen management**
- **Cross-platform terminal behavior simulation**

### 3. Command Simulator (`command_simulator.rs`)
- **Interactive command execution** simulation
- **Output capture and validation**
- **Command history management**
- **Auto-completion testing**

### 4. Output Validator (`output_validator.rs`)
- **Pattern-based output validation**
- **ANSI sequence validation**
- **Table format verification**
- **Error detection and reporting**

### 5. Performance Monitor (`performance_monitor.rs`)
- **Startup time measurement**
- **Command execution timing**
- **Memory usage monitoring**
- **SLO (Service Level Objective) validation**

### 6. Cross-Platform Tester (`cross_platform.rs`)
- **Platform capability detection**
- **Terminal feature validation**
- **Compatibility issue identification**
- **Multi-platform test execution**

### 7. Integration Tester (`integration.rs`)
- **High-level test orchestration**
- **Comprehensive test suite execution**
- **Result analysis and reporting**
- **CI/CD integration support**

## Features

### Command Interface Testing
- ✅ Search commands (`/search`, `/find`)
- ✅ Configuration management (`/config`)
- ✅ Role management (`/role`)
- ✅ Knowledge graph operations (`/graph`, `/replace`, `/thesaurus`)
- ✅ Utility commands (`/help`, `/clear`, `/quit`)

### REPL Functionality Testing
- ✅ Interactive mode validation
- ✅ Multi-line input handling
- ✅ Command history navigation
- ✅ Auto-completion verification

### Cross-Platform Compatibility
- ✅ ANSI color support detection
- ✅ Unicode character handling
- ✅ Terminal capability validation
- ✅ Platform-specific testing (Linux, macOS, Windows)

### Performance Validation
- ✅ Startup time benchmarking
- ✅ Command execution timing
- ✅ Memory usage monitoring
- ✅ Stress testing capabilities

## Usage

### Command Line Interface

```bash
# Run comprehensive integration tests
cargo run --bin terraphim-tui-tester test --performance --cross-platform

# Run smoke tests only
cargo run --bin terraphim-tui-tester smoke

# Run with custom configuration
cargo run --bin terraphim-tui-tester test \
  --performance \
  --cross-platform \
  --stress-commands 200 \
  --stress-concurrency 20 \
  --timeout 45 \
  --width 140 \
  --height 40 \
  --output results.txt
```

### Programmatic Usage

```rust
use terraphim_validation::testing::tui::integration::{TuiIntegrationTester, IntegrationTestConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let config = IntegrationTestConfig {
        enable_performance: true,
        enable_cross_platform: true,
        enable_stress_testing: true,
        stress_test_commands: 100,
        stress_test_concurrency: 10,
        timeout_seconds: 30,
        terminal_width: 120,
        terminal_height: 30,
    };

    let mut tester = TuiIntegrationTester::new(config);
    let results = tester.run_integration_tests().await?;

    if results.overall_success {
        println!("✅ All tests passed!");
    } else {
        println!("❌ Tests failed. See report for details.");
    }

    Ok(())
}
```

## Test Categories

### 1. Functional Testing
- Command parsing and execution
- Output formatting validation
- Error handling verification
- Interactive mode testing

### 2. Performance Testing
- Startup time validation
- Command execution speed
- Memory consumption monitoring
- Concurrent operation testing

### 3. Compatibility Testing
- Terminal capability detection
- ANSI escape sequence support
- Unicode character rendering
- Cross-platform behavior validation

### 4. Integration Testing
- End-to-end workflow validation
- Component interaction testing
- CI/CD pipeline integration
- Release qualification

## Success Criteria

- **95%+ TUI functionality coverage** across all commands and features
- **Cross-platform compatibility** on Linux, macOS, and Windows
- **Performance benchmarks** meeting defined SLAs
- **Automated testing** runnable in CI/CD pipelines
- **Comprehensive error handling** validation

## Architecture

```
TUI Testing Framework
├── harness.rs          # Main test orchestration
├── mock_terminal.rs    # Terminal emulation
├── command_simulator.rs # Command execution
├── output_validator.rs # Output validation
├── performance_monitor.rs # Performance metrics
├── cross_platform.rs   # Platform testing
└── integration.rs      # High-level integration
```

## Dependencies

- `tokio` - Async runtime
- `anyhow` - Error handling
- `regex` - Pattern matching
- `sysinfo` - System information
- `term_size` - Terminal dimensions

## Integration with CI/CD

The framework is designed to integrate seamlessly with CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Run TUI Tests
  run: cargo run --bin terraphim-tui-tester test --performance --cross-platform --output tui-test-results.txt

- name: Upload Test Results
  uses: actions/upload-artifact@v3
  with:
    name: tui-test-results
    path: tui-test-results.txt
```

## Contributing

When adding new TUI features:
1. Add corresponding test cases to the appropriate modules
2. Update validation patterns in `output_validator.rs`
3. Ensure cross-platform compatibility
4. Add performance benchmarks if applicable
5. Update this documentation

## Troubleshooting

### Common Issues

1. **Binary not found**: Ensure terraphim-repl is built and available
2. **Permission denied**: Check file permissions for test artifacts
3. **Timeout errors**: Increase timeout values for slow systems
4. **ANSI issues**: Some terminals may not support all escape sequences

### Debugging

Enable verbose logging:
```bash
RUST_LOG=debug cargo run --bin terraphim-tui-tester test
```

Run individual test components:
```rust
// Test just the harness
let harness = TuiTestHarness::default().await?;
let result = harness.test_command("/help").await?;
```