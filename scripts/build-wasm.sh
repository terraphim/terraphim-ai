#!/bin/bash
# Build Terraphim Automata WASM module
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Building Terraphim Automata WASM module...${NC}"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Installing..."
    cargo install wasm-pack
fi

# Navigate to wasm-test directory
cd "$(dirname "$0")/../crates/terraphim_automata/wasm-test"

# Parse arguments
TARGET="${1:-web}"
BUILD_TYPE="${2:-dev}"

case "$BUILD_TYPE" in
    release|prod)
        echo -e "${GREEN}Building optimized release...${NC}"
        wasm-pack build --release --target "$TARGET" --out-dir pkg
        ;;
    dev|debug)
        echo -e "${GREEN}Building development version...${NC}"
        wasm-pack build --dev --target "$TARGET" --out-dir pkg
        ;;
    *)
        echo "Unknown build type: $BUILD_TYPE"
        echo "Usage: $0 [web|nodejs|bundler] [dev|release]"
        exit 1
        ;;
esac

echo -e "${GREEN}✓ Build complete!${NC}"
echo "Output directory: crates/terraphim_automata/wasm-test/pkg"

# Show file sizes
if [ -f "pkg/terraphim_automata_wasm_test_bg.wasm" ]; then
    WASM_SIZE=$(du -h pkg/terraphim_automata_wasm_test_bg.wasm | cut -f1)
    echo -e "WASM file size: ${BLUE}${WASM_SIZE}${NC}"

    if command -v gzip &> /dev/null; then
        GZIP_SIZE=$(gzip -c pkg/terraphim_automata_wasm_test_bg.wasm | wc -c | awk '{print $1/1024 "K"}')
        echo -e "Gzipped size: ${BLUE}${GZIP_SIZE}${NC}"
    fi
fi

echo -e "\n${GREEN}Usage:${NC}"
echo "  Web:     import init from './pkg/terraphim_automata_wasm_test.js'"
echo "  Node.js: const wasm = require('./pkg/terraphim_automata_wasm_test.js')"
