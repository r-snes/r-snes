# Builds the R-SNES .deb package. Run through packaging/build-release.sh.
FROM ubuntu:22.04 AS build

ARG RUST_VERSION
ENV DEBIAN_FRONTEND=noninteractive
ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    libsdl2-dev \
    dpkg-dev

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION:-1.95.0}
RUN cargo install --locked cargo-deb

WORKDIR /src
COPY . .

# The cargo registry and target/ are cache mounts so they are reused between
# builds. target/ gets its own id per Dockerfile: build scripts compiled on
# another distro would not run here.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,id=r-snes-target-deb,target=/src/target \
    cargo deb --output /src/r-snes.deb

FROM scratch
ARG VERSION
COPY --from=build /src/r-snes.deb /r-snes_${VERSION}_amd64.deb
