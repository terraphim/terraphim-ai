#!/bin/bash

echo "🧪 Testing Auto-Summarization with Ollama"

# Kill any existing servers
pkill -f "cargo run" 2>/dev/null || true
sleep 2

# Start server with Ollama configuration in background
echo "🚀 Starting server with Ollama configuration..."
RUST_LOG=debug cargo run --release --features ollama -- --config terraphim_server/default/ollama_llama_config.json > server.log 2>&1 &
SERVER_PID=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 15

# Check if server started successfully
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ Server failed to start. Check server.log for errors."
    cat server.log | tail -20
    exit 1
fi

# Look for server startup in logs
if grep -q "Started on" server.log; then
    echo "✅ Server started successfully"
    SERVER_URL=$(grep "Started on" server.log | head -1 | sed 's/.*Started on //')
    echo "🌐 Server URL: $SERVER_URL"
else
    echo "⚠️  Server may not have started properly. Logs:"
    cat server.log | tail -10
    SERVER_URL="http://127.0.0.1:8000"
fi

# Test search with auto-summarization
echo "🔍 Testing search with auto-summarization..."
SEARCH_RESPONSE=$(curl -s -X POST "${SERVER_URL}/documents/search" \
    -H "Content-Type: application/json" \
    -d '{"search_term": "rust", "role": "Llama Rust Engineer"}')

echo "📋 Search response received"

# Check if LLM client was built
echo "🔧 Checking LLM client creation logs..."
if grep -q "Building LLM client for role" server.log; then
    echo "✅ LLM client creation attempted"
    grep "Building LLM client for role" server.log | tail -3
else
    echo "❌ No LLM client creation logs found"
fi

# Check if auto-summarization was triggered
echo "🤖 Checking auto-summarization logs..."
if grep -q "Applying LLM AI summarization" server.log; then
    echo "✅ Auto-summarization triggered"
    grep "Applying LLM AI summarization" server.log | tail -3
else
    echo "❌ Auto-summarization not triggered"
fi

# Check if Ollama was called
echo "🦙 Checking Ollama API calls..."
if grep -q "Ollama" server.log; then
    echo "✅ Ollama mentions found"
    grep "Ollama" server.log | tail -5
else
    echo "❌ No Ollama API calls found"
fi

# Check search results
echo "📊 Analyzing search results..."
echo "$SEARCH_RESPONSE" | jq -r '.documents[0].description // "No description found"' | head -1

# Kill server
echo "🛑 Stopping server..."
kill $SERVER_PID 2>/dev/null || true
sleep 2

# Show final log summary
echo "📜 Final log summary..."
echo "Total log lines: $(wc -l < server.log)"
echo "Last 5 lines:"
tail -5 server.log

echo "✅ Test completed. Check server.log for full details."
