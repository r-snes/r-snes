# Linux build environment for R-SNES (.deb, .rpm, .tar.gz).
#
# Ubuntu 22.04 is used on purpose: a Linux binary only runs on systems whose
# glibc is at least as new as the one it was built against, so building on an
# older distro makes the packages work on more systems.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# build-essential : C toolchain + strip
# libsdl2-dev     : SDL2 headers and library to link against
# dpkg-dev        : dpkg-shlibdeps, used by cargo-deb for `depends = "$auto"`
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        pkg-config \
        libsdl2-dev \
        dpkg-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable

RUN cargo install --locked cargo-deb cargo-generate-rpm
