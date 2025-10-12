# Using Terraphim Desktop with Claude via MCP

Terraphim AI provides **two ways** to expose MCP (Model Context Protocol) server functionality for Claude Desktop integration:

## MCP Server Options

### Option 1: Desktop Binary with MCP Mode (✅ **RECOMMENDED**)

The `terraphim-ai-desktop` binary can run in MCP server mode, providing the exact same configuration as the desktop application:

```bash
./target/debug/terraphim-ai-desktop mcp-server
```

**Advantages:**
- ✅ **Identical configuration** to desktop app (uses `build_default_desktop()`)
- ✅ **Terraphim Engineer role** with local knowledge graph by default
- ✅ **Consistent behavior** between GUI and MCP modes
- ✅ **Single binary** for both desktop and MCP server functionality

### Option 2: Standalone MCP Server Binary

The dedicated `terraphim_mcp_server` binary provides MCP server functionality:

```bash
./target/debug/terraphim_mcp_server
```

**Note:** This binary now also uses the desktop configuration for consistency.

## Prerequisites

* Build the Terraphim AI binaries:
  ```bash
  cd /path/to/terraphim-ai

  # Build desktop binary (includes MCP server mode)
  cargo build --package terraphim-ai-desktop

  # Optional: Build standalone MCP server
  cargo build --package terraphim_mcp_server
  ```

* Claude Desktop ≥ v0.3.0 with MCP client capability.

> **Important:** Always use **absolute paths** to binaries in Claude Desktop configuration.

## Claude Desktop Configuration

### ✅ **RECOMMENDED: Desktop Binary Method**

Configure Claude Desktop to use the desktop binary in MCP server mode:

**Executable:**
```
/Users/alex/projects/terraphim/terraphim-ai/target/debug/terraphim-ai-desktop
```

**Arguments:**
```
mcp-server
```

### Alternative: Standalone MCP Server Method

**Executable:**
```
/Users/alex/projects/terraphim/terraphim-ai/target/debug/terraphim_mcp_server
```

**Arguments:** *(leave empty)*

## Configuration Steps

1. Open **Claude Desktop → Settings → Tools → Custom MCP backend**
2. Set **Executable** to the **absolute path** of your chosen binary
3. Set **Arguments** as specified above
4. *(Optional)* Enable **Reuse process** to keep the server alive between sessions
5. Save and connect

## Default Configuration

Both MCP server options now use the **desktop configuration** which provides:

- **Default Role:** "Terraphim Engineer"
- **Knowledge Graph:** Local KG built from `docs/src/kg/` (10 entries)
- **Relevance Function:** TerraphimGraph
- **Haystacks:** Local markdown files in `docs/src/`
- **Theme:** Superhero (for desktop UI)

## Verification

Test your setup manually:

```bash
# Test desktop binary in MCP mode
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0.0"}}}' | ./target/debug/terraphim-ai-desktop mcp-server 2>/dev/null

# Test standalone MCP server
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0.0"}}}' | ./target/debug/terraphim_mcp_server 2>/dev/null
```

Both should return a JSON response with `"serverInfo":{"name":"terraphim-mcp","version":"0.1.0"}`.

## Available Search Capabilities

With the default Terraphim Engineer configuration, Claude can search for:

- **Terraphim-specific terms:** "terraphim-graph", "graph embeddings", "knowledge graph based embeddings"
- **Technical concepts:** "haystack", "service", "middleware", "provider"
- **General terms:** "graph", "agent", "datasource"

Example search queries that work:
- "How does terraphim-graph scoring work?"
- "What is a haystack in Terraphim?"
- "Explain graph embeddings"

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Claude hangs on **initializing** | Ensure binary is reachable and **redirect stderr** (`2>&1`) |
| `search` returns empty results | ✅ **FIXED:** Both binaries now use desktop configuration with proper KG |
| Binary not found (ENOENT error) | **Use absolute path** to binary |
| Read-only file system (os error 30) | ✅ **FIXED:** MCP server uses `/tmp/terraphim-logs` for logging |
| No search results for known terms | Verify the binary uses desktop configuration (should find 10 KG entries) |

## Related Commands

* **GUI mode (default):** `./target/debug/terraphim-ai-desktop`
* **MCP server mode (desktop binary):** `./target/debug/terraphim-ai-desktop mcp-server`
* **Standalone MCP server:** `./target/debug/terraphim_mcp_server`
* **Set log directory:** `TERRAPHIM_LOG_DIR=/custom/path ./target/debug/...`

---

## Why Two Options?

- **Desktop Binary (`terraphim-ai-desktop mcp-server`)**: ✅ **Best for most users** - guarantees identical behavior to desktop app
- **Standalone MCP Server (`terraphim_mcp_server`)**: Useful for deployment scenarios where you only need MCP functionality

Both now use the same desktop configuration ensuring consistent search results and knowledge graph functionality.

**Tags:**

- terraphim-graph
