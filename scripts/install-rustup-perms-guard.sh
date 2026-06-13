#!/usr/bin/env bash
# install-rustup-perms-guard.sh — wrap rustup binaries to auto-repair toolchain +x.
#
# Installs rustup-with-perms.sh to ~/.local/bin and replaces each target rustup
# with a stub that sets RUSTUP_REAL and execs the wrapper. Idempotent.
# Refs terraphim/terraphim-ai#2463.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WRAPPER_SRC="$REPO_ROOT/scripts/rustup-with-perms.sh"
FIX_SRC="$REPO_ROOT/scripts/fix-rust-toolchain-perms.sh"
LOCAL_BIN="${HOME}/.local/bin"
CRON_MARKER="# terraphim rust toolchain perms guard"

install -d "$LOCAL_BIN"
install -m 0755 "$WRAPPER_SRC" "$LOCAL_BIN/rustup-with-perms.sh"
install -m 0755 "$FIX_SRC" "$LOCAL_BIN/fix-rust-toolchain-perms.sh"

wrap_rustup_in() {
  local bindir="$1"
  local rustup="$bindir/rustup"

  [[ -d "$bindir" ]] || return 0
  [[ -e "$rustup" || -L "$rustup" ]] || return 0

  if [[ -f "$rustup" ]] && grep -q 'rustup-with-perms.sh' "$rustup" 2>/dev/null; then
    return 0
  fi

  local real="$bindir/rustup.real"
  if [[ ! -f "$real" ]]; then
    if [[ -L "$rustup" ]]; then
      local target
      target="$(readlink -f "$rustup")"
      cp -a "$target" "$real"
      rm -f "$rustup"
    elif [[ -f "$rustup" ]]; then
      mv "$rustup" "$real"
    else
      echo "install-rustup-perms-guard: skip $rustup (missing)" >&2
      return 0
    fi
    chmod +x "$real"
  fi

  cat >"$rustup" <<EOF
#!/usr/bin/env bash
export RUSTUP_REAL="$real"
export RUSTUP_INVOKE_AS="\$(basename "\$0")"
exec "$LOCAL_BIN/rustup-with-perms.sh" "\$@"
EOF
  chmod +x "$rustup"
  echo "wrapped: $rustup -> rustup.real"
}

# Primary rustup used by native-ci and interactive shells.
wrap_rustup_in "${HOME}/.cargo/bin"

# Isolated cargo-runner installs (share ~/.rustup toolchains).
for env_file in "${HOME}"/.cargo-runner-*/env; do
  [[ -f "$env_file" ]] || continue
  bindir="$(dirname "$env_file")/bin"
  wrap_rustup_in "$bindir"
done

# Daily safety net (in addition to runner-health-check every 10 min).
if command -v crontab >/dev/null 2>&1; then
  existing="$(crontab -l 2>/dev/null || true)"
  if ! echo "$existing" | grep -Fq "$CRON_MARKER"; then
    {
      echo "$existing"
      echo "15 4 * * * $LOCAL_BIN/fix-rust-toolchain-perms.sh >> $HOME/logs/rust-toolchain-perms.log 2>&1 $CRON_MARKER"
    } | crontab -
    echo "cron: daily fix-rust-toolchain-perms at 04:15"
  else
    echo "cron: already installed"
  fi
fi

mkdir -p "$HOME/logs"
echo "install-rustup-perms-guard: done"