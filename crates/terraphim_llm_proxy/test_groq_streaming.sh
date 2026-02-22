#!/bin/bash

# Test streaming with Groq to capture detailed logs
echo "🧪 Testing Groq streaming with enhanced logging..."

# Load API keys from environment or 1Password
if [ -z "$GROQ_API_KEY" ]; then
    echo "🔑 Loading GROQ_API_KEY from 1Password..."
    export GROQ_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/groq-api-key")
fi

if [ -z "$OPENROUTER_API_KEY" ]; then
    echo "🔑 Loading OPENROUTER_API_KEY from 1Password..."
    export OPENROUTER_API_KEY=$(op read "op://TerraphimPlatform/TruthForge.api-keys/openrouter-api-key")
fi

# Kill any existing proxy on port 3459
pkill -f "terraphim-llm-proxy.*3459" || true
sleep 2

# Start proxy with detailed logging on port 3459
echo "🚀 Starting proxy with Groq on port 3459..."
RUST_LOG=debug ./target/release/terraphim-llm-proxy --config config.with-groq.toml --port 3459 > groq_streaming_debug.log 2>&1 &
PROXY_PID=$!

echo "📝 Proxy PID: $PROXY_PID"
sleep 3

# Test streaming request
echo "📡 Sending streaming request to Groq..."
curl -X POST http://127.0.0.1:3459/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk_test_proxy_key_for_claude_code_testing_12345" \
  -d '{
    "model": "groq:llama-3.1-8b-instant",
    "messages": [{"role": "user", "content": "What is 2+2? Answer in one word."}],
    "stream": true,
    "max_tokens": 10
  }' \
  --no-buffer -v > groq_streaming_response.log 2>&1

echo "📊 Curl exit code: $?"

# Wait a moment for logs to be written
sleep 2

# Kill proxy
kill $PROXY_PID 2>/dev/null || true

echo "✅ Streaming test completed"
echo "📋 Check the following files:"
echo "  - groq_streaming_debug.log (proxy logs)"
echo "  - groq_streaming_response.log (curl response)"

# Show the last 50 lines of proxy logs
echo -e "\n🔍 Last 50 lines of proxy debug logs:"
tail -50 groq_streaming_debug.log