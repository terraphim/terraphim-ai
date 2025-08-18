#!/bin/bash

# End-to-End KG Lookup Test Script
# Tests the complete KG lookup flow from server startup to API response

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 Terraphim KG Lookup E2E Test${NC}"
echo -e "${BLUE}===============================${NC}\n"

# Function to cleanup on exit
cleanup() {
    if [ ! -z "$SERVER_PID" ]; then
        echo -e "\n${YELLOW}🧹 Cleaning up server process...${NC}"
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo -e "${RED}❌ Error: Please run this script from the terraphim-ai project root${NC}"
    exit 1
fi

# Step 1: Validate setup
echo -e "${BLUE}📋 Step 1: Validating setup...${NC}"
if ! ./scripts/validate_kg_setup.sh; then
    echo -e "${RED}❌ Setup validation failed. Please fix the issues above.${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Setup validation passed${NC}\n"

# Step 2: Build server
echo -e "${BLUE}🔨 Step 2: Building server...${NC}"
if ! cargo build --bin terraphim_server; then
    echo -e "${RED}❌ Server build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Server built successfully${NC}\n"

# Step 3: Start server in background
echo -e "${BLUE}🚀 Step 3: Starting server...${NC}"
cargo run --bin terraphim_server > server.log 2>&1 &
SERVER_PID=$!

echo -e "${GREEN}✅ Server started with PID: $SERVER_PID${NC}"
echo -e "${BLUE}⏳ Waiting for server startup and KG building...${NC}"

# Wait for server to be ready and build KG
MAX_WAIT=60
WAIT_COUNT=0
SERVER_READY=false

while [ $WAIT_COUNT -lt $MAX_WAIT ]; do
    if curl -s http://127.0.0.1:8000/health > /dev/null 2>&1; then
        SERVER_READY=true
        break
    fi
    sleep 2
    WAIT_COUNT=$((WAIT_COUNT + 2))
    echo -e "${YELLOW}  Waiting... (${WAIT_COUNT}s/${MAX_WAIT}s)${NC}"
done

if [ "$SERVER_READY" = false ]; then
    echo -e "${RED}❌ Server failed to start within ${MAX_WAIT} seconds${NC}"
    echo -e "${RED}📄 Server logs:${NC}"
    cat server.log
    exit 1
fi

echo -e "${GREEN}✅ Server is ready${NC}\n"

# Step 4: Check configuration
echo -e "${BLUE}🔧 Step 4: Checking server configuration...${NC}"
CONFIG_RESPONSE=$(curl -s http://127.0.0.1:8000/config)
echo -e "Config response: $CONFIG_RESPONSE" | head -c 200
echo "..."

# Check if Terraphim Engineer role exists
if echo "$CONFIG_RESPONSE" | grep -q "Terraphim Engineer"; then
    echo -e "${GREEN}✅ Terraphim Engineer role found in server config${NC}"
else
    echo -e "${RED}❌ Terraphim Engineer role not found in server config${NC}"
    echo -e "${RED}📄 Full config response:${NC}"
    echo "$CONFIG_RESPONSE" | jq . 2>/dev/null || echo "$CONFIG_RESPONSE"
    exit 1
fi

# Step 5: Test KG lookup API
echo -e "\n${BLUE}🔍 Step 5: Testing KG lookup API...${NC}"

# Test terms that should exist in the knowledge graph
TEST_TERMS=("service" "haystack" "terraphim-graph" "graph" "system")

for term in "${TEST_TERMS[@]}"; do
    echo -e "${BLUE}Testing term: '$term'${NC}"
    
    # URL encode the role name and term
    ENCODED_ROLE=$(python3 -c "import urllib.parse; print(urllib.parse.quote('Terraphim Engineer'))")
    ENCODED_TERM=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$term'))")
    
    URL="http://127.0.0.1:8000/roles/${ENCODED_ROLE}/kg_search?term=${ENCODED_TERM}"
    echo -e "  URL: $URL"
    
    RESPONSE=$(curl -s "$URL")
    STATUS=$(echo "$RESPONSE" | jq -r '.status' 2>/dev/null || echo "unknown")
    RESULTS_COUNT=$(echo "$RESPONSE" | jq -r '.results | length' 2>/dev/null || echo "0")
    
    echo -e "  Status: $STATUS"
    echo -e "  Results: $RESULTS_COUNT"
    
    if [ "$STATUS" = "success" ] && [ "$RESULTS_COUNT" -gt "0" ]; then
        echo -e "  ${GREEN}✅ Found $RESULTS_COUNT documents for '$term'${NC}"
        
        # Show the top result
        FIRST_TITLE=$(echo "$RESPONSE" | jq -r '.results[0].title' 2>/dev/null || echo "N/A")
        FIRST_RANK=$(echo "$RESPONSE" | jq -r '.results[0].rank' 2>/dev/null || echo "N/A")
        echo -e "  📄 Top result: '$FIRST_TITLE' (rank: $FIRST_RANK)"
    else
        echo -e "  ${YELLOW}⚠️  No results for '$term'${NC}"
        # Show first 200 chars of response for debugging
        echo -e "  Response preview: $(echo "$RESPONSE" | head -c 200)..."
    fi
    echo ""
done

# Step 6: Test with expected working term
echo -e "${BLUE}🎯 Step 6: Testing with specific expected terms...${NC}"

# These should definitely work if KG is built correctly
EXPECTED_TERMS=("service" "haystack")
SUCCESS_COUNT=0

for term in "${EXPECTED_TERMS[@]}"; do
    ENCODED_ROLE=$(python3 -c "import urllib.parse; print(urllib.parse.quote('Terraphim Engineer'))")
    ENCODED_TERM=$(python3 -c "import urllib.parse; print(urllib.parse.quote('$term'))")
    URL="http://127.0.0.1:8000/roles/${ENCODED_ROLE}/kg_search?term=${ENCODED_TERM}"
    
    RESPONSE=$(curl -s "$URL")
    STATUS=$(echo "$RESPONSE" | jq -r '.status' 2>/dev/null || echo "unknown")
    RESULTS_COUNT=$(echo "$RESPONSE" | jq -r '.results | length' 2>/dev/null || echo "0")
    
    if [ "$STATUS" = "success" ] && [ "$RESULTS_COUNT" -gt "0" ]; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        echo -e "${GREEN}✅ '$term' lookup successful${NC}"
    else
        echo -e "${RED}❌ '$term' lookup failed${NC}"
    fi
done

# Step 7: Final evaluation
echo -e "\n${BLUE}📊 Step 7: Final evaluation...${NC}"

if [ $SUCCESS_COUNT -eq ${#EXPECTED_TERMS[@]} ]; then
    echo -e "${GREEN}🎉 SUCCESS: All expected KG lookups working correctly!${NC}"
    echo -e "${GREEN}✅ Your KG lookup functionality is properly configured${NC}"
    EXIT_CODE=0
else
    echo -e "${RED}❌ FAILURE: Only $SUCCESS_COUNT out of ${#EXPECTED_TERMS[@]} expected lookups working${NC}"
    echo -e "${RED}❌ Please check server logs for KG building issues${NC}"
    EXIT_CODE=1
fi

# Show relevant server logs
echo -e "\n${BLUE}📄 Relevant server logs:${NC}"
echo -e "${BLUE}=======================${NC}"
grep -E "(Knowledge graph|Building rolegraph|Found.*markdown files|Successfully loaded|KG|role)" server.log | tail -20 || echo "No relevant logs found"

# Cleanup
cleanup

exit $EXIT_CODE 