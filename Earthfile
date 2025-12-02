VERSION --cache-persist-option --global-cache 0.7
PROJECT applied-knowledge-systems/terraphim-project
IMPORT ./desktop AS desktop
IMPORT github.com/earthly/lib/rust AS rust
FROM ubuntu:20.04

ARG TARGETARCH
ARG TARGETOS
ARG TARGETPLATFORM
ARG --global tag=$TARGETOS-$TARGETARCH
ARG --global TARGETARCH
IF [ "$TARGETARCH" = amd64 ]
    ARG --global ARCH=x86_64
ELSE
    ARG --global ARCH=$TARGETARCH
END

WORKDIR /code

pipeline:
  BUILD desktop+build
  BUILD +build-debug-native
  BUILD +fmt
  BUILD +lint
  BUILD +test
  BUILD +build-native
  BUILD +docs-pages

rustlib:
  BUILD +install
  BUILD +build

native:
  BUILD +install-native
  BUILD +build-native


# Creates a `./artifact/bin` folder with all binaries
build-all:
  BUILD +build # x86_64-unknown-linux-gnu (main build working ✅)
  # TODO: Fix OpenSSL cross-compilation issues for musl targets
  # BUILD +cross-build --TARGET=x86_64-unknown-linux-musl
  # BUILD +cross-build --TARGET=armv7-unknown-linux-musleabihf
  # BUILD +cross-build --TARGET=aarch64-unknown-linux-musl
  # Errors
  # BUILD +cross-build --TARGET=aarch64-apple-darwin

docker-all:
  BUILD --platform=linux/amd64 +docker-musl --TARGET=x86_64-unknown-linux-musl
  BUILD --platform=linux/arm/v7 +docker-musl --TARGET=armv7-unknown-linux-musleabihf
  BUILD --platform=linux/arm64/v8 +docker-musl --TARGET=aarch64-unknown-linux-musl

# this install builds from base OS without registry dependencies
install:
  FROM ubuntu:20.04
  ENV DEBIAN_FRONTEND=noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN=true
  RUN apt-get update -qq
  RUN apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config musl-tools musl-dev
  RUN update-ca-certificates
  # Install Rust from official installer
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
  ENV PATH="/root/.cargo/bin:$PATH"
  ENV CARGO_HOME="/root/.cargo"
  RUN rustup component add clippy
  RUN rustup component add rustfmt
  DO rust+INIT --keep_fingerprints=true
  RUN cargo install cross --locked
  RUN cargo install ripgrep
  # Install Docker client for cross tool
  RUN apt-get update && apt-get install -y gnupg lsb-release software-properties-common
  RUN curl -fsSL https://download.docker.com/linux/ubuntu/gpg | apt-key add -
  RUN add-apt-repository "deb [arch=amd64] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable"
  RUN apt-get update && apt-get install -y docker-ce-cli
  # Install common cross-compilation targets
  RUN rustup target add x86_64-unknown-linux-musl
  RUN rustup target add aarch64-unknown-linux-gnu
  RUN rustup target add armv7-unknown-linux-musleabihf
  RUN rustup target add aarch64-unknown-linux-musl
  RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
  RUN bash -c "source $HOME/.nvm/nvm.sh && nvm install 16.15.1"
  RUN bash -c "source $HOME/.nvm/nvm.sh && npm install -g yarn"
  # Save locally instead of pushing to registry
  SAVE IMAGE terraphim_builder:local

# this install doesn't use rust lib and Earthly cache
install-native:
  FROM ubuntu:20.04
  ENV DEBIAN_FRONTEND=noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN=true
  RUN apt-get update -qq
  RUN apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config musl-tools musl-dev libclang-dev clang
  RUN update-ca-certificates
  # Install Rust from official installer
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
  ENV PATH="/root/.cargo/bin:$PATH"
  ENV CARGO_HOME="/root/.cargo"
  RUN rustup component add clippy
  RUN rustup component add rustfmt
  RUN cargo install ripgrep
  RUN cargo install cross --locked
  # Install Docker client for cross tool
  RUN apt-get update && apt-get install -y docker.io
  # Install common cross-compilation targets
  RUN rustup target add x86_64-unknown-linux-musl
  RUN rustup target add aarch64-unknown-linux-gnu
  RUN rustup target add armv7-unknown-linux-musleabihf
  RUN rustup target add aarch64-unknown-linux-musl
  RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
  RUN bash -c "source $HOME/.nvm/nvm.sh && nvm install 16.15.1"
  RUN bash -c "source $HOME/.nvm/nvm.sh && npm install -g yarn"
  # Save locally instead of pushing to registry
  SAVE IMAGE terraphim_builder_native:local

