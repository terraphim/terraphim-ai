#!/bin/bash
# Cleanup build artifacts to prevent target directory bloat
# Usage: ./scripts/cleanup-build.sh [options]
# Options:
#   --aggressive  Remove more artifacts (docs, unused deps)
#   --dry-run     Show what would be deleted without actually deleting

set -euo pipefail

AGGRESSIVE=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --aggressive)
            AGGRESSIVE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--aggressive] [--dry-run]"
            exit 1
            ;;
    esac
done

cd "$(git rev-parse --show-toplevel)" || exit 1

echo "=== Terraphim Build Cleanup ==="
echo "Mode: $([ "$DRY_RUN" = true ] && echo 'DRY RUN (no changes)' || echo 'ACTIVE')"
echo "Aggressive: $AGGRESSIVE"
echo ""

# Calculate current size
echo "Current target directory size:"
du -sh target/ 2>/dev/null || echo "No target directory found"
echo ""

# Cleanup commands
cleanup_commands=(
    "Remove old release builds older than 7 days: find target/release -name '*.rlib' -mtime +7 -delete"
    "Remove debug rlibs older than 3 days: find target/debug -name '*.rlib' -mtime +3 -delete"
    "Remove doc directory: rm -rf target/doc"
    "Remove incremental compilation: rm -rf target/*/incremental"
)

if [ "$AGGRESSIVE" = true ]; then
    cleanup_commands+=(
        "Remove all debug builds: cargo clean --debug"
        "Remove example builds: find target -name 'examples' -type d -exec rm -rf {} +"
        "Remove benchmark builds: find target -name 'benches' -type d -exec rm -rf {} +"
    )
fi

# Execute cleanup
for cmd in "${cleanup_commands[@]}"; do
    echo "Executing: $cmd"
    if [ "$DRY_RUN" = false ]; then
        eval "$cmd" 2>/dev/null || true
    fi
done

echo ""

# Show sizes of large directories
echo "=== Large directories in target ==="

find target -maxdepth 2 -type d -exec du -sh {} \; 2>/dev/null | sort -rh | head -20 || true

echo ""

# Final size after cleanup
echo "Target directory size after cleanup:"
du -sh target/ 2>/dev/null || echo "No target directory found"

echo ""
if [ "$DRY_RUN" = false ]; then
    echo "Cleanup completed!"
    echo "Run with --dry-run to preview changes"
else
    echo "Dry run completed. Run without --dry-run to execute cleanup."
fi
