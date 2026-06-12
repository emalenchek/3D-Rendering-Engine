# 3D-Rendering-Engine

A high-performance 3D rendering engine, text-encoded in both directions:

- **Text in** — scenes described in a small, human-writable declarative DSL
- **Text out** — a from-scratch software rasterizer presenting frames as ASCII/Unicode + ANSI color in the terminal (browser/WASM frontend planned)

Currently in **Phase 2 complete** (solid shaded renderer) — loads OBJ models and renders
them as solid, depth-tested, diffuse-shaded surfaces in the terminal across ASCII, truecolor,
and half-block output modes.

## Quick start

```sh
# prerequisite: rustup (https://rustup.rs) — the pinned toolchain auto-installs
cargo run -p tte-cli -- --help                                   # CLI usage
cargo run -p tte-cli -- view crates/tte-core/tests/data/cube.obj # spin a shaded cube
cargo run -p tte-cli -- view --mode truecolor <model.obj>        # 24-bit color
cargo test                                                       # full test suite
```

Full setup guide: [docs/03-dev-setup.md](docs/03-dev-setup.md)

## Documentation

- **[Project brief](docs/00-project-brief.md)** — vision, decisions, MVP definition, roadmap, risks
- **[Requirements spec & test plan](docs/01-requirements-spec.md)** — requirement IDs mapped to functional e2e tests
- **[Test harness guide](docs/02-test-harness.md)** — test kinds, golden-frame snapshots, e2e patterns, CI gates
- **[Dev environment setup](docs/03-dev-setup.md)** — prerequisites, build, dependencies, tooling
- Research reports (cited, confidence-rated):
  1. [3D engine architectures & API design](docs/research/01-engine-architectures.md)
  2. [ASCII/terminal 3D rendering prior art & performance limits](docs/research/02-ascii-terminal-rendering.md)
  3. [Text scene-description formats & DSL design](docs/research/03-scene-formats-dsl.md)
  4. [Language evaluation: Rust / C / C++ / Zig / Go](docs/research/04-language-evaluation.md)
  5. [MVP scoping precedents & effort estimates](docs/research/05-mvp-scoping.md)
  6. [Rust testing best practices](docs/research/06-rust-testing-best-practices.md)

## Headline conclusions

- **Language: Rust** — the only candidate strong on native SIMD, WASM SIMD/threads, race-free parallel rasterization, and terminal ecosystem simultaneously.
- **Architecture: two layers** — a three.js-style retained scene tree for users, flattened per frame into a flat draw list for the rasterizer.
- **Output: per-cell z-buffer + swappable presenters** — ASCII luminance ramp, half-block truecolor, Braille wireframe; diff-based escape emission.
- **DSL: strictly declarative, KDL-style grammar** with a glTF-like content model; heavy geometry imported from OBJ/glTF, never inlined.
- **MVP ≈ 35–65 h / ~2 KLOC** across 4 phases: terminal wireframe → shaded renderer → interactive orbit → scene DSL. WASM frontend and SIMD/multithreading follow.
