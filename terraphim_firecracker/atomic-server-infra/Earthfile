VERSION 0.7
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

all:
    BUILD \
        --platform=linux/amd64 \
        --platform=linux/aarch64 \
        # --platform=linux/arm/v7 \
        # --platform=linux/arm/v6 \
        +build-atomic

build-atomic:
  FROM rust:1.67
  RUN rustup target add $ARCH-unknown-linux-musl
  RUN apt update && apt install -y musl-tools musl-dev
  RUN update-ca-certificates
  WORKDIR /app
  #COPY --dir server lib cli desktop Cargo.lock Cargo.toml .
  GIT CLONE --branch master https://github.com/applied-knowledge-systems/atomic-data-rust.git .
  RUN cargo build --release --bin atomic-server --config net.git-fetch-with-cli=true --target $ARCH-unknown-linux-musl
  RUN strip -s /app/target/$ARCH-unknown-linux-musl/release/atomic-server
  SAVE ARTIFACT /app/target/$ARCH-unknown-linux-musl/release/atomic-server AS LOCAL atomic-server

code:
  GIT CLONE --branch v5.10 https://github.com/torvalds/linux.git linux.git
  #RUN cd linux.git && wget --no-check-certificate https://raw.githubusercontent.com/firecracker-microvm/firecracker/main/resources/guest_configs/microvm-kernel-ci-$ARCH-5.10.config -O .config
  SAVE ARTIFACT linux.git AS LOCAL linux.git

build:
  FROM +code
  ENV DEBIAN_FRONTEND noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN true
  RUN apt-get update
  RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc
  WORKDIR /opt/linux.git
  COPY microvm-kernel-ci-$ARCH-5.10.config .config
  COPY initramfs_list usr/
  COPY +build-atomic/atomic-server usr/
  RUN mkdir usr/db && touch usr/db/config.toml
  RUN make oldconfig
  IF [ "$TARGETARCH" = "aarch64" ]
      RUN make Image
      SAVE ARTIFACT ./arch/arm64/boot/Image AS LOCAL Image
  ELSE
      RUN make vmlinux
      SAVE ARTIFACT ./vmlinux AS LOCAL vmlinux
  END

tar2ext4:
    FROM golang:alpine
    WORKDIR src
    GIT CLONE https://github.com/microsoft/hcsshim .
    RUN go build ./cmd/tar2ext4

    SAVE ARTIFACT tar2ext4

kernel:
    FROM alpine:3.16
    RUN wget https://s3.amazonaws.com/spec.ccfc.min/img/hello/kernel/hello-vmlinux.bin -O kernel.bin
    SAVE ARTIFACT kernel.bin

rootfs:
    FROM earthly/dind

    COPY +tar2ext4/tar2ext4 /usr/bin/tar2ext4

    WORKDIR imgbuild
    
    WITH DOCKER --load my-build:latest=+test-image
        RUN docker save my-build:latest > image.tar
    END

    RUN mkdir image rootfs && tar -xf image.tar -C image

    FOR layer IN $(jq -r '.[].Layers | @sh' ./image/manifest.json | tr -d \')
        RUN tar -xf ./image/$layer -C rootfs
    END

    WORKDIR rootfs
    RUN mkdir -pv rootfs/{sbin,dev,proc,run,sys,var} && \
        tar -cf ../rootfs.tar *
    
    WORKDIR /imgbuild

    RUN tar2ext4 -i rootfs.tar -o rootfs.ext4 && \
        tune2fs -O ^read-only rootfs.ext4 && \
        dd if=/dev/zero bs=50M seek=1 count=0 of=rootfs.ext4 && \
        resize2fs rootfs.ext4 50M

    SAVE ARTIFACT rootfs.ext4

test-image2:
    FROM alpine:3.16
    RUN mkdir -p /.cache/atomic-data/search_index
    RUN apk update && \
        apk upgrade && \
        apk add openrc neofetch --no-cache
    RUN rm -f /sbin/init && ln -sf /usr/bin/neofetch /sbin/init


test-image:
    FROM ubuntu:20.04
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    RUN apt-get update
    RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends systemctl
    #TODO: copy service file from atomic repo
    #TODO: set default parameters to atomic data dir and atomic port
    #TODO: set hostname based on kernel parameters
    RUN false
    COPY +build-atomic/atomic-server /usr/bin/atomic-server

firecracker:
    FROM alpine:3.16
    WORKDIR artifacts

    COPY +rootfs/rootfs.ext4 .
    COPY +kernel/kernel.bin .

    SAVE ARTIFACT rootfs.ext4 AS LOCAL rootfs.ext4
    SAVE ARTIFACT kernel.bin AS LOCAL kernel.bin
