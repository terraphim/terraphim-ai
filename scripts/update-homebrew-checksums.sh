#!/bin/bash
set -e

# Update Homebrew formulas with SHA256 checksums from built binaries
# Usage: ./scripts/update-homebrew-checksums.sh <version> <repl_binary_path> <cli_binary_path>

VERSION=${1:-"1.0.0"}
REPL_BINARY=${2:-"releases/v${VERSION}/terraphim-repl-linux-x86_64"}
CLI_BINARY=${3:-"releases/v${VERSION}/terraphim-cli-linux-x86_64"}

echo "Updating Homebrew formulas for v${VERSION}"
echo "REPL binary: $REPL_BINARY"
echo "CLI binary: $CLI_BINARY"

# Check if binaries exist
if [ ! -f "$REPL_BINARY" ]; then
    echo "Error: REPL binary not found: $REPL_BINARY"
    exit 1
fi

if [ ! -f "$CLI_BINARY" ]; then
    echo "Error: CLI binary not found: $CLI_BINARY"
    exit 1
fi

# Calculate SHA256 checksums
REPL_SHA256=$(sha256sum "$REPL_BINARY" | cut -d' ' -f1)
CLI_SHA256=$(sha256sum "$CLI_BINARY" | cut -d' ' -f1)

echo "REPL SHA256: $REPL_SHA256"
echo "CLI SHA256: $CLI_SHA256"

# Update terraphim-repl.rb
if [ -f "homebrew-formulas/terraphim-repl.rb" ]; then
    echo "Updating terraphim-repl.rb..."

    # Update version
    sed -i "s/version \".*\"/version \"$VERSION\"/" homebrew-formulas/terraphim-repl.rb

    # Update download URL
    sed -i "s|download/v.*/terraphim-repl-linux|download/v$VERSION/terraphim-repl-linux|" homebrew-formulas/terraphim-repl.rb

    # Update SHA256 (find the Linux section and update)
    sed -i "/on_linux do/,/end/{s/sha256 \".*\"/sha256 \"$REPL_SHA256\"/}" homebrew-formulas/terraphim-repl.rb

    echo "✓ Updated terraphim-repl.rb"
else
    echo "Warning: homebrew-formulas/terraphim-repl.rb not found"
fi

# Update terraphim-cli.rb
if [ -f "homebrew-formulas/terraphim-cli.rb" ]; then
    echo "Updating terraphim-cli.rb..."

    # Update version
    sed -i "s/version \".*\"/version \"$VERSION\"/" homebrew-formulas/terraphim-cli.rb

    # Update download URL
    sed -i "s|download/v.*/terraphim-cli-linux|download/v$VERSION/terraphim-cli-linux|" homebrew-formulas/terraphim-cli.rb

    # Update SHA256 (find the Linux section and update)
    sed -i "/on_linux do/,/end/{s/sha256 \".*\"/sha256 \"$CLI_SHA256\"/}" homebrew-formulas/terraphim-cli.rb

    echo "✓ Updated terraphim-cli.rb"
else
    echo "Warning: homebrew-formulas/terraphim-cli.rb not found"
fi

echo ""
echo "Homebrew formulas updated successfully!"
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff homebrew-formulas/"
echo "  2. Commit: git add homebrew-formulas/ && git commit -m 'Update Homebrew formulas for v${VERSION}'"
echo "  3. Push: git push"