source-native:
  FROM +install-native
  WORKDIR /code
  CACHE --sharing shared --persist /code/vendor
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
  COPY --keep-ts desktop+build/dist /code/desktop/dist
  # Exclude problematic firecracker from vendoring for now
  RUN mkdir -p terraphim_firecracker
  RUN echo '[package]\nname = "terraphim-firecracker"\nversion = "0.1.0"\nedition = "2021"' > terraphim_firecracker/Cargo.toml
  RUN mkdir -p .cargo
  RUN cargo vendor > .cargo/config.toml
  SAVE ARTIFACT .cargo/config.toml AS LOCAL .cargo/config.toml
  SAVE ARTIFACT /code

build-native:
  FROM +source-native
  WORKDIR /code
  RUN cargo build --release
  SAVE ARTIFACT /code/target/release/terraphim_server AS LOCAL artifact/bin/terraphim_server

build-debug-native:
  FROM +source-native
  WORKDIR /code
  RUN cargo build
  SAVE ARTIFACT /code/target/debug/terraphim_server AS LOCAL artifact/bin/terraphim_server_debug
  # Save the entire target directory for reuse by fmt, lint, test
  SAVE ARTIFACT /code/target /target

workspace-debug:
  FROM +source-native
  WORKDIR /code
  # Copy the pre-built target directory from build-debug-native
  COPY +build-debug-native/target /code/target
  # The workspace now has all compiled artifacts ready

source:
  FROM +install
  WORKDIR /code
  CACHE --sharing shared --persist /code/vendor
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
  RUN mkdir -p .cargo
  RUN cargo vendor > .cargo/config.toml
  DO rust+CARGO --args=fetch

cross-build:
  FROM +source
  ARG --required TARGET
  DO rust+SET_CACHE_MOUNTS_ENV
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
  # Use cargo directly for musl targets, cross for others
  IF [ "$TARGET" = "x86_64-unknown-linux-musl" ]
    RUN --mount=$EARTHLY_RUST_CARGO_HOME_CACHE --mount=$EARTHLY_RUST_TARGET_CACHE \
        CC_x86_64_unknown_linux_musl=musl-gcc \
        cargo build --target $TARGET --release \
        --package terraphim_server \
        --package terraphim_mcp_server \
        --package terraphim_tui
  ELSE
    # For non-musl targets, we would use cross here but it requires Docker daemon
    # For now, skip complex targets that need cross
    RUN echo "Cross-compilation for $TARGET requires Docker daemon access"
    RUN exit 1
  END
  DO rust+COPY_OUTPUT --output=".*" # Copies all files to ./target
  # Test the binaries (note: TUI binary uses hyphen, not underscore)
  RUN ./target/$TARGET/release/terraphim_server --version
  RUN ./target/$TARGET/release/terraphim_mcp_server --version
  RUN ./target/$TARGET/release/terraphim-tui --version
  # Save all three binaries
  SAVE ARTIFACT ./target/$TARGET/release/terraphim_server AS LOCAL artifact/bin/terraphim_server-$TARGET
  SAVE ARTIFACT ./target/$TARGET/release/terraphim_mcp_server AS LOCAL artifact/bin/terraphim_mcp_server-$TARGET
  SAVE ARTIFACT ./target/$TARGET/release/terraphim-tui AS LOCAL artifact/bin/terraphim_tui-$TARGET

