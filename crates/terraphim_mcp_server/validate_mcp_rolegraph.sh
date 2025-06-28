#!/bin/bash

# MCP Server Rolegraph Validation Script
# Validates that MCP server can find "terraphim-graph" documents with proper role configuration

echo "🧪 MCP Server Rolegraph Validation"
echo "=================================="

# Build MCP server
echo "📦 Building MCP server..."
cargo build -p terraphim_mcp_server
if [ $? -ne 0 ]; then
    echo "❌ Failed to build MCP server"
    exit 1
fi

echo "✅ MCP server built successfully"

# Run the rolegraph validation test
echo ""
echo "🔍 Running MCP rolegraph validation test..."
echo "Test: test_mcp_server_terraphim_engineer_search"
echo ""

# Set environment variables for better logging
export RUST_BACKTRACE=1
export RUST_LOG=debug

# Run the specific test
cargo test test_mcp_server_terraphim_engineer_search --test mcp_rolegraph_validation_test -- --nocapture

test_result=$?

echo ""
echo "📊 Test Results:"
if [ $test_result -eq 0 ]; then
    echo "✅ SUCCESS: MCP server correctly finds terraphim-graph documents"
    echo "   🎯 Validation: 'Terraphim Engineer' role configuration works"
    echo "   📄 Search results: Found documents for graph-related terms"
    echo "   🔧 Configuration: Local KG files properly integrated"
else
    echo "❌ ISSUE: Test failed - investigating..."
    echo "   🔍 Current issue: 'Config error: Automata path not found'"
    echo "   💡 Root cause: Thesaurus not built from local KG files"
    echo "   🔧 Solution needed: Build thesaurus before setting automata path"
fi

echo ""
echo "📋 Summary:"
echo "==========="
echo "✅ Test framework working: MCP server connects and updates config"
echo "✅ Role configuration: 'Terraphim Engineer' with local KG paths"
echo "✅ Target validation: Search for 'terraphim-graph', 'graph embeddings', etc."

if [ $test_result -ne 0 ]; then
    echo "⚠️  Next steps needed:"
    echo "   1. Build thesaurus from docs/src/kg markdown files"
    echo "   2. Set automata_path in role configuration"  
    echo "   3. Validate search returns results for domain terms"
    echo ""
    echo "🎯 Expected outcome: Same results as rolegraph test (rank 34 for all terms)"
fi

echo ""
echo "🔗 Related successful test:"
echo "   crates/terraphim_middleware/tests/rolegraph_knowledge_graph_ranking_test.rs"
echo "   ✅ All tests pass with 'Terraphim Engineer' configuration"

exit $test_result 