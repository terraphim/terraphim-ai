#!/usr/bin/env bash
# fix-port-exposure.sh
# Remediation for port exposure security issue (Refs #435)
#
# Fixes unauthenticated LLM proxy and Ollama services bound to 0.0.0.0
# These services must only listen on 127.0.0.1 (loopback) to prevent
# exposure to the internet.
#
# Requires root/sudo access. Run as: sudo bash scripts/fix-port-exposure.sh

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "Error: This script must be run as root (sudo bash $0)" >&2
    exit 1
fi

echo "=== Port Exposure Remediation (Refs #435) ==="

# Fix 1: terraphim-llm-proxy -- change 0.0.0.0 to 127.0.0.1
LLM_PROXY_CONFIG="/etc/terraphim-llm-proxy/config.toml"
if [[ -f "$LLM_PROXY_CONFIG" ]]; then
    # Backup original
    cp "$LLM_PROXY_CONFIG" "${LLM_PROXY_CONFIG}.backup.$(date +%Y%m%d%H%M%S)"
    # Apply fix
    sed -i 's/^host = "0\.0\.0\.0"/host = "127.0.0.1"/' "$LLM_PROXY_CONFIG"
    echo "[OK] $LLM_PROXY_CONFIG: host changed to 127.0.0.1"
    # Restart service if running
    if systemctl is-active --quiet terraphim-llm-proxy 2>/dev/null; then
        systemctl restart terraphim-llm-proxy
        echo "[OK] terraphim-llm-proxy restarted"
    fi
else
    echo "[WARN] $LLM_PROXY_CONFIG not found, skipping"
fi

# Fix 2: Ollama -- change OLLAMA_HOST from 0.0.0.0 to 127.0.0.1
OLLAMA_CONF="/etc/systemd/system/ollama.service.d/host.conf"
if [[ -f "$OLLAMA_CONF" ]]; then
    # Backup original
    cp "$OLLAMA_CONF" "${OLLAMA_CONF}.backup.$(date +%Y%m%d%H%M%S)"
    # Apply fix
    sed -i 's/OLLAMA_HOST=0\.0\.0\.0/OLLAMA_HOST=127.0.0.1/' "$OLLAMA_CONF"
    echo "[OK] $OLLAMA_CONF: OLLAMA_HOST changed to 127.0.0.1"
    # Reload systemd and restart ollama
    systemctl daemon-reload
    if systemctl is-active --quiet ollama 2>/dev/null; then
        systemctl restart ollama
        echo "[OK] ollama restarted"
    fi
else
    echo "[WARN] $OLLAMA_CONF not found, skipping"
fi

echo ""
echo "=== Verification ==="
echo "Run: ss -tlnp | grep -E '3456|11434'"
echo "Expected: both services show 127.0.0.1 (not 0.0.0.0)"
echo ""

# Verify bindings if ss is available
if command -v ss &>/dev/null; then
    echo "Current port bindings:"
    ss -tlnp 2>/dev/null | grep -E "3456|11434" || echo "(services not currently running)"
fi

echo ""
echo "=== Done ==="
