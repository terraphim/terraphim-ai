#!/usr/bin/env bash
# Build the Svelte frontend and copy output to terraphim_server/dist/
# so that cargo build embeds a working web UI via rust-embed.
#
# Usage:
#   ./scripts/build-frontend-for-server.sh
#   ./scripts/build-frontend-for-server.sh --clean  # remove non-essential theme files
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$SCRIPT_DIR")"
DESKTOP_DIR="$ROOT/desktop"
DIST_DIR="$ROOT/terraphim_server/dist"

echo "Building frontend from $DESKTOP_DIR ..."

cd "$DESKTOP_DIR"

# Install dependencies
if command -v yarn &>/dev/null; then
    yarn install --frozen-lockfile
elif command -v bun &>/dev/null; then
    bun install
else
    npm ci
fi

# Build
if command -v yarn &>/dev/null; then
    yarn build
elif command -v bun &>/dev/null; then
    bunx vite build
else
    npx vite build
fi

echo "Copying build output to $DIST_DIR ..."

# Clean previous build
if [ -d "$DIST_DIR" ]; then
    /usr/bin/find "$DIST_DIR" -mindepth 1 -delete 2>/dev/null || find "$DIST_DIR" -mindepth 1 -delete
fi
mkdir -p "$DIST_DIR"

# Copy fresh build
cp -a "$DESKTOP_DIR/dist/." "$DIST_DIR/"

# Optionally trim non-essential bulmaswatch files (scss, source maps, thumbnails)
if [[ "${1:-}" == "--clean" ]]; then
    echo "Trimming non-essential bulmaswatch files ..."
    /usr/bin/find "$DIST_DIR/assets/bulmaswatch" \
        \( -name "*.scss" -o -name "*.css.map" -o -name "*.png" \
           -o -name "*.ico" -o -name ".jsbeautifyrc" -o -name "Gemfile" \
           -o -name "bookmarklet.js" -o -name "package.json" \) \
        -delete 2>/dev/null || true
    /usr/bin/find "$DIST_DIR/assets/bulmaswatch" -name "api" -type d \
        -exec rm -r {} + 2>/dev/null || true
fi

# Strip trailing whitespace from generated JS (pre-commit hooks check for it)
sed -i 's/[[:space:]]*$//' "$DIST_DIR"/assets/*.js 2>/dev/null || true

echo "Frontend copied to $DIST_DIR"
echo "Run 'cargo build -p terraphim_server' to embed the new assets."
