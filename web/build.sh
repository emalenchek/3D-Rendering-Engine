#!/usr/bin/env bash
# Build the WASM module(s) + JS bindings for the browser frontend (FR-5.4, FR-9.1).
#
# Pipeline (research report 08, decision V4): cargo build → wasm-bindgen → wasm-opt.
# Emits TWO artifacts that share one ABI-identical JS glue:
#   web/pkg/tte_wasm_bg.wasm         — SIMD (+simd128); fast, needs iOS Safari ≥ 16.4
#   web/pkg/tte_wasm_bg.scalar.wasm  — scalar fallback; loads everywhere
# The loader (web/app.js) feature-detects wasm SIMD and init()s the right one — a
# `+simd128` module *fails to instantiate* on iOS < 16.4, so the fallback restores
# universal load (research 14 §1). Both produce byte-identical frames (FR-7.4).
#
# Serve web/ over HTTP (the wasm MIME must be application/wasm; `python3 -m
# http.server` does this correctly).
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="web/pkg"
TARGET="wasm32-unknown-unknown"
PROFILE="release-wasm"
WASM="target/$TARGET/$PROFILE/tte_wasm.wasm"
BUDGET=256000 # NFR-7 / NFR-20: 250 KB raw, per artifact.

# Post-MVP features modern rustc/LLVM emit by default; wasm-opt rejects them as
# invalid unless the matching features are enabled.
BASE_FEATURES=(
  --enable-bulk-memory
  --enable-nontrapping-float-to-int
  --enable-sign-ext
  --enable-mutable-globals
  --enable-multivalue
  --enable-reference-types
)

# bindgen_opt <out_bg_wasm> [extra wasm-opt feature flags...]
# Generates bindings for the current target/$WASM into a temp dir, keeps a single
# shared JS glue in $OUT (asserting it matches across builds — an ABI-drift guard),
# size-optimizes the wasm, and copies it to <out_bg_wasm> with a budget check.
bindgen_opt() {
  local out="$1"
  shift
  local tmp
  tmp="$(mktemp -d)"
  wasm-bindgen "$WASM" --out-dir "$tmp" --target web --no-typescript

  if [ ! -f "$OUT/tte_wasm.js" ]; then
    cp "$tmp/tte_wasm.js" "$OUT/tte_wasm.js"
  elif ! cmp -s "$tmp/tte_wasm.js" "$OUT/tte_wasm.js"; then
    echo "!! wasm-bindgen glue differs between the SIMD and scalar builds —" >&2
    echo "   the loader assumes one ABI-identical glue (FR-9.2). Aborting." >&2
    exit 1
  fi

  local bg="$tmp/tte_wasm_bg.wasm"
  if command -v wasm-opt >/dev/null 2>&1; then
    echo "› wasm-opt -Oz → $out"
    wasm-opt -Oz "${BASE_FEATURES[@]}" "$@" "$bg" -o "$bg"
  else
    echo "› wasm-opt not found — skipping size pass for $out"
  fi
  cp "$bg" "$out"

  local size gz
  size=$(wc -c <"$out")
  gz=$(gzip -c "$out" | wc -c)
  printf '  %s = %d bytes (%d gzipped)\n' "$out" "$size" "$gz"
  if [ "$size" -gt "$BUDGET" ]; then
    echo "!! $out over the 250 KB budget (NFR-7/NFR-20)" >&2
    exit 1
  fi
  rm -rf "$tmp"
}

rm -rf "$OUT"
mkdir -p "$OUT"

# 1) SIMD build — uses the +simd128 rustflag from .cargo/config.toml + `simd` feature.
echo "› cargo build (SIMD, $PROFILE, $TARGET)"
cargo build -p tte-wasm --profile "$PROFILE" --target "$TARGET"
bindgen_opt "$OUT/tte_wasm_bg.wasm" --enable-simd

# 2) Scalar fallback — drop the `simd` feature and override RUSTFLAGS to remove
#    +simd128 (empty RUSTFLAGS takes precedence over .cargo/config.toml).
echo "› cargo build (scalar fallback, $PROFILE, $TARGET)"
RUSTFLAGS="" cargo build -p tte-wasm --no-default-features --profile "$PROFILE" --target "$TARGET"
bindgen_opt "$OUT/tte_wasm_bg.scalar.wasm"

echo "› done: $OUT/{tte_wasm.js, tte_wasm_bg.wasm (simd), tte_wasm_bg.scalar.wasm}"
