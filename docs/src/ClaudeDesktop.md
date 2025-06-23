# Using Terraphim Desktop with Claude via MCP

Terraphim AI Desktop can act as a **Model-Context Protocol** (MCP) server so that Claude Desktop (Anthropic) can directly query your local knowledge graph.

## Prerequisites

* Terraphim AI Desktop binary is built (`cargo build --package terraphim-ai-desktop` or download a release).
* Claude Desktop ≥ v0.3.0 with MCP client capability.

## Start the MCP server

Run the desktop binary with the `mcp-server` sub-command (defined in `tauri.conf.json`).  This skips the GUI and exposes the MCP JSON-RPC interface over *stdin/stdout*:

```bash
# head-less server
./terraphim-ai-desktop mcp-server  \
  > /tmp/terraphim_mcp.log   # optional log redirection
```

You may also pass the flag form:

```bash
./terraphim-ai-desktop --mcp-server
```

The server prints nothing to stdout other than MCP JSON-RPC messages, so it is safe to pipe its output directly to an MCP-aware client.

## Point Claude Desktop to the server

Claude desktop expects an executable that speaks MCP on *stdio*.  Configure it to launch Terraphim in server mode:

1. Open **Settings → Tools → Custom MCP backend**.
2. Set *Executable* to the absolute path of `terraphim-ai-desktop`.
3. In *Arguments* enter `mcp-server`.
4. (Optional) enable *Reuse process* so Claude keeps the Terraphim process alive between sessions.

Save and connect—Claude will send the standard `initialize` handshake followed by tool calls such as `search`.

> ℹ️  Terraphim automatically returns ranked search results from your local haystacks.  Update `config.json` as usual to tweak roles, thesauri, or haystacks; the running server picks up `update_config_tool` calls from Claude.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| Claude hangs on **initializing** | Ensure `terraphim-ai-desktop` is reachable and not printing extra logs to stdout.  Redirect logs to file (`> /tmp/terraphim_mcp.log`). |
| `search` returns empty results | Verify your `Config` points to correct haystack paths and that documents are indexed. |
| Binary not found on Windows | Use `terraphim-ai-desktop.exe` in *Executable* field and keep it in the same folder as its dependent DLLs. |

## Related commands

* **GUI mode (default)** – double-click the binary or run `./terraphim-ai-desktop`.
* **Head-less server** – `./terraphim-ai-desktop mcp-server`.
* **Log directory** – set `TERRAPHIM_LOG_DIR` env var (defaults to `./logs`).

---
For advanced usage and integration test examples see `crates/terraphim_mcp_server/tests/desktop_mcp_integration.rs`. 