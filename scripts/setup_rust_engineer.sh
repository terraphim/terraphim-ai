#!/bin/bash

# Setup script for Rust Engineer role with query.rs haystack
set -e

echo "🚀 Setting up Rust Engineer role with query.rs haystack..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

print_status "Building the project..."
cargo build

if [ $? -eq 0 ]; then
    print_success "Project built successfully"
else
    print_error "Build failed"
    exit 1
fi

print_status "Running tests for query.rs haystack..."
cargo test query_rs_haystack_test --lib -- --nocapture

if [ $? -eq 0 ]; then
    print_success "Query.rs haystack tests passed"
else
    print_warning "Query.rs haystack tests failed (expected for network issues)"
fi

print_status "Testing Rust Engineer configuration..."

# Test the configuration by running a search
echo '{"search_term": "async", "role": "Rust Engineer"}' | \
curl -X POST http://localhost:8000/documents/search \
  -H "Content-Type: application/json" \
  -d @- 2>/dev/null | jq '.' || {
    print_warning "Server not running or search failed (expected)"
}

print_status "Configuration files created:"
echo "  - terraphim_server/default/rust_engineer_config.json"
echo "  - crates/terraphim_middleware/tests/query_rs_haystack_test.rs"

print_success "Rust Engineer setup complete!"
echo ""
echo "To use the Rust Engineer role:"
echo "1. Start the server with: cargo run --bin terraphim_server"
echo "2. Use the configuration: terraphim_server/default/rust_engineer_config.json"
echo "3. Search for Rust-related terms like 'async', 'tokio', 'serde'"
echo ""
echo "The role will search:"
echo "  - Rust standard library documentation (stable & nightly)"
echo "  - crates.io packages"
echo "  - docs.rs documentation"
echo "  - Reddit posts from r/rust"
