#!/bin/bash
#
# Shared binary discovery for terraphim-agent.
# Source this file in hooks to use discover_terraphim_agent function.
#
# Usage:
#   source "$(dirname "$0")/terraphim-discover.sh"
#   AGENT=$(discover_terraphim_agent)
#   if [ -n "$AGENT" ]; then
#       "$AGENT" replace ...
#   fi
#

discover_terraphim_agent() {
    # Check PATH first
    if command -v terraphim-agent >/dev/null 2>&1; then
        echo "terraphim-agent"
        return 0
    fi

    # Check local release build
    if [ -x "./target/release/terraphim-agent" ]; then
        echo "./target/release/terraphim-agent"
        return 0
    fi

    # Check cargo home
    local cargo_bin="$HOME/.cargo/bin/terraphim-agent"
    if [ -x "$cargo_bin" ]; then
        echo "$cargo_bin"
        return 0
    fi

    # Not found
    return 1
}
