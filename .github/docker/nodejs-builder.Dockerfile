# Terraphim AI Node.js Builder
# Cross-compilation environment for building NAPI native modules
# Supports building aarch64-unknown-linux-gnu from x86_64 runners

ARG NODE_VERSION=20
FROM node:${NODE_VERSION}-bookworm

# Set environment variables for non-interactive installation
ENV DEBIAN_FRONTEND=noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN=true

# Install system dependencies for cross-compilation
RUN apt-get update -qq && \
    apt-get install -yqq --no-install-recommends \
        # Build essentials
        build-essential \
        ca-certificates \
        wget \
        git \
        curl \
        pkg-config \
        # SSL/TLS for host
        openssl \
        libssl-dev \
        # Cross-compilation tools for aarch64
        gcc-aarch64-linux-gnu \
        g++-aarch64-linux-gnu \
        libc6-dev-arm64-cross \
        # LLVM/Clang for bindgen
        clang \
        libclang-dev \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Download and build OpenSSL for aarch64 cross-compilation
ENV OPENSSL_VERSION=3.0.15
RUN cd /tmp && \
    wget -q https://github.com/openssl/openssl/releases/download/openssl-${OPENSSL_VERSION}/openssl-${OPENSSL_VERSION}.tar.gz && \
    tar xzf openssl-${OPENSSL_VERSION}.tar.gz && \
    cd openssl-${OPENSSL_VERSION} && \
    ./Configure linux-aarch64 --prefix=/usr/aarch64-linux-gnu --cross-compile-prefix=aarch64-linux-gnu- no-shared && \
    make -j$(nproc) && \
    make install_sw && \
    cd / && rm -rf /tmp/openssl-*

# Set OpenSSL environment variables for aarch64 cross-compilation
ENV OPENSSL_DIR_aarch64_unknown_linux_gnu=/usr/aarch64-linux-gnu \
    OPENSSL_LIB_DIR_aarch64_unknown_linux_gnu=/usr/aarch64-linux-gnu/lib64 \
    OPENSSL_INCLUDE_DIR_aarch64_unknown_linux_gnu=/usr/aarch64-linux-gnu/include

# Install Rust toolchain with modern version (supports edition 2024)
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal && \
    rustup target add aarch64-unknown-linux-gnu && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Set environment variables for cross-compilation
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar

# Set Rust environment variables
ENV CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=0

# Install Yarn 1.x (classic) - project uses yarn.lock v1 format
# First disable corepack's yarn and remove the existing symlink
RUN corepack disable yarn && \
    rm -f /usr/local/bin/yarn /usr/local/bin/yarnpkg && \
    npm install -g yarn@1

# Create working directory
WORKDIR /build

# Labels for metadata
LABEL org.opencontainers.image.title="Terraphim AI Node.js Builder" \
      org.opencontainers.image.description="Cross-compilation environment for NAPI native modules" \
      org.opencontainers.image.vendor="Terraphim AI"
