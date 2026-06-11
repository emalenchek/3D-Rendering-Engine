# Project Brief: A Text-Encoded 3D Rendering Engine

Date: 2026-06-11 · Status: **Proposed** — synthesized from the five research reports in [`docs/research/`](research/).

## 1. Vision

A high-performance 3D rendering engine that is text-encoded in both directions:

- **Text in** — scenes are described in a small, human-writable declarative DSL (meshes referenced from standard asset files, not inlined).
- **Text out** — frames are rasterized by a from-scratch software pipeline and presented as ASCII/Unicode characters with ANSI color in a terminal, and later in a browser via WASM.

Architecture goal: a portable **core library** (math + scene model + rasterizer + cell-buffer output) with thin frontends — native terminal first, browser/WASM second.

### A note on the inspiration

Research found that [ASCILINE](https://github.com/YusufB5/ASCILINE) is a *video→ASCII streamer* (Python/OpenCV/WebSocket/Canvas), **not** a 3D engine ([report 02](research/02-ascii-terminal-rendering.md)). The actual technical lineage for this project is donut.c, TermGL, and rendersloth. That makes this project more novel than first assumed: there is no widely-adopted, general-purpose, DSL-driven, performance-oriented text-output 3D engine today. ASCILINE still contributes useful ideas: tiered color modes, colored-block "pixel mode", and a web canvas frontend that sidesteps terminal throughput limits.

## 2. Key research findings (one paragraph per report)

1. **[Engine architectures](research/01-engine-architectures.md)** — The proven minimal retained-mode API is three.js's: `Scene`/`Node`/`Mesh(Geometry, Material)`/`Camera`/`Light` + one entry point `render(scene, camera)`. The industry consensus (Forsyth's "Scene Graphs — Just Say No", Filament's flat entity scene) is to *not* let the transform tree drive the renderer: flatten to a flat draw list each frame. The canonical rasterizer pipeline is 8 fixed stages; the one stage tutorials skip that breaks free cameras is **homogeneous near-plane clipping**.
2. **[ASCII/terminal rendering](research/02-ascii-terminal-rendering.md)** — Every terminal 3D renderer converges on a per-cell z-buffer + luminance→character ramp. A terminal cell carries exactly one fg + one bg color, so **half-blocks (2×1)** are the only sub-cell mode with full color per sub-pixel (and square pixels); **Braille (4×2)** gives 8× dot resolution for mono wireframes. 200×50 truecolor at 60 FPS is feasible on modern terminals *if* the emitter does diff-based redraw, SGR batching, one buffered write per frame, and DEC-2026 synchronized output. Portable floor: 30 FPS.
3. **[Scene formats & DSL](research/03-scene-formats-dsl.md)** — POV-Ray and OpenSCAD prove people love hand-writing nested-block scenes *and* that embedding a programming language in the format kills tooling. glTF has the right content model (node tree + TRS + materials + cameras) and the wrong human syntax (indices, no comments). Recommendation: a strictly declarative DSL on a **KDL-style node grammar**, glTF-like content model with names instead of indices, defaults everywhere, heavy geometry referenced from external files. Parse speed of hand-written files is a non-issue; spend effort on error messages and hot-reload.
4. **[Language evaluation](research/04-language-evaluation.md)** — **Rust** is the only candidate strong on every required axis: stable wasm SIMD128, working wasm threads (wasm-bindgen-rayon), compile-time data-race-free tile-parallel rasterization (rayon), best terminal stack (crossterm/ratatui), best tooling (cargo). C++/Emscripten is the battle-tested runner-up (Figma-class proof) if browser-threaded perf were the *only* criterion. Zig is held back by pre-1.0 churn; Go is eliminated (no wasm threads/SIMD).
5. **[MVP scoping](research/05-mvp-scoping.md)** — tinyrenderer anchors the effort math: a complete software renderer is ~500 LOC / 10–20 h. At terminal resolution (200×50 = 10k cells ≈ 200× fewer pixels than 1080p), **rasterization is never the MVP bottleneck — terminal I/O is** (per-cell color SGR measured ~40× slower than plain text on slow emulators). Scope guard: the MVP deliverable is a *demo* (load OBJ, orbit it, shaded, in a terminal), not "an engine"; abstract only once a second frontend exists.

## 3. Decisions

| # | Decision | Rationale (report) |
|---|---|---|
| D1 | **Language: Rust** (stable toolchain; `std::arch`/`wide` for SIMD now, `std::simd` when stable; nightly only for the eventual wasm-threads build) | 04 |
| D2 | **Two-layer scene architecture**: user-facing three.js-style retained tree → flattened once per frame into a flat draw list the rasterizer consumes | 01 |
| D3 | **Fixed 8-stage software pipeline** with an internal vertex/fragment seam (tinyrenderer Lesson-6 pattern); homogeneous near-plane handling from day one (cull first, true clip in hardening phase) | 01, 05 |
| D4 | **Output is a presentation layer** over an intensity/color cell grid: ASCII luminance ramp (universal fallback), half-block truecolor (high-fidelity default), Braille (wireframe mode). Designed as a swappable trait so Sixel/Kitty pixel backends and the WASM canvas can slot in later | 02 |
| D5 | **Terminal emitter discipline**: front/back cell buffers with diff redraw, SGR run merging, single buffered write per frame, DEC-2026 when detected, capability-tiered color (mono→16→256→truecolor) | 02, 05 |
| D6 | **DSL: strictly declarative, KDL-style node grammar**; glTF-like content model (node tree, TRS, named materials/meshes, camera, lights); no loops/macros — programmatic scenes come from a builder API that emits the DSL | 03 |
| D7 | **Asset path separate from DSL**: `mesh "file.obj"` references; minimal OBJ subset (v/vt/vn/f, negative indices, fan triangulation) | 03, 05 |
| D8 | **Performance strategy**: correctness first; SIMD/multithreading deferred until profiling shows raster-bound (the data says terminal I/O binds first at cell resolutions; SIMD matters once sub-cell modes and WASM-canvas raise effective resolution) | 05 |

## 4. MVP definition

**Deliverable:** `tte render scene.kdl` (and `tte view model.obj`) — load a scene/OBJ, orbit it with WASD/+−, flat/Gouraud-shaded with a directional light, at ≥30 FPS in any modern terminal.

In scope:
1. Math: `Vec3`/`Vec4`/`Mat4` (mul, lookAt, perspective) — no external math lib
2. OBJ loader (minimal subset, derive normals when missing)
3. Pipeline: MVP transforms → back-face cull → near-plane cull → perspective divide → viewport (terminal cell aspect folded in) → edge-function raster → 1/z z-buffer → perspective-correct interpolation
4. Shading: flat + Gouraud diffuse (one directional + ambient)
5. Output modes: ASCII ramp `.,-~:;=!*#$@` + half-block truecolor; capability detection
6. Terminal frontend: alternate screen, raw mode (termios), diff redraw, orbit camera, frame timing readout, resize handling
7. Scene DSL v0: scene/node/box/sphere/mesh-ref/material/camera/light, defaults everywhere, friendly parse errors, hot-reload

Explicitly out (phase 5+): textures, shadows, full Sutherland–Hodgman clipping (cull-only first), MTL materials, robust triangulation, animation system, ECS, SIMD/threads, pixel protocols, mouse input.

## 5. Roadmap

Effort grounded in the precedents table in [report 05](research/05-mvp-scoping.md) (tinyrenderer: 10–20 h core; weekend-rasterizer data points). Assumes a competent programmer new to graphics.

| Phase | Scope | Est. |
|---|---|---|
| **1. Wireframe in terminal** | math types, OBJ parse, projection, Bresenham lines to cell buffer, spinning model — de-risks terminal I/O on day one | 8–15 h |
| **2. Solid shaded renderer** | edge-function fill, z-buffer, back-face cull, flat+Gouraud, ASCII ramp + half-block truecolor | 10–20 h |
| **3. Interactive orbit** | raw mode input, spherical orbit camera, frame pacing, near-plane cull, resize | 5–10 h |
| **4. Scene DSL ("text in")** | KDL-grammar parser, scene model w/ named materials & mesh refs, diagnostics, hot-reload | 10–20 h |
| **5. Hardening + WASM frontend** | true near clipping, color quantization polish; wasm32 build, cell-grid→canvas presenter, demo page | 15–30 h |
| **6. Performance push** | tile-based multithreading (rayon), SIMD raster loop, sub-cell blitters (quadrant/sextant w/ 2-color quantization), pixel protocols (Kitty/Sixel), benchmarks as CI artifacts | open-ended |

MVP = phases 1–4: **~35–65 h, ~1.5–2.5 KLOC**.

## 6. Risks

1. **Terminal emulator variability** — the 40× per-cell-color penalty on slow emulators is the dominant perf risk; mitigated by D5 discipline + capability tiers, and ultimately by the WASM frontend (which bypasses terminals entirely).
2. **WASM threads need nightly Rust + COOP/COEP headers** — keep the core single-thread-clean so the wasm build works on stable scalar first; threads are a phase-6 upgrade.
3. **Scope creep** — the "write games, not engines" failure mode; guarded by defining MVP as a demo and deferring abstraction until the second frontend exists.
4. **Sub-cell mode font/terminal support** — sextants/octants are patchy; half-blocks + Braille are the safe v1 pair with auto-degradation.

## 7. Open questions

- Project/crate name (working name in examples: `tte` — text-to-everything? — to be bikeshed).
- Whether DSL v0 uses the `kdl` crate directly or a hand-rolled ~300-line parser with scene-specific sugar (bare `(x y z)` vectors, degree literals) — decide at phase 4 start.
- License (MIT/Apache-2.0 dual is the Rust-ecosystem default).
