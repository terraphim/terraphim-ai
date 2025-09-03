#!/bin/bash

# Test MCP autocomplete functionality via stdio transport
echo "Testing MCP autocomplete via stdio transport..."

# Start the MCP server in the background with stdio transport
TERRAPHIM_SETTINGS_PATH="$(pwd)/../terraphim_settings/default/settings_local_dev.toml" \
cargo run -- --verbose > server_output.log 2>&1 &
SERVER_PID=$!

echo "MCP server started with PID: $SERVER_PID"
sleep 5

# Test 1: List available tools
echo "Testing tools/list..."
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
tee -a server_output.log

# Test 2: Build autocomplete index
echo "Testing build_autocomplete_index..."
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"build_autocomplete_index","arguments":{"role":"Terraphim Engineer"}}}' | \
tee -a server_output.log

# Test 3: Test autocomplete with snippets
echo "Testing autocomplete_with_snippets..."
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"autocomplete_with_snippets","arguments":{"query":"terraphim","limit":5,"role":"Terraphim Engineer"}}}' | \
tee -a server_output.log

# Test 4: Test autocomplete terms
echo "Testing autocomplete_terms..."
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"autocomplete_terms","arguments":{"query":"terraphim","limit":5,"role":"Terraphim Engineer"}}}' | \
tee -a server_output.log

# Wait a bit for processing
sleep 3

# Stop the server
echo "Stopping MCP server..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

echo "Test completed. Check server_output.log for results."
echo "Server output:"
cat server_output.log
