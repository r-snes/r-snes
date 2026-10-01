# Builds the standalone R-SNES Windows executable. Run through
# packaging/build-release.sh.
FROM ubuntu:24.04 AS build

ENV DEBIAN_FRONTEND=noninteractive
ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    cmake \
    mingw-w64

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.95.0
RUN rustup target add x86_64-pc-windows-gnu

WORKDIR /src
COPY . .

# The cargo registry and target/ are cache mounts so they are reused between
# builds. target/ gets its own id per Dockerfile: build scripts compiled on
# another distro would not run here. target/ only exists during this step, so
# the exe is copied out of it here too.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,id=r-snes-target-windows,target=/src/target \
    cargo build --release --target x86_64-pc-windows-gnu \
    && cp target/x86_64-pc-windows-gnu/release/r-snes.exe .

FROM scratch
ARG VERSION
COPY --from=build /src/r-snes.exe /r-snes-${VERSION}-windows-x86_64.exe
