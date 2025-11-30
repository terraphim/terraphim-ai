# Summary: TESTING_SCRIPTS_README.md

## Purpose
Comprehensive documentation for testing scripts used in Novel editor autocomplete integration with Terraphim's knowledge graph system. Provides automated testing workflows and service management.

**Updated for v1.0.0**: Now includes testing for multi-language packages (Rust, Node.js, Python) and comprehensive validation of autocomplete functionality across all platforms.

## Key Scripts
- **quick-start-autocomplete.sh**: Interactive menu with preset configurations (full, mcp, dev, test, status, stop)
- **start-autocomplete-test.sh**: Main testing script with full control over services and configuration
- **stop-autocomplete-test.sh**: Clean shutdown of all testing services with graceful or force stop options
- **test-novel-autocomplete-integration.js**: Validation script for MCP server integration

## Services & Ports
- **MCP Server**: Port 8001 (default) - Autocomplete API
- **Axum Server**: Dynamic port - Alternative backend
- **Desktop App**: N/A - Tauri window for Novel editor UI

## Testing Scenarios
1. **Full Testing**: Everything needed (MCP + Desktop + Tests)
2. **MCP Only**: Backend development
3. **Desktop Development**: UI work
4. **Tests Only**: Validation
5. **Development Mode**: MCP + Desktop without tests

## Key Features
- Process management with PID files (`pids/`)
- Log aggregation (`logs/mcp_server.log`, `logs/desktop_app.log`)
- Real-time monitoring capabilities
- Force stop option for stuck processes
- Prerequisites checking (Rust, Node.js, commands)

## Command Options
**start-autocomplete-test.sh**:
- `-m, --mcp-only`: Start only MCP server
- `-d, --desktop-only`: Start only desktop app
- `-t, --test-only`: Run only integration tests
- `-p, --port PORT`: Set MCP server port (default: 8001)
- `--no-desktop`: Skip desktop app startup
- `--no-tests`: Skip integration tests
- `--verbose`: Enable verbose logging

**stop-autocomplete-test.sh**:
- `-s, --status`: Check status of running services
- `-f, --force`: Force kill all processes

## Troubleshooting
- Port conflicts: `lsof -i :8001` to find conflicting process
- MCP not responding: Check `logs/mcp_server.log`
- Desktop won't start: Verify `cd desktop && yarn install`
- No autocomplete: Verify MCP server running, backend connection, known terms

## Success Indicators
- MCP server: "✅ MCP server is ready!"
- Desktop app: "✅ Desktop app started (PID: 12345)"
- Integration tests: "✅ autocomplete_terms working"
