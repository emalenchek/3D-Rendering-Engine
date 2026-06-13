# tte — browser frontend

The same Rust engine that renders in your terminal, compiled to WebAssembly and
drawn to an HTML canvas as a colored character grid (v2.0 Phase 5).

## Build & run

Prerequisites (one-time):

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.123   # match the wasm-bindgen crate
# optional, shrinks the binary further:
#   install binaryen (provides wasm-opt) via your package manager
```

Build the module and serve:

```sh
./web/build.sh                 # → web/pkg/{tte_wasm.js, tte_wasm_bg.wasm}
python3 -m http.server -d web  # serve with the correct application/wasm MIME
# open http://localhost:8000
```

`build.sh` runs `cargo build` → `wasm-bindgen` → `wasm-opt` (if present) and fails
if the binary exceeds the 250 KB budget (NFR-7). Current size: ~98 KB raw /
~47 KB gzipped, even without wasm-opt.

## Use

- **Drag** to orbit, **scroll / pinch** to zoom (mouse and touch).
- **Preset** picks a built-in model/scene; **Output** switches ASCII / truecolor /
  half-block.
- Edit the **Scene** box — it re-renders live (last good scene is kept on a parse error).

## How it works

- `tte-wasm` (a `wasm-bindgen` cdylib over `tte-core`) exposes a `Renderer`: it
  parses OBJ/DSL, orbits, rasterizes, and returns each frame as three flat typed
  arrays (`glyphs`, `fg`, `bg`). It contains **no `web-sys`** — the module is
  data-only (decision V4a).
- `renderer.js` draws those arrays via a pre-rendered **glyph atlas** + per-cell
  `drawImage` (decision V8, research report 07), full redraw each frame.
- `app.js` wires pointer input, the editor, and the animation loop.

## Hosting

The single-threaded build needs no special headers and works on GitHub Pages
(add a `.nojekyll` file; Pages serves `application/wasm` correctly in production).