build:
  FROM +source
  DO rust+SET_CACHE_MOUNTS_ENV
  # Build each package separately to ensure all binaries are created
  DO rust+CARGO --args="build --offline --release --package terraphim_server" --output="release/[^/\.]+"
  DO rust+CARGO --args="build --offline --release --package terraphim_mcp_server" --output="release/[^/\.]+"
  DO rust+CARGO --args="build --offline --release --package terraphim_tui" --output="release/[^/\.]+"
  # Debug: Check what binaries were actually created
  RUN find /code/target/release -name "*terraphim*" -type f -exec ls -la {} \;
  # Test all binaries (note: TUI binary uses hyphen, not underscore)
  RUN /code/target/release/terraphim_server --version
  RUN /code/target/release/terraphim_mcp_server --version
  RUN /code/target/release/terraphim-tui --version
  # Save all three binaries
  SAVE ARTIFACT /code/target/release/terraphim_server AS LOCAL artifact/bin/terraphim_server-
  SAVE ARTIFACT /code/target/release/terraphim_mcp_server AS LOCAL artifact/bin/terraphim_mcp_server-
  SAVE ARTIFACT /code/target/release/terraphim-tui AS LOCAL artifact/bin/terraphim_tui-

build-debug:
  FROM +source
  DO rust+SET_CACHE_MOUNTS_ENV
  COPY --keep-ts desktop+build/dist /code/terraphim-server/dist
  DO rust+CARGO --args="build" --output="debug/[^/\.]+"
  RUN ./target/debug/terraphim_server --version
  SAVE ARTIFACT ./target/debug/terraphim_server AS LOCAL artifact/bin/terraphim_server_debug

test:
  FROM +workspace-debug
  # DO rust+SET_CACHE_MOUNTS_ENV
  # COPY --chmod=0755 +build-debug/terraphim_server /code/terraphim_server_debug
  GIT CLONE https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
  # RUN --mount=$EARTHLY_RUST_CARGO_HOME_CACHE --mount=$EARTHLY_RUST_TARGET_CACHE nohup /code/terraphim_server_debug & sleep 5 && cargo test;
  RUN cargo test --workspace
  #DO rust+CARGO --args="test --offline"

fmt:
  FROM +workspace-debug
  RUN cargo fmt --check

lint:
  FROM +workspace-debug
  RUN cargo clippy --workspace --all-targets --all-features --exclude terraphim_firecracker

build-focal:
  FROM ubuntu:20.04
  ENV DEBIAN_FRONTEND noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN true
  RUN apt-get update -qq
  RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config
  WORKDIR /code
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  COPY --keep-ts desktop+build/dist /code/terraphim-server/dist
  RUN curl https://pkgx.sh | sh
  RUN pkgx +openssl cargo build --release
  SAVE ARTIFACT /code/target/release/terraphim_server AS LOCAL artifact/bin/terraphim_server_focal

build-jammy:
  FROM ubuntu:22.04
  ENV DEBIAN_FRONTEND noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN true
  RUN apt-get update -qq
  RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config
  RUN update-ca-certificates
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # RUN rustup toolchain install stable
  WORKDIR /code
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  IF [ "$CARGO_HOME" = "" ]
    ENV CARGO_HOME="$HOME/.cargo"
  END
  IF ! echo $PATH | grep -E -q "(^|:)$CARGO_HOME/bin($|:)"
    ENV PATH="$PATH:$CARGO_HOME/bin"
  END
  RUN ./desktop/scripts/yarn_and_build.sh
  # COPY --keep-ts desktop+build/dist /code/terraphim-server/dist
  RUN cargo build --release
  SAVE ARTIFACT /code/target/release/terraphim_server AS LOCAL artifact/bin/terraphim_server_jammy

