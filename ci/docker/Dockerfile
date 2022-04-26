#
# This Dockerfile is used to provide a binary-only container that other
# Dockerfiles can `COPY` from.
#
# Use like this in your project's Dockerfile:
#
#    `COPY --from=kaspar030/laze:<version>-<codename>` /laze /usr/bin
#
# `version`: every semver combination should work (0, 0.1, 0.1.17, ...)
# `codename` is either one of "buster" and "bullseye", or one of "focal" and "jammy",
# or "debian-latest", or "ubuntu-latest".
#
ARG DEBIAN_FROM=ubuntu:latest
FROM ${DEBIAN_FROM} AS builder

LABEL maintainer="Kaspar Schleiser <kaspar@schleiser.de>"

ENV DEBIAN_FRONTEND noninteractive

# install rust if not available.
RUN if test -z "$(command -v cargo)"; then apt-get update && apt-get -y install cargo ; fi

RUN command -v cargo

ARG LAZE_SOURCE=laze
RUN cargo install ${LAZE_SOURCE}

RUN for location in /usr/local/bin/laze /root/.cargo/bin/laze; do \
        test -f $${location} && cp $${location}; \
    done

FROM scratch

COPY --from=builder /laze /
