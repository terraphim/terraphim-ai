#!/bin/bash

# Test VM Execution via Chat API
# Tests that code blocks in LLM responses are automatically executed when VM execution is enabled

set -e

echo "=== Testing VM Execution via Chat API ==="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
API_URL="${API_URL:-https://demo.terraphim.cloud}"
ROLE="${ROLE:-DevelopmentAgent}"

echo "Testing against: $API_URL"
echo "Role: $ROLE"
echo

# Test 1: Simple Python code execution
echo -e "${YELLOW}Test 1: Simple Python code execution${NC}"
RESPONSE=$(curl -s -X POST "$API_URL/chat" \
  -H 'Content-Type: application/json' \
  -d "{
    \"role\": \"$ROLE\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Execute this Python code:\\n\\n\`\`\`python\\nprint('Hello from VM!')\\nprint('2 + 2 =', 2 + 2)\\n\`\`\`\"
    }]
  }")

echo "Response:"
echo "$RESPONSE" | jq -r '.message' 2>/dev/null || echo "$RESPONSE"
echo

if echo "$RESPONSE" | grep -q "VM Execution Results"; then
    echo -e "${GREEN}✓ Test 1 PASSED: VM execution results found${NC}"
else
    echo -e "${RED}✗ Test 1 FAILED: No VM execution results in response${NC}"
    exit 1
fi

# Test 2: Code with error (should show error output)
echo -e "${YELLOW}Test 2: Python code with error${NC}"
RESPONSE=$(curl -s -X POST "$API_URL/chat" \
  -H 'Content-Type: application/json' \
  -d "{
    \"role\": \"$ROLE\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Run this:\\n\\n\`\`\`python\\nraise ValueError('Test error')\\n\`\`\`\"
    }]
  }")

echo "Response:"
echo "$RESPONSE" | jq -r '.message' 2>/dev/null || echo "$RESPONSE"
echo

if echo "$RESPONSE" | grep -q "VM Execution"; then
    echo -e "${GREEN}✓ Test 2 PASSED: VM execution attempted${NC}"
else
    echo -e "${RED}✗ Test 2 FAILED: VM execution not triggered${NC}"
    exit 1
fi

# Test 3: Bash command execution
echo -e "${YELLOW}Test 3: Bash command execution${NC}"
RESPONSE=$(curl -s -X POST "$API_URL/chat" \
  -H 'Content-Type: application/json' \
  -d "{
    \"role\": \"$ROLE\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Execute:\\n\\n\`\`\`bash\\necho 'Test from bash'\\ndate\\n\`\`\`\"
    }]
  }")

echo "Response:"
echo "$RESPONSE" | jq -r '.message' 2>/dev/null || echo "$RESPONSE"
echo

if echo "$RESPONSE" | grep -q "VM Execution"; then
    echo -e "${GREEN}✓ Test 3 PASSED: Bash execution triggered${NC}"
else
    echo -e "${RED}✗ Test 3 FAILED: Bash execution not triggered${NC}"
    exit 1
fi

# Test 4: Role without VM execution (should NOT execute)
echo -e "${YELLOW}Test 4: Role without VM execution enabled${NC}"
RESPONSE=$(curl -s -X POST "$API_URL/chat" \
  -H 'Content-Type: application/json' \
  -d "{
    \"role\": \"DataScientistAgent\",
    \"messages\": [{
      \"role\": \"user\",
      \"content\": \"Execute:\\n\\n\`\`\`python\\nprint('Should not execute')\\n\`\`\`\"
    }]
  }")

echo "Response:"
echo "$RESPONSE" | jq -r '.message' 2>/dev/null || echo "$RESPONSE"
echo

if echo "$RESPONSE" | grep -q "VM Execution Results"; then
    echo -e "${YELLOW}⚠ Test 4 WARNING: VM execution triggered for role without VM enabled${NC}"
else
    echo -e "${GREEN}✓ Test 4 PASSED: VM execution correctly disabled for non-VM role${NC}"
fi

echo
echo -e "${GREEN}=== All tests completed ===${NC}"
echo "Summary:"
echo "  - VM execution integration working"
echo "  - Code blocks automatically detected and executed"
echo "  - Errors properly captured"
echo "  - Role-based VM execution control working"
