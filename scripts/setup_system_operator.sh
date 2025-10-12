#!/bin/bash

# Setup script for Terraphim System Operator with remote knowledge graph
# This script will:
# 1. Clone the system-operator repository
# 2. Set up the server configuration
# 3. Start the server with the system operator configuration

set -e

echo "🚀 Setting up Terraphim System Operator with remote knowledge graph..."

# Configuration
SYSTEM_OPERATOR_DIR="${SYSTEM_OPERATOR_DIR:-/tmp/system_operator}"
CONFIG_FILE="terraphim_server/default/system_operator_config.json"
SERVER_SETTINGS="terraphim_server/default/settings_system_operator_server.toml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}📁 Setting up system operator directory...${NC}"

# Create directory if it doesn't exist
mkdir -p "$SYSTEM_OPERATOR_DIR"

# Clone or update the repository
if [ -d "$SYSTEM_OPERATOR_DIR/.git" ]; then
    echo -e "${YELLOW}📦 Repository already exists, updating...${NC}"
    cd "$SYSTEM_OPERATOR_DIR"
    git pull origin main
else
    echo -e "${YELLOW}📦 Cloning system-operator repository...${NC}"
    git clone https://github.com/terraphim/system-operator.git "$SYSTEM_OPERATOR_DIR"
fi

# Check if repository was cloned successfully
if [ ! -d "$SYSTEM_OPERATOR_DIR/pages" ]; then
    echo -e "${RED}❌ Failed to clone repository or pages directory not found${NC}"
    exit 1
fi

# Count files
FILE_COUNT=$(find "$SYSTEM_OPERATOR_DIR/pages" -name "*.md" | wc -l)
echo -e "${GREEN}✅ Repository setup complete with $FILE_COUNT markdown files${NC}"

# Get the absolute path to the project root (we're already in the right place)
PROJECT_ROOT="$(pwd)"

echo -e "${BLUE}⚙️  Server configuration ready:${NC}"
echo -e "   📄 Config file: $CONFIG_FILE"
echo -e "   🔧 Settings file: $SERVER_SETTINGS"
echo -e "   📚 Documents: $SYSTEM_OPERATOR_DIR/pages ($FILE_COUNT files)"
echo -e "   🌐 Remote KG: https://staging-storage.terraphim.io/thesaurus_Default.json"

echo -e "${GREEN}🎉 Setup complete!${NC}"
echo -e "${BLUE}To start the server with system operator configuration:${NC}"
echo -e "   ${YELLOW}cargo run --bin terraphim_server -- --config $CONFIG_FILE${NC}"

echo -e "${BLUE}Available roles in this configuration:${NC}"
echo -e "   🔧 System Operator (default) - Uses TerraphimGraph with remote KG"
echo -e "   👷 Engineer - Uses TerraphimGraph with remote KG"
echo -e "   📝 Default - Uses TitleScorer for basic search"

echo -e "${BLUE}💡 The configuration includes:${NC}"
echo -e "   ✅ Remote knowledge graph from staging-storage.terraphim.io"
echo -e "   ✅ Local document indexing from GitHub repository"
echo -e "   ✅ Read-only document access (safe for production)"
echo -e "   ✅ Multiple search backends (Ripgrep + TerraphimGraph)"
