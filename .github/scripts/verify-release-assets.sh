#!/usr/bin/env bash
# Verify that all expected release assets are present
# Usage: verify-release-assets.sh <version> <release-assets-dir>

set -euo pipefail

VERSION="${1:-}"
ASSETS_DIR="${2:-./release-assets}"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version> [assets-dir]"
    echo "Example: $0 1.20.1 ./release-assets"
    exit 1
fi

echo "Verifying release assets for version $VERSION in $ASSETS_DIR"
echo ""

# List all downloaded files
echo "Assets found:"
find "$ASSETS_DIR" -maxdepth 1 -type f | sort
echo ""

# Define expected assets per platform
# Format: "filename:description"
EXPECTED_ASSETS=(
    # macOS Intel
    "terraphim-agent-x86_64-apple-darwin:macOS Intel raw binary"
    "terraphim-agent-${VERSION}-x86_64-apple-darwin.tar.gz:macOS Intel archive"
    "terraphim-grep-x86_64-apple-darwin:macOS Intel raw binary"
    "terraphim-grep-${VERSION}-x86_64-apple-darwin.tar.gz:macOS Intel archive"
    "terraphim-cli-x86_64-apple-darwin:macOS Intel raw binary"
    "terraphim-cli-${VERSION}-x86_64-apple-darwin.tar.gz:macOS Intel archive"
    
    # macOS Apple Silicon
    "terraphim-agent-aarch64-apple-darwin:macOS Apple Silicon raw binary"
    "terraphim-agent-${VERSION}-aarch64-apple-darwin.tar.gz:macOS Apple Silicon archive"
    "terraphim-grep-aarch64-apple-darwin:macOS Apple Silicon raw binary"
    "terraphim-grep-${VERSION}-aarch64-apple-darwin.tar.gz:macOS Apple Silicon archive"
    
    # macOS Universal
    "terraphim-agent-universal-apple-darwin:macOS universal raw binary"
    "terraphim-grep-universal-apple-darwin:macOS universal raw binary"
    
    # Linux x86_64 GNU
    "terraphim-agent-x86_64-unknown-linux-gnu:Linux x86_64 GNU raw binary"
    "terraphim-agent-${VERSION}-x86_64-unknown-linux-gnu.tar.gz:Linux x86_64 GNU archive"
    "terraphim-grep-x86_64-unknown-linux-gnu:Linux x86_64 GNU raw binary"
    "terraphim-grep-${VERSION}-x86_64-unknown-linux-gnu.tar.gz:Linux x86_64 GNU archive"
    
    # Linux x86_64 MUSL
    "terraphim-agent-x86_64-unknown-linux-musl:Linux x86_64 MUSL raw binary"
    "terraphim-agent-${VERSION}-x86_64-unknown-linux-musl.tar.gz:Linux x86_64 MUSL archive"
    
    # Linux ARM64
    "terraphim-agent-aarch64-unknown-linux-musl:Linux ARM64 MUSL raw binary"
    "terraphim-agent-${VERSION}-aarch64-unknown-linux-musl.tar.gz:Linux ARM64 MUSL archive"
    
    # Windows
    "terraphim-agent-x86_64-pc-windows-msvc.exe:Windows x86_64 binary"
    "terraphim-agent-${VERSION}-x86_64-pc-windows-msvc.zip:Windows x86_64 archive"
)

MISSING=0
FOUND=0

for entry in "${EXPECTED_ASSETS[@]}"; do
    filename="${entry%%:*}"
    description="${entry#*:}"
    filepath="$ASSETS_DIR/$filename"
    
    if [ -f "$filepath" ]; then
        echo "  ✅ $description"
        FOUND=$((FOUND + 1))
    else
        echo "  ❌ $description ($filename)"
        MISSING=$((MISSING + 1))
    fi
done

echo ""
echo "Results: $FOUND found, $MISSING missing"

if [ $MISSING -gt 0 ]; then
    echo "❌ Verification failed: $MISSING expected assets are missing"
    echo ""
    echo "Note: Some assets may be intentionally missing if their build was excluded."
    echo "Check the CI build logs for any failed build jobs."
    exit 1
fi

echo "✅ All expected assets verified successfully"
exit 0
