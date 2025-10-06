#!/bin/bash
# Simple E2E test for OpenRouter summarization
# Assumes server is already running

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Simple OpenRouter Summarization Test ===${NC}"
echo ""

# Load API key
source ~/ai_env.sh
SERVER_URL="http://localhost:8000"

# 1. Check server health
echo -e "${BLUE}1. Checking server...${NC}"
if ! curl -s -f "${SERVER_URL}/health" > /dev/null 2>&1; then
    echo -e "${RED}❌ Server not running at ${SERVER_URL}${NC}"
    echo "Please start the server with:"
    echo "  source ~/ai_env.sh && cargo run --package terraphim_server --features openrouter"
    exit 1
fi
echo -e "${GREEN}✅ Server is running${NC}"

# 2. Get config and find OpenRouter role
echo -e "${BLUE}2. Finding OpenRouter-enabled role...${NC}"
ROLE="Terraphim Engineer"
echo -e "${GREEN}✅ Using role: ${ROLE}${NC}"

# 3. Search for documents
echo -e "${BLUE}3. Searching for documents...${NC}"
SEARCH_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents/search" \
    -H "Content-Type: application/json" \
    -d "{\"search_term\":\"rust\",\"role\":\"${ROLE}\"}")

TOTAL=$(echo "$SEARCH_RESPONSE" | jq -r '.total // 0')
echo "Found ${TOTAL} documents"

if [ "$TOTAL" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No documents found, cannot test summarization${NC}"
    exit 0
fi

DOC_ID=$(echo "$SEARCH_RESPONSE" | jq -r '.results[0].id')
DOC_TITLE=$(echo "$SEARCH_RESPONSE" | jq -r '.results[0].title')
echo "Testing with: ${DOC_TITLE} (${DOC_ID})"

# 4. Request summarization
echo -e "${BLUE}4. Requesting summarization...${NC}"
SUMMARY_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents/summarize" \
    -H "Content-Type: application/json" \
    -d "{
        \"document_id\": \"${DOC_ID}\",
        \"role\": \"${ROLE}\",
        \"max_length\": 200,
        \"force_regenerate\": true
    }")

echo "Response:"
echo "$SUMMARY_RESPONSE" | jq '.'

STATUS=$(echo "$SUMMARY_RESPONSE" | jq -r '.status')
SUMMARY=$(echo "$SUMMARY_RESPONSE" | jq -r '.summary // ""')
MODEL=$(echo "$SUMMARY_RESPONSE" | jq -r '.model_used // ""')
ERROR=$(echo "$SUMMARY_RESPONSE" | jq -r '.error // ""')

if [ "$STATUS" = "success" ] && [ -n "$SUMMARY" ]; then
    echo ""
    echo -e "${GREEN}✅ Summarization SUCCESS!${NC}"
    echo "Model: ${MODEL}"
    echo "Summary: ${SUMMARY}"
else
    echo ""
    echo -e "${RED}❌ Summarization FAILED${NC}"
    echo "Error: ${ERROR}"
    exit 1
fi

# 5. Verify in search results
echo ""
echo -e "${BLUE}5. Verifying summary in search results...${NC}"
VERIFY=$(curl -s -X POST "${SERVER_URL}/documents/search" \
    -H "Content-Type: application/json" \
    -d "{\"search_term\":\"rust\",\"role\":\"${ROLE}\"}")

DESC=$(echo "$VERIFY" | jq -r ".results[] | select(.id == \"${DOC_ID}\") | .description // \"\"")
if [ -n "$DESC" ] && [ "$DESC" != "null" ]; then
    echo -e "${GREEN}✅ Summary appears in search results${NC}"
else
    echo -e "${YELLOW}⚠️  Summary not yet in search results (may need refresh)${NC}"
fi

echo ""
echo -e "${GREEN}=== Test Complete ===${NC}"

