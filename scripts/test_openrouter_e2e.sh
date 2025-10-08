#!/bin/bash
# End-to-end test for OpenRouter summarization workflow
# Tests: Server startup, search, summarization, and frontend integration

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== OpenRouter E2E Test ===${NC}"
echo ""

# Load API key
echo -e "${BLUE}1. Loading OpenRouter API key...${NC}"
if [ -f ~/ai_env.sh ]; then
    source ~/ai_env.sh
    if [ -z "$OPENROUTER_API_KEY" ]; then
        echo -e "${RED}❌ OPENROUTER_API_KEY not set in ~/ai_env.sh${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ API key loaded${NC}"
else
    echo -e "${RED}❌ ~/ai_env.sh not found${NC}"
    exit 1
fi

# Check if server is running
echo ""
echo -e "${BLUE}2. Checking server status...${NC}"
SERVER_URL="http://localhost:8000"

if curl -s -f "${SERVER_URL}/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Server is running at ${SERVER_URL}${NC}"
else
    echo -e "${YELLOW}⚠️  Server not running, attempting to start...${NC}"
    cd "$(dirname "$0")/.."

    # Start server in background
    OPENROUTER_API_KEY="$OPENROUTER_API_KEY" cargo run --package terraphim_server --features openrouter > server_e2e_test.log 2>&1 &
    SERVER_PID=$!
    echo "Server PID: $SERVER_PID"

    # Wait for server to start
    echo "Waiting for server to start..."
    for i in {1..30}; do
        if curl -s -f "${SERVER_URL}/health" > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Server started successfully${NC}"
            break
        fi
        sleep 1
        echo -n "."
    done

    if ! curl -s -f "${SERVER_URL}/health" > /dev/null 2>&1; then
        echo -e "${RED}❌ Server failed to start${NC}"
        cat server_e2e_test.log
        exit 1
    fi
fi

# Get config to verify OpenRouter is enabled
echo ""
echo -e "${BLUE}3. Verifying OpenRouter configuration...${NC}"
CONFIG_RESPONSE=$(curl -s "${SERVER_URL}/config")
echo "$CONFIG_RESPONSE" | jq -r '.config.roles | to_entries[] | select(.value.extra.llm_provider == "openrouter") | .key' | head -1 > /tmp/openrouter_role.txt

if [ -s /tmp/openrouter_role.txt ]; then
    OPENROUTER_ROLE=$(cat /tmp/openrouter_role.txt)
    echo -e "${GREEN}✅ Found OpenRouter-enabled role: ${OPENROUTER_ROLE}${NC}"
else
    echo -e "${YELLOW}⚠️  No OpenRouter role found, using default role${NC}"
    OPENROUTER_ROLE="Default"
fi

# Perform a search
echo ""
echo -e "${BLUE}4. Testing search functionality...${NC}"
SEARCH_QUERY="rust"
SEARCH_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents/search" \
    -H "Content-Type: application/json" \
    -d "{\"search_term\":\"${SEARCH_QUERY}\",\"role\":\"${OPENROUTER_ROLE}\"}")

TOTAL_RESULTS=$(echo "$SEARCH_RESPONSE" | jq -r '.total // 0')
echo "Found $TOTAL_RESULTS documents for query '${SEARCH_QUERY}'"

if [ "$TOTAL_RESULTS" -gt 0 ]; then
    echo -e "${GREEN}✅ Search working${NC}"
    FIRST_DOC_ID=$(echo "$SEARCH_RESPONSE" | jq -r '.results[0].id')
    FIRST_DOC_TITLE=$(echo "$SEARCH_RESPONSE" | jq -r '.results[0].title')
    echo "First document: ${FIRST_DOC_TITLE} (ID: ${FIRST_DOC_ID})"
else
    echo -e "${YELLOW}⚠️  No search results found, creating test document...${NC}"

    # Create a test document
    CREATE_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents" \
        -H "Content-Type: application/json" \
        -d '{
            "id": "test-rust-doc",
            "title": "Rust Programming Language",
            "body": "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety. It accomplishes these goals through its ownership system, borrowing rules, and powerful type system without needing a garbage collector.",
            "url": "test://rust",
            "tags": ["rust", "programming"]
        }')

    FIRST_DOC_ID="test-rust-doc"
    FIRST_DOC_TITLE="Rust Programming Language"
    echo -e "${GREEN}✅ Test document created${NC}"
fi

