#!/bin/bash
set -e

BASE_URL="${1:-http://localhost:8080}"
echo "Testing MCP API at $BASE_URL"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

function test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local data="$4"
    local expected_status="${5:-200}"

    echo -e "${YELLOW}Testing: $name${NC}"

    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi

    # Split response and status code
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    if [ "$status_code" = "$expected_status" ]; then
        echo -e "${GREEN}✓ $name - Status: $status_code${NC}"
        echo "$body" | jq '.' 2>/dev/null || echo "$body"
        return 0
    else
        echo -e "${RED}✗ $name - Expected: $expected_status, Got: $status_code${NC}"
        echo "$body"
        return 1
    fi
}

echo "======================================"
echo "Phase 4 End-to-End Testing"
echo "======================================"
echo ""

# Test 1: Health Check
echo "Test 1: Health Check"
test_endpoint "Health Endpoint" "GET" "/metamcp/health"
echo ""

# Test 2: List Namespaces (should be empty initially)
echo "Test 2: List Namespaces"
test_endpoint "List Namespaces" "GET" "/metamcp/namespaces"
echo ""

# Test 3: Create a Namespace
echo "Test 3: Create Namespace"
NAMESPACE_DATA='{
  "name": "test-namespace",
  "description": "End-to-end test namespace",
  "user_id": "test-user",
  "config_json": "{\"name\":\"test-namespace\",\"servers\":[{\"name\":\"test-server\",\"transport\":\"STDIO\",\"command\":\"echo\",\"args\":[\"test\"],\"url\":null,\"bearer_token\":null,\"env\":null}],\"tool_overrides\":{},\"enabled\":true}",
  "enabled": true,
  "visibility": "Private"
}'
test_endpoint "Create Namespace" "POST" "/metamcp/namespaces" "$NAMESPACE_DATA"
NAMESPACE_UUID=$(curl -s -X POST "$BASE_URL/metamcp/namespaces" \
    -H "Content-Type: application/json" \
    -d "$NAMESPACE_DATA" | jq -r '.namespace.uuid' 2>/dev/null || echo "")
echo "Created Namespace UUID: $NAMESPACE_UUID"
echo ""

# Test 4: Get Namespace
if [ -n "$NAMESPACE_UUID" ] && [ "$NAMESPACE_UUID" != "null" ]; then
    echo "Test 4: Get Namespace"
    test_endpoint "Get Namespace" "GET" "/metamcp/namespaces/$NAMESPACE_UUID"
    echo ""
fi

# Test 5: Create Endpoint
echo "Test 5: Create Endpoint"
ENDPOINT_DATA="{
  \"name\": \"test-endpoint\",
  \"namespace_uuid\": \"$NAMESPACE_UUID\",
  \"auth_type\": \"none\",
  \"user_id\": \"test-user\",
  \"enabled\": true
}"
test_endpoint "Create Endpoint" "POST" "/metamcp/endpoints" "$ENDPOINT_DATA"
ENDPOINT_UUID=$(curl -s -X POST "$BASE_URL/metamcp/endpoints" \
    -H "Content-Type: application/json" \
    -d "$ENDPOINT_DATA" | jq -r '.endpoint.uuid' 2>/dev/null || echo "")
echo "Created Endpoint UUID: $ENDPOINT_UUID"
echo ""

# Test 6: List Endpoints
echo "Test 6: List Endpoints"
test_endpoint "List Endpoints" "GET" "/metamcp/endpoints"
echo ""

# Test 7: Get Endpoint
if [ -n "$ENDPOINT_UUID" ] && [ "$ENDPOINT_UUID" != "null" ]; then
    echo "Test 7: Get Endpoint"
    test_endpoint "Get Endpoint" "GET" "/metamcp/endpoints/$ENDPOINT_UUID"
    echo ""
fi

# Test 8: List Tools for Endpoint
if [ -n "$ENDPOINT_UUID" ] && [ "$ENDPOINT_UUID" != "null" ]; then
    echo "Test 8: List Tools for Endpoint"
    test_endpoint "List Tools" "GET" "/metamcp/endpoints/$ENDPOINT_UUID/tools"
    echo ""
fi

# Test 9: Execute Tool (will fail with placeholder, but tests the endpoint)
if [ -n "$ENDPOINT_UUID" ] && [ "$ENDPOINT_UUID" != "null" ]; then
    echo "Test 9: Execute Tool"
    TOOL_DATA='{"arguments": {"test": "value"}}'
    test_endpoint "Execute Tool" "POST" "/metamcp/endpoints/$ENDPOINT_UUID/tools/test__tool" "$TOOL_DATA" "500" || true
    echo "(Expected to fail with placeholder - MCP client integration pending)"
    echo ""
fi

# Test 10: Get Audit Trail
echo "Test 10: Get Audit Trail"
test_endpoint "List Audits" "GET" "/metamcp/audits"
echo ""

# Test 11: Get OpenAPI Spec
echo "Test 11: Get OpenAPI Specification"
test_endpoint "OpenAPI Spec" "GET" "/metamcp/openapi.json"
echo ""

# Test 12: Verify OpenAPI Schema
echo "Test 12: Verify OpenAPI Schema Structure"
OPENAPI_RESPONSE=$(curl -s "$BASE_URL/metamcp/openapi.json")
if echo "$OPENAPI_RESPONSE" | jq -e '.openapi' >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Valid OpenAPI structure${NC}"
    echo "OpenAPI Version: $(echo "$OPENAPI_RESPONSE" | jq -r '.openapi')"
    echo "API Title: $(echo "$OPENAPI_RESPONSE" | jq -r '.info.title')"
    echo "API Version: $(echo "$OPENAPI_RESPONSE" | jq -r '.info.version')"
    echo "Paths:"
    echo "$OPENAPI_RESPONSE" | jq -r '.paths | keys[]' | sed 's/^/  - /'
else
    echo -e "${RED}✗ Invalid OpenAPI structure${NC}"
fi
echo ""

# Cleanup
if [ -n "$ENDPOINT_UUID" ] && [ "$ENDPOINT_UUID" != "null" ]; then
    echo "Cleanup: Delete Endpoint"
    test_endpoint "Delete Endpoint" "DELETE" "/metamcp/endpoints/$ENDPOINT_UUID" "" "204" || true
    echo ""
fi

if [ -n "$NAMESPACE_UUID" ] && [ "$NAMESPACE_UUID" != "null" ]; then
    echo "Cleanup: Delete Namespace"
    test_endpoint "Delete Namespace" "DELETE" "/metamcp/namespaces/$NAMESPACE_UUID" "" "204" || true
    echo ""
fi

echo "======================================"
echo -e "${GREEN}End-to-End Testing Complete!${NC}"
echo "======================================"
