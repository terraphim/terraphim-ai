#!/bin/bash
# test_server_api.sh - Comprehensive Server API functionality test

set -euo pipefail

SERVER_BINARY="./target/release/terraphim_server"
SERVER_URL="http://localhost:8000"
TEST_LOG="server_test_results_$(date +%Y%m%d_%H%M%S).log"
PASS_COUNT=0
FAIL_COUNT=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Terraphim Server API Functional Test ===" | tee $TEST_LOG
echo "Started at: $(date)" | tee -a $TEST_LOG
echo "" | tee -a $TEST_LOG

# Start server
echo "Starting server..." | tee -a $TEST_LOG
$SERVER_BINARY &
SERVER_PID=$!
sleep 3

# Ensure cleanup on exit
trap "echo 'Cleaning up...'; kill $SERVER_PID 2>/dev/null || true" EXIT

# Function to test endpoint
test_endpoint() {
    local method="$1"
    local endpoint="$2"
    local data="$3"
    local expected="$4"
    local description="$5"

    echo -e "${YELLOW}Testing:${NC} $description" | tee -a $TEST_LOG
    echo "Endpoint: $method $endpoint" | tee -a $TEST_LOG

    # Execute request
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\nHTTP_STATUS:%{http_code}" "$SERVER_URL$endpoint" || true)
    else
        response=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -X "$method" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$SERVER_URL$endpoint" || true)
    fi

    # Extract status code
    http_status=$(echo "$response" | grep HTTP_STATUS | cut -d: -f2)
    body=$(echo "$response" | grep -v HTTP_STATUS)

    # Check response
    if [ "$http_status" -ge 200 ] && [ "$http_status" -lt 300 ]; then
        if [ -n "$expected" ] && echo "$body" | grep -qi "$expected"; then
            echo -e "${GREEN}✓ PASS${NC} (Status: $http_status)" | tee -a $TEST_LOG
            ((PASS_COUNT++))
        elif [ -z "$expected" ]; then
            echo -e "${GREEN}✓ PASS${NC} (Status: $http_status)" | tee -a $TEST_LOG
            ((PASS_COUNT++))
        else
            echo -e "${RED}✗ FAIL${NC} - Expected content not found" | tee -a $TEST_LOG
            echo "Expected: $expected" | tee -a $TEST_LOG
            echo "Got: $body" | tee -a $TEST_LOG
            ((FAIL_COUNT++))
        fi
    else
        echo -e "${RED}✗ FAIL${NC} - HTTP Status: $http_status" | tee -a $TEST_LOG
        echo "Response: $body" | tee -a $TEST_LOG
        ((FAIL_COUNT++))
    fi
    echo "---" | tee -a $TEST_LOG
}

# Test all endpoints
echo "=== API Endpoint Tests ===" | tee -a $TEST_LOG

# Health check
test_endpoint "GET" "/health" "" "OK" "Health check endpoint"

# Configuration
test_endpoint "GET" "/config" "" "status.*success" "Get configuration"
test_endpoint "POST" "/config" '{"selected_role":"Default"}' "" "Update configuration"

# Search
test_endpoint "POST" "/search" '{"query":"test","role":"Default"}' "" "Search documents"

# Chat
test_endpoint "POST" "/chat" '{"message":"Hello","conversation_id":"test123"}' "" "Send chat message"

# Roles
test_endpoint "GET" "/roles" "" "" "Get available roles"

# Thesaurus
test_endpoint "GET" "/thesaurus/Default" "" "" "Get thesaurus for role"

# Autocomplete
test_endpoint "POST" "/autocomplete" '{"query":"ter","role":"Default"}' "" "Get autocomplete suggestions"

# Error handling tests
echo -e "\n=== Error Handling Tests ===" | tee -a $TEST_LOG

# Test invalid endpoint
response=$(curl -s -o /dev/null -w "%{http_code}" "$SERVER_URL/invalid_endpoint")
if [ "$response" = "404" ]; then
    echo -e "${GREEN}✓ PASS${NC} - Invalid endpoint returns 404" | tee -a $TEST_LOG
    ((PASS_COUNT++))
else
    echo -e "${RED}✗ FAIL${NC} - Invalid endpoint returned $response instead of 404" | tee -a $TEST_LOG
    ((FAIL_COUNT++))
fi

# Test malformed JSON
response=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d "{invalid json}" \
    "$SERVER_URL/search")
if [ "$response" = "400" ] || [ "$response" = "422" ]; then
    echo -e "${GREEN}✓ PASS${NC} - Malformed JSON returns 400/422" | tee -a $TEST_LOG
    ((PASS_COUNT++))
else
    echo -e "${RED}✗ FAIL${NC} - Malformed JSON returned $response" | tee -a $TEST_LOG
    ((FAIL_COUNT++))
fi

# Performance test
echo -e "\n=== Performance Tests ===" | tee -a $TEST_LOG

# Measure response time
start_time=$(date +%s%N)
curl -s "$SERVER_URL/health" > /dev/null
end_time=$(date +%s%N)
response_time=$(( (end_time - start_time) / 1000000 ))

if [ $response_time -lt 100 ]; then
    echo -e "${GREEN}✓ PASS${NC} - Health check response time: ${response_time}ms" | tee -a $TEST_LOG
    ((PASS_COUNT++))
else
    echo -e "${YELLOW}⚠ SLOW${NC} - Health check response time: ${response_time}ms" | tee -a $TEST_LOG
fi

# Generate summary
echo -e "\n=== Test Summary ===" | tee -a $TEST_LOG
echo "Total Tests: $((PASS_COUNT + FAIL_COUNT))" | tee -a $TEST_LOG
echo -e "${GREEN}Passed: $PASS_COUNT${NC}" | tee -a $TEST_LOG
echo -e "${RED}Failed: $FAIL_COUNT${NC}" | tee -a $TEST_LOG
echo "Pass Rate: $(( PASS_COUNT * 100 / (PASS_COUNT + FAIL_COUNT) ))%" | tee -a $TEST_LOG
echo "Completed at: $(date)" | tee -a $TEST_LOG

# Exit with status
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}" | tee -a $TEST_LOG
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}" | tee -a $TEST_LOG
    exit 1
fi
