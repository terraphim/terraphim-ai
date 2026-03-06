#!/bin/bash

echo "==================================================================="
echo "DETAILED ROLE SWITCHING DEMONSTRATION"
echo "==================================================================="
echo ""

BINARY="./target/release/terraphim-agent"

# Show full role list with indicators
echo "1. SHOWING ALL AVAILABLE ROLES (with current role indicator ▶)"
echo "----------------------------------------------------------------"
echo -e "/role list\n/quit" | $BINARY repl 2>/dev/null | grep -A 10 "Available roles:"
echo ""

echo "2. SWITCHING TO 'Rust Engineer' ROLE"
echo "----------------------------------------------------------------"
echo -e "/role select \"Rust Engineer\"\n/role list\n/quit" | $BINARY repl 2>/dev/null | grep -A 10 -E "(Switched to role:|Available roles:)"
echo ""

echo "3. PERFORMING SEARCH WITH 'Rust Engineer' ROLE"
echo "----------------------------------------------------------------"
echo -e "/role select \"Rust Engineer\"\n/search async\n/quit" | $BINARY repl 2>/dev/null | head -20 | tail -15
echo ""

echo "4. SWITCHING TO 'Default' ROLE"
echo "----------------------------------------------------------------"
echo -e "/role select Default\n/role list\n/quit" | $BINARY repl 2>/dev/null | grep -A 10 -E "(Switched to role:|Available roles:)"
echo ""

echo "5. PERFORMING SEARCH WITH 'Default' ROLE"
echo "----------------------------------------------------------------"
echo -e "/role select Default\n/search async\n/quit" | $BINARY repl 2>/dev/null | head -20 | tail -15
echo ""

echo "6. CONFIG VERIFICATION - Showing role persistence"
echo "----------------------------------------------------------------"
echo -e "/role select \"Terraphim Engineer\"\n/config show\n/quit" | $BINARY repl 2>/dev/null | grep -B2 -A2 "selected_role"
echo ""

echo "==================================================================="
echo "DEMONSTRATION COMPLETE"
echo "===================================================================="
