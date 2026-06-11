#!/usr/bin/env bash
# fix-rust-toolchain-perms.sh — restore +x on rustup toolchain binaries.
#
# bigbox rustup installs (2026-06-08) left bin/* as -rw-r--r--, breaking
# native-ci at cargo fmt (Permission denied os error 13). Run after every
# rustup update/install. Refs terraphim/terraphim-ai#2463.
set -euo pipefail

RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

if [[ ! -d "$RUSTUP_HOME/toolchains" ]]; then
  echo "fix-rust-toolchain-perms: no toolchains under $RUSTUP_HOME" >&2
  exit 0
fi

fixed=0
while IFS= read -r -d '' bin; do
  if [[ ! -x "$bin" ]]; then
    chmod +x "$bin"
    echo "fixed: $bin"
    fixed=$((fixed + 1))
  fi
done < <(find "$RUSTUP_HOME/toolchains" -path '*/bin/*' -type f -print0 2>/dev/null)

# rustup shim itself (cargo -> rustup symlink target)
if [[ -f "$HOME/.cargo/bin/rustup" && ! -x "$HOME/.cargo/bin/rustup" ]]; then
  chmod +x "$HOME/.cargo/bin/rustup"
  echo "fixed: $HOME/.cargo/bin/rustup"
  fixed=$((fixed + 1))
fi

echo "fix-rust-toolchain-perms: $fixed file(s) repaired"