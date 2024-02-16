VERSION 0.7
PROJECT applied-knowledge-systems/terraphim-firecracker
FROM ubuntu:18.04

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

ci-pipeline:
  PIPELINE
  TRIGGER push main
  TRIGGER pr main
  BUILD --allow-privileged +rootfs

all:
    BUILD \
        --platform=linux/amd64 \
        --platform=linux/aarch64 \
        # --platform=linux/arm/v7 \
        # --platform=linux/arm/v6 \
        +build-atomic


code:
  GIT CLONE --branch v5.10 https://github.com/torvalds/linux.git linux.git
  #RUN cd linux.git && wget --no-check-certificate https://raw.githubusercontent.com/firecracker-microvm/firecracker/main/resources/guest_configs/microvm-kernel-ci-$ARCH-5.10.config -O .config
  SAVE ARTIFACT linux.git AS LOCAL linux.git

build-kernel:
  ENV DEBIAN_FRONTEND noninteractive
  ENV DEBCONF_NONINTERACTIVE_SEEN true
  RUN apt-get update
  RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends build-essential bison flex ca-certificates openssl libssl-dev bc wget 
  WORKDIR /opt/linux.git
  COPY +code/linux.git .
  RUN wget --no-check-certificate https://raw.githubusercontent.com/firecracker-microvm/firecracker/main/resources/guest_configs/microvm-kernel-ci-$ARCH-5.10.config -O .config
  # COPY microvm-kernel-ci-$ARCH-5.10.config .config
  RUN make oldconfig
  IF [ "$TARGETARCH" = "aarch64" ]
      RUN make Image
      SAVE ARTIFACT ./arch/arm64/boot/Image AS LOCAL Image
  ELSE
      RUN make vmlinux
      SAVE ARTIFACT ./vmlinux AS LOCAL vmlinux
  END


kernel:
    FROM alpine:3.16
    RUN wget https://s3.amazonaws.com/spec.ccfc.min/img/hello/kernel/hello-vmlinux.bin -O kernel.bin
    SAVE ARTIFACT kernel.bin

rootfs:
    FROM earthly/dind
    ENV SIZE="16000M"
    RUN echo $SIZE
    WORKDIR /rootfs
    ENV img_file = "/rootfs/rootfs.bionic"
    RUN echo $rootfs_image
    RUN truncate -s "$SIZE" "$img_file"
    RUN mkfs.ext4 -F "$img_file"
    WITH DOCKER --load my-build:latest=+terraphim-python
        RUN --privileged mount -o loop $img_file /mnt/ && export CONTAINER_ID=$(docker run -td my-build:latest /bin/bash); docker cp --archive $CONTAINER_ID:/ /mnt/ && umount /mnt/
    END
    SAVE ARTIFACT rootfs.bionic AS LOCAL ./images/bionic/terraphim-bionic.local.rootfs


