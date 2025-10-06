#!/bin/bash
set -e

echo "🚀 Setting up Terraphim test environment with LOCAL services..."
echo "============================================================="

# Get the project root directory
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# Export test environment variables
if [ -f .env.test ]; then
    export $(cat .env.test | grep -v '^#' | grep -v '^$' | xargs)
    echo "✅ Loaded test environment from .env.test"
else
    echo "❌ .env.test file not found! Creating basic configuration..."
    cat > .env.test << 'EOF'
TERRAPHIM_SERVER_PORT=8000
ATOMIC_SERVER_URL=http://localhost:9883
ATOMIC_SERVER_SECRET=test-secret-key
MCP_SERVER_URL=http://localhost:8001
OLLAMA_BASE_URL=http://127.0.0.1:11434
OLLAMA_MODEL=llama3.2:3b
TEST_MODE=true
RUST_LOG=info
TERRAPHIM_INITIALIZED=true
EOF
    export $(cat .env.test | xargs)
fi

# Function to check if a port is in use
port_in_use() {
    netstat -an | grep -q ":$1.*LISTEN" 2>/dev/null || lsof -i :$1 >/dev/null 2>&1
}

# Function to wait for a service to be ready
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    
    echo "⏳ Waiting for $name to be ready..."
    for i in $(seq 1 $max_attempts); do
        if curl -s "$url" > /dev/null 2>&1; then
            echo "   ✓ $name is ready!"
            return 0
        fi
        if [ $i -eq $max_attempts ]; then
            echo "   ⚠️ $name did not start within ${max_attempts} seconds"
            return 1
        fi
        sleep 1
    done
}

# Stop any existing services (except Ollama which is managed externally)
echo ""
echo "🛑 Stopping existing Terraphim services..."
pkill -f atomic-server 2>/dev/null || true
pkill -f terraphim_mcp_server 2>/dev/null || true  
pkill -f terraphim_server 2>/dev/null || true

# Clean up old PID files
rm -f /tmp/atomic-server.pid /tmp/mcp.pid /tmp/terraphim.pid

echo ""
echo "📋 Checking service dependencies..."

# Check Ollama is running
echo "1️⃣ Checking Ollama..."
if ! pgrep -f "ollama" > /dev/null; then
    echo "   ❌ Ollama is not running. Please start Ollama first."
    echo "      On macOS: Open Ollama.app or run 'ollama serve'"
    echo "      On Linux: Run 'ollama serve'"
    exit 1
fi
echo "   ✓ Ollama is running"

# Verify required Ollama model is available
if ! ollama list | grep -q "llama3.2:3b"; then
    echo "   📥 Pulling llama3.2:3b model..."
    ollama pull llama3.2:3b || {
        echo "   ⚠️ Failed to pull model, continuing anyway..."
    }
else
    echo "   ✓ llama3.2:3b model is available"
fi

# Check Atomic Server binary
echo ""
echo "2️⃣ Checking Atomic Server..."
ATOMIC_BINARY="../atomic-server/target/release/atomic-server"
if [ -f "$ATOMIC_BINARY" ]; then
    echo "   ✓ Atomic Server binary found at $ATOMIC_BINARY"
else
    echo "   ⚠️ Atomic Server binary not found at $ATOMIC_BINARY"
    echo "      Continuing without Atomic Server..."
    ATOMIC_BINARY=""
fi

echo ""
echo "🔧 Starting services..."

# Start local Atomic Server if available
if [ -n "$ATOMIC_BINARY" ]; then
    echo "3️⃣ Starting Atomic Server on port 9883..."
    if port_in_use 9883; then
        echo "   ⚠️ Port 9883 is already in use, skipping Atomic Server"
    else
        cd "$(dirname "$ATOMIC_BINARY")"
        nohup ./atomic-server --port 9883 --initialize > /tmp/atomic-server.log 2>&1 &
        echo $! > /tmp/atomic-server.pid
        cd "$PROJECT_ROOT"
        
        # Wait for Atomic Server with timeout
        if wait_for_service "http://localhost:9883" "Atomic Server"; then
            echo "   ✅ Atomic Server started successfully"
        else
            echo "   ⚠️ Atomic Server may not be fully ready, check /tmp/atomic-server.log"
        fi
    fi
else
    echo "3️⃣ Skipping Atomic Server (binary not found)"
fi

# Build and start MCP Server
echo ""
echo "4️⃣ Starting MCP Server on port 8001..."
if port_in_use 8001; then
    echo "   ⚠️ Port 8001 is already in use, skipping MCP Server"
else
    cd crates/terraphim_mcp_server
    echo "   🔨 Building MCP Server..."
    cargo build --release
    echo "   🚀 Starting MCP Server..."
    nohup cargo run --release -- --sse --bind 127.0.0.1:8001 > /tmp/mcp.log 2>&1 &
    echo $! > /tmp/mcp.pid
    cd "$PROJECT_ROOT"
    
    # Wait a moment for MCP server to start
    sleep 3
    echo "   ✅ MCP Server started (check /tmp/mcp.log for details)"
fi

# Build and start Terraphim Server
echo ""
echo "5️⃣ Starting Terraphim Server on port 8000..."
if port_in_use 8000; then
    echo "   ⚠️ Port 8000 is already in use, skipping Terraphim Server"
else
    echo "   🔨 Building Terraphim Server..."
    cargo build --release -p terraphim_server
    echo "   🚀 Starting Terraphim Server..."
    nohup cargo run --release -p terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json > /tmp/terraphim.log 2>&1 &
    echo $! > /tmp/terraphim.pid
    
    # Wait for Terraphim Server
    if wait_for_service "http://localhost:8000/health" "Terraphim Server"; then
        echo "   ✅ Terraphim Server started successfully"
    else
        echo "   ⚠️ Terraphim Server may not be fully ready, check /tmp/terraphim.log"
    fi
fi

# Final service verification
echo ""
echo "✅ Service Status Summary:"
echo "=========================="

echo -n "🧠 Ollama (11434): "
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✓ Ready"
else
    echo "✗ Not responding"
fi

echo -n "⚛️  Atomic Server (9883): "
if curl -s http://localhost:9883 > /dev/null 2>&1; then
    echo "✓ Ready"
else
    echo "✗ Not responding"
fi

echo -n "🔗 MCP Server (8001): "
if curl -s http://localhost:8001 > /dev/null 2>&1; then
    echo "✓ Ready"
else
    echo "⚠️ May be in stdio mode (check /tmp/mcp.log)"
fi

echo -n "🌐 Terraphim Server (8000): "
if curl -s http://localhost:8000/health > /dev/null 2>&1; then
    echo "✓ Ready"
else
    echo "✗ Not responding"
fi

echo ""
echo "📝 Service Information:"
echo "======================"
echo "📁 Logs available at:"
echo "   • Atomic Server: /tmp/atomic-server.log"
echo "   • MCP Server: /tmp/mcp.log"
echo "   • Terraphim Server: /tmp/terraphim.log"
echo ""
echo "🔧 Process IDs stored in:"
echo "   • Atomic Server: /tmp/atomic-server.pid"
echo "   • MCP Server: /tmp/mcp.pid"
echo "   • Terraphim Server: /tmp/terraphim.pid"
echo ""
echo "🧪 Run tests with:"
echo "   cargo test --workspace"
echo "   cargo test -p terraphim_service"
echo "   cargo test -p terraphim_middleware"
echo "   RUST_LOG=debug cargo test -- --nocapture"
echo ""
echo "🛑 To stop services:"
echo "   ./scripts/test_env_teardown.sh"
echo ""
echo "🎯 Test environment is ready!"