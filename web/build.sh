#!/usr/bin/env bash
# Build the WASM module + JS bindings for the browser frontend (FR-5.4).
#
# Pipeline (research report 08, decision V4): cargo build → wasm-bindgen → wasm-opt.
# wasm-pack would be the one-liner equivalent; the explicit pipeline pins the
# wasm-bindgen CLI to the crate version and is dependency-light.
#
# Output: web/pkg/{tte_wasm.js, tte_wasm_bg.wasm}. Serve web/ over HTTP (the wasm
# MIME must be application/wasm; `python3 -m http.server` does this correctly).
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="web/pkg"
TARGET="wasm32-unknown-unknown"
PROFILE="release-wasm"

echo "› cargo build ($PROFILE, $TARGET)"
cargo build -p tte-wasm --profile "$PROFILE" --target "$TARGET"

WASM="target/$TARGET/$PROFILE/tte_wasm.wasm"

echo "› wasm-bindgen → $OUT"
rm -rf "$OUT"
wasm-bindgen "$WASM" --out-dir "$OUT" --target web --no-typescript

# Optional size pass: shrink with wasm-opt if binaryen is installed.
# Modern rustc/LLVM emit the post-MVP feature set by default (bulk-memory,
# nontrapping-fptoint, sign-ext, …); wasm-opt rejects them as invalid unless
# the same features are enabled, so mirror the toolchain's default set here.
BG="$OUT/tte_wasm_bg.wasm"
WASM_OPT_FEATURES=(
  --enable-bulk-memory
  --enable-nontrapping-float-to-int
  --enable-sign-ext
  --enable-mutable-globals
  --enable-multivalue
  --enable-reference-types
)
if command -v wasm-opt >/dev/null 2>&1; then
  echo "› wasm-opt -Oz"
  wasm-opt -Oz "${WASM_OPT_FEATURES[@]}" "$BG" -o "$BG"
else
  echo "› wasm-opt not found — skipping (install binaryen to shrink further)"
fi

SIZE=$(wc -c < "$BG")
GZ=$(gzip -c "$BG" | wc -c)
printf '› done: %s = %d bytes (%d gzipped)\n' "$BG" "$SIZE" "$GZ"
# NFR-7 budget: 250 KB raw.
if [ "$SIZE" -gt 256000 ]; then
  echo "!! over the 250 KB budget (NFR-7)" >&2
  exit 1
fi
