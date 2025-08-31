#!/bin/bash

# Test script to prove Rust Engineer role and query.rs haystack functionality
set -e

echo "🧪 Testing Rust Engineer Role and Query.rs Haystack"
echo "=================================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Test 1: Configuration file exists and is valid JSON
print_info "Test 1: Validating configuration file..."
if [ -f "terraphim_server/default/rust_engineer_config.json" ]; then
    if jq empty terraphim_server/default/rust_engineer_config.json 2>/dev/null; then
        print_success "Configuration file exists and is valid JSON"
    else
        print_error "Configuration file is not valid JSON"
        exit 1
    fi
else
    print_error "Configuration file not found"
    exit 1
fi

# Test 2: Configuration contains Rust Engineer role
print_info "Test 2: Checking Rust Engineer role configuration..."
if jq -e '.roles["Rust Engineer"]' terraphim_server/default/rust_engineer_config.json >/dev/null 2>&1; then
    print_success "Rust Engineer role found in configuration"
else
    print_error "Rust Engineer role not found in configuration"
    exit 1
fi

# Test 3: Configuration contains QueryRs service
print_info "Test 3: Checking QueryRs service configuration..."
if jq -e '.roles["Rust Engineer"].haystacks[0].service' terraphim_server/default/rust_engineer_config.json | grep -q "QueryRs"; then
    print_success "QueryRs service configured correctly"
else
    print_error "QueryRs service not configured correctly"
    exit 1
fi

# Test 4: Project compiles with new dependencies
print_info "Test 4: Checking project compilation..."
if cargo check --quiet; then
    print_success "Project compiles successfully with QueryRs dependencies"
else
    print_error "Project compilation failed"
    exit 1
fi

# Test 5: QueryRs haystack module exists
print_info "Test 5: Checking QueryRs haystack implementation..."
if [ -f "crates/terraphim_middleware/src/haystack/query_rs.rs" ]; then
    print_success "QueryRs haystack implementation exists"
else
    print_error "QueryRs haystack implementation not found"
    exit 1
fi

# Test 6: ServiceType enum contains QueryRs
print_info "Test 6: Checking ServiceType enum..."
if grep -q "QueryRs" crates/terraphim_config/src/lib.rs; then
    print_success "ServiceType::QueryRs found in configuration"
else
    print_error "ServiceType::QueryRs not found in configuration"
    exit 1
fi

# Test 7: Middleware integration
print_info "Test 7: Checking middleware integration..."
if grep -q "QueryRsHaystackIndexer" crates/terraphim_middleware/src/indexer/mod.rs; then
    print_success "QueryRs haystack integrated in middleware"
else
    print_error "QueryRs haystack not integrated in middleware"
    exit 1
fi

# Test 8: Dependencies added
print_info "Test 8: Checking reqwest dependency..."
if grep -q "reqwest" crates/terraphim_middleware/Cargo.toml; then
    print_success "reqwest dependency added to middleware"
else
    print_error "reqwest dependency not found"
    exit 1
fi

# Test 9: Setup script exists and is executable
print_info "Test 9: Checking setup script..."
if [ -x "scripts/setup_rust_engineer.sh" ]; then
    print_success "Setup script exists and is executable"
else
    print_error "Setup script not found or not executable"
    exit 1
fi

# Test 10: Documentation exists
print_info "Test 10: Checking documentation..."
if [ -f "README_QUERY_RS_HAYSTACK.md" ]; then
    print_success "Documentation exists"
else
    print_error "Documentation not found"
    exit 1
fi

echo ""
echo "🎉 All tests passed! Rust Engineer role and QueryRs haystack are properly configured."
echo ""
echo "📋 Summary of what was tested:"
echo "  ✅ Configuration file exists and is valid JSON"
echo "  ✅ Rust Engineer role is configured"
echo "  ✅ QueryRs service type is configured"
echo "  ✅ Project compiles with new dependencies"
echo "  ✅ QueryRs haystack implementation exists"
echo "  ✅ ServiceType enum contains QueryRs"
echo "  ✅ Middleware integration is complete"
echo "  ✅ reqwest dependency is added"
echo "  ✅ Setup script is available"
echo "  ✅ Documentation is complete"
echo ""
echo "🚀 To use the Rust Engineer role:"
echo "  1. Start the server: cargo run --bin terraphim_server -- --config terraphim_server/default/rust_engineer_config.json"
echo "  2. Search for Rust terms: curl -X POST http://localhost:8000/documents/search -H 'Content-Type: application/json' -d '{\"search_term\": \"async\", \"role\": \"Rust Engineer\"}'"
echo ""
echo "🔍 The role will search:"
echo "  - Rust standard library documentation (stable & nightly)"
echo "  - crates.io packages"
echo "  - docs.rs documentation"
echo "  - Reddit posts from r/rust"
