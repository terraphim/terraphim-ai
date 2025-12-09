# Terraphim AI Build Environment
# This creates a reusable Docker layer with all build dependencies

ARG UBUNTU_VERSION=22.04
FROM ubuntu:${UBUNTU_VERSION}

# Set environment variables for non-interactive installation
ENV DEBIAN_FRONTEND=noninteractive
ENV DEBCONF_NONINTERACTIVE_SEEN=true

# Install all system dependencies in a single layer
RUN apt-get update -qq && \
    apt-get install -yqq --no-install-recommends \
        # Build essentials
        build-essential \
        bison \
        flex \
        ca-certificates \
        bc \
        wget \
        git \
        curl \
        cmake \
        pkg-config \
        # SSL/TLS
        openssl \
        libssl-dev \
        # Cross-compilation tools
        musl-tools \
        musl-dev \
        gcc-aarch64-linux-gnu \
        libc6-dev-arm64-cross \
        gcc-arm-linux-gnueabihf \
        libc6-dev-armhf-cross \
        # LLVM/Clang for bindgen (RocksDB)
        clang \
        libclang-dev \
        llvm-dev \
        libc++-dev \
        libc++abi-dev \
        # GTK/GLib for desktop builds
        libglib2.0-dev \
        libgtk-3-dev \
        libwebkit2gtk-4.0-dev \
        libsoup2.4-dev \
        libjavascriptcoregtk-4.0-dev \
        libappindicator3-dev \
        librsvg2-dev \
        # Additional tools
        software-properties-common \
        gpg-agent \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Install Rust toolchain (use stable - don't pin to a specific version)
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain stable --profile minimal && \
    rustup component add clippy rustfmt && \
    rustup target add x86_64-unknown-linux-gnu && \
    rustup target add aarch64-unknown-linux-gnu && \
    rustup target add armv7-unknown-linux-gnueabihf && \
    rustup target add x86_64-unknown-linux-musl && \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME

# Set environment variables for cross-compilation
ENV CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++ \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CC_x86_64_unknown_linux_musl=musl-gcc

# Set Rust environment variables
ENV CARGO_TERM_COLOR=always \
    CARGO_INCREMENTAL=0

# Create working directory
WORKDIR /workspace

# Install cargo-deb for package creation
RUN cargo install cargo-deb --locked

# Labels for metadata
LABEL org.opencontainers.image.title="Terraphim AI Builder" \
      org.opencontainers.image.description="Build environment for Terraphim AI with all dependencies" \
      org.opencontainers.image.version="${UBUNTU_VERSION}" \
      org.opencontainers.image.vendor="Terraphim AI"
