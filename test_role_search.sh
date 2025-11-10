#!/bin/bash

# Test role switching and search functionality
echo "==================================================================="
echo "DEMONSTRATING ROLE SWITCHING AND SEARCH FUNCTIONALITY"
echo "==================================================================="
echo ""

BINARY="./target/release/terraphim-tui"

# Test 1: Check initial role and search
echo "TEST 1: Initial state - checking current role and doing a search"
echo "----------------------------------------------------------------"
echo "Commands: /role list, /search rust"
echo ""
echo -e "/role list\n/search rust\n/quit" | $BINARY repl 2>/dev/null | grep -E "(Available roles:|▶|Found [0-9]+ result)"
echo ""

# Test 2: Switch to Default role and search
echo "TEST 2: Switch to 'Default' role and search again"
echo "----------------------------------------------------------------"
echo "Commands: /role select Default, /role list, /search rust"
echo ""
echo -e "/role select Default\n/role list\n/search rust\n/quit" | $BINARY repl 2>/dev/null | grep -E "(Switched to role:|Available roles:|▶|Found [0-9]+ result)"
echo ""

# Test 3: Switch to Rust Engineer role and search
echo "TEST 3: Switch to 'Rust Engineer' role and search"
echo "----------------------------------------------------------------"
echo "Commands: /role select \"Rust Engineer\", /role list, /search rust"
echo ""
echo -e "/role select \"Rust Engineer\"\n/role list\n/search rust\n/quit" | $BINARY repl 2>/dev/null | grep -E "(Switched to role:|Available roles:|▶|Found [0-9]+ result)"
echo ""

# Test 4: Switch to Terraphim Engineer role and search
echo "TEST 4: Switch to 'Terraphim Engineer' role and search"
echo "----------------------------------------------------------------"
echo "Commands: /role select \"Terraphim Engineer\", /role list, /search rust"
echo ""
echo -e "/role select \"Terraphim Engineer\"\n/role list\n/search rust\n/quit" | $BINARY repl 2>/dev/null | grep -E "(Switched to role:|Available roles:|▶|Found [0-9]+ result)"
echo ""

# Test 5: Verify role persistence in config
echo "TEST 5: Check that role selection persists in configuration"
echo "----------------------------------------------------------------"
echo "Command: /config show (looking for selected_role field)"
echo ""
echo -e "/config show\n/quit" | $BINARY repl 2>/dev/null | grep -E "selected_role"
echo ""

echo "==================================================================="
echo "TEST COMPLETE"
echo "==================================================================="
