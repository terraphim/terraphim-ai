# Terraphim MCP Server - End-to-End Tests

This directory contains end-to-end tests for the Terraphim MCP Server implementation. The tests use the official MCP Python SDK to verify that the server correctly implements the Model Context Protocol.

## Test Overview

The tests validate:

1. **Tool Listing**: Verifies that the server responds correctly to `list_tools` requests
2. **Resource Listing**: Verifies that the server responds correctly to `list_resources` requests
3. **Search Tool**: Verifies that the server's search tool can be called
4. **Resource Reading**: Verifies that the server can handle `read_resource` requests

## Prerequisites

- Python 3.8 or higher
- `uv` package manager (if not installed, the test runner will attempt to install it)
- Rust toolchain (for building the server)

## Running the Tests

To run the tests, use the provided shell script:

```bash
./run_mcp_e2e_tests.sh
```

### Options

- `--debug`: Enables verbose debugging output
- `--expect-data`: Use this flag if you expect real data to be available (stricter test validation)

## Test Environment

The test runner sets up:

1. A Python virtual environment (`.venv` directory)
2. Installs the MCP Python SDK
3. Builds the Terraphim MCP Server release binary
4. Runs the test script

## Environment Variables

The test runner sets these environment variables:

- `TERRAPHIM_LOG_DIR`: Directory for log files
- `RUST_LOG`: Log level for Rust logging
- `TERRAPHIM_CONFIG`: Path to the Terraphim configuration file used for testing
- `TERRAPHIM_FIXTURES_DIR`: Directory for test fixtures (if applicable)

## Test Results

In the default test environment (without real data), the following behavior is expected:

1. `test_list_tools` should pass and find the "search" tool
2. `test_list_resources` should pass, possibly with an empty resources list
3. `test_search_tool` should pass, but might return an error about missing role configuration
4. `test_read_resources` should pass, handling expected errors for non-existent resources

## Troubleshooting

### "Role `Default` not found in config" Error

This error is expected and handled in a test environment without proper configuration. The tests are designed to pass even with this error, as it verifies that the API is responding correctly.

### Test Installation Issues

If you encounter issues with the Python environment:

1. Delete the `.venv` directory
2. Run the test script again to create a fresh environment

### Server Startup Problems

If the server fails to start:

1. Check logs in the `logs` directory
2. Verify that the server binary can be executed directly
3. Ensure no other instance of the server is running on the same port

## Manual Test Execution

To run tests manually:

```bash
# Activate the virtual environment
source .venv/bin/activate

# Run the test script directly
python mcp_e2e_test.py --binary /path/to/terraphim_mcp_server --debug
```

## Extending the Tests

To add new tests:

1. Add new test methods to the `TerraphimMcpTester` class in `mcp_e2e_test.py`
2. Follow the pattern of existing test methods
3. Add your new test to the `test_functions` list in the `run_tests` method

## Log Inspection

Test logs are written to:
- Console output
- The directory specified by `TERRAPHIM_LOG_DIR` (defaults to project root `/logs`) 