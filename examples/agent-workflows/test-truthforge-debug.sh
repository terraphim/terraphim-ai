#!/bin/bash
set -e

echo "🧪 TruthForge Debug Mode Test Suite"
echo "===================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
PASSED=0
FAILED=0

# Test helper
test_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((PASSED++))
}

test_fail() {
    echo -e "${RED}✗${NC} $1"
    ((FAILED++))
}

test_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Check if backend is running
echo "1️⃣  Checking Backend Server..."
if curl -s http://localhost:8000/health > /dev/null 2>&1; then
    test_pass "Backend server is running on port 8000"
else
    test_fail "Backend server is NOT running"
    echo ""
    echo "Please start the backend server first:"
    echo "  cargo run --features openrouter"
    exit 1
fi

# Check backend endpoints
echo ""
echo "2️⃣  Testing Backend API Endpoints..."

# Test /config endpoint
if curl -s http://localhost:8000/config | grep -q "status"; then
    test_pass "/config endpoint responds"
else
    test_fail "/config endpoint not working"
fi

# Test /list_openrouter_models endpoint
echo '{"api_key": null}' | curl -s -X POST http://localhost:8000/list_openrouter_models \
    -H "Content-Type: application/json" \
    -d @- > /tmp/models-response.json 2>&1

if grep -q "status" /tmp/models-response.json; then
    test_pass "/list_openrouter_models endpoint responds"
    test_info "Response: $(cat /tmp/models-response.json | jq -r '.status' 2>/dev/null || echo 'unknown')"
else
    test_fail "/list_openrouter_models endpoint not working"
fi

# Check frontend files
echo ""
echo "3️⃣  Checking Frontend Files..."

FILES=(
    "examples/agent-workflows/shared/debug-panel.js"
    "examples/agent-workflows/shared/debug-panel.css"
    "examples/agent-workflows/shared/settings-manager.js"
    "examples/agent-workflows/shared/settings-ui.js"
    "examples/agent-workflows/shared/settings-modal.html"
    "examples/agent-workflows/shared/api-client.js"
    "examples/agent-workflows/6-truthforge-debate/index.html"
    "examples/agent-workflows/6-truthforge-debate/app.js"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        test_pass "File exists: $file"
    else
        test_fail "File missing: $file"
    fi
done

# Check for debug mode integration in files
echo ""
echo "4️⃣  Validating Debug Mode Integration..."

if grep -q "enable-debug-mode" examples/agent-workflows/shared/settings-modal.html; then
    test_pass "Debug checkbox present in settings modal"
else
    test_fail "Debug checkbox missing from settings modal"
fi

if grep -q "enableDebugMode" examples/agent-workflows/shared/settings-manager.js; then
    test_pass "Debug settings in settings manager"
else
    test_fail "Debug settings missing from settings manager"
fi

if grep -q "setDebugMode" examples/agent-workflows/shared/api-client.js; then
    test_pass "Debug methods in API client"
else
    test_fail "Debug methods missing from API client"
fi

if grep -q "DebugPanel" examples/agent-workflows/6-truthforge-debate/app.js; then
    test_pass "Debug panel integrated in TruthForge app"
else
    test_fail "Debug panel not integrated in TruthForge app"
fi

if grep -q "setupDebugMode" examples/agent-workflows/6-truthforge-debate/app.js; then
    test_pass "Debug setup method in TruthForge app"
else
    test_fail "Debug setup method missing from TruthForge app"
fi

# Check script paths
echo ""
echo "5️⃣  Validating Script Paths..."

if grep -q '../shared/debug-panel.js' examples/agent-workflows/6-truthforge-debate/index.html; then
    test_pass "Debug panel script path correct"
else
    test_fail "Debug panel script path incorrect"
fi

if grep -q '../shared/settings-manager.js' examples/agent-workflows/6-truthforge-debate/index.html; then
    test_pass "Settings manager script path correct"
else
    test_fail "Settings manager script path incorrect"
fi

if grep -q '../shared/api-client.js' examples/agent-workflows/6-truthforge-debate/index.html; then
    test_pass "API client script path correct"
else
    test_fail "API client script path incorrect"
fi

# Summary
echo ""
echo "======================================"
echo "Test Summary:"
echo -e "${GREEN}Passed: $PASSED${NC}"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED${NC}"
else
    echo -e "${GREEN}Failed: $FAILED${NC}"
fi
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Open http://localhost:8081/6-truthforge-debate/test-debug-mode.html"
    echo "  2. Check browser console for errors"
    echo "  3. Click 'Test Debug Logging' button"
    echo "  4. Verify debug panel shows entries"
    echo "  5. Open settings and toggle debug mode"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
