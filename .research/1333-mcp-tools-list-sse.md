# Research Document: #1333 MCP tools/list over SSE/HTTP

**Status**: Research Complete — Root Cause Identified
**Author**: Echo (Twin Maintainer)
**Date**: 2026-05-25

## Problem Statement
`tools/list` works over stdio transport but returns empty or fails to route over HTTP/SSE transport. External MCP clients (Claude Desktop, Cursor) see zero available tools when connecting via HTTP.

## Root Cause Analysis

### Current State
`McpService::get_info()` in `crates/terraphim_mcp_server/src/lib.rs:2935` returns:

```rust
fn get_info(&self) -> ServerInfo {
    ServerInfo {
        server_info: rmcp::model::Implementation { ... },
        instructions: Some("..."),
        ..Default::default()
    }
}
```

The `..Default::default()` sets `capabilities` to `ServerCapabilities::default()`, where ALL fields are `None`:
- `tools: None`
- `resources: None`
- `prompts: None`
- etc.

### Why Stdio Works
Stdio transport clients (test harness, `TokioChildProcess`) call `list_tools()` directly on the service proxy without checking initialization capabilities. The `ServerHandler::list_tools` override in `McpService` is invoked correctly.

### Why SSE/HTTP Fails
Proper MCP clients (Claude Desktop, Cursor, any HTTP-first consumer) check `ServerCapabilities` during the `initialize` handshake. When `tools: None`, they conclude the server has no tools and never call `tools/list`. The rmcp library may also short-circuit `tools/list` requests when the capability is not advertised.

## Fix Design

### File Changes
| File | Change |
|------|--------|
| `crates/terraphim_mcp_server/src/lib.rs` | Update `get_info()` to set `capabilities.tools = Some(ToolsCapability::default())` |
| `crates/terraphim_mcp_server/tests/test_tools_list.rs` | Add SSE transport test verifying `tools/list` over HTTP |

### Code Change
```rust
fn get_info(&self) -> ServerInfo {
    ServerInfo {
        server_info: rmcp::model::Implementation { ... },
        instructions: Some("...".to_string()),
        capabilities: rmcp::model::ServerCapabilities {
            tools: Some(rmcp::model::ToolsCapability::default()),
            ..Default::default()
        },
    }
}
```

### Test Strategy
1. **Unit test**: Verify `get_info()` returns non-empty capabilities
2. **Integration test**: Start SSE server, connect via HTTP client, send `tools/list`, assert non-empty tools array

## Risks
- None identified — this is a configuration fix, no logic changes
- No new dependencies required

## Next Steps
1. Implement `get_info()` fix
2. Write SSE integration test
3. Run quality gates (`cargo check`, `clippy`, `test`)
4. Submit PR
