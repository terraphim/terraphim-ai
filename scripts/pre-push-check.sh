#!/bin/bash
#
# Pre-push hook: cargo check --workspace --all-features
#
# Catches --all-features compilation breaks on the developer machine before
# a full CI round-trip is needed. Runs in under 90 seconds on a warm cache.
#
# Escape hatch (for WIP pushes to non-main branches):
#   SKIP_ALL_FEATURES_CHECK=1 git push
#
# This hook is installed by scripts/install-hooks.sh as .git/hooks/pre-push
# and works alongside the CI gate added by issue #1295.
#
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ "${SKIP_ALL_FEATURES_CHECK:-0}" = "1" ]; then
    echo -e "${YELLOW}[pre-push] SKIP_ALL_FEATURES_CHECK=1: skipping --all-features check${NC}"
    exit 0
fi

echo -e "${GREEN}[pre-push] Running cargo check --workspace --all-features ...${NC}"
echo "           (Set SKIP_ALL_FEATURES_CHECK=1 to bypass on WIP branches)"

if cargo check --workspace --all-features 2>&1; then
    echo -e "${GREEN}[pre-push] All features check passed.${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}[pre-push] ERROR: cargo check --workspace --all-features failed.${NC}"
    echo -e "${RED}           Fix the compilation errors above before pushing.${NC}"
    echo ""
    echo "           To bypass (WIP branches only):"
    echo "             SKIP_ALL_FEATURES_CHECK=1 git push"
    echo ""
    exit 1
fi
