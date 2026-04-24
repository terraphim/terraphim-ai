#!/bin/bash
# chroot.sh -- runs INSIDE a privileged ubuntu:22.04 container during rootfs
# build. Installs systemd, sshd, gcc/build-essential, Rust toolchain, and
# sccache. Pattern adapted from
# https://github.com/firecracker-microvm/firecracker/blob/main/resources/chroot.sh
#
# Outputs are extracted by build.sh after this script completes.

set -eu -o pipefail
set -x
PS4='+\t '

# Base packages: systemd-sysv is mandatory so PID 1 is real systemd and sshd
# actually starts. openssh-server + iproute2 + curl give us the runtime;
# build-essential + pkg-config + libssl-dev + ca-certificates let us actually
# compile Rust crates.
packages="
  udev
  systemd-sysv
  openssh-server
  iproute2
  ca-certificates
  curl
  build-essential
  pkg-config
  libssl-dev
  git
  python3-minimal
"

export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -yqq --no-install-recommends $packages
apt-get autoremove -yqq

# Hostname.
echo "rust-ci" > /etc/hostname

# Delete root password so console autologin works without prompting.
passwd -d root

# Serial console autologin (matches upstream Firecracker pattern).
mkdir -p /etc/systemd/system/serial-getty@ttyS0.service.d
cat > /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin root -o '-p -- \\u' --keep-baud 115200,38400,9600 %I dumb
EOF

# Mask resolved so it doesn't take over /etc/resolv.conf at boot. Disabling
# via .wants symlinks is not enough -- Ubuntu enables it via /lib alias too,
# and on boot it would replace /etc/resolv.conf with a symlink to its stub
# resolver at 127.0.0.53 (which has no upstream → DNS dead in the VM).
ln -sf /dev/null /etc/systemd/system/systemd-resolved.service
ln -sf /dev/null /etc/systemd/system/systemd-timesyncd.service
rm -f /etc/systemd/system/dbus-org.freedesktop.resolve1.service

# NOTE: /etc/resolv.conf is bind-mounted by Docker during this build, so any
# write here is transient. The final file is written by build.sh on the host
# side after tar extraction. See upstream firecracker resources/rebuild.sh.

# Disable Predictable Network Interface Names so kernel ip= -> eth0.
ln -sf /dev/null /etc/systemd/network/99-default.link

# Enable sshd at boot.
systemctl enable ssh

# Install Rust stable + clippy + rustfmt via rustup.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal
. /root/.cargo/env
rustup component add clippy rustfmt

# Install sccache 0.8.2 binary.
curl -fsSL https://github.com/mozilla/sccache/releases/download/v0.8.2/sccache-v0.8.2-x86_64-unknown-linux-musl.tar.gz \
    | tar xz -C /tmp
mv /tmp/sccache-v0.8.2-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache
chmod +x /usr/local/bin/sccache

# Bake sccache + rust env into /etc/environment (read by systemd + ssh).
# Endpoint is fcbr0 bridge IP, reachable from every VM that fcctl-web spawns
# on the TAP devices attached to fcbr0. See
# .docs/adr-rust-build-cache-seaweedfs.md.
cat >> /etc/environment <<'EOF'
PATH="/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
RUSTC_WRAPPER=/usr/local/bin/sccache
SCCACHE_BUCKET=rust-cache
SCCACHE_ENDPOINT=http://172.26.0.1:8333
SCCACHE_S3_USE_SSL=false
SCCACHE_REGION=us-east-1
SCCACHE_S3_KEY_PREFIX=terraphim-ai
AWS_ACCESS_KEY_ID=any
AWS_SECRET_ACCESS_KEY=any
CARGO_INCREMENTAL=0
EOF

# Make sure SSH preserves the env file.
echo 'PermitUserEnvironment yes' >> /etc/ssh/sshd_config
echo 'AcceptEnv RUSTC_WRAPPER SCCACHE_* AWS_* CARGO_*' >> /etc/ssh/sshd_config

# Workspace dir for cargo runs.
mkdir -p /workspace

# Trim image: remove docs / man / locale.
rm -rf /usr/share/doc /usr/share/man /usr/share/info /usr/share/locale
apt-get clean
rm -rf /var/lib/apt/lists/*
