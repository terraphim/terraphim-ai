#!/bin/bash

# Script to start Terraphim server from the agent-workflows directory
# This ensures proper paths and directory structure

set -e  # Exit on any error

# Get the absolute path to the terraphim-ai project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "🚀 Starting Terraphim AI Server from examples/agent-workflows"
echo "📁 Project root: $PROJECT_ROOT"

# Change to project root directory
cd "$PROJECT_ROOT"

# Ensure required directories exist
echo "📋 Checking required directories..."
if [ ! -d "docs/src" ]; then
    echo "⚠️  Creating docs/src directory for haystack..."
    mkdir -p docs/src
    echo "ℹ️  This is a sample document for testing search functionality." > docs/src/sample.md
    echo "🔍 You can add more markdown files here for search indexing." >> docs/src/sample.md
fi

if [ ! -d "terraphim_server/fixtures/haystack" ]; then
    echo "⚠️  Creating terraphim_server/fixtures/haystack directory..."
    mkdir -p terraphim_server/fixtures/haystack
    echo "ℹ️  This is a sample document in the fixtures haystack." > terraphim_server/fixtures/haystack/sample.md
fi

# Print current configuration
echo "🔧 Configuration:"
echo "   - Config file: terraphim_server/default/ollama_llama_config.json"
echo "   - Haystack path from config: docs/src"
echo "   - Knowledge graph path: docs/src/kg"
echo "   - Working directory: $(pwd)"

# Clear any existing saved config to force using our config file
echo "🧹 Clearing any cached config to ensure fresh start..."
rm -rf ~/.config/terraphim 2>/dev/null || true
rm -rf ~/.cache/terraphim 2>/dev/null || true

# Start the server and update config via REST API
echo "🌟 Starting server..."
cargo run --release &
SERVER_PID=$!

# Wait for server to be ready
echo "⏳ Waiting for server to start..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:8000/health > /dev/null 2>&1; then
        echo "✅ Server is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Server failed to start after 30 seconds"
        kill $SERVER_PID 2>/dev/null
        exit 1
    fi
    sleep 1
done

# Update configuration via REST API
echo "🔧 Updating server configuration via REST API..."
if curl -s -X POST http://127.0.0.1:8000/config \
    -H "Content-Type: application/json" \
    -d @"$PROJECT_ROOT/terraphim_server/default/ollama_llama_config.json" \
    > /dev/null; then
    echo "✅ Configuration updated successfully!"
else
    echo "❌ Failed to update configuration"
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

echo "🎉 Server is running with correct configuration!"
echo "📋 You can now test:"
echo "   - Search: curl -X GET 'http://127.0.0.1:8000/documents/search?search_term=test&role=Rust+Engineer'"
echo "   - Search (alt): curl -X GET 'http://127.0.0.1:8000/documents/search?query=test&role=Rust+Engineer'"
echo "   - Workflows: /workflows/route, /workflows/parallel, etc."
echo "   - Auto-summarization: POST to /documents/summarize"
echo ""
echo "🛑 Press Ctrl+C to stop the server"

# Keep script running and forward signals to server
trap "echo '🛑 Stopping server...'; kill $SERVER_PID 2>/dev/null; exit 0" INT TERM
wait $SERVER_PID
