#!/bin/bash

# Test script to verify role-specific search behavior
# Usage: ./scripts/test_role_search_behavior.sh

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🔍 Testing Role-Specific Search Behavior"
echo "========================================"

# Test messages
INIT_MSG='{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "test", "version": "1.0.0"}}}'
SEARCH_TESTING='{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "search", "arguments": {"query": "testing", "limit": 3}}}'
SEARCH_GRAPH='{"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "search", "arguments": {"query": "graph", "limit": 3}}}'

echo
echo "🧪 Test 1: MCP Server with Desktop Profile (Default)"
echo "Expected: Should find 'graph' results from Terraphim Engineer KG"
echo "Expected: Should find fewer 'testing' results (not in Terraphim Engineer domain)"
echo

# Test desktop profile (default)
echo "Initializing..."
echo "$INIT_MSG" | timeout 10s ./target/debug/terraphim_mcp_server --profile desktop 2>/dev/null | head -1

echo "Searching for 'testing'..."
echo "$SEARCH_TESTING" | timeout 10s ./target/debug/terraphim_mcp_server --profile desktop 2>/dev/null | head -5 | tail -4

echo "Searching for 'graph'..."
echo "$SEARCH_GRAPH" | timeout 10s ./target/debug/terraphim_mcp_server --profile desktop 2>/dev/null | head -5 | tail -4

echo
echo "🧪 Test 2: MCP Server with Server Profile"
echo "Expected: Should find different results due to remote KG"
echo

# Test server profile
echo "Initializing..."
echo "$INIT_MSG" | timeout 10s ./target/debug/terraphim_mcp_server --profile server 2>/dev/null | head -1

echo "Searching for 'testing'..."
echo "$SEARCH_TESTING" | timeout 10s ./target/debug/terraphim_mcp_server --profile server 2>/dev/null | head -5 | tail -4

echo "Searching for 'graph'..."
echo "$SEARCH_GRAPH" | timeout 10s ./target/debug/terraphim_mcp_server --profile server 2>/dev/null | head -5 | tail -4

echo
echo "🧪 Test 3: Desktop Binary in MCP Mode"
echo "Expected: Should match desktop profile results"
echo

# Test desktop binary
echo "Initializing..."
echo "$INIT_MSG" | timeout 10s ./target/debug/terraphim-ai-desktop mcp-server 2>/dev/null | head -1

echo "Searching for 'testing'..."
echo "$SEARCH_TESTING" | timeout 10s ./target/debug/terraphim-ai-desktop mcp-server 2>/dev/null | head -5 | tail -4

echo "Searching for 'graph'..."
echo "$SEARCH_GRAPH" | timeout 10s ./target/debug/terraphim-ai-desktop mcp-server 2>/dev/null | head -5 | tail -4

echo
echo "✅ Role search behavior testing complete"
echo "Compare the results above to verify role-specific search functionality"
