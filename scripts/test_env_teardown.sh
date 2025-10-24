#!/bin/bash

echo "🛑 Stopping Terraphim test services..."
echo "====================================="

# Function to stop a service by PID file
stop_service() {
    local service_name=$1
    local pid_file=$2
    local process_pattern=$3

    echo "Stopping $service_name..."

    # Try to stop using PID file first
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo "  • Stopping $service_name (PID: $pid)"
            kill "$pid" 2>/dev/null
            sleep 2

            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                echo "  • Force stopping $service_name"
                kill -9 "$pid" 2>/dev/null
            fi
        fi
        rm -f "$pid_file"
    else
        echo "  • No PID file found for $service_name"
    fi

    # Fallback: kill by process pattern
    if [ -n "$process_pattern" ]; then
        pkill -f "$process_pattern" 2>/dev/null && echo "  • Stopped remaining $service_name processes"
    fi
}

# Stop Terraphim Server
stop_service "Terraphim Server" "/tmp/terraphim.pid" "terraphim_server"

# Stop MCP Server
stop_service "MCP Server" "/tmp/mcp.pid" "terraphim_mcp_server"

# Stop Atomic Server
stop_service "Atomic Server" "/tmp/atomic-server.pid" "atomic-server.*--port 9883"

# Clean up any remaining log files
echo ""
echo "🧹 Cleaning up log files..."
for log_file in /tmp/atomic-server.log /tmp/mcp.log /tmp/terraphim.log; do
    if [ -f "$log_file" ]; then
        echo "  • Removing $log_file"
        rm -f "$log_file"
    fi
done

# Verify ports are free
echo ""
echo "✅ Verifying ports are free..."
for port in 8000 8001 9883; do
    if netstat -an 2>/dev/null | grep -q ":$port.*LISTEN" || lsof -i :$port >/dev/null 2>&1; then
        echo "  ⚠️  Port $port is still in use"
    else
        echo "  ✓ Port $port is free"
    fi
done

echo ""
echo "📝 Notes:"
echo "• Ollama is left running (managed externally)"
echo "• To stop Ollama: Close Ollama.app or run 'pkill -f ollama'"
echo "• If ports are still in use, you may need to manually kill processes"
echo ""
echo "✅ Teardown complete!"