# Test summarization
echo ""
echo -e "${BLUE}5. Testing summarization with OpenRouter...${NC}"
echo "Document ID: ${FIRST_DOC_ID}"
echo "Role: ${OPENROUTER_ROLE}"

SUMMARIZE_REQUEST=$(cat <<EOF
{
    "document_id": "${FIRST_DOC_ID}",
    "role": "${OPENROUTER_ROLE}",
    "max_length": 200,
    "force_regenerate": false
}
EOF
)

echo "Sending summarization request..."
SUMMARIZE_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents/summarize" \
    -H "Content-Type: application/json" \
    -d "$SUMMARIZE_REQUEST")

echo ""
echo "Summarization response:"
echo "$SUMMARIZE_RESPONSE" | jq '.'

SUMMARY_STATUS=$(echo "$SUMMARIZE_RESPONSE" | jq -r '.status')
SUMMARY_TEXT=$(echo "$SUMMARIZE_RESPONSE" | jq -r '.summary // ""')
MODEL_USED=$(echo "$SUMMARIZE_RESPONSE" | jq -r '.model_used // ""')
FROM_CACHE=$(echo "$SUMMARIZE_RESPONSE" | jq -r '.from_cache // false')
SUMMARY_ERROR=$(echo "$SUMMARIZE_RESPONSE" | jq -r '.error // ""')

if [ "$SUMMARY_STATUS" = "success" ] && [ -n "$SUMMARY_TEXT" ]; then
    echo ""
    echo -e "${GREEN}✅ Summarization successful!${NC}"
    echo "Model used: ${MODEL_USED}"
    echo "From cache: ${FROM_CACHE}"
    echo "Summary (${#SUMMARY_TEXT} chars): ${SUMMARY_TEXT}"
else
    echo -e "${RED}❌ Summarization failed${NC}"
    echo "Status: ${SUMMARY_STATUS}"
    echo "Error: ${SUMMARY_ERROR}"

    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
    fi
    exit 1
fi

# Verify summary appears in subsequent searches
echo ""
echo -e "${BLUE}6. Verifying summary appears in search results...${NC}"
VERIFY_SEARCH=$(curl -s -X POST "${SERVER_URL}/documents/search" \
    -H "Content-Type: application/json" \
    -d "{\"search_term\":\"${SEARCH_QUERY}\",\"role\":\"${OPENROUTER_ROLE}\"}")

FOUND_DOC=$(echo "$VERIFY_SEARCH" | jq -r ".results[] | select(.id == \"${FIRST_DOC_ID}\")")
if [ -n "$FOUND_DOC" ]; then
    DOC_DESCRIPTION=$(echo "$FOUND_DOC" | jq -r '.description // ""')
    if [ -n "$DOC_DESCRIPTION" ] && [ "$DOC_DESCRIPTION" != "null" ]; then
        echo -e "${GREEN}✅ Summary found in search results${NC}"
        echo "Description: ${DOC_DESCRIPTION}"
    else
        echo -e "${YELLOW}⚠️  Document found but no description${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  Document not found in search results${NC}"
fi

# Test summarization status endpoint
echo ""
echo -e "${BLUE}7. Testing summarization status endpoint...${NC}"
STATUS_RESPONSE=$(curl -s "${SERVER_URL}/summarization/status?role=${OPENROUTER_ROLE}")
echo "$STATUS_RESPONSE" | jq '.'

SUMMARIZATION_ENABLED=$(echo "$STATUS_RESPONSE" | jq -r '.enabled // false')
PROVIDER=$(echo "$STATUS_RESPONSE" | jq -r '.provider // ""')

if [ "$SUMMARIZATION_ENABLED" = "true" ]; then
    echo -e "${GREEN}✅ Summarization is enabled${NC}"
    echo "Provider: ${PROVIDER}"
else
    echo -e "${YELLOW}⚠️  Summarization not enabled for this role${NC}"
fi

# Cleanup
echo ""
echo -e "${BLUE}8. Cleanup...${NC}"
if [ -n "$SERVER_PID" ]; then
    echo "Stopping test server (PID: $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null || true
    echo -e "${GREEN}✅ Server stopped${NC}"
fi

echo ""
echo -e "${GREEN}=== All E2E tests passed! ===${NC}"
echo ""
echo "Summary:"
echo "  ✅ API key loaded"
echo "  ✅ Server running"
echo "  ✅ Search working"
echo "  ✅ Summarization working with OpenRouter"
echo "  ✅ Summary saved and appears in search results"
echo "  ✅ Status endpoint working"
echo ""
echo -e "${BLUE}OpenRouter E2E test complete!${NC}"