docker-native:
  FROM ubuntu:20.04
  # Install minimal runtime dependencies
  RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
  ARG tags="ghcr.io/applied-knowledge-systems/terraphim-server:latest"
  # Copy all three binaries from main build
  COPY --chmod=0755 (+build/terraphim_server) /usr/local/bin/terraphim_server
  COPY --chmod=0755 (+build/terraphim_mcp_server) /usr/local/bin/terraphim_mcp_server
  COPY --chmod=0755 (+build/terraphim-tui) /usr/local/bin/terraphim-tui
  # Test binaries
  RUN /usr/local/bin/terraphim_server --version
  RUN /usr/local/bin/terraphim_mcp_server --version
  RUN /usr/local/bin/terraphim-tui --version
  # Default to main server
  ENV TERRAPHIM_SERVER_HOSTNAME="127.0.0.1:8000"
  ENV TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
  EXPOSE 8000
  ENTRYPOINT ["/usr/local/bin/terraphim_server"]
  SAVE IMAGE --push ${tags}

docker-musl:
  FROM alpine:3.18
  # You can pass multiple tags, space separated
  # TODO: Re-enable once OpenDAL->reqsign OpenSSL issue is resolved
  # ARG tags="ghcr.io/applied-knowledge-systems/terraphim-server:latest"
  # ARG --required TARGET
  # COPY --chmod=0755 --platform=linux/amd64 (+cross-build/terraphim_server --TARGET=${TARGET}) /terraphim_server
  # RUN /terraphim_server --version
  # ENV TERRAPHIM_SERVER_HOSTNAME="127.0.0.1:8000"
  # ENV TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
  # EXPOSE 8000
  # ENTRYPOINT ["/terraphim_server"]
  # SAVE IMAGE --push ${tags}
  RUN echo "MUSL cross-compilation temporarily disabled due to OpenDAL->reqsign OpenSSL dependency"

docker-aarch64:
  FROM ubuntu:20.04
  ENV DEBIAN_FRONTEND=noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN=true
  RUN apt-get update && apt-get upgrade -y
  RUN apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config libssl-dev g++-aarch64-linux-gnu libc6-dev-arm64-cross
  # Install Rust from official installer
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.85.0
  ENV PATH="/root/.cargo/bin:$PATH"
  ENV CARGO_HOME="/root/.cargo"
  RUN rustup target add aarch64-unknown-linux-gnu
  RUN rustup toolchain install stable-aarch64-unknown-linux-gnu

  WORKDIR /code
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./

  ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
      CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
      CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
  ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
  RUN cargo build --release --target aarch64-unknown-linux-gnu
  SAVE ARTIFACT /code/target/aarch64-unknown-linux-gnu/release/terraphim_server AS LOCAL artifact/bin/terraphim_server_linux-aarch64
  # CMD ["cargo", "build","--release","--target", "aarch64-unknown-linux-gnu"]

docker-slim:
    FROM debian:buster-slim
    COPY +build/terraphim_server terraphim_server
    EXPOSE 8000
    ENTRYPOINT ["./terraphim_server"]
    SAVE IMAGE aks/terraphim_server:buster

docker-scratch:
    FROM scratch
    COPY +build/terraphim_server terraphim_server
    EXPOSE 8000
    ENTRYPOINT ["./terraphim_server"]
    SAVE IMAGE aks/terraphim_server:scratch

docs-deps:
  FROM +install-native
  RUN cargo install mdbook
  RUN cargo install mdbook-epub
  RUN cargo install mdbook-linkcheck
  RUN cargo install mdbook-sitemap-generator
  # RUN cargo install --git https://github.com/typst/typst typst-cli
  # RUN cargo install --git https://github.com/terraphim/mdbook-typst.git
  RUN cargo install mdbook-mermaid
  RUN cargo install mdbook-alerts
  RUN apt update && apt install libvips-dev -y
  RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.5/install.sh | bash
  RUN bash -c "source $HOME/.nvm/nvm.sh && nvm install 20 && npm install -g netlify-cli"



docs-pages:
  FROM +docs-deps
  COPY --keep-ts docs /docs
  WORKDIR /docs
  RUN mdbook --version
  RUN mdbook build
  RUN mdbook-sitemap-generator -d docs.terraphim.ai -o /docs/book/html/sitemap.xml
  RUN --secret NETLIFY_AUTH_TOKEN=NETLIFY_TOKEN bash -c "source $HOME/.nvm/nvm.sh && netlify deploy --dir /docs/book/html --prod --auth $NETLIFY_AUTH_TOKEN --site docs-terraphim-ai"
