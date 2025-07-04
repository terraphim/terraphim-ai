# Using Terraphim Desktop with Claude via MCP

Terraphim AI Desktop can act as a **Model-Context Protocol** (MCP) server so that Claude Desktop (Anthropic) can directly query your local knowledge graph.

## Prerequisites

* Build the Terraphim AI Desktop binary:
  ```bash
  cd /path/to/terraphim-ai
  cargo build --package terraphim-ai-desktop
  ```
  The binary will be at `./target/debug/terraphim-ai-desktop` (or `./target/release/terraphim-ai-desktop` for release builds).
* Claude Desktop ≥ v0.3.0 with MCP client capability.

## Start the MCP server

Run the desktop binary with the `mcp-server` sub-command. This skips the GUI and exposes the MCP JSON-RPC interface over *stdin/stdout*:

```bash
# Headless server (recommended for Claude integration)
./target/debug/terraphim-ai-desktop mcp-server \
  > /tmp/terraphim_mcp.log 2>&1   # optional: redirect logs to file
```

Or, equivalently:

```bash
./target/debug/terraphim-ai-desktop --mcp-server
```

> **Note:** The server prints only MCP JSON-RPC messages to stdout. Always redirect logs to a file to avoid interfering with MCP communication.

## Configure Claude Desktop

Claude Desktop expects an executable that speaks MCP on *stdio*. Configure it to launch Terraphim in server mode:

1. Open **Settings → Tools → Custom MCP backend**.
2. Set **Executable** to the absolute path of your `terraphim-ai-desktop` binary (e.g., `/absolute/path/to/terraphim-ai/target/debug/terraphim-ai-desktop`).
3. Set **Arguments** to:
   ```
   mcp-server
   ```
4. *(Optional)* Enable **Reuse process** so Claude keeps the Terraphim process alive between sessions.

Save and connect—Claude will send the standard `initialize` handshake followed by tool calls such as `search`.

> ℹ️  Terraphim automatically returns ranked search results from your local haystacks. Update your configuration as usual to tweak roles, thesauri, or haystacks; the running server picks up `update_config_tool` calls from Claude.

## Troubleshooting

| Symptom                                 | Fix                                                                                   |
|------------------------------------------|--------------------------------------------------------------------------------------|
| Claude hangs on **initializing**         | Ensure `terraphim-ai-desktop` is reachable and not printing extra logs to stdout.    |
|                                          | Redirect logs to a file as shown above.                                              |
| `search` returns empty results           | Verify your config points to correct haystack paths and that documents are indexed.  |
| Binary not found on Windows              | Use `terraphim-ai-desktop.exe` in *Executable* and keep it with its dependent DLLs.  |
| No search results or wrong role/graph    | Check your configuration and ensure the correct role and knowledge graph are loaded. |
| MCP server does not start                | Rebuild the binary and check for errors.                                             |

## Related commands

* **GUI mode (default):** double-click the binary or run `./target/debug/terraphim-ai-desktop`.
* **Headless MCP server:** `./target/debug/terraphim-ai-desktop mcp-server`.
* **Log directory:** set `TERRAPHIM_LOG_DIR` env var (defaults to `./logs`).

---
For advanced usage and integration test examples see `crates/terraphim_mcp_server/tests/desktop_mcp_integration.rs`. 