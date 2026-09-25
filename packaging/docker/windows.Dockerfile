# Windows cross-compilation environment for R-SNES.
#
# Targets x86_64-pc-windows-gnu with the mingw-w64 toolchain, since the MSVC
# toolchain can't run in a Linux container. SDL2 is built from source
# (sdl2 "bundled" feature) and linked statically, so CMake is required.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

# build-essential : host C toolchain (for build scripts)
# mingw-w64       : cross gcc, windres (icon), objdump (DLL check)
# cmake           : builds the bundled SDL2
# zip             : packs the release archive
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        cmake \
        mingw-w64 \
        zip \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup target add x86_64-pc-windows-gnu
