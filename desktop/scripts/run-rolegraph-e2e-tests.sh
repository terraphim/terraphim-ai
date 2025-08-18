#!/bin/bash

# Rolegraph End-to-End Test Runner
# Supports both local development and CI environments

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# CI detection
if [ -n "$CI" ]; then
    echo -e "${BLUE}🧪 Running in CI environment${NC}"
    CI_MODE=true
else
    echo -e "${BLUE}🧪 Running in local development environment${NC}"
    CI_MODE=false
fi

echo -e "${BLUE}🧪 Rolegraph End-to-End Test Runner${NC}"
echo "==================================="

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SERVER_DIR="$(cd "$PROJECT_ROOT/../terraphim_server" && pwd)"
DESKTOP_DIR="$PROJECT_ROOT"

echo -e "${BLUE}📁 Project root:${NC} $PROJECT_ROOT"
echo -e "${BLUE}📁 Server directory:${NC} $SERVER_DIR"
echo -e "${BLUE}📁 Desktop directory:${NC} $DESKTOP_DIR"

# Check prerequisites
echo -e "\n${BLUE}🔍 Checking prerequisites...${NC}"

# Check for Cargo
if command -v cargo &> /dev/null; then
    echo -e "${GREEN}✅ Cargo available${NC}"
else
    echo -e "${RED}❌ Cargo not found${NC}"
    exit 1
fi

# Check for Bun (preferred) or Yarn
if command -v bun &> /dev/null; then
    echo -e "${GREEN}✅ Bun available (using for package management)${NC}"
    PACKAGE_MANAGER="bun"
elif command -v yarn &> /dev/null; then
    echo -e "${GREEN}✅ Yarn available (using for package management)${NC}"
    PACKAGE_MANAGER="yarn"
else
    echo -e "${RED}❌ Neither Bun nor Yarn found${NC}"
    exit 1
fi

# Check for Node.js
if command -v node &> /dev/null; then
    echo -e "${GREEN}✅ Node.js available${NC}"
else
    echo -e "${RED}❌ Node.js not found${NC}"
    exit 1
fi

# Check for Playwright
if [ -f "$DESKTOP_DIR/node_modules/.bin/playwright" ]; then
    echo -e "${GREEN}✅ Playwright available${NC}"
else
    echo -e "${YELLOW}⚠️ Playwright not found, will install${NC}"
fi

# Build terraphim_server
echo -e "\n${BLUE}🔨 Building terraphim_server...${NC}"
cd "$SERVER_DIR"

if [ "$CI_MODE" = true ]; then
    echo "Building server in release mode for CI..."
    cargo build --release --bin terraphim_server
    SERVER_BINARY="$PROJECT_ROOT/../target/release/terraphim_server"
else
    echo "Building server in debug mode (faster)..."
    cargo build --bin terraphim_server
    SERVER_BINARY="$PROJECT_ROOT/../target/debug/terraphim_server"
fi

if [ -f "$SERVER_BINARY" ]; then
    echo -e "${GREEN}✅ Server build completed${NC}"
    echo -e "${GREEN}✅ Using $([ "$CI_MODE" = true ] && echo "release" || echo "debug") server binary${NC}"
    echo -e "${GREEN}✅ Server binary verified: $SERVER_BINARY${NC}"
else
    echo -e "${RED}❌ Server binary not found at expected location${NC}"
    exit 1
fi

# Create test configuration
echo -e "\n${BLUE}⚙️  Creating test configuration...${NC}"
cd "$DESKTOP_DIR"

# Create test config directory if it doesn't exist
mkdir -p test-config

# Create test configuration file
cat > test-config.json << 'EOF'
{
  "id": "Desktop",
  "global_shortcut": "Ctrl+Shift+T",
  "roles": {
    "Terraphim Engineer": {
      "shortname": "Terraphim Engineer",
      "name": "Terraphim Engineer",
      "relevance_function": "TerraphimGraph",
      "theme": "lumen",
      "kg": {
        "automata_path": null,
        "knowledge_graph_local": {
          "input_type": "Markdown",
          "path": "./docs/src/kg"
        },
        "public": true,
        "publish": true
      },
      "haystacks": [
        {
          "location": "./docs/src",
          "service": "Ripgrep",
          "read_only": true,
          "atomic_server_secret": null
        }
      ],
      "extra": {}
    }
  },
  "default_role": "Terraphim Engineer",
  "selected_role": "Terraphim Engineer"
}
EOF

echo -e "${GREEN}✅ Test configuration created${NC}"

