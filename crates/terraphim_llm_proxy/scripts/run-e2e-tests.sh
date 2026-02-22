#!/bin/bash
# Automated E2E test suite for Terraphim LLM Proxy
# Tests all routing scenarios and validates functionality

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROXY_URL="http://localhost:3456"
TEST_API_KEY="${PROXY_API_KEY:-sk_test_e2e_proxy_key_for_validation_12345678901234567890}"

echo "==================================="
echo "Terraphim LLM Proxy E2E Test Suite"
echo "==================================="
echo "Proxy URL: $PROXY_URL"
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Counters
PASS=0
FAIL=0

# Test function
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -n "Testing: $test_name... "

    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        PASS=$((PASS + 1))
        return 0
    else
        echo -e "${RED}FAIL${NC}"
        FAIL=$((FAIL + 1))
        return 1
    fi
}

# Test 1: Health Check
echo "=== Basic Connectivity Tests ==="
run_test "Health endpoint" \
    "curl -sf $PROXY_URL/health | grep -q 'OK'"

# Test 2: Authentication - Missing API Key
run_test "Reject missing API key" \
    "test \$(curl -s -o /dev/null -w '%{http_code}' -X POST $PROXY_URL/v1/messages/count_tokens \
    -H 'Content-Type: application/json' \
    -d '{\"model\":\"test\",\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}') -eq 401"

# Test 3: Authentication - Invalid API Key
run_test "Reject invalid API key" \
    "test \$(curl -s -o /dev/null -w '%{http_code}' -X POST $PROXY_URL/v1/messages/count_tokens \
    -H 'Content-Type: application/json' \
    -H 'x-api-key: wrong_key' \
    -d '{\"model\":\"test\",\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}') -eq 401"

# Test 4: Authentication - Valid API Key
run_test "Accept valid API key" \
    "test \$(curl -s -o /dev/null -w '%{http_code}' -X POST $PROXY_URL/v1/messages/count_tokens \
    -H 'Content-Type: application/json' \
    -H \"x-api-key: $TEST_API_KEY\" \
    -d '{\"model\":\"test\",\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}') -eq 200"

echo ""
echo "=== Token Counting Tests ==="

# Test 5: Token Counting - Simple Text
echo -n "Testing: Token counting simple text... "
TOKENS=$(curl -s -X POST $PROXY_URL/v1/messages/count_tokens \
  -H "x-api-key: $TEST_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello, world!"}]}' \
  | jq -r '.input_tokens')

if [ "$TOKENS" -ge 3 ] && [ "$TOKENS" -le 6 ]; then
    echo -e "${GREEN}PASS${NC} (tokens: $TOKENS, expected: 3-6)"
    PASS=$((PASS + 1))
else
    echo -e "${RED}FAIL${NC} (tokens: $TOKENS, expected: 3-6)"
    FAIL=$((FAIL + 1))
fi

# Test 6: Token Counting - With System Prompt
echo -n "Testing: Token counting with system prompt... "
TOKENS=$(curl -s -X POST $PROXY_URL/v1/messages/count_tokens \
  -H "x-api-key: $TEST_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model":"test",
    "messages":[{"role":"user","content":"Hello!"}],
    "system":"You are a helpful assistant."
  }' | jq -r '.input_tokens')

if [ "$TOKENS" -ge 10 ]; then
    echo -e "${GREEN}PASS${NC} (tokens: $TOKENS, expected: >10)"
    PASS=$((PASS + 1))
else
    echo -e "${RED}FAIL${NC} (tokens: $TOKENS, expected: >10)"
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Request Format Tests ==="

# Test 7: Bearer Token Authentication
run_test "Bearer token authentication" \
    "test \$(curl -s -o /dev/null -w '%{http_code}' -X POST $PROXY_URL/v1/messages/count_tokens \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Bearer $TEST_API_KEY' \
    -d '{\"model\":\"test\",\"messages\":[{\"role\":\"user\",\"content\":\"Hi\"}]}') -eq 200"

# Test 8: JSON Response Format
echo -n "Testing: JSON response format... "
RESPONSE=$(curl -s -X POST $PROXY_URL/v1/messages/count_tokens \
  -H "x-api-key: $TEST_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Test"}]}')

if echo "$RESPONSE" | jq -e '.input_tokens' > /dev/null 2>&1; then
    echo -e "${GREEN}PASS${NC}"
    PASS=$((PASS + 1))
else
    echo -e "${RED}FAIL${NC}"
    echo "Response: $RESPONSE"
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Test Summary ==="
echo -e "Passed: ${GREEN}$PASS${NC}"
echo -e "Failed: ${RED}$FAIL${NC}"
echo "Total: $((PASS + FAIL))"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
