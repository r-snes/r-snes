#!/usr/bin/env bash
# Builds the R-SNES release artifacts inside Docker containers and gathers
# them in dist/v<version>/, ready to be uploaded to a GitHub release:
#
#   rsnes-<version>-windows-x86_64.zip
#   rsnes_<version>-1_amd64.deb
#   rsnes-<version>-1.x86_64.rpm
#   rsnes-<version>-linux-x86_64.tar.gz
#   SHA256SUMS
#
# Usage: packaging/build-release.sh [all|linux|windows]   (default: all)

set -euo pipefail

# --- Configuration -----------------------------------------------------------
CRATE=r-snes                      # binary crate name (cargo -p)
BIN=r-snes                        # executable name
CRATE_DIR=r-snes                  # crate folder, relative to the workspace root
ASSETS_DIR="$CRATE_DIR/assets"   # .desktop file and PNG icon
WIN_TARGET=x86_64-pc-windows-gnu

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCKER_DIR="$ROOT/packaging/docker"
WHAT="${1:-all}"

case "$WHAT" in
    all|linux|windows) ;;
    *) echo "Usage: $0 [all|linux|windows]" >&2; exit 1 ;;
esac

command -v docker >/dev/null || { echo "error: docker is not installed" >&2; exit 1; }

# --- Version -----------------------------------------------------------------
# Read from the crate's Cargo.toml, or from [workspace.package] in the root
# Cargo.toml if the crate uses `version.workspace = true`.
read_version() {
    grep -m1 -E '^version[[:space:]]*=' "$1" 2>/dev/null | cut -d'"' -f2 || true
}
VERSION="$(read_version "$ROOT/$CRATE_DIR/Cargo.toml")"
[[ -n "$VERSION" ]] || VERSION="$(read_version "$ROOT/Cargo.toml")"
[[ -n "$VERSION" ]] || { echo "error: could not read the version from Cargo.toml" >&2; exit 1; }

OUT="$ROOT/dist/v$VERSION"
rm -rf "$OUT"
mkdir -p "$OUT"

echo "==> Building R-SNES v$VERSION ($WHAT) into ${OUT#"$ROOT"/}"

# --- Helpers -----------------------------------------------------------------
build_image() {   # build_image <name> <dockerfile>
    echo "==> Preparing Docker image $1"
    docker build -q -t "$1" -f "$DOCKER_DIR/$2" "$DOCKER_DIR" >/dev/null
}

# Runs a script inside a container. The source tree is mounted at /src, but
# /src/target is a separate named volume per image: host builds are left
# untouched and incremental builds are cached between runs.
run_in() {   # run_in <image> <target-volume> <script>
    docker run --rm \
        -v "$ROOT:/src" \
        -v "$2:/src/target" \
        -v rsnes-cargo-registry:/usr/local/cargo/registry \
        -v "$OUT:/out" \
        -e VERSION="$VERSION" -e CRATE="$CRATE" -e BIN="$BIN" \
        -e CRATE_DIR="$CRATE_DIR" -e ASSETS_DIR="$ASSETS_DIR" \
        -e WIN_TARGET="$WIN_TARGET" \
        -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
        -w /src \
        "$1" bash -euo pipefail -c "$3"
}

# --- Linux: binary, .deb, .rpm, tarball --------------------------------------
LINUX_SCRIPT=$(cat <<'EOF'
echo "==> [linux] cargo build"
cargo build --release -p "$CRATE"

echo "==> [linux] .deb"
cargo deb -p "$CRATE" --no-build
cp "target/debian/${CRATE}_${VERSION}"-*_amd64.deb /out/

echo "==> [linux] .rpm"
cargo generate-rpm -p "$CRATE_DIR"
cp "target/generate-rpm/${CRATE}-${VERSION}"-*.rpm /out/

echo "==> [linux] .tar.gz"
name="$CRATE-$VERSION-linux-x86_64"
stage="/tmp/$name"
mkdir -p "$stage"
cp "target/release/$BIN" "$stage/"
for f in "$ASSETS_DIR/$BIN.desktop" "$ASSETS_DIR/$BIN.png" LICENSE* README*; do
    if [[ -e "$f" ]]; then cp "$f" "$stage/"; fi
done
tar -C /tmp -czf "/out/$name.tar.gz" "$name"

chown "$HOST_UID:$HOST_GID" /out/*
EOF
)

# --- Windows: cross-compiled .exe in a zip ------------------------------------
WINDOWS_SCRIPT=$(cat <<'EOF'
echo "==> [windows] cargo build"
cargo build --release -p "$CRATE" --target "$WIN_TARGET"
exe="target/$WIN_TARGET/release/$BIN.exe"

# Refuse to ship an exe that still needs non-system DLLs.
echo "==> [windows] checking DLL dependencies"
deps="$(x86_64-w64-mingw32-objdump -p "$exe" | awk '/DLL Name/ {print $3}')"
echo "$deps" | sed 's/^/      /'
if grep -qiE '^(SDL2|libwinpthread|libgcc_s|libstdc\+\+|vcruntime)' <<<"$deps"; then
    echo "error: $BIN.exe depends on a DLL that won't exist on users' machines" >&2
    exit 1
fi

echo "==> [windows] .zip"
name="$CRATE-$VERSION-windows-x86_64"
stage="/tmp/$name"
mkdir -p "$stage"
cp "$exe" "$stage/"
for f in LICENSE* README*; do
    if [[ -e "$f" ]]; then cp "$f" "$stage/"; fi
done
(cd /tmp && zip -qr "/out/$name.zip" "$name")

chown "$HOST_UID:$HOST_GID" /out/*
EOF
)

# --- Run ---------------------------------------------------------------------
if [[ "$WHAT" == all || "$WHAT" == linux ]]; then
    build_image rsnes-build-linux linux.Dockerfile
    run_in rsnes-build-linux rsnes-target-linux "$LINUX_SCRIPT"
fi

if [[ "$WHAT" == all || "$WHAT" == windows ]]; then
    build_image rsnes-build-windows windows.Dockerfile
    run_in rsnes-build-windows rsnes-target-windows "$WINDOWS_SCRIPT"
fi

# --- Checksums ---------------------------------------------------------------
(cd "$OUT" && sha256sum -- * > SHA256SUMS)

echo
echo "==> Done. Release files in ${OUT#"$ROOT"/}:"
ls -lh "$OUT"
