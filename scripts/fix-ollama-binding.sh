#!/usr/bin/env bash
# Restrict Ollama service to listen only on localhost (127.0.0.1:11434)
# instead of all interfaces (0.0.0.0:11434).
#
# Run as root or with sudo. Requires systemd.
#
# Issue: #632 - port 11434 exposed to all network interfaces is a
# critical remote access risk per security-sentinel audit.

set -euo pipefail

SYSTEMD_OVERRIDE_DIR="/etc/systemd/system/ollama.service.d"
OVERRIDE_FILE="${SYSTEMD_OVERRIDE_DIR}/localhost-binding.conf"

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: This script must be run as root (or with sudo)." >&2
    exit 1
fi

echo "Creating systemd override to bind Ollama to 127.0.0.1 only..."

mkdir -p "${SYSTEMD_OVERRIDE_DIR}"

cat > "${OVERRIDE_FILE}" << 'EOF'
[Service]
Environment="OLLAMA_HOST=127.0.0.1:11434"
EOF

echo "Reloading systemd daemon..."
systemctl daemon-reload

echo "Restarting Ollama service..."
systemctl restart ollama

echo "Verifying Ollama is now bound to 127.0.0.1 only..."
sleep 2
if ss -tlnp | grep -q "127.0.0.1:11434"; then
    echo "SUCCESS: Ollama is now listening on 127.0.0.1:11434 only."
elif ss -tlnp | grep -q ":11434"; then
    echo "WARNING: Ollama is still listening on all interfaces."
    ss -tlnp | grep ":11434"
    exit 1
else
    echo "WARNING: Port 11434 not found. Check Ollama service status."
    systemctl status ollama --no-pager
    exit 1
fi
