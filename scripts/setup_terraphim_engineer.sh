#!/bin/bash

# Setup script for Terraphim Engineer with local knowledge graph
# This script will:
# 1. Validate the docs/src directory structure
# 2. Set up the server configuration for Terraphim engineering content
# 3. Build knowledge graph from local markdown files

set -e

echo "🚀 Setting up Terraphim Engineer with local knowledge graph..."

# Configuration
DOCS_SRC_DIR="./docs/src"
KG_DIR="./docs/src/kg"
CONFIG_FILE="terraphim_server/default/terraphim_engineer_config.json"
SERVER_SETTINGS="terraphim_server/default/settings_terraphim_engineer_server.toml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}📁 Validating Terraphim documentation structure...${NC}"

# Check if docs/src directory exists
if [ ! -d "$DOCS_SRC_DIR" ]; then
    echo -e "${RED}❌ Documentation directory not found: $DOCS_SRC_DIR${NC}"
    echo -e "${RED}   Make sure you're running this from the project root${NC}"
    exit 1
fi

# Check if knowledge graph directory exists
if [ ! -d "$KG_DIR" ]; then
    echo -e "${RED}❌ Knowledge graph directory not found: $KG_DIR${NC}"
    exit 1
fi

# Count documentation files
DOC_COUNT=$(find "$DOCS_SRC_DIR" -name "*.md" | wc -l)
KG_COUNT=$(find "$KG_DIR" -name "*.md" | wc -l)

echo -e "${GREEN}✅ Documentation structure validated${NC}"
echo -e "${GREEN}   📚 Documentation files: $DOC_COUNT${NC}"
echo -e "${GREEN}   🧠 Knowledge graph files: $KG_COUNT${NC}"

# List the KG files for verification
echo -e "${BLUE}🧠 Knowledge graph files found:${NC}"
find "$KG_DIR" -name "*.md" | while read file; do
    filename=$(basename "$file")
    size=$(wc -c < "$file")
    echo -e "${YELLOW}   ✓ $filename (${size} bytes)${NC}"
done

# List sample documentation files
echo -e "${BLUE}📚 Sample documentation files:${NC}"
find "$DOCS_SRC_DIR" -name "*.md" | head -5 | while read file; do
    filename=$(basename "$file")
    size=$(wc -c < "$file")
    echo -e "${YELLOW}   ✓ $filename (${size} bytes)${NC}"
done

# Verify configuration files exist
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}❌ Configuration file not found: $CONFIG_FILE${NC}"
    exit 1
fi

if [ ! -f "$SERVER_SETTINGS" ]; then
    echo -e "${RED}❌ Server settings file not found: $SERVER_SETTINGS${NC}"
    exit 1
fi

echo -e "${BLUE}⚙️  Terraphim Engineer configuration ready:${NC}"
echo -e "   📄 Config file: $CONFIG_FILE"
echo -e "   🔧 Settings file: $SERVER_SETTINGS"
echo -e "   📚 Documents: $DOCS_SRC_DIR ($DOC_COUNT files)"
echo -e "   🧠 Local KG: $KG_DIR ($KG_COUNT files)"

echo -e "${GREEN}🎉 Setup complete!${NC}"
echo -e "${BLUE}To start the server with Terraphim Engineer configuration:${NC}"
echo -e "   ${YELLOW}cargo run --bin terraphim_server -- --config $CONFIG_FILE${NC}"

echo -e "${BLUE}Available roles in this configuration:${NC}"
echo -e "   🔧 Terraphim Engineer (default) - Uses local KG from docs/src/kg"
echo -e "   👷 Engineer - Uses local KG from docs/src/kg"  
echo -e "   📝 Default - Uses TitleScorer for basic search"

echo -e "${BLUE}💡 The configuration includes:${NC}"
echo -e "   ✅ Local knowledge graph built from docs/src/kg"
echo -e "   ✅ Document indexing from docs/src"
echo -e "   ✅ Read-only document access (safe for development)"
echo -e "   ✅ TerraphimGraph ranking with local content"

echo -e "${BLUE}🔍 Knowledge graph content focus:${NC}"
echo -e "   ✅ Terraphim architecture and design"
echo -e "   ✅ Service documentation"
echo -e "   ✅ Haystack integration guides"
echo -e "   ✅ Engineering best practices"

echo -e "${YELLOW}💡 Note: This configuration builds KG from local files during server startup${NC}"
echo -e "${YELLOW}   First startup may take 10-30 seconds to build the knowledge graph${NC}" 