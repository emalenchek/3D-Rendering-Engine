# Changelog

## v2.2.0 — Mobile compatibility & performance

Make the live demo **load everywhere again** and run **smoothly on mobile** — every
change presentation-layer or build/loader, so the engine's byte-identical output, the
data-only WASM boundary, and the Pages deploy are untouched. Driven by `docs/research/14`
(mobile compatibility); scope in `docs/06-v2.2-scope.md`.

### Compatibility (Phase 9)
- **Scalar wasm fallback** restores load on iOS Safari < 16.4: a `+simd128` module
  fails to instantiate there, blanking the demo since v2.1. `web/build.sh` now emits two
  artifacts (`tte_wasm_bg.wasm` SIMD + `tte_wasm_bg.scalar.wasm`) sharing one ABI-identical
  glue, and `app.js` feature-detects wasm SIMD and loads the right one. CI smoke-tests both.

### Canvas2D presenter performance (Phase 10)
- **One-composite presenter**: the per-cell `globalCompositeOperation` glyph tinting (the
  prime mobile-Safari bottleneck) is batched into a single whole-frame composite — bit-
  identical output, guarded by a Playwright pixel-equality test.
- **DPR cap** (≤2, decoupled from `devicePixelRatio`) + iOS canvas-area guard.
- **Adaptive resolution** (scales the cell grid to the measured FPS, aspect preserved),
  a **~30 fps cap** on touch devices, and **pause while hidden**. Policy is a pure,
  unit-tested `nextScale()`; a new wasm `Renderer.resize()` keeps the subject on resize.
- **OffscreenCanvas + Worker** path: the WASM rasterizer *and* the presenter run off the
  UI thread when available, with an automatic main-thread fallback (forced via `?noworker`,
  and self-recovering on worker error/timeout/blank-atlas).

### GPU presenter (Phase 11)
- **Additive WebGL2 presenter**: uploads the cell grid as textures and blends fg over bg
  in one fullscreen shader, offloading compositing to the GPU. Runtime-selected with a
  Canvas2D fallback (`?nogl`); matches the Canvas2D output within tolerance (CI-tested).
  WebGPU deferred (iOS 26+). Now the live demo's default where supported.

### Notes
- The demo status line shows the live `render` vs `present` split for on-device profiling
  (`docs/research/14b`). WASM threads remain deferred (GitHub Pages can't set COOP/COEP;
  `docs/research/10`). `tte` CLI and terminal output are unchanged.

## v2.1.0 — Profile-guided SIMD + live demo

Make the engine *faster* (a profile-guided SIMD geometry stage) and *visible* (a
live, auto-deployed, CI-tested demo). Scope in `docs/05-v2.1-scope.md`; de-risking
research in `docs/research/11`–`12`.

### Live demo deployment (Phase 8)
- `.github/workflows/deploy.yml`: the `web/` WASM demo auto-deploys to GitHub Pages
  on every merge to `main` via the official `configure/upload/deploy-pages` flow,
  with license attribution bundled into the published site.
- CI `demo-smoke` job: a headless-Chromium Playwright test serves the built demo and
  asserts it boots (WASM init + a live FPS readout, zero console/page errors).

### Profile-guided SIMD (Phase 7)
- **Profiled first** (FR-7.1, `docs/research/11b`): the geometry stage dominates the
  100k-tri frame, but the per-vertex `Mat4×Vec4` transform is only ~4% of it — so the
  kernel was re-scoped to vectorize the *whole* per-triangle stage, not just transforms.
- **`wide::f32x8` geometry kernel** behind an opt-in `simd` feature: per-vertex
  transforms (8 vertices/lane) and the per-triangle stage — near-cull, face normal,
  Lambert shading, perspective divide + viewport, row span (8 triangles/lane). The
  integer rasterizer is untouched.
- **Byte-identical** to the scalar path (FR-7.4): no `mul_add`/FMA, scalar op order
  replayed, so output matches bit-for-bit on SSE2/AVX2 and the wasm-simd128 build. A
  parity test guards the transform pass, the triangle list, and the rendered frame;
  CI runs it on both AVX2 and the SSE2 baseline.
