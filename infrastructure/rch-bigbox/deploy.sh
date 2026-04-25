#!/bin/bash
# Deploy RCH bigbox config. Idempotent; safe to re-run.
# Run on bigbox as user alex with sudo available.
#
# Prerequisites (one-time):
#   curl -fsSL https://raw.githubusercontent.com/Dicklesworthstone/remote_compilation_helper/main/install.sh | bash
#   ~/.ssh/id_ed25519 exists and pubkey is in ~/.ssh/authorized_keys

set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"

echo "=== /dp -> /data/projects symlink ==="
sudo mkdir -p /data/projects
sudo ln -sfn /data/projects /dp
ls -la /dp

echo "=== /data/projects/terraphim-ai bind-mount ==="
SRC=/home/alex/projects/terraphim/terraphim-ai
DST=/data/projects/terraphim-ai
if mountpoint -q "$DST"; then
    echo "$DST already a mountpoint"
else
    sudo mkdir -p "$DST"
    sudo mount --bind "$SRC" "$DST"
    echo "bind-mounted $SRC -> $DST"
fi
# Persist
if ! grep -qE "^$SRC\s+$DST\s+none\s+bind" /etc/fstab; then
    echo "$SRC  $DST  none  bind  0  0" | sudo tee -a /etc/fstab > /dev/null
    echo "added to /etc/fstab"
fi

echo "=== sshd Match drop-in for localhost ==="
sudo install -m 644 -o root -g root \
    "$HERE/10-rch-localhost.conf" \
    /etc/ssh/sshd_config.d/10-rch-localhost.conf
sudo sshd -t && sudo systemctl reload ssh
echo "sshd reloaded"

echo "=== ~/.config/rch/ files ==="
mkdir -p "$HOME/.config/rch"
install -m 644 "$HERE/workers.toml" "$HOME/.config/rch/workers.toml"
install -m 644 "$HERE/config.toml" "$HOME/.config/rch/config.toml"

echo "=== rchd systemd user unit ==="
mkdir -p "$HOME/.config/systemd/user"
install -m 644 "$HERE/rchd.service" "$HOME/.config/systemd/user/rchd.service"
systemctl --user daemon-reload
systemctl --user enable rchd
systemctl --user restart rchd

sleep 2
echo "=== verify ==="
~/.local/bin/rch status | head -10
~/.local/bin/rch workers probe --all 2>&1 | grep -E "OK|FAIL"
