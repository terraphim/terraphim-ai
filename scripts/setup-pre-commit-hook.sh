#!/bin/bash

# Install Pre-commit Hook Script
# Installs the pre-commit hook for Terraphim AI
# Usage: ./scripts/setup-pre-commit-hook.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Installing Pre-commit Hook${NC}"
echo "============================="

# Get project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

# Check if we're in a git repository
if [[ ! -d "$PROJECT_ROOT/.git" ]]; then
    echo -e "${RED}❌ Not in a git repository${NC}"
    echo -e "${YELLOW}This script must be run from within a git repository.${NC}"
    exit 1
fi

# Create hooks directory if it doesn't exist
if [[ ! -d "$HOOKS_DIR" ]]; then
    echo -e "${BLUE}📁 Creating hooks directory...${NC}"
    mkdir -p "$HOOKS_DIR"
fi

# Pre-commit hook source and destination
PRE_COMMIT_SOURCE="$SCRIPT_DIR/pre-commit-hook.sh"
PRE_COMMIT_DEST="$HOOKS_DIR/pre-commit"

# Check if source exists
if [[ ! -f "$PRE_COMMIT_SOURCE" ]]; then
    echo -e "${RED}❌ Pre-commit hook source not found: $PRE_COMMIT_SOURCE${NC}"
    exit 1
fi

# Check if hook already exists
if [[ -f "$PRE_COMMIT_DEST" ]]; then
    echo -e "${YELLOW}⚠️  Pre-commit hook already exists${NC}"

    # Backup existing hook
    BACKUP_FILE="$PRE_COMMIT_DEST.backup.$(date +%Y%m%d-%H%M%S)"
    echo -e "${BLUE}📋 Backing up existing hook to: $BACKUP_FILE${NC}"
    cp "$PRE_COMMIT_DEST" "$BACKUP_FILE"

    read -p "Do you want to replace the existing pre-commit hook? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Installation cancelled.${NC}"
        exit 0
    fi
fi

# Install the hook
echo -e "${BLUE}📦 Installing pre-commit hook...${NC}"
cp "$PRE_COMMIT_SOURCE" "$PRE_COMMIT_DEST"
chmod +x "$PRE_COMMIT_DEST"

echo -e "${GREEN}✅ Pre-commit hook installed successfully!${NC}"
echo ""
echo "What this does:"
echo "  ✓ Runs quick checks before each commit"
echo "  ✓ Checks code formatting (cargo fmt)"
echo "  ✓ Runs clippy linting"
echo "  ✓ Validates cargo check"
echo "  ✓ Runs unit tests"
echo "  ✓ Checks frontend dependencies"
echo ""
echo "Usage:"
echo "  • Normal commit:     git commit -m 'Your message'"
echo "  • Bypass checks:    git commit --no-verify -m 'Your message'"
echo "  • Remove hook:       rm .git/hooks/pre-commit"
echo ""
echo "🚀 Your next commit will be automatically validated!"

# Test the hook
echo -e "\n${BLUE}🧪 Testing pre-commit hook...${NC}"
if bash "$PRE_COMMIT_DEST" --test 2>/dev/null; then
    echo -e "${GREEN}✅ Pre-commit hook test passed${NC}"
else
    echo -e "${YELLOW}⚠️  Pre-commit hook test failed (this is normal if dependencies aren't installed)${NC}"
fi

echo -e "\n${GREEN}Installation complete!${NC}"