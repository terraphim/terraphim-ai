# Terraphim MCP Server Learnings

- Running `./run_mcp_e2e_tests.sh` shows `mcp` client hangs waiting for `initialize` response.
- Server logs indicate it starts correctly, creates roles, and logs "Initialized Terraphim MCP service", so startup finishes.
- The hang is during MCP handshake, not remote thesaurus fetch (remote URL resolves quickly).
- Need to investigate why `rmcp` server doesn't send `initialize` response; may require explicit handler or use of `ServiceExt::serve` API.