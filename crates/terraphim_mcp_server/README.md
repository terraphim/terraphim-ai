# Terraphim MCP Server

A server implementation of the Model Context Protocol (MCP) for Terraphim.

## Features

- Full implementation of the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol/spec)
- Supports resource listing, searching, and reading for Terraphim knowledge graphs
- Provides tools for search and resource management
- RESTful API for interacting with Terraphim using the MCP protocol
- Compatible with MCP clients and applications

## Prerequisites

- Rust (latest stable version recommended)
- For running tests:
  - Python 3.8 or higher
  - uv package manager (installed automatically by the test script if missing)

## Building

To build the MCP server:

```bash
cargo build --release -p terraphim_mcp_server
```

The binary will be available at `target/release/terraphim_mcp_server`.

## Running

```bash
terraphim_mcp_server [OPTIONS]
```

### Options

- `--host <HOST>`: Host address to bind to (default: 127.0.0.1)
- `--port <PORT>`: Port to listen on (default: 8080)
- `--help`: Print help information
- `--version`: Print version information

## End-to-End Testing

The project includes end-to-end tests using the official MCP Python SDK.

### Automated Testing

The easiest way to run end-to-end tests is using the provided script:

```bash
cd crates/terraphim_mcp_server/tests/e2e
./run_mcp_e2e_tests.sh
```

This script will:
1. Create a Python virtual environment using uv
2. Install the MCP SDK (`mcp[cli]`)
3. Build the release binary
4. Run the tests
5. Output logs to the `logs` directory

To run tests in debug mode with more verbose logging:

```bash
./run_mcp_e2e_tests.sh --debug
```

### Manual Testing

To manually run the tests:

1. Build the release binary:
   ```bash
   cargo build --release -p terraphim_mcp_server
   ```

2. Install the MCP Python SDK:
   ```bash
   uv pip install 'mcp[cli]'
   ```

3. Run the test script:
   ```bash
   cd crates/terraphim_mcp_server/tests/e2e
   python mcp_e2e_test.py --binary ../../../target/release/terraphim_mcp_server
   ```

   For more verbose logging, add the `--debug` flag:
   ```bash
   python mcp_e2e_test.py --binary ../../../target/release/terraphim_mcp_server --debug
   ```

## Test Coverage

The end-to-end tests cover the following functionality:

1. Listing available MCP tools
2. Listing resources
3. Searching using the search tool
4. Reading resource content by URI

## Log Files

Test logs and server logs are stored in the `logs` directory at the root of the project.

## Development

### Adding New Tests

To add new tests, modify the `mcp_e2e_test.py` file in the `tests/e2e` directory. The test framework is designed to be easily extensible with new test cases.

## License

This project is licensed under [LICENSE] - see the LICENSE file for details. 