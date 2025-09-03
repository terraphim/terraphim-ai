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
  BUILD +build # x86_64-unknown-linux-gnu
  BUILD +cross-build --TARGET=x86_64-unknown-linux-musl
  BUILD +cross-build --TARGET=armv7-unknown-linux-musleabihf
  BUILD +cross-build --TARGET=aarch64-unknown-linux-musl
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
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.82
  ENV PATH="/root/.cargo/bin:$PATH"
  ENV CARGO_HOME="/root/.cargo"
  RUN rustup component add clippy
  RUN rustup component add rustfmt
  DO rust+INIT --keep_fingerprints=true
  RUN cargo install cross
  RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
  RUN cargo install ripgrep
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
  RUN apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config musl-tools musl-dev
  RUN update-ca-certificates
  # Install Rust from official installer
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.82
  ENV PATH="/root/.cargo/bin:$PATH"
  ENV CARGO_HOME="/root/.cargo"
  RUN rustup component add clippy
  RUN rustup component add rustfmt
  RUN cargo install ripgrep
  RUN cargo install cross
  RUN ./scripts/yarn_and_build.sh
  # Save locally instead of pushing to registry
  SAVE IMAGE terraphim_builder_native:local

source-native:
  FROM +install-native
  WORKDIR /code
  CACHE --sharing shared --persist /code/vendor
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
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

source:
  FROM +install
  WORKDIR /code
  COPY --keep-ts Cargo.toml Cargo.lock ./
  COPY --keep-ts --dir terraphim_server desktop default crates ./
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
  DO rust+CARGO --args=fetch

cross-build:
  FROM +source
  ARG --required TARGET
  DO rust+SET_CACHE_MOUNTS_ENV
  COPY --keep-ts desktop+build/dist /code/terraphim_server/dist
  WITH DOCKER
    RUN --mount=$EARTHLY_RUST_CARGO_HOME_CACHE --mount=$EARTHLY_RUST_TARGET_CACHE  cross build --target $TARGET --release
  END
  DO rust+COPY_OUTPUT --output=".*" # Copies all files to ./target
   RUN ./target/$TARGET/release/terraphim-server --version
  SAVE ARTIFACT ./target/$TARGET/release/terraphim-server AS LOCAL artifact/bin/terraphim_server-$TARGET

build:
  FROM +source
  DO rust+SET_CACHE_MOUNTS_ENV
  DO rust+CARGO --args="build --offline --release" --output="release/[^/\.]+"
  RUN /code/target/release/terraphim_server --version
  SAVE ARTIFACT /code/target/release/terraphim_server AS LOCAL artifact/bin/terraphim_server-$TARGET

build-debug:
  FROM +source
  DO rust+SET_CACHE_MOUNTS_ENV
  COPY --keep-ts desktop+build/dist /code/terraphim-server/dist
  DO rust+CARGO --args="build" --output="debug/[^/\.]+"
  RUN ./target/debug/terraphim_server --version
  SAVE ARTIFACT ./target/debug/terraphim_server AS LOCAL artifact/bin/terraphim_server_debug

test:
  FROM +build-debug-native
  # DO rust+SET_CACHE_MOUNTS_ENV
  # COPY --chmod=0755 +build-debug/terraphim_server /code/terraphim_server_debug
  GIT CLONE https://github.com/terraphim/INCOSE-Systems-Engineering-Handbook.git /tmp/system_operator/
  # RUN --mount=$EARTHLY_RUST_CARGO_HOME_CACHE --mount=$EARTHLY_RUST_TARGET_CACHE nohup /code/terraphim_server_debug & sleep 5 && cargo test;
  RUN cargo test --workspace
  #DO rust+CARGO --args="test --offline"

fmt:
  FROM +build-debug-native
  RUN cargo fmt --check

lint:
  FROM +build-debug-native
  RUN cargo clippy --no-deps --all-features --all-targets

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
  FROM ubuntu:20.04
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

docker-musl:
  FROM alpine:3.18
  # You can pass multiple tags, space separated
  # SAVE IMAGE --push ghcr.io/applied-knowledge-systems/terraphim-fastapiapp:bionic
  ARG tags="ghcr.io/applied-knowledge-systems/terraphim-server:latest"
  ARG --required TARGET
  COPY --chmod=0755 --platform=linux/amd64 (+cross-build/terraphim_server --TARGET=${TARGET}) /terraphim_server
  RUN /terraphim_server --version
  ENV TERRAPHIM_SERVER_HOSTNAME="127.0.0.1:8000"
  ENV TERRAPHIM_SERVER_API_ENDPOINT="http://localhost:8000/api"
  EXPOSE 8000
  ENTRYPOINT ["/terraphim_server"]
  SAVE IMAGE --push ${tags}

docker-aarch64:
  FROM ubuntu:24.04
  ENV DEBIAN_FRONTEND=noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN=true
  RUN apt-get update && apt-get upgrade -y
  RUN apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget git curl cmake pkg-config libssl-dev g++-aarch64-linux-gnu libc6-dev-arm64-cross
  # Install Rust from official installer
  RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.75.0
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
