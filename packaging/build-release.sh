#!/usr/bin/env bash
# Builds the R-SNES release files with Docker and puts them in dist/v<version>/,
# ready to be uploaded to the release page:
#
#   r-snes-<version>-windows-x86_64.exe   (packaging/docker/windows.Dockerfile)
#   r-snes_<version>-1_amd64.deb          (packaging/docker/deb.Dockerfile)
#   r-snes-<version>-1.x86_64.rpm         (packaging/docker/rpm.Dockerfile)
#   SHA256SUMS
#
# Usage: packaging/build-release.sh [windows] [deb] [rpm]   (default: all of them)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | cut -d'"' -f2)"
RUST_VERSION="1.95.0"
OUT="$ROOT/dist/v$VERSION"

targets=(${*:-windows deb rpm})
for target in "${targets[@]}"; do
    if [[ ! -f "$ROOT/packaging/docker/$target.Dockerfile" ]]; then
        echo "error: unknown target '$target' (expected windows, deb or rpm)" >&2
        exit 1
    fi
done

mkdir -p "$OUT"
for target in "${targets[@]}"; do
    echo "==> Building $target"
    docker build -f "$ROOT/packaging/docker/$target.Dockerfile" --build-arg VERSION="$VERSION" --build-arg RUST_VERSION="$RUST_VERSION" --output "$OUT" "$ROOT"
done

cd "$OUT"
rm -f SHA256SUMS
sha256sum -- * > SHA256SUMS

echo "==> Release files for v$VERSION are in $OUT:"
ls -lh
