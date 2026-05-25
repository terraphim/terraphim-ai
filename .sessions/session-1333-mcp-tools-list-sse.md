# Incomplete Handoff: #1333 MCP tools/list SSE/HTTP Fix

**Agent**: pi session
**Date**: 2026-05-25
**Issue**: #1333 — fix(mcp): register tools/list handler for HTTP/SSE transport
**Branch**: `task/1333-mcp-tools-list-sse` (pushed to origin)

## What's Done

1. **Root cause identified** (by prior agent session): `McpService::get_info()` returned `ServerCapabilities::default()` where `tools: None`. Proper MCP clients check capabilities during `initialize` handshake and never call `tools/list` when `tools` capability is not advertised.

2. **Core fix committed** (`b3d961d7`): Updated `get_info()` in `crates/terraphim_mcp_server/src/lib.rs` to set:
   - `capabilities.tools = Some(ToolsCapability::default())`
   - `capabilities.resources = Some(ResourcesCapability::default())`

3. **Verified**: `cargo check -p terraphim_mcp_server` passes, all existing tests pass (`test_tools_list`, `test_mcp_fixes_validation`).

4. **Pushed** to `origin/task/1333-mcp-tools-list-sse`.

## What Remains

1. **SSE integration test**: Write a test that starts the MCP server in SSE mode (`--sse`), connects via HTTP client, and verifies `tools/list` returns non-empty tools. This is the acceptance criteria from the issue:
   ```
   Given  the MCP server is started with HTTP/SSE transport on port 3001
   When   a client sends {"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
   Then   the response contains a non-empty `tools` array
   ```
   File: `crates/terraphim_mcp_server/tests/test_tools_list_sse.rs`
   Pattern: Use `reqwest` or raw TCP to send JSON-RPC to the SSE server's `/message` endpoint after establishing SSE connection at `/sse`.

2. **Unit test for `get_info()`**: Verify `get_info()` returns `capabilities.tools.is_some()` and `capabilities.resources.is_some()`. Quick in-memory test, no server needed.

3. **Run full quality gates**: `cargo clippy -p terraphim_mcp_server -- -D warnings && cargo fmt --all -- --check`

4. **Create PR**: 
   ```bash
   gtr create-pull --owner terraphim --repo terraphim-ai --title "Fix #1333: advertise MCP tools/resources capabilities for SSE/HTTP clients" --base main --head task/1333-mcp-tools-list-sse
   ```

5. **Post comment on issue #1333** with implementation summary.

## Next-Agent Starting Position

- Branch is checked out and pushed: `task/1333-mcp-tools-list-sse`
- The core fix is in place at `crates/terraphim_mcp_server/src/lib.rs` line ~2935 (`get_info()`)
- Next: write SSE integration test, run clippy/fmt, create PR
- The rmcp SSE client API: use `rmcp::transport::sse_client` or manual HTTP with `reqwest`
- Dev-dependencies already include `rmcp` with `transport-sse-server` feature; may need `transport-sse-client` or use raw HTTP
