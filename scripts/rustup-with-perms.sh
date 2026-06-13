#!/usr/bin/env bash
# rustup-with-perms.sh — delegate to rustup.real, then repair toolchain +x.
#
# rustup 1.29.0 on Linux can leave toolchains/*/bin/* as 644 after update/install,
# breaking CI (cargo fmt: Permission denied os error 13). Refs #2463.
#
# argv[0] fix (Refs #2462): exec -a preserves the proxy name (cargo, rustfmt, etc.)
# so rustup.real dispatches to the correct toolchain binary. Shell scripts cannot
# receive argv[0] via exec -a, so the caller must export RUSTUP_INVOKE_AS first.
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

if rustup_needs_perm_fix "$@"; then
  # Actual rustup command: run normally so we can repair perms afterwards.
  "$RUSTUP_REAL" "$@"
  status=$?
  if [[ $status -eq 0 ]]; then
    if [[ -x "$FIX_SCRIPT" ]]; then
      "$FIX_SCRIPT" || echo "rustup-with-perms: warn: $FIX_SCRIPT failed" >&2
    else
      echo "rustup-with-perms: warn: fix script not found: $FIX_SCRIPT" >&2
    fi
  fi
  exit "$status"
else
  # Proxy dispatch (cargo, rustfmt, …): must preserve argv[0] so rustup.real
  # dispatches to the correct toolchain binary instead of treating args as a
  # toolchain override. RUSTUP_INVOKE_AS is exported by the ~./cargo/bin/rustup
  # wrapper because exec -a does not cross shell-script boundaries.
  exec -a "${RUSTUP_INVOKE_AS:-$(basename "$0")}" "$RUSTUP_REAL" "$@"
fi