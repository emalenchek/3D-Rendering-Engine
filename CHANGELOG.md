# Changelog

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
