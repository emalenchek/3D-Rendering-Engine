# 3D-Rendering-Engine

A high-performance 3D rendering engine, text-encoded in both directions:

- **Text in** — scenes described in a small, human-writable declarative DSL
- **Text out** — a from-scratch software rasterizer presenting frames as ASCII/Unicode + ANSI color in the terminal (browser/WASM frontend planned)

Currently in the **research & scoping** stage. This repository holds the project documentation now and will hold the source code as development begins.

## Documentation

- **[Project brief](docs/00-project-brief.md)** — vision, decisions, MVP definition, roadmap, risks
- Research reports (cited, confidence-rated):
  1. [3D engine architectures & API design](docs/research/01-engine-architectures.md)
  2. [ASCII/terminal 3D rendering prior art & performance limits](docs/research/02-ascii-terminal-rendering.md)
  3. [Text scene-description formats & DSL design](docs/research/03-scene-formats-dsl.md)
  4. [Language evaluation: Rust / C / C++ / Zig / Go](docs/research/04-language-evaluation.md)
  5. [MVP scoping precedents & effort estimates](docs/research/05-mvp-scoping.md)

## Headline conclusions

- **Language: Rust** — the only candidate strong on native SIMD, WASM SIMD/threads, race-free parallel rasterization, and terminal ecosystem simultaneously.
- **Architecture: two layers** — a three.js-style retained scene tree for users, flattened per frame into a flat draw list for the rasterizer.
- **Output: per-cell z-buffer + swappable presenters** — ASCII luminance ramp, half-block truecolor, Braille wireframe; diff-based escape emission.
- **DSL: strictly declarative, KDL-style grammar** with a glTF-like content model; heavy geometry imported from OBJ/glTF, never inlined.
- **MVP ≈ 35–65 h / ~2 KLOC** across 4 phases: terminal wireframe → shaded renderer → interactive orbit → scene DSL. WASM frontend and SIMD/multithreading follow.
