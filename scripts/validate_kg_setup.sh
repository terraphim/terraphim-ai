#!/bin/bash

# Terraphim KG Setup Validation Script
# Validates that all required files are in place for Terraphim Engineer configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Terraphim KG Setup Validation${NC}"
echo -e "${BLUE}=================================${NC}\n"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo -e "${RED}❌ Error: Please run this script from the terraphim-ai project root${NC}"
    exit 1
fi

echo -e "${BLUE}📂 Checking required directories and files...${NC}"

# Check terraphim_engineer_config.json
CONFIG_FILE="terraphim_server/default/terraphim_engineer_config.json"
if [ -f "$CONFIG_FILE" ]; then
    echo -e "${GREEN}✅ Configuration file: $CONFIG_FILE${NC}"
else
    echo -e "${RED}❌ Missing configuration file: $CONFIG_FILE${NC}"
    exit 1
fi

# Check docs/src directory
DOCS_DIR="docs/src"
if [ -d "$DOCS_DIR" ]; then
    DOC_COUNT=$(find "$DOCS_DIR" -name "*.md" -type f | wc -l)
    echo -e "${GREEN}✅ Documentation directory: $DOCS_DIR (${DOC_COUNT} markdown files)${NC}"
else
    echo -e "${RED}❌ Missing documentation directory: $DOCS_DIR${NC}"
    exit 1
fi

# Check docs/src/kg directory
KG_DIR="docs/src/kg"
if [ -d "$KG_DIR" ]; then
    KG_COUNT=$(find "$KG_DIR" -name "*.md" -type f | wc -l)
    echo -e "${GREEN}✅ Knowledge graph directory: $KG_DIR (${KG_COUNT} markdown files)${NC}"
    
    # List KG files
    echo -e "${BLUE}📋 KG Files:${NC}"
    find "$KG_DIR" -name "*.md" -type f | while read file; do
        echo -e "  - $(basename "$file")"
    done
else
    echo -e "${RED}❌ Missing knowledge graph directory: $KG_DIR${NC}"
    exit 1
fi

echo ""

# Validate configuration content
echo -e "${BLUE}🔧 Validating configuration content...${NC}"

# Check if configuration has Terraphim Engineer role
if grep -q '"Terraphim Engineer"' "$CONFIG_FILE"; then
    echo -e "${GREEN}✅ Terraphim Engineer role found in configuration${NC}"
else
    echo -e "${RED}❌ Terraphim Engineer role not found in configuration${NC}"
    exit 1
fi

# Check if configuration has terraphim-graph relevance function
if grep -q '"terraphim-graph"' "$CONFIG_FILE"; then
    echo -e "${GREEN}✅ TerraphimGraph relevance function configured${NC}"
else
    echo -e "${YELLOW}⚠️  TerraphimGraph relevance function not found${NC}"
fi

# Check if configuration has local KG path
if grep -q '"docs/src/kg"' "$CONFIG_FILE"; then
    echo -e "${GREEN}✅ Local knowledge graph path configured${NC}"
else
    echo -e "${RED}❌ Local knowledge graph path not configured correctly${NC}"
    exit 1
fi

echo ""

# Check server binary
echo -e "${BLUE}🔨 Checking server binary...${NC}"
if [ -f "target/debug/terraphim_server" ]; then
    echo -e "${GREEN}✅ Debug server binary exists${NC}"
elif [ -f "target/release/terraphim_server" ]; then
    echo -e "${GREEN}✅ Release server binary exists${NC}"
else
    echo -e "${YELLOW}⚠️  Server binary not found. Run: cargo build --bin terraphim_server${NC}"
fi

echo ""

# Provide next steps
echo -e "${BLUE}🚀 Next Steps:${NC}"
echo -e "${GREEN}1. Build the server:${NC} cargo build --bin terraphim_server"
echo -e "${GREEN}2. Start the server:${NC} cargo run --bin terraphim_server"
echo -e "${GREEN}3. Check server logs for KG building progress${NC}"
echo -e "${GREEN}4. Test KG lookup:${NC} curl \"http://127.0.0.1:8000/roles/Terraphim%20Engineer/kg_search?term=service\""

echo ""
echo -e "${GREEN}✅ All validations passed! Your Terraphim KG setup is ready.${NC}" 