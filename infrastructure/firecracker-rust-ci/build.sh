#!/bin/bash
# build.sh -- produce rust-ci.ext4, a Firecracker-bootable Ubuntu 22.04
# rootfs with systemd-as-PID-1, sshd, Rust stable, sccache, and the
# SeaweedFS S3 cache env baked in.
#
# Pattern: privileged docker run on ubuntu:22.04 image, run chroot.sh inside,
# tar out the resulting filesystem, convert tar -> ext4 with hcsshim's
# tar2ext4 tool. Adapted from upstream firecracker-microvm/firecracker
# (resources/rebuild.sh + tools/functions:build_rootfs).
#
# Usage:
#   ./build.sh <ssh_pubkey_path> <output_path>
#
# Example:
#   ./build.sh ~/projects/terraphim/firecracker-rust/firecracker-ci-artifacts/ubuntu-22.04.id_rsa \
#              ~/projects/terraphim/firecracker-rust/firecracker-ci-artifacts/rust-ci.ext4

set -euo pipefail
PS4='+\t '

SSH_KEY=${1:?usage: $0 <ssh_private_key_path> <output_ext4_path>}
OUT=${2:?usage: $0 <ssh_private_key_path> <output_ext4_path>}

if [ ! -r "$SSH_KEY" ]; then
    echo "ERROR: ssh private key $SSH_KEY not readable" >&2
    exit 1
fi

WORK=$(mktemp -d -t fc-rust-ci-XXXXXX)
trap 'rm -rf "$WORK"' EXIT
echo "Working dir: $WORK"

cp -a "$(dirname "$0")"/{chroot.sh,overlay} "$WORK/"

# Derive the matching pubkey and stage it where the overlay expects it.
ssh-keygen -y -f "$SSH_KEY" > "$WORK/overlay/root/.ssh/authorized_keys"
chmod 600 "$WORK/overlay/root/.ssh/authorized_keys"

mkdir -p "$WORK/rootfs"

echo "=== Phase 1: build filesystem inside privileged ubuntu:22.04 container ==="
docker run --rm --privileged \
    -v "$WORK:/work" -w /work \
    ubuntu:22.04 \
    bash -c '
        set -eu
        # Apply overlay first so chroot.sh can reference any prepared files.
        cp -rv /work/overlay/* /
        bash /work/chroot.sh
        # Tar back into the work dir (excluding mounted-in dirs).
        cd /
        tar c \
            --exclude="./proc/*" --exclude="./sys/*" \
            --exclude="./dev/*" --exclude="./run/*" \
            --exclude="./tmp/*" --exclude="./work/*" \
            ./bin ./etc ./home ./lib ./lib64 ./root ./sbin ./usr ./var \
            > /work/rootfs.tar
    '

echo "=== Phase 2a: post-process tar (inject /etc/resolv.conf) ==="
# Docker bind-mounts /etc/resolv.conf inside the build container, so any write
# from chroot.sh is transient. Inject a static resolver into the tar on the
# host side. Same approach as upstream firecracker resources/rebuild.sh.
STAGE="$WORK/stage"
mkdir -p "$STAGE"
sudo tar -xf "$WORK/rootfs.tar" -C "$STAGE"
sudo tee "$STAGE/etc/resolv.conf" > /dev/null <<'EOF'
nameserver 1.1.1.1
nameserver 8.8.8.8
EOF
sudo tar -cf "$WORK/rootfs.tar" -C "$STAGE" .

echo "=== Phase 2b: convert tar -> ext4 via tar2ext4 ==="
TAR2EXT4=$(which tar2ext4 2>/dev/null || true)
if [ -z "$TAR2EXT4" ]; then
    # Build tar2ext4 in a Go container if not on PATH.
    docker run --rm -v "$WORK:/work" -w /work golang:1-alpine sh -c '
        apk add --no-cache git
        git clone --depth 1 https://github.com/microsoft/hcsshim
        cd hcsshim
        GOTOOLCHAIN=auto go build -o /work/tar2ext4 ./cmd/tar2ext4
    '
    TAR2EXT4="$WORK/tar2ext4"
fi

# 4 GB rootfs, plenty for Rust toolchain + workspace builds.
"$TAR2EXT4" -i "$WORK/rootfs.tar" -o "$OUT"
tune2fs -O ^read-only "$OUT"
dd if=/dev/zero bs=1M seek=4096 count=0 of="$OUT"
e2fsck -y -f "$OUT" || true
resize2fs "$OUT"

ls -lh "$OUT"
echo "=== done: $OUT ==="