bionic-image:
    FROM ghcr.io/applied-knowledge-systems/terraphim-fastapiapp:bionic
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    RUN rm -rfv /var/lib/apt/lists/*
    RUN apt clean
    RUN apt-get update
    RUN DEBIAN_FRONTEND=noninteractive DEBCONF_NONINTERACTIVE_SEEN=true TZ=Etc/UTC apt-get install -yqq --no-install-recommends udev systemd-sysv openssh-server iproute2 curl \
      dbus \
      kmod \
      iputils-ping \
      net-tools \
      rng-tools \
      sudo \
      wget
    RUN systemctl enable redis 
    RUN apt-get clean && rm -rf /var/lib/apt/lists/*
    RUN mkdir "/etc/systemd/system/serial-getty@ttyS0.service.d/"
    # COPY autologin.conf "/etc/systemd/system/serial-getty@ttyS0.service.d/autologin.conf"
    RUN rm -f /etc/systemd/system/multi-user.target.wants/systemd-resolved.service
    RUN rm -f /etc/systemd/system/dbus-org.freedesktop.resolve1.service
    RUN rm -f /etc/systemd/system/sysinit.target.wants/systemd-timesyncd.service
    # RUN echo 'root:Docker!' | chpasswd
    RUN mkdir keypairs
    RUN ssh-keygen -t ed25519 -q -N "" -f keypairs/terraphim
    RUN mkdir -m 0600 -p /root/.ssh/
    RUN cp keypairs/terraphim.pub /root/.ssh/authorized_keys
    RUN cp keypairs/terraphim.pub /root/.ssh/authorized_keys && chmod 644 /root/.ssh/authorized_keys
    SAVE ARTIFACT keypairs AS LOCAL keypairs


bionic-terraphim:
    FROM +bionic-image
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    RUN useradd --no-log-init --create-home --shell /bin/bash --home-dir /terraphim terraphim 
    COPY scripts/cmdline.sh /usr/bin/cmdline.sh
    COPY --dir scripts /opt/rediscluster
    COPY scripts/rgcluster.service /etc/systemd/system/
    RUN chmod +x /opt/rediscluster/rgservice.sh
    RUN systemctl enable rgcluster
    COPY --chmod 644 +bionic-image/keypairs/terraphim.pub /terraphim/.ssh/authorized_keys
    RUN chown -R terraphim:terraphim /terraphim
    SAVE IMAGE --push ghcr.io/applied-knowledge-systems/terraphim-server:bionic

fetch-terraphim-platform-code:
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    GIT CLONE https://github.com/terraphim/terraphim-platform-pipeline.git terraphim-platform-pipeline.git
    SAVE ARTIFACT terraphim-platform-pipeline.git

fetch-fastapi-code:
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    RUN apt-get update && apt-get install -yqq --no-install-recommends git ca-certificates
    RUN update-ca-certificates
    RUN git clone --recurse-submodules https://github.com/terraphim/terraphim-cloud-fastapi.git terraphim-cloud-fastapi.git
    SAVE ARTIFACT terraphim-cloud-fastapi.git

fetch-ui-code:
    GIT CLONE https://github.com/terraphim/terraphim-ui-svelte-ts.git terraphim-ui-svelte-ts.git
    SAVE ARTIFACT terraphim-ui-svelte-ts.git

terraphim-python:
    FROM +bionic-terraphim
    ENV DEBIAN_FRONTEND noninteractive
    ENV DEBCONF_NONINTERACTIVE_SEEN true
    RUN apt-get update && apt-get install -yqq --no-install-recommends build-essential git ca-certificates curl gnupg
    RUN update-ca-certificates
    RUN systemctl enable fastapi
    WORKDIR /code/
    COPY +fetch-terraphim-platform-code/terraphim-platform-pipeline.git ./terraphim-platform-pipeline
    RUN python3.8 -m venv /code/venv_terraphim_platform
    RUN /bin/bash -c "source /code/venv_terraphim_platform/bin/activate && pip3 install -r terraphim-platform-pipeline/requirements_local.txt"
    RUN /bin/bash -c "source /code/venv_terraphim_platform/bin/activate && pip3 install gears-cli==1.1.3"
    RUN cp /code/terraphim-platform-pipeline/service/terraphim-pipeline.service /etc/systemd/system/
    RUN chmod +x /code/terraphim-platform-pipeline/service/start-pipeline.sh
    RUN systemctl enable terraphim-pipeline
    SAVE IMAGE terraphim-python:latest
    SAVE IMAGE --push ghcr.io/terraphim/terraphim-python:bionic

save-keys:
    FROM +bionic-image
    COPY --chmod 644 +bionic-image/keypairs keypairs
    SAVE ARTIFACT keypairs AS LOCAL keypairs

firecracker-minimal:
    FROM ubuntu:18.04
    WORKDIR artifacts

    COPY +rootfs/rootfs.bionic .
    COPY +kernel/kernel.bin .

    SAVE ARTIFACT rootfs.bionic AS LOCAL ./images_test/bionic/terraphim-bionic.local.rootfs
    SAVE ARTIFACT kernel.bin AS LOCAL kernel.bin
