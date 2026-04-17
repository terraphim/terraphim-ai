#!/usr/bin/env bash
# Setup script for the Terraphim System Operator demo.
#
# Clones the terraphim/system-operator Logseq vault into a durable path
# under ~/.config/terraphim/system_operator, counts the pages and synonym
# files, and prints the commands to drive it via terraphim_server and the
# terraphim-agent CLI.
#
# Override the target path with SYSTEM_OPERATOR_DIR=<path> if you want it
# elsewhere; the previous /tmp default lost the clone on reboot.

set -euo pipefail

SYSTEM_OPERATOR_DIR="${SYSTEM_OPERATOR_DIR:-$HOME/.config/terraphim/system_operator}"
CONFIG_FILE="terraphim_server/default/system_operator_config.json"
SERVER_SETTINGS="terraphim_server/default/settings_system_operator_server.toml"

echo "[setup] Target directory: ${SYSTEM_OPERATOR_DIR}"

mkdir -p "${SYSTEM_OPERATOR_DIR}"

if [ -d "${SYSTEM_OPERATOR_DIR}/.git" ]; then
    echo "[setup] Repository exists, updating..."
    git -C "${SYSTEM_OPERATOR_DIR}" pull --ff-only origin main
else
    echo "[setup] Cloning system-operator repository..."
    git clone https://github.com/terraphim/system-operator.git "${SYSTEM_OPERATOR_DIR}"
fi

if [ ! -d "${SYSTEM_OPERATOR_DIR}/pages" ]; then
    echo "[setup] ERROR: pages/ directory not found after clone." >&2
    exit 1
fi

PAGE_COUNT=$(find "${SYSTEM_OPERATOR_DIR}/pages" -name "*.md" | wc -l | tr -d ' ')
SYN_COUNT=$(grep -l "^synonyms::" "${SYSTEM_OPERATOR_DIR}/pages/"*.md 2>/dev/null | wc -l | tr -d ' ')

echo
echo "[setup] Repository ready:"
echo "  - Pages:             ${PAGE_COUNT} markdown files"
echo "  - Synonym entries:   ${SYN_COUNT} Terraphim-format files"
echo "  - Config:            ${CONFIG_FILE}"
echo "  - Settings:          ${SERVER_SETTINGS}"
echo
echo "[setup] Drive via terraphim_server:"
echo "  cargo run --bin terraphim_server -- --config ${CONFIG_FILE}"
echo
echo "[setup] Drive via terraphim-agent CLI (after adding the role to"
echo "        ~/.config/terraphim/embedded_config.json, see how-to docs):"
echo "  terraphim-agent config reload"
echo "  terraphim-agent search --role \"System Operator\" --limit 5 \"RFP\""
echo
echo "[setup] Available roles in the server config:"
echo "  - System Operator (default): TerraphimGraph, Logseq KG, MBSE vocabulary"
echo "  - Engineer:                  TerraphimGraph, same KG, engineering theme"
echo "  - Default:                   TitleScorer, basic text matching"
echo
echo "[setup] Done."
