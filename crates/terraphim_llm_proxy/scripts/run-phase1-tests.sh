#!/bin/bash
# Phase 1: Basic Proxy Validation Tests
# Tests proxy startup, health, and token counting

set -e

echo "═══════════════════════════════════════════════════════════"
echo "Phase 1: Proxy Setup & Basic Validation"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Use the API key from config.toml (hardcoded for testing)
export PROXY_API_KEY="sk_test_key_12345678901234567890123456789012"

echo "Environment:"
echo "  PROXY_API_KEY: ${PROXY_API_KEY:0:30}..."
echo "  Config: config.toml"
echo ""

# Start proxy in background
echo "Starting proxy..."
RUST_LOG=info ./target/release/terraphim-llm-proxy --config config.toml > /tmp/proxy-test.log 2>&1 &
PROXY_PID=$!

# Wait for startup
sleep 3

# Test 1: Health Check
echo "Test 1: Health endpoint"
HEALTH=$(curl -s http://localhost:3456/health)
if [ "$HEALTH" = "OK" ]; then
    echo "  ✅ PASS: Health endpoint returns OK"
else
    echo "  ❌ FAIL: Health returned: $HEALTH"
    kill $PROXY_PID
    exit 1
fi
echo ""

# Test 2: Token Counting
echo "Test 2: Token counting"
RESPONSE=$(curl -s -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Hello, world!"}]}')

TOKENS=$(echo "$RESPONSE" | jq -r '.input_tokens' 2>/dev/null)

if [ -n "$TOKENS" ] && [ "$TOKENS" -ge 3 ] && [ "$TOKENS" -le 12 ]; then
    echo "  ✅ PASS: Token count = $TOKENS (expected 3-12)"
else
    echo "  ❌ FAIL: Token count = $TOKENS (expected 3-12)"
    echo "  Response: $RESPONSE"
    kill $PROXY_PID
    exit 1
fi
echo ""

# Test 3: Authentication - Valid Key
echo "Test 3: Valid API key accepted"
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' \
  -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: $PROXY_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Test"}]}')

if [ "$HTTP_CODE" = "200" ]; then
    echo "  ✅ PASS: Valid API key accepted (HTTP $HTTP_CODE)"
else
    echo "  ❌ FAIL: HTTP $HTTP_CODE (expected 200)"
    kill $PROXY_PID
    exit 1
fi
echo ""

# Test 4: Authentication - Invalid Key
echo "Test 4: Invalid API key rejected"
HTTP_CODE=$(curl -s -o /dev/null -w '%{http_code}' \
  -X POST http://localhost:3456/v1/messages/count_tokens \
  -H "x-api-key: wrong_key" \
  -H "Content-Type: application/json" \
  -d '{"model":"test","messages":[{"role":"user","content":"Test"}]}')

if [ "$HTTP_CODE" = "401" ]; then
    echo "  ✅ PASS: Invalid API key rejected (HTTP $HTTP_CODE)"
else
    echo "  ❌ FAIL: HTTP $HTTP_CODE (expected 401)"
    kill $PROXY_PID
    exit 1
fi
echo ""

# Cleanup
echo "Stopping proxy..."
kill $PROXY_PID
wait $PROXY_PID 2>/dev/null || true

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "Phase 1 Complete: All basic validation tests passed ✅"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Next: Run Phase 2 tests with Claude Code integration"
echo "Command: claude --settings proxy-settings.json --print 'Hello'"