- **~1.5–1.6×** on the geometry stage at 100k tris @ 400×200, single-thread (NFR-13).
  The **live demo** ships with wasm SIMD too (`+simd128`, W5) — byte-identical frames.
- Benches: criterion reads the scalar-vs-simd wall-clock; an `iai-callgrind` bench
  (`iai_geom`) gives deterministic instruction counts as a CI regression gate (FR-7.5).

## v2.0.0 — Browser frontend + performance push

The engine now runs in the browser *and* renders in parallel on multiple cores —
the two original "portable core, two frontends" and "performant" goals. Grounded in
de-risking research (`docs/research/07`–`10`); scope in `docs/04-v2.0-scope.md`.

### Browser / WASM frontend (Phase 5)
- New `tte-wasm` crate: a `wasm-bindgen` `Renderer` over `tte-core` — load OBJ/DSL,
  orbit, render, and pull frames out as typed arrays. No `web-sys` (data-only boundary).
- Core `web_frame` export: `Framebuffer` → per-cell `{glyph, fg, bg}`, shared natively
  and in WASM (identical frames).
- `web/` demo: glyph-atlas Canvas2D renderer, mouse **and touch** orbit/zoom, live scene
  editor, presets. Build via `web/build.sh` (cargo → wasm-bindgen → wasm-opt).
- WASM binary ~98 KB raw / ~47 KB gzipped (well under the 250 KB budget).
- `wasm-bindgen-test` smoke tests run the real module in Node; CI `wasm` job.

### Performance push (Phase 6)
- **Integer edge-function rasterization**: exact sub-pixel `orient2d` + top-left rule —
  deterministic *and* watertight (replaces the float-coverage seam workaround).
- **Tile-based multithreaded rasterization** (rayon) plus a parallelized geometry stage,
  behind a `parallel` feature (default native; off for WASM). Byte-identical to the
  scalar path (proven by a parity test). ~1.9× on a 4-core machine.
- Expanded benchmarks (100k-triangle / 400×200); CI feature matrix.
- Deferred with rationale: SIMD inner loop (FR-6.2) and WASM threads (FR-6.5) — the
  integer-edge foundation leaves both as clean reserved follow-ups.

### Notes
- `tte` CLI and terminal output are unchanged and fully compatible.

## v1.0.0 — MVP (text-encoded 3D rendering engine)

First release. A from-scratch software 3D renderer that is text-encoded in both
directions: scenes described in a human-writable DSL ("text in"), rendered to
ASCII/Unicode + ANSI color in the terminal ("text out"). Built in Rust per the
research in `docs/research/`.

### Engine (`tte-core`)
- Math: `Vec3`/`Vec4`/`Mat4` with TRS, look-at, perspective projection
- Wavefront OBJ loader (minimal subset; fan triangulation; derived normals)
- Software pipeline: MVP transform → near-plane cull → perspective divide →
  viewport → edge-function rasterization with a z-buffer
- Shading: directional Lambert light, flat and Gouraud modes
- Output presenters: ASCII luminance ramp, 24-bit truecolor, half-block
- Orbit camera (spherical)
- Scene DSL: KDL-style parser, named materials, primitives (cube/sphere/plane),
  external mesh references, nested transform groups; round-trippable serializer

### CLI (`tte`)
- `tte view <model.obj | scene.scene>` — interactive orbit viewer
  (arrows/hjkl orbit, +/− zoom, space auto-orbit, r reset, q quit)
- `--render`, `--shading`, `--mode`, `--yaw/--pitch/--radius` options
- `--headless` deterministic frame dump for tests/pipelines
- Live hot-reload of scene files

### Quality
- 138 tests (unit, integration, golden-frame, property/fuzz) + PTY smoke tests
- CI: rustfmt, clippy `-D warnings`, cargo-nextest on Linux/macOS/Windows,
  cargo-deny; criterion benchmarks

### Known limitations / post-MVP roadmap
See `docs/01-requirements-spec.md` §5. Near-plane handling is cull-not-clip;
the WASM/browser frontend and the SIMD + multithreading performance push are
planned next.
