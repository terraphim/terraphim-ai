# Simple Dockerfile for Terraphim Server
FROM ubuntu:22.04

ARG RUST_VERSION=1.85.0

# Install system dependencies
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq && apt-get install -yqq --no-install-recommends \
    build-essential \
    bison \
    flex \
    ca-certificates \
    openssl \
    libssl-dev \
    pkg-config \
    git \
    curl \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION}
ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /app

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release --package terraphim_server

# Copy the binary to a location in PATH
RUN cp target/release/terraphim_server /usr/local/bin/

# Create non-root user
RUN useradd --create-home --shell /bin/bash terraphim
RUN chown -R terraphim:terraphim /home/terraphim

# Switch to non-root user
USER terraphim
WORKDIR /home/terraphim

# Create config directory
RUN mkdir -p .config/terraphim

# Environment variables
ENV TERRAPHIM_SERVER_HOSTNAME="0.0.0.0:8000"
ENV TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
ENV RUST_LOG="info"
ENV RUST_BACKTRACE="1"

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Expose port
EXPOSE 8000

# Default command
ENTRYPOINT ["terraphim_server"]
CMD ["--config", "/home/terraphim/.config/terraphim/config.json"]