#!/bin/bash

# Test streaming with OpenRouter to verify it works as well as Groq
echo "🧪 Testing OpenRouter streaming with enhanced logging..."

# Load API keys from environment or 1Password
if [ -z "$OPENROUTER_API_KEY" ]; then
    echo "🔑 Loading OPENROUTER_API_KEY from 1Password..."
    export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
fi

# Kill any existing proxy on port 3459
pkill -f "terraphim-llm-proxy.*3459" || true
sleep 2

# Start proxy with detailed logging on port 3459
echo "🚀 Starting proxy with OpenRouter on port 3459..."
RUST_LOG=debug ./target/release/terraphim-llm-proxy --config config.with-groq.toml --port 3459 > openrouter_streaming_debug.log 2>&1 &
PROXY_PID=$!

echo "📝 Proxy PID: $PROXY_PID"
sleep 3

# Test streaming request
echo "📡 Sending streaming request to OpenRouter..."
curl -X POST http://127.0.0.1:3459/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "openrouter:anthropic/claude-sonnet-4.5",
    "messages": [{"role": "user", "content": "What is 2+2? Answer in one word."}],
    "stream": true,
    "max_tokens": 10
  }' \
  --no-buffer -v > openrouter_streaming_response.log 2>&1

echo "📊 Curl exit code: $?"

# Wait a moment for logs to be written
sleep 2

# Kill proxy
kill $PROXY_PID 2>/dev/null || true

echo "✅ OpenRouter streaming test completed"
echo "📋 Check the following files:"
echo "  - openrouter_streaming_debug.log (proxy logs)"
echo "  - openrouter_streaming_response.log (curl response)"

# Show the last 50 lines of proxy logs
echo -e "\n🔍 Last 50 lines of proxy debug logs:"
tail -50 openrouter_streaming_debug.log