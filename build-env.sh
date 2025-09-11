#!/bin/bash
# Build environment setup script
# Source this file to set up cross-compilation environment variables

# Cross-compilation environment variables
export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
export CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

export CC_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-gcc"
export CXX_armv7_unknown_linux_gnueabihf="arm-linux-gnueabihf-g++"
export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER="arm-linux-gnueabihf-gcc"

export CC_x86_64_unknown_linux_musl="musl-gcc"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="musl-gcc"

# OpenSSL configuration for cross-compilation
export PKG_CONFIG_ALLOW_CROSS=1

echo "Cross-compilation environment configured"
echo "Available targets:"
echo "  - x86_64-unknown-linux-gnu (native)"
echo "  - aarch64-unknown-linux-gnu (ARM64)"
echo "  - armv7-unknown-linux-gnueabihf (ARMv7)"
echo "  - x86_64-unknown-linux-musl (musl)"

echo ""
echo "Usage:"
echo "  source build-env.sh"
echo "  cargo build --target <target> --release"
