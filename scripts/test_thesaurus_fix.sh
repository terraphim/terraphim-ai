#!/bin/bash

# Test script to verify thesaurus loading fix
set -e

echo "🔧 Testing thesaurus loading fix..."

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Please run this script from the terraphim-ai root directory"
    exit 1
fi

# Check if docs/src/kg exists
if [ ! -d "docs/src/kg" ]; then
    echo "❌ docs/src/kg directory not found"
    exit 1
fi

echo "✅ Found docs/src/kg directory"

# Check for required markdown files
required_files=("terraphim-graph.md" "haystack.md" "service.md")
for file in "${required_files[@]}"; do
    if [ -f "docs/src/kg/$file" ]; then
        echo "✅ Found $file"
    else
        echo "⚠️  Missing $file (this might cause issues)"
    fi
done

# Clean up any existing sled databases
echo "🧹 Cleaning up existing sled databases..."
rm -rf /tmp/terraphim_engineer_sled
rm -rf /tmp/opendal/sled

# Test the service layer
echo "🧪 Testing service layer thesaurus loading..."
cd crates/terraphim_service
cargo test test_ensure_thesaurus_loaded_terraphim_engineer -- --nocapture

echo "✅ Thesaurus loading test completed!"

# Test the complete server configuration
echo "🧪 Testing complete server configuration..."
cd ../../terraphim_server
cargo test -- --nocapture

echo "✅ All tests completed successfully!"
