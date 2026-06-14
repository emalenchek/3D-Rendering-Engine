# Changelog

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
