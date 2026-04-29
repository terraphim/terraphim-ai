#!/bin/bash
#
# fix-adf-permissions.sh -- Fix file permissions and clean backup files
# for the AI Dark Factory orchestrator configuration.
#
# Usage:
#   sudo ./fix-adf-permissions.sh
#
set -euo pipefail

ADF_DIR="/opt/ai-dark-factory"

echo "=== Fixing ADF permissions ==="

# Fix orchestrator.toml permissions (contains webhook secret)
if [[ -f "$ADF_DIR/orchestrator.toml" ]]; then
    echo "Setting $ADF_DIR/orchestrator.toml to 600..."
    chmod 600 "$ADF_DIR/orchestrator.toml"
fi

# Fix conf.d permissions
if [[ -d "$ADF_DIR/conf.d" ]]; then
    echo "Setting $ADF_DIR/conf.d/*.toml to 600..."
    chmod 600 "$ADF_DIR/conf.d"/*.toml 2>/dev/null || true
fi

# Set directory ownership (adjust user as needed)
# echo "Setting $ADF_DIR ownership to alex:alex..."
# chown -R alex:alex "$ADF_DIR"

echo "=== Cleaning backup files ==="

# Remove .bak-* files from conf.d
if [[ -d "$ADF_DIR/conf.d" ]]; then
    backup_count=$(find "$ADF_DIR/conf.d" -name '.bak-*' -type f | wc -l)
    if [[ $backup_count -gt 0 ]]; then
        echo "Removing $backup_count backup files from $ADF_DIR/conf.d..."
        find "$ADF_DIR/conf.d" -name '.bak-*' -type f -delete
    else
        echo "No backup files found in $ADF_DIR/conf.d"
    fi
fi

echo "=== Done ==="
echo ""
echo "Verification:"
ls -la "$ADF_DIR/orchestrator.toml"
ls -la "$ADF_DIR/conf.d/" 2>/dev/null || true
