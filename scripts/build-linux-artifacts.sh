#!/usr/bin/env bash
# Build all Linux artifacts (backend debs + desktop deb/rpm/appimage)
# Requirements: cargo, rust toolchain, yarn, tauri CLI (1.x), cargo-deb, appimagetool

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Ensuring appimagetool exists"
APPIMG_BIN="${HOME}/bin/appimagetool"
if [[ ! -x "$APPIMG_BIN" ]]; then
  mkdir -p "${HOME}/bin"
  wget -q -O "$APPIMG_BIN" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x "$APPIMG_BIN"
fi
export PATH="${HOME}/bin:${PATH}"
export TAURI_BUNDLE_APPIMAGE_BUNDLE_BIN="$APPIMG_BIN"

echo "==> Building Rust workspace (release)"
cargo build --release

echo "==> Building deb packages (cargo-deb)"
cargo deb -p terraphim_server
cargo deb -p terraphim_agent

echo "==> Installing desktop dependencies"
cd "$ROOT/desktop"
yarn install --frozen-lockfile

echo "==> Building desktop bundles (deb, rpm)"
yarn tauri build --bundles deb rpm --target x86_64-unknown-linux-gnu

echo "==> Attempting AppImage via tauri (may fail due to gtk plugin)"
if yarn tauri build --bundles appimage --target x86_64-unknown-linux-gnu; then
  echo "Tauri AppImage build succeeded."
else
  echo "Tauri AppImage build failed; attempting manual appimagetool packaging."
fi

APPDIR="$ROOT/target/x86_64-unknown-linux-gnu/release/bundle/appimage/terraphim-desktop.AppDir"
if [[ -d "$APPDIR" ]]; then
  echo "==> Building AppImage manually from $APPDIR"
  (cd "$(dirname "$APPDIR")" && appimagetool "$APPDIR" terraphim-desktop_1.0.0_amd64.AppImage)
else
  echo "!! AppDir not found; skipping manual AppImage build."
fi

echo "Artifacts:"
echo "  Backend debs: target/debian/"
echo "  Desktop deb/rpm/appimage: target/x86_64-unknown-linux-gnu/release/bundle/{deb,rpm,appimage}/"

