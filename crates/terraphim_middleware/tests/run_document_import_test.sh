#!/bin/bash

# Script to run the document import test for Terraphim Atomic Server integration
# This test imports documents from the /src path into Atomic Server and searches them

set -e

echo "🚀 Terraphim Document Import Test"
echo "=================================="

# Check if Atomic Server is running
echo "Checking if Atomic Server is running..."
if ! curl -s http://localhost:9883 > /dev/null; then
    echo "❌ Atomic Server is not running on http://localhost:9883"
    echo "Please start Atomic Server first:"
    echo "  atomic-server --port 9883"
    exit 1
fi
echo "✅ Atomic Server is running"

# Check if .env file exists
if [ ! -f "../../../.env" ]; then
    echo "❌ .env file not found in project root"
    echo "Please create a .env file with:"
    echo "  ATOMIC_SERVER_URL=http://localhost:9883"
    echo "  ATOMIC_SERVER_SECRET=your_secret_here"
    exit 1
fi
echo "✅ .env file found"

# Check if src directory exists
if [ ! -d "../../../docs/src" ]; then
    echo "❌ docs/src directory not found in project root"
    echo "This test requires markdown files in the docs/src directory"
    exit 1
fi
echo "✅ docs/src directory found"

# Count markdown files
MD_COUNT=$(find ../../../docs/src -name "*.md" | wc -l)
echo "📄 Found $MD_COUNT markdown files in docs/src directory"

# Run the test
echo ""
echo "Running document import test..."
echo "This test will:"
echo "  1. Import up to 10 markdown files from src/ into Atomic Server"
echo "  2. Search the imported documents"
echo "  3. Verify search results"
echo "  4. Clean up imported documents"
echo ""

cd ../../..

# Run the specific test
cargo test --package terraphim_middleware test_document_import_and_search -- --nocapture

echo ""
echo "✅ Document import test completed!"
echo ""
echo "To run other tests:"
echo "  cargo test --package terraphim_middleware test_single_document_import_and_search -- --nocapture"
echo "  cargo test --package terraphim_middleware test_document_import_edge_cases -- --nocapture" 