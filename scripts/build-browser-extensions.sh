#!/bin/bash

# Build script for Terraphim Browser Extensions
# This script builds both TerraphimAIParseExtension and TerraphimAIContext extensions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" &> /dev/null && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BROWSER_EXT_DIR="$PROJECT_ROOT/browser_extensions"

print_info "🚀 Starting browser extensions build process"
print_info "Project root: $PROJECT_ROOT"

# Check if browser extensions directory exists
if [ ! -d "$BROWSER_EXT_DIR" ]; then
    print_error "Browser extensions directory not found: $BROWSER_EXT_DIR"
    exit 1
fi

# Function to check required tools
check_requirements() {
    print_info "🔍 Checking build requirements..."

    # Check for wasm-pack
    if ! command -v wasm-pack &> /dev/null; then
        print_error "wasm-pack is required but not installed"
        print_info "Install with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
        exit 1
    fi

    # Check for npm
    if ! command -v npm &> /dev/null; then
        print_error "npm is required but not installed"
        exit 1
    fi

    # Check for zip
    if ! command -v zip &> /dev/null; then
        print_error "zip is required but not installed"
        print_info "Install with: sudo apt-get install zip"
        exit 1
    fi

    print_success "All requirements satisfied"
}

# Function to build WASM component
build_wasm() {
    print_info "🔧 Building WASM component for TerraphimAIParseExtension..."

    local wasm_dir="$BROWSER_EXT_DIR/TerraphimAIParseExtension/wasm"

    if [ ! -d "$wasm_dir" ]; then
        print_error "WASM directory not found: $wasm_dir"
        return 1
    fi

    cd "$wasm_dir"

    # Clean previous build
    print_info "Cleaning previous WASM build..."
    rm -rf pkg/

    # Build WASM package
    print_info "Compiling WASM package..."
    if wasm-pack build --target web --out-dir pkg; then
        print_success "WASM build completed successfully"
    else
        print_error "WASM build failed"
        return 1
    fi

    cd "$PROJECT_ROOT"
}

# Function to validate extension manifest
validate_manifest() {
    local manifest_file="$1"
    local extension_name="$2"

    print_info "📋 Validating manifest for $extension_name..."

    if [ ! -f "$manifest_file" ]; then
        print_error "Manifest file not found: $manifest_file"
        return 1
    fi

    # Basic JSON validation
    if ! python3 -m json.tool "$manifest_file" > /dev/null 2>&1; then
        print_error "Invalid JSON in manifest: $manifest_file"
        return 1
    fi

    # Check required fields
    if ! grep -q '"manifest_version"' "$manifest_file"; then
        print_error "Missing manifest_version in $manifest_file"
        return 1
    fi

    if ! grep -q '"name"' "$manifest_file"; then
        print_error "Missing name in $manifest_file"
        return 1
    fi

    if ! grep -q '"version"' "$manifest_file"; then
        print_error "Missing version in $manifest_file"
        return 1
    fi

    print_success "Manifest validation passed for $extension_name"
}

# Function to prepare extension directory
prepare_extension() {
    local ext_name="$1"
    local ext_dir="$BROWSER_EXT_DIR/$ext_name"

    print_info "📦 Preparing $ext_name extension..."

    if [ ! -d "$ext_dir" ]; then
        print_error "Extension directory not found: $ext_dir"
        return 1
    fi

    cd "$ext_dir"

    # Validate manifest
    if ! validate_manifest "$ext_dir/manifest.json" "$ext_name"; then
        return 1
    fi

    # Remove development files
    print_info "Cleaning development files for $ext_name..."
    find . -name ".git*" -type f -delete 2>/dev/null || true
    find . -name "*.tmp" -type f -delete 2>/dev/null || true
    find . -name "*.log" -type f -delete 2>/dev/null || true
    find . -name "node_modules" -type d -exec rm -rf {} + 2>/dev/null || true
    find . -name "package-lock.json" -type f -delete 2>/dev/null || true

    print_success "$ext_name extension prepared successfully"
    cd "$PROJECT_ROOT"
}

# Function to run security checks
run_security_checks() {
    print_info "🔒 Running security checks on extensions..."

    # Run API key detection script if available
    if [ -f "$SCRIPT_DIR/check-api-keys.sh" ]; then
        if ! "$SCRIPT_DIR/check-api-keys.sh"; then
            print_error "Security check failed - potential API keys detected"
            return 1
        fi
    else
        print_warning "Security check script not found, skipping"
    fi

    # Check for hardcoded credentials in extension files
    local found_issues=false

    for ext_dir in "$BROWSER_EXT_DIR"/*; do
        if [ -d "$ext_dir" ]; then
            local ext_name=$(basename "$ext_dir")
            print_info "Checking $ext_name for hardcoded credentials..."

            # Look for potential hardcoded credentials
            if grep -r -i -E "(api_key|apikey|secret|token|account_id).*[=:]\s*['\"][a-zA-Z0-9_-]{10,}['\"]" "$ext_dir" --exclude-dir=node_modules 2>/dev/null; then
                print_error "Potential hardcoded credentials found in $ext_name"
                found_issues=true
            fi
        fi
    done

    if [ "$found_issues" = true ]; then
        print_error "Security issues detected - build aborted"
        return 1
    fi

    print_success "Security checks passed"
}

# Main build function
main() {
    print_info "🏗️ Building Terraphim Browser Extensions"

    # Check requirements
    check_requirements

    # Run security checks
    run_security_checks

    # Build WASM component for TerraphimAIParseExtension
    build_wasm

    # Prepare extensions
    prepare_extension "TerraphimAIParseExtension"
    prepare_extension "TerraphimAIContext"

    print_success "🎉 Browser extensions build completed successfully!"
    print_info "Extensions are ready for packaging in:"
    print_info "  - $BROWSER_EXT_DIR/TerraphimAIParseExtension"
    print_info "  - $BROWSER_EXT_DIR/TerraphimAIContext"
    print_info ""
    print_info "Next steps:"
    print_info "  1. Run ./scripts/package-browser-extensions.sh to create release packages"
    print_info "  2. Test the extensions locally before distribution"
}

# Run main function
main "$@"