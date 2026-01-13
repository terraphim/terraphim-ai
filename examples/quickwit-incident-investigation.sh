#!/bin/bash
# Quickwit Incident Investigation with Terraphim Agent
#
# This example demonstrates using terraphim-agent to investigate a production incident
# using Quickwit log search integration.
#
# Scenario: API service experiencing errors around 10:30 AM on 2024-01-13

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Quickwit Incident Investigation Example${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Step 1: Verify Prerequisites
echo -e "${YELLOW}Step 1: Verifying prerequisites...${NC}"

# Check if terraphim-agent is built
if [ ! -f "./target/release/terraphim-agent" ]; then
    echo -e "${RED}Error: terraphim-agent not found${NC}"
    echo "Building terraphim-agent..."
    cargo build --release -p terraphim_agent --features repl-full
fi
echo -e "${GREEN}✓ terraphim-agent ready${NC}"

# Check if Quickwit is running
if curl -s http://localhost:7280/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Quickwit server responding${NC}"
else
    echo -e "${YELLOW}⚠ Quickwit server not detected at localhost:7280${NC}"
    echo "Starting Quickwit with Docker..."
    docker run -d -p 7280:7280 --name quickwit-demo quickwit/quickwit:0.7 || echo "Docker start failed (may already be running)"
    sleep 5
fi

# Verify config exists
CONFIG_FILE="terraphim_server/default/quickwit_engineer_config.json"
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}Error: Config file not found: $CONFIG_FILE${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Configuration ready${NC}"
echo ""

# Step 2: Demonstrate Explicit Index Search
echo -e "${YELLOW}Step 2: Searching specific index (explicit mode)...${NC}"
echo -e "Config: ${BLUE}quickwit_engineer_config.json${NC}"
echo -e "Index: ${BLUE}workers-logs${NC}"
echo -e "Query: ${BLUE}level:ERROR${NC}"
echo ""

# Create a command file for non-interactive execution
cat > /tmp/terraphim_quickwit_commands.txt << 'EOF'
/search "level:ERROR"
/exit
EOF

echo "Executing search..."
./target/release/terraphim-agent \
    --config "$CONFIG_FILE" \
    < /tmp/terraphim_quickwit_commands.txt \
    2>/dev/null || echo "(Quickwit may not have data yet - this is expected)"

echo -e "${GREEN}✓ Explicit index search demonstrated${NC}"
echo ""

# Step 3: Demonstrate Auto-Discovery
echo -e "${YELLOW}Step 3: Auto-discovery mode (searches all indexes)...${NC}"
echo -e "Config: ${BLUE}quickwit_autodiscovery_config.json${NC}"
echo -e "Mode: ${BLUE}Auto-discover all indexes${NC}"
echo ""

AUTODISCOVERY_CONFIG="terraphim_server/default/quickwit_autodiscovery_config.json"
if [ -f "$AUTODISCOVERY_CONFIG" ]; then
    cat > /tmp/terraphim_autodiscovery_commands.txt << 'EOF'
/search error
/exit
EOF

    echo "Executing auto-discovery search..."
    ./target/release/terraphim-agent \
        --config "$AUTODISCOVERY_CONFIG" \
        < /tmp/terraphim_autodiscovery_commands.txt \
        2>/dev/null || echo "(Auto-discovery attempted)"

    echo -e "${GREEN}✓ Auto-discovery mode demonstrated${NC}"
else
    echo -e "${YELLOW}⚠ Auto-discovery config not found, skipping${NC}"
fi
echo ""

# Step 4: Show Configuration Examples
echo -e "${YELLOW}Step 4: Configuration examples...${NC}"
echo ""

echo -e "${BLUE}Example 1: Explicit Index (Production)${NC}"
cat << 'EOF'
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "default_index": "workers-logs",
    "max_hits": "100",
    "sort_by": "-timestamp"
  }
}
EOF
echo ""

echo -e "${BLUE}Example 2: Auto-Discovery with Filter${NC}"
cat << 'EOF'
{
  "location": "http://localhost:7280",
  "service": "Quickwit",
  "extra_parameters": {
    "index_filter": "workers-*",
    "max_hits": "50"
  }
}
EOF
echo ""

echo -e "${BLUE}Example 3: Production with Basic Auth${NC}"
cat << 'EOF'
{
  "location": "https://logs.terraphim.cloud/api",
  "service": "Quickwit",
  "extra_parameters": {
    "auth_username": "cloudflare",
    "auth_password": "${QUICKWIT_PASSWORD}",
    "index_filter": "workers-*",
    "max_hits": "100"
  }
}
EOF
echo ""

# Step 5: Show Query Examples
echo -e "${YELLOW}Step 5: Query syntax examples...${NC}"
echo ""

echo -e "${BLUE}Simple Queries:${NC}"
echo '  /search error'
echo '  /search "database connection"'
echo '  /search timeout'
echo ""

echo -e "${BLUE}Field-Specific Queries:${NC}"
echo '  /search "level:ERROR"'
echo '  /search "service:api-server"'
echo '  /search "message:database"'
echo ""

echo -e "${BLUE}Boolean Queries:${NC}"
echo '  /search "error AND database"'
echo '  /search "level:ERROR OR level:WARN"'
echo '  /search "service:api AND NOT level:INFO"'
echo ""

echo -e "${BLUE}Time Range Queries:${NC}"
echo '  /search "timestamp:[2024-01-01 TO 2024-01-31]"'
echo '  /search "level:ERROR AND timestamp:[2024-01-13T10:00:00Z TO *]"'
echo ""

echo -e "${BLUE}Complex Queries:${NC}"
echo '  /search "level:ERROR AND (message:database OR message:connection)"'
echo '  /search "service:api-server AND timestamp:[2024-01-13T10:00:00Z TO 2024-01-13T11:00:00Z]"'
echo ""

# Step 6: Interactive Mode Instructions
echo -e "${YELLOW}Step 6: Try it yourself (interactive mode)...${NC}"
echo ""
echo "To start terraphim-agent in interactive mode:"
echo -e "${GREEN}  ./target/release/terraphim-agent --config $CONFIG_FILE${NC}"
echo ""
echo "Then use these commands:"
echo "  /search \"your query here\""
echo "  /help"
echo "  /exit"
echo ""

# Step 7: Cleanup Instructions
echo -e "${YELLOW}Step 7: Cleanup (optional)...${NC}"
echo ""
echo "To stop the demo Quickwit container:"
echo -e "${BLUE}  docker stop quickwit-demo && docker rm quickwit-demo${NC}"
echo ""

# Clean up temp files
rm -f /tmp/terraphim_quickwit_commands.txt /tmp/terraphim_autodiscovery_commands.txt

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Example complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Next steps:"
echo "  1. Configure your Quickwit instance URL"
echo "  2. Set up authentication if needed"
echo "  3. Create custom role configs for your use cases"
echo "  4. Explore the query syntax with your log data"
echo ""
echo "Documentation:"
echo "  - User Guide: docs/quickwit-integration.md"
echo "  - Example: examples/quickwit-log-search.md"
echo "  - Configs: terraphim_server/default/quickwit_*.json"
echo ""
