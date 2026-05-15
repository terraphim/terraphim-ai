#!/bin/bash
# @file install.sh
# Install script for terraphim-rlm OpenCode plugin and Claude Code hook.
#
# Usage:
#   ./install.sh           # Interactive install
#   ./install.sh --opencode   # OpenCode only
#   ./install.sh --claude     # Claude Code only

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OPENCODE_PLUGIN_DIR="${HOME}/.config/opencode/plugin"
OPENCODE_PLUGINS_DIR="${HOME}/.config/opencode/plugins"
CLAUDE_HOOKS_DIR="${HOME}/.claude/hooks"

# Required runtime dependencies for the Claude Code hook (terraphim-rlm-hook.sh).
# Warn but do not block if missing - operator may install them out of band.
if ! command -v jq >/dev/null 2>&1; then
    echo "WARNING: 'jq' is not installed but is required by terraphim-rlm-hook.sh."
    echo "         Install it via your platform package manager:"
    echo "           macOS:   brew install jq"
    echo "           Ubuntu:  apt-get install jq"
    echo "           Fedora:  dnf install jq"
    echo
fi

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --opencode    Install OpenCode plugin only"
    echo "  --claude      Install Claude Code hook only"
    echo "  --both        Install both (default)"
    echo "  --uninstall   Uninstall"
    echo "  --help        Show this help"
    exit 0
}

install_opencode() {
    echo "Installing OpenCode plugin..."

    if [ -d "$OPENCODE_PLUGIN_DIR" ]; then
        cp "${SCRIPT_DIR}/terraphim-rlm.js" "$OPENCODE_PLUGIN_DIR/"
        echo "  -> Copied to $OPENCODE_PLUGIN_DIR/terraphim-rlm.js"
    fi

    if [ -d "$OPENCODE_PLUGINS_DIR" ]; then
        cp "${SCRIPT_DIR}/terraphim-rlm.js" "$OPENCODE_PLUGINS_DIR/"
        echo "  -> Copied to $OPENCODE_PLUGINS_DIR/terraphim-rlm.js"
    fi

    if [ ! -d "$OPENCODE_PLUGIN_DIR" ] && [ ! -d "$OPENCODE_PLUGINS_DIR" ]; then
        echo "ERROR: OpenCode config directory not found"
        echo "  Expected: $OPENCODE_PLUGIN_DIR or $OPENCODE_PLUGINS_DIR"
        return 1
    fi

    echo "OpenCode plugin installed!"
}

install_claude() {
    echo "Installing Claude Code hook..."

    mkdir -p "$CLAUDE_HOOKS_DIR"

    cp "${SCRIPT_DIR}/terraphim-rlm-hook.sh" "$CLAUDE_HOOKS_DIR/"
    chmod +x "$CLAUDE_HOOKS_DIR/terraphim-rlm-hook.sh"

    echo "  -> Copied to $CLAUDE_HOOKS_DIR/terraphim-rlm-hook.sh"

    if [ -f "${HOME}/.claude/settings.local.json" ]; then
        echo "  -> Claude Code settings found at ${HOME}/.claude/settings.local.json"
        echo "  -> Add the hook manually in settings.local.json:"
        echo ""
        echo '  {
    "hooks": {
      "PreToolUse": [{
        "matcher": "Bash",
        "hooks": [{
          "type": "command",
          "command": "~/.claude/hooks/terraphim-rlm-hook.sh"
        }]
      }]
    }
  }'
    else
        cat > "${HOME}/.claude/settings.local.json" << 'EOF'
{
  "hooks": {
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/terraphim-rlm-hook.sh"
      }]
    }]
  }
}
EOF
        echo "  -> Created ${HOME}/.claude/settings.local.json"
    fi

    echo "Claude Code hook installed!"
}

uninstall() {
    echo "Uninstalling..."

    rm -f "$OPENCODE_PLUGIN_DIR/terraphim-rlm.js" 2>/dev/null || true
    rm -f "$OPENCODE_PLUGINS_DIR/terraphim-rlm.js" 2>/dev/null || true
    rm -f "$CLAUDE_HOOKS_DIR/terraphim-rlm-hook.sh" 2>/dev/null || true

    echo "Uninstalled!"
}

TARGET="both"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --opencode) TARGET="opencode"; shift ;;
        --claude) TARGET="claude"; shift ;;
        --both) TARGET="both"; shift ;;
        --uninstall) TARGET="uninstall"; shift ;;
        --help) usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
done

case "$TARGET" in
    opencode) install_opencode ;;
    claude) install_claude ;;
    both)
        install_opencode
        echo ""
        install_claude
        ;;
    uninstall) uninstall ;;
esac

echo ""
echo "Done!"
