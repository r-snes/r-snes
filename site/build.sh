#!/usr/bin/env bash
# Assembles the GitHub Pages website in _site/:
#
#   /            landing page              (site/pages/)
#   /coverage/   coverage dashboard        (site/pages/coverage/ + the tarpaulin reports)
#   /book/       guides and plugin API     (mdBook, site/book/)
#   /api/        Rust API documentation    (cargo doc)
#
# Usage: site/build.sh [coverage-dir]
#
#   coverage-dir: folder holding tarpaulin-report.html and lcov.info, as written by
#                 `cargo tarpaulin --out html --out lcov`. Without it, the site is built
#                 without coverage data.
#
# Needs mdbook in PATH (or set MDBOOK to its path).

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/_site"
COVERAGE="${1:-}"

rm -rf "$OUT"
mkdir -p "$OUT"

echo "==> Copying pages"
cp -r "$ROOT/site/pages/." "$OUT/"
cp "$ROOT/assets/r-snes.png" "$OUT/assets/logo.png"

echo "==> Building the book"
"${MDBOOK:-mdbook}" build "$ROOT/site/book" --dest-dir "$OUT/book"

echo "==> Building the API documentation"
cargo doc --manifest-path "$ROOT/Cargo.toml" --workspace --no-deps
cp -r "${CARGO_TARGET_DIR:-$ROOT/target}/doc" "$OUT/api"

if [[ -n "$COVERAGE" ]]; then
    echo "==> Copying coverage reports"
    cp "$COVERAGE/tarpaulin-report.html" "$COVERAGE/lcov.info" "$OUT/coverage/"
fi

commit="$(git -C "$ROOT" rev-parse HEAD)"
date="$(git -C "$ROOT" show -s --format=%cs HEAD)"
printf '{ "commit": "%s", "date": "%s" }\n' "$commit" "$date" > "$OUT/build-info.json"

echo "==> Website ready in $OUT"
