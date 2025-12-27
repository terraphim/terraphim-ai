#!/bin/bash
#
# Terraphim Hooks Installer
# Installs Claude Code and Git hooks for Terraphim AI capabilities
#
# Usage:
#   ./scripts/install-terraphim-hooks.sh [--easy-mode] [--git-only] [--claude-only]
#
# Options:
#   --easy-mode     Install everything with sensible defaults (recommended)
#   --git-only      Install only Git hooks (prepare-commit-msg)
#   --claude-only   Install only Claude Code hooks (PreToolUse)
#   --verbose       Enable verbose replacement logging
#   --help          Show this help message
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_status() {
    local status=$1
    local message=$2
    case "$status" in
        "SUCCESS") echo -e "${GREEN}✓${NC} $message" ;;
        "FAIL")    echo -e "${RED}✗${NC} $message" ;;
        "WARN")    echo -e "${YELLOW}⚠${NC} $message" ;;
        "INFO")    echo -e "  $message" ;;
    esac
}

show_help() {
    head -20 "$0" | tail -16
    exit 0
}

# Parse arguments
INSTALL_GIT=true
INSTALL_CLAUDE=true
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --easy-mode)   INSTALL_GIT=true; INSTALL_CLAUDE=true; shift ;;
        --git-only)    INSTALL_GIT=true; INSTALL_CLAUDE=false; shift ;;
        --claude-only) INSTALL_GIT=false; INSTALL_CLAUDE=true; shift ;;
        --verbose)     VERBOSE=true; shift ;;
        --help|-h)     show_help ;;
        *)             echo "Unknown option: $1"; show_help ;;
    esac
done

echo "Terraphim Hooks Installer"
echo "========================="
echo ""

# Check prerequisites
print_status "INFO" "Checking prerequisites..."

# Check for jq (required for Claude hooks)
if ! command -v jq >/dev/null 2>&1; then
    print_status "WARN" "jq not found - Claude hooks require jq for JSON parsing"
    print_status "INFO" "Install with: brew install jq (macOS) or apt install jq (Linux)"
    INSTALL_CLAUDE=false
fi

# Check for terraphim-agent
AGENT=""
if command -v terraphim-agent >/dev/null 2>&1; then
    AGENT="terraphim-agent"
    print_status "SUCCESS" "Found terraphim-agent in PATH"
elif [ -x "$PROJECT_DIR/target/release/terraphim-agent" ]; then
    AGENT="$PROJECT_DIR/target/release/terraphim-agent"
    print_status "SUCCESS" "Found terraphim-agent at $AGENT"
elif [ -x "$HOME/.cargo/bin/terraphim-agent" ]; then
    AGENT="$HOME/.cargo/bin/terraphim-agent"
    print_status "SUCCESS" "Found terraphim-agent at $AGENT"
else
    print_status "WARN" "terraphim-agent not found"
    print_status "INFO" "Building terraphim-agent..."
    if (cd "$PROJECT_DIR" && cargo build -p terraphim_agent --release 2>/dev/null); then
        AGENT="$PROJECT_DIR/target/release/terraphim-agent"
        print_status "SUCCESS" "Built terraphim-agent"
    else
        print_status "FAIL" "Failed to build terraphim-agent"
        print_status "INFO" "Run: cargo build -p terraphim_agent --release"
        exit 1
    fi
fi

echo ""

# Install Git hooks
if [ "$INSTALL_GIT" = true ]; then
    print_status "INFO" "Installing Git hooks..."

    # Check if .git directory exists
    if [ -d "$PROJECT_DIR/.git" ]; then
        # Create hooks directory if needed
        mkdir -p "$PROJECT_DIR/.git/hooks"

        # Install prepare-commit-msg hook
        if [ -f "$PROJECT_DIR/scripts/hooks/prepare-commit-msg" ]; then
            cp "$PROJECT_DIR/scripts/hooks/prepare-commit-msg" "$PROJECT_DIR/.git/hooks/"
            chmod +x "$PROJECT_DIR/.git/hooks/prepare-commit-msg"
            print_status "SUCCESS" "Installed prepare-commit-msg hook"
        else
            print_status "FAIL" "prepare-commit-msg hook source not found"
        fi

        # Install pre-commit hook if not already present
        if [ ! -f "$PROJECT_DIR/.git/hooks/pre-commit" ]; then
            if [ -f "$PROJECT_DIR/scripts/hooks/pre-commit" ]; then
                cp "$PROJECT_DIR/scripts/hooks/pre-commit" "$PROJECT_DIR/.git/hooks/"
                chmod +x "$PROJECT_DIR/.git/hooks/pre-commit"
                print_status "SUCCESS" "Installed pre-commit hook"
            fi
        else
            print_status "INFO" "pre-commit hook already exists (skipped)"
        fi
    else
        print_status "WARN" "Not a Git repository - skipping Git hooks"
    fi
fi

echo ""

# Install Claude Code hooks
if [ "$INSTALL_CLAUDE" = true ]; then
    print_status "INFO" "Installing Claude Code hooks..."

    # Ensure .claude/hooks directory exists
    mkdir -p "$PROJECT_DIR/.claude/hooks"

    # Copy hook script
    if [ -f "$PROJECT_DIR/.claude/hooks/npm_to_bun_guard.sh" ]; then
        chmod +x "$PROJECT_DIR/.claude/hooks/npm_to_bun_guard.sh"
        print_status "SUCCESS" "npm_to_bun_guard.sh hook ready"
    else
        print_status "FAIL" "npm_to_bun_guard.sh not found in .claude/hooks/"
    fi

    # Check if settings.local.json has hooks configured
    if [ -f "$PROJECT_DIR/.claude/settings.local.json" ]; then
        if grep -q "PreToolUse" "$PROJECT_DIR/.claude/settings.local.json"; then
            print_status "SUCCESS" "Claude hooks already configured in settings.local.json"
        else
            print_status "WARN" "Claude hooks not configured in settings.local.json"
            print_status "INFO" "Add this to .claude/settings.local.json:"
            echo '  "hooks": {'
            echo '    "PreToolUse": [{'
            echo '      "matcher": "Bash",'
            echo '      "hooks": [{ "type": "command", "command": ".claude/hooks/npm_to_bun_guard.sh" }]'
            echo '    }]'
            echo '  }'
        fi
    fi
fi

echo ""

# Set verbose mode if requested
if [ "$VERBOSE" = true ]; then
    print_status "INFO" "Enabling verbose mode..."
    echo "export TERRAPHIM_VERBOSE=1" >> "$HOME/.bashrc" 2>/dev/null || true
    echo "export TERRAPHIM_VERBOSE=1" >> "$HOME/.zshrc" 2>/dev/null || true
    print_status "SUCCESS" "Added TERRAPHIM_VERBOSE=1 to shell config"
fi

echo ""
echo "Installation complete!"
echo ""
echo "What's installed:"
[ "$INSTALL_GIT" = true ] && echo "  - Git prepare-commit-msg hook (Claude → Terraphim AI attribution)"
[ "$INSTALL_CLAUDE" = true ] && echo "  - Claude PreToolUse hook (npm/yarn/pnpm → bun replacement)"
echo ""
echo "To test:"
echo "  echo 'npm install' | terraphim-agent replace"
echo "  echo '{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"npm install\"}}' | .claude/hooks/npm_to_bun_guard.sh"
echo ""
echo "NOTE: Restart Claude Code to apply hook changes."
