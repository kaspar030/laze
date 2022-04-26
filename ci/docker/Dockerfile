#
FROM ubuntu:latest AS builder

LABEL maintainer="Kaspar Schleiser <kaspar@schleiser.de>"

ENV DEBIAN_FRONTEND noninteractive

RUN \
    echo 'Upgrading all system packages to the latest available versions' >&2 && \
    apt-get update && apt-get -y dist-upgrade \
    && echo 'Installing Rust' >&2 && \
    apt-get -y install cargo \
    && echo 'Cleaning up installation files' >&2 && \
        apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN cargo install laze

FROM scratch

COPY --from=builder /root/.cargo/bin/laze /
