#!/bin/bash
# Test Terraphim Automata WASM module
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Testing Terraphim Automata WASM module...${NC}"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack is not installed. Installing..."
    cargo install wasm-pack
fi

# Navigate to wasm-test directory
cd "$(dirname "$0")/../crates/terraphim_automata/wasm-test"

# Parse arguments
BROWSER="${1:-chrome}"
MODE="${2:-headless}"

case "$BROWSER" in
    chrome|chromium)
        BROWSER_FLAG="--chrome"
        ;;
    firefox)
        BROWSER_FLAG="--firefox"
        ;;
    safari)
        BROWSER_FLAG="--safari"
        ;;
    node|nodejs)
        BROWSER_FLAG="--node"
        ;;
    *)
        echo -e "${YELLOW}Unknown browser: $BROWSER, defaulting to Chrome${NC}"
        BROWSER_FLAG="--chrome"
        ;;
esac

# Build mode flags
if [ "$MODE" = "headless" ]; then
    MODE_FLAG="--headless"
else
    MODE_FLAG=""
fi

echo -e "${GREEN}Running WASM tests with $BROWSER...${NC}"

# Run tests
wasm-pack test $MODE_FLAG $BROWSER_FLAG

echo -e "\n${GREEN}✓ All tests passed!${NC}"

# Additional validation
echo -e "\n${BLUE}Validating WASM build compatibility...${NC}"
cargo check -p terraphim_automata --target wasm32-unknown-unknown --features wasm

echo -e "${GREEN}✓ WASM build compatibility verified!${NC}"

echo -e "\n${GREEN}Available test commands:${NC}"
echo "  Chrome (headless):  ./scripts/test-wasm.sh chrome headless"
echo "  Firefox (headless): ./scripts/test-wasm.sh firefox headless"
echo "  Node.js:            ./scripts/test-wasm.sh node"
echo "  Chrome (browser):   ./scripts/test-wasm.sh chrome interactive"
