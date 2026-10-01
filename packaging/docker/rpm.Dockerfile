# Builds the R-SNES .rpm package. Run through packaging/build-release.sh.
FROM fedora:43 AS build

ARG RUST_VERSION
ENV PATH=/root/.cargo/bin:$PATH

RUN dnf install -y gcc pkgconf-pkg-config SDL2-devel wayland-devel

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION:-1.95.0}
RUN cargo install --locked cargo-generate-rpm

WORKDIR /src
COPY . .
RUN mkdir /out

# The cargo registry and target/ are cache mounts so they are reused between
# builds. target/ gets its own id per Dockerfile: build scripts compiled on
# another distro would not run here. target/ only exists during this step, so
# the package is made from the binary here too.
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,id=r-snes-target-rpm,target=/src/target \
    cargo build --release \
    && cargo generate-rpm --output /out/

FROM scratch
COPY --from=build /out/ /
