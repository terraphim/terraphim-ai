#!/bin/bash

# Start MCP server with local dev settings (non-locking database)
export TERRAPHIM_SETTINGS_PATH="$(pwd)/../terraphim_settings/default/settings_local_dev.toml"

echo "Starting MCP server with local dev settings..."
echo "Settings path: $TERRAPHIM_SETTINGS_PATH"
echo "Using non-locking database backends (memory, dashmap, sqlite)"

# Start the server
cargo run -- --sse --bind 127.0.0.1:8001 --verbose
