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
RUN if test -z "$(command -v cargo)"; then apt-get update && apt-get -y install build-essential curl ; fi
RUN if test -z "$(command -v cargo)"; then curl https://sh.rustup.rs -sSf | sh -s -- -y ; fi

# rust:buster ships 1.79, bump that
RUN if test "${DEBIAN_FROM}" = "rust:buster"; then rustup toolchain install stable ; fi

# this works on both debian and ubuntu
ENV PATH="/root/.cargo/bin:${PATH}"

# override this to e.g., "--git https://github.com/kaspar030/laze --rev foo"
ARG LAZE_SOURCE=laze

# build laze
RUN cargo install ${LAZE_SOURCE}
RUN cp $(command -v laze) /

FROM scratch

COPY --from=builder /laze /
