#!/bin/bash
# Start Terraphim LLM Proxy using 1Password CLI for secrets
# Requires: op CLI installed and authenticated

set -e

CONFIG_FILE="${1:-config.toml}"

echo "Starting Terraphim LLM Proxy with 1Password secrets..."
echo "Configuration: $CONFIG_FILE"
echo ""

# Check if op CLI is available
if ! command -v op &> /dev/null; then
    echo "Error: 1Password CLI (op) not found"
    echo "Install: https://developer.1password.com/docs/cli/get-started/"
    exit 1
fi

# Check if authenticated
if ! op account list &> /dev/null; then
    echo "Error: Not authenticated with 1Password"
    echo "Run: op signin"
    exit 1
fi

echo "1Password CLI: ✅ Available and authenticated"
echo "Starting proxy with injected secrets..."
echo ""

# Start proxy with op run to inject secrets
op run --env-file=.env.op -- ./target/release/terraphim-llm-proxy --config "$CONFIG_FILE"
