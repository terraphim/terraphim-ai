#!/bin/bash
# Deploy RCH bigbox config. Idempotent; safe to re-run.
# Run on bigbox as user alex with sudo available.
#
# Prerequisites (one-time):
#   curl -fsSL https://raw.githubusercontent.com/Dicklesworthstone/remote_compilation_helper/main/install.sh | bash
#   ~/.ssh/id_ed25519 exists and pubkey is in ~/.ssh/authorized_keys
#
# Topology decision: rch v1.0.16 hardcodes /data/projects as the canonical
# project root and ignores RCH_CANONICAL_PROJECT_ROOT (verified). To make
# every project under /home/alex/projects/* (current and future) eligible
# for rch dispatch without per-project bind-mount maintenance, we replace
# /data/projects with a symlink to /home/alex/projects. rch canonicalize()
# resolves both sides to /home/alex/projects, so any subdirectory is in
# scope. Projects outside this tree (e.g. /home/alex/terraphim-ai, the ADF
# terraphim orchestrator's working dir) fail the canonical-root check and
# fall open to local cargo (rch's fail-open design).

set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"

echo "=== topology: /data/projects -> /home/alex/projects ==="
mkdir -p /home/alex/projects
if [ -L /data/projects ]; then
    cur=$(readlink /data/projects)
    if [ "$cur" = "/home/alex/projects" ]; then
        echo "OK already symlinked"
    else
        echo "REPOINT /data/projects (was -> $cur)"
        sudo ln -sfn /home/alex/projects /data/projects
    fi
elif [ -d /data/projects ]; then
    # Migrating from a real /data/projects directory: drain it into
    # /home/alex/projects, prune any in-place bind-mounts.
    while read -r line; do
        [[ "$line" =~ ^/.*/data/projects/.*bind ]] || continue
        dst=$(echo "$line" | awk '{print $2}')
        mountpoint -q "$dst" && sudo umount "$dst"
    done < <(grep -E "^/.*/data/projects/.*bind" /etc/fstab || true)
    sudo sed -i '\#/data/projects.*bind#d' /etc/fstab

    for entry in /data/projects/*; do
        [ -e "$entry" ] || continue
        base=$(basename "$entry")
        [ -e "/home/alex/projects/$base" ] && continue
        sudo mv "$entry" "/home/alex/projects/$base"
    done
    sudo find /data/projects -mindepth 1 -maxdepth 1 -type d -empty -delete 2>/dev/null || true
    sudo rmdir /data/projects
    sudo ln -s /home/alex/projects /data/projects
fi
ls -la /data/projects

echo
echo "=== /dp -> /data/projects alias (rch topology audit) ==="
sudo ln -sfn /data/projects /dp
ls -la /dp

echo
echo "=== sshd Match drop-in for localhost ==="
sudo install -m 644 -o root -g root \
    "$HERE/10-rch-localhost.conf" \
    /etc/ssh/sshd_config.d/10-rch-localhost.conf
sudo sshd -t && sudo systemctl reload ssh

echo
echo "=== ~/.config/rch/ files ==="
mkdir -p "$HOME/.config/rch"
install -m 644 "$HERE/workers.toml" "$HOME/.config/rch/workers.toml"
install -m 644 "$HERE/config.toml" "$HOME/.config/rch/config.toml"

echo
echo "=== rchd systemd user unit ==="
mkdir -p "$HOME/.config/systemd/user"
install -m 644 "$HERE/rchd.service" "$HOME/.config/systemd/user/rchd.service"
systemctl --user daemon-reload
systemctl --user enable rchd
systemctl --user restart rchd

sleep 2
echo
echo "=== verify ==="
~/.local/bin/rch status | head -10
~/.local/bin/rch workers probe --all 2>&1 | grep -E "OK|FAIL"
