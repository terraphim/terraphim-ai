#!/bin/bash
# Generate temporary Tauri keys for testing
# Usage: ./scripts/generate-tauri-keys.sh

set -euo pipefail

echo "🔐 Generating temporary Tauri signing keys..."

# Generate keys in desktop directory
cd desktop
cargo tauri keygen --name "Terraphim Test" --email "test@terraphim.ai"

echo ""
echo "✅ Keys generated successfully!"
echo ""
echo "📋 Generated files:"
ls -la .tauri/ 2>/dev/null || echo "No .tauri directory found"

echo ""
echo "⚠️ IMPORTANT:"
echo "These are TEST keys for development only!"
echo "Generate production keys using:"
echo "cargo tauri keygen --name 'Terraphim Platform' --email 'releases@terraphim.ai'"
echo ""

if [[ -d ".tauri" ]]; then
    echo "🔑 Key contents:"
    echo "Private key: .tauri/terraphim-test.key"
    echo "Public key: .tauri/terraphim-test.pub"
    echo "Credential: .tauri/terraphim-test.cred"
    
    echo ""
    echo "📝 Adding keys to tauri.conf.json..."
    
    # Update tauri.conf.json with generated keys
    private_key=$(cat .tauri/terraphim-test.key | tr -d '\n' | tr -d '\r')
    public_key=$(cat .tauri/terraphim-test.pub | tr -d '\n' | tr -d '\r')
    
    # Update tauri.conf.json (this needs manual editing or jq)
    echo ""
    echo "⚠️ Please manually update src-tauri/tauri.conf.json with:"
    echo "{ \"tauri\": { \"bundle\": { \"signing\": { \"privateKey\": \"$private_key\", \"publicKey\": \"$public_key\" } } } }"
fi