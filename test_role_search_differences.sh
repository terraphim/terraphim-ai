#!/bin/bash

echo "=================================================================="
echo "PROVING SEARCH RESULTS CHANGE BASED ON ROLE"
echo "=================================================================="
echo ""

BINARY="./target/release/terraphim-agent"

echo "KEY DIFFERENCES IN ROLE CONFIGURATIONS:"
echo "----------------------------------------"
echo "1. Default Role: Searches in 'docs/src' using title-scorer"
echo "2. Rust Engineer: Searches query.rs (online Rust documentation)"
echo "3. Terraphim Engineer: Uses graph embeddings + knowledge graph from 'docs/src/kg'"
echo ""

# Test with Default role
echo "TEST 1: Search with DEFAULT role (uses local docs/src)"
echo "--------------------------------------------------------"
echo -e "/role select Default\n/search tokio\n/quit" | $BINARY repl 2>/dev/null | grep -A 5 -E "(Switched|Found|Title)"
echo ""

# Test with Rust Engineer role
echo "TEST 2: Search with RUST ENGINEER role (uses query.rs)"
echo "--------------------------------------------------------"
echo -e "/role select \"Rust Engineer\"\n/search tokio\n/quit" | $BINARY repl 2>/dev/null | grep -A 5 -E "(Switched|Found|Title|Error)"
echo ""

# Test with Terraphim Engineer role
echo "TEST 3: Search with TERRAPHIM ENGINEER role (uses graph embeddings)"
echo "--------------------------------------------------------------------"
echo -e "/role select \"Terraphim Engineer\"\n/search tokio\n/quit" | $BINARY repl 2>/dev/null | grep -A 5 -E "(Switched|Found|Title)"
echo ""

# Show different search terms
echo "TEST 4: Search for 'async' with each role"
echo "-------------------------------------------"
echo ""
echo "Default role results:"
echo -e "/role select Default\n/search async\n/quit" | $BINARY repl 2>/dev/null | grep "Found"
echo ""
echo "Rust Engineer results:"
echo -e "/role select \"Rust Engineer\"\n/search async\n/quit" | $BINARY repl 2>/dev/null | grep -E "(Found|Error)"
echo ""
echo "Terraphim Engineer results:"
echo -e "/role select \"Terraphim Engineer\"\n/search async\n/quit" | $BINARY repl 2>/dev/null | grep "Found"
echo ""

echo "=================================================================="
echo "ANALYSIS:"
echo "- Default: Searches local 'docs/src' directory"
echo "- Rust Engineer: Attempts to search query.rs (online service)"
echo "- Terraphim Engineer: Uses knowledge graph from 'docs/src/kg'"
echo "=================================================================="