# Verify knowledge graph files
echo -e "\n${BLUE}📚 Verifying knowledge graph files...${NC}"
KG_PATH="$PROJECT_ROOT/../docs/src/kg"

if [ -d "$KG_PATH" ]; then
    echo -e "${GREEN}✅ Knowledge graph directory exists${NC}"
    
    # Check for specific files
    for file in "terraphim-graph.md" "haystack.md" "service.md"; do
        if [ -f "$KG_PATH/$file" ]; then
            echo -e "${GREEN}✅ $file found${NC}"
        else
            echo -e "${YELLOW}⚠️ $file not found${NC}"
        fi
    done
else
    echo -e "${YELLOW}⚠️ Knowledge graph directory not found: $KG_PATH${NC}"
fi

# Check desktop dependencies
echo -e "\n${BLUE}📦 Checking desktop dependencies...${NC}"
cd "$DESKTOP_DIR"

if [ -d "node_modules" ]; then
    echo "Dependencies already installed, skipping..."
else
    echo "Installing dependencies..."
    if [ "$PACKAGE_MANAGER" = "bun" ]; then
        bun install
    else
        yarn install
    fi
fi
echo -e "${GREEN}✅ Dependencies check passed${NC}"

# Check desktop app build
echo -e "\n${BLUE}🔨 Checking desktop app build...${NC}"
if [ -d "dist" ]; then
    echo "Desktop app already built, skipping..."
else
    echo "Building desktop app..."
    if [ "$PACKAGE_MANAGER" = "bun" ]; then
        bun run build
    else
        yarn run build
    fi
fi
echo -e "${GREEN}✅ Desktop app build check passed${NC}"

# Set environment variables
echo -e "\n${BLUE}🌍 Environment variables set:${NC}"
export RUST_LOG=debug
export CONFIG_PATH="$DESKTOP_DIR/test-config.json"
export SERVER_PORT=8000
export FRONTEND_URL="http://localhost:5173"
export SERVER_BINARY_PATH="$SERVER_BINARY"

echo "RUST_LOG=debug"
echo "CONFIG_PATH=$CONFIG_PATH"
echo "SERVER_PORT=$SERVER_PORT"
echo "FRONTEND_URL=$FRONTEND_URL"
echo "SERVER_BINARY_PATH=$SERVER_BINARY_PATH"

# Install Playwright if needed
if [ ! -f "$DESKTOP_DIR/node_modules/.bin/playwright" ]; then
    echo -e "\n${BLUE}🎭 Installing Playwright...${NC}"
    if [ "$PACKAGE_MANAGER" = "bun" ]; then
        bunx playwright install
    else
        npx playwright install
    fi
fi

# Run tests
echo -e "\n${BLUE}🧪 Running rolegraph end-to-end tests...${NC}"
echo "Test file: tests/e2e/rolegraph-search-validation.spec.ts"

if [ "$CI_MODE" = true ]; then
    echo "Note: Running in CI mode with headless browser"
    # CI-specific settings
    export CI=true
    export PLAYWRIGHT_HEADLESS=true
    
    # Run tests with CI configuration
    if [ "$PACKAGE_MANAGER" = "bun" ]; then
        bunx playwright test tests/e2e/rolegraph-search-validation.spec.ts --reporter=github,html,json
    else
        npx playwright test tests/e2e/rolegraph-search-validation.spec.ts --reporter=github,html,json
    fi
else
    echo "Note: Using yarn run dev for frontend (faster than Tauri)"
    # Local development settings
    if [ "$PACKAGE_MANAGER" = "bun" ]; then
        bunx playwright test tests/e2e/rolegraph-search-validation.spec.ts
    else
        npx playwright test tests/e2e/rolegraph-search-validation.spec.ts
    fi
fi

# Check test results
TEST_EXIT_CODE=$?

echo -e "\n${BLUE}📊 Test Results Summary:${NC}"
if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed!${NC}"
else
    echo -e "${RED}❌ Some tests failed${NC}"
fi

# Show test report location
if [ -d "test-results" ]; then
    echo -e "${BLUE}📋 Test results available in: test-results/${NC}"
    echo -e "${BLUE}📋 HTML report: test-results/playwright-report/index.html${NC}"
fi

# Cleanup
echo -e "\n${BLUE}🧹 Cleanup...${NC}"
if [ -f "test-config.json" ]; then
    rm test-config.json
    echo -e "${GREEN}✅ Test configuration cleaned up${NC}"
fi

# Exit with test result code
exit $TEST_EXIT_CODE 