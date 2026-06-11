#!/usr/bin/env bash
# rustup-with-perms.sh — delegate to rustup.real, then repair toolchain +x.
#
# rustup 1.29.0 on Linux can leave toolchains/*/bin/* as 644 after update/install,
# breaking CI (cargo fmt: Permission denied os error 13). Refs #2463.
set -euo pipefail

if [[ -z "${RUSTUP_REAL:-}" ]]; then
  echo "rustup-with-perms: RUSTUP_REAL not set" >&2
  exit 1
fi

if [[ ! -x "$RUSTUP_REAL" ]]; then
  echo "rustup-with-perms: missing executable: $RUSTUP_REAL" >&2
  exit 1
fi

FIX_SCRIPT="${FIX_RUST_PERMS_SCRIPT:-$HOME/.local/bin/fix-rust-toolchain-perms.sh}"

rustup_needs_perm_fix() {
  case "${1:-}" in
    update | install | default | toolchain | component | target) return 0 ;;
    self) [[ "${2:-}" == update ]] && return 0 ;;
  esac
  return 1
}

"$RUSTUP_REAL" "$@"
status=$?

if [[ $status -eq 0 ]] && rustup_needs_perm_fix "$@"; then
  if [[ -x "$FIX_SCRIPT" ]]; then
    "$FIX_SCRIPT" || echo "rustup-with-perms: warn: $FIX_SCRIPT failed" >&2
  else
    echo "rustup-with-perms: warn: fix script not found: $FIX_SCRIPT" >&2
  fi
fi

exit "$status"