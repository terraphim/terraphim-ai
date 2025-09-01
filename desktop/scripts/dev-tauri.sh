#!/bin/bash

# Terraphim Tauri Development Script
# This script starts the Vite dev server first, then starts Tauri development

echo "🚀 Starting Terraphim Tauri Development..."

# Function to check if a port is open
wait_for_port() {
    local port=$1
    local max_attempts=30
    local attempt=1

    echo "⏳ Waiting for Vite dev server on port $port..."

    while [ $attempt -le $max_attempts ]; do
        if curl -s "http://localhost:$port" > /dev/null 2>&1; then
            echo "✅ Vite dev server is ready on port $port"
            return 0
        fi

        echo "   Attempt $attempt/$max_attempts - waiting for server..."
        sleep 2
        attempt=$((attempt + 1))
    done

    echo "❌ Timeout waiting for Vite dev server on port $port"
    return 1
}

# Start Vite dev server in background
echo "📦 Starting Vite dev server..."
yarn dev &
VITE_PID=$!

# Wait for Vite to be ready
if wait_for_port 5173; then
    echo "🎯 Starting Tauri development..."
    # Start Tauri dev
    yarn tauri:dev
else
    echo "❌ Failed to start Vite dev server"
    kill $VITE_PID 2>/dev/null
    exit 1
fi
