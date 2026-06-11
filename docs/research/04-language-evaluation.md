# Language Evaluation: High-Performance Portable 3D Software Renderer (Native Terminal + WASM)

Date: 2026-06-11. Candidates: Rust, C, C++, Zig, Go. Target: one core compiled natively (terminal app) and to browser WASM, with SIMD + multithreaded rasterization.

---

## Per-language assessment

### Rust

- **WASM toolchain**: most mature high-level wasm story of any language. `wasm32-unknown-unknown` tier-2 target; wasm-bindgen/wasm-pack auto-generate JS/TS bindings; wasm-opt integration. Small libraries reach tens of KB (single-digit KB after `opt-level="z"` + LTO + wasm-opt). Caveat: some crates and std functions compile for wasm but fail at runtime. — HIGH. [Thinking About WebAssembly (2026)](https://medium.com/codex/thinking-about-webassembly-read-this-before-choosing-rust-or-zig-b22c30594563)
- **WASM SIMD128**: stable. `-C target-feature=+simd128`; `std::arch::wasm32` intrinsics are stable; portable SIMD lowers to simd128. — HIGH.
- **WASM threads**: works via [wasm-bindgen-rayon](https://github.com/RReverser/wasm-bindgen-rayon) (GoogleChromeLabs; actively maintained, tested with nightly-2025-11-15): Rayon thread pool on Web Workers + SharedArrayBuffer. Caveat: requires **nightly** + rebuilding std with `-C target-feature=+atomics,+bulk-memory` — not first-class in stable Rust yet. — HIGH.
- **Native SIMD**: `std::simd` (portable) still **nightly-only** as of late 2025; `std::arch` (SSE/AVX/NEON) intrinsics ARE stable with `is_x86_feature_detected!` + `#[target_feature]`. Stable-channel portable SIMD via crates `wide`, `pulp`, `multiversion`. SIMD multiversioning ergonomics was a 2025 Rust project goal. — HIGH. [State of SIMD in Rust 2025](https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d), [rust-project-goals](https://rust-lang.github.io/rust-project-goals/2025h1/simd-multiversioning.html)
- **Multithreaded rasterization**: rayon gives work-stealing tile parallelism in ~5 lines (`par_chunks_mut` over framebuffer tiles); the borrow checker makes parallel framebuffer writes **provably data-race-free at compile time** — unique among the candidates. — HIGH.
- **Terminal**: crossterm (Linux/macOS/Windows by default) + ratatui — the most active TUI ecosystem of any language. — HIGH. [ratatui backends](https://ratatui.rs/concepts/backends/comparison/)
- **Comparable projects**: [tiny-skia](https://github.com/linebender/tiny-skia) (Rust port of Skia's CPU pipeline) is 20–100% slower than Skia on x86-64 (100–300% on ARM) but faster than cairo/raqote; notably, Skia's own peak perf depends on clang-only vector extensions (gcc/msvc builds are 15–30x slower) — Rust gets within ~2x of a compiler-fragile C++ ceiling, portably. Newer [vello_cpu](https://linebender.org/blog/tmil-19/) beats Skia/Cairo in many benchmarks. Bevy runs on wasm but its core still doesn't use web multithreading ([issue #4078](https://github.com/bevyengine/bevy/issues/4078); bevy CLI labels wasm threads ["unstable"](https://thebevyflock.github.io/bevy_cli/cli/web/multi-threading.html)) — wasm threads in Rust work but are opt-in, not free. — HIGH.
- **Ecosystem**: cargo is best-in-class (deps, tests, benches, cross-compilation via rustup targets). `#[no_mangle] extern "C"` + cdylib gives clean C-ABI export for future bindings. — HIGH.

### C++

- **WASM toolchain**: Emscripten is the oldest, most complete toolchain (POSIX shims, filesystem emulation, GL→WebGL); binary size good with `-Os`. Bindings via embind or hand-written extern "C" — more manual than wasm-bindgen. Some churn at edges (Emscripten's WebGPU bindings now deprecated in favor of Dawn's emdawnwebgpu). — HIGH. [Figma WebGPU blog](https://www.figma.com/blog/figma-rendering-powered-by-webgpu/)
- **WASM SIMD + threads**: most mature: `-msimd128` with `wasm_simd128.h` (and SSE/NEON intrinsic emulation headers), `-pthread` maps pthreads→Web Workers + SharedArrayBuffer transparently. — HIGH. [web.dev wasm threads](https://web.dev/articles/webassembly-threads)
- **Native SIMD**: gold standard — `<immintrin.h>`/NEON intrinsics, auto-vectorization, OpenMP SIMD, ISPC as a dedicated SPMD compiler. — HIGH.
- **Threads**: std::thread/TBB mature; **no compile-time data-race protection** — parallel framebuffer writes are correct only by discipline (tile ownership), validated by TSan. — HIGH.
- **Terminal**: [FTXUI](https://github.com/ArthurSonzogni/FTXUI) — modern C++ TUI, explicitly supports Linux/macOS/Windows/BSD/WASM. Can also use C libs (notcurses). — HIGH.
- **Comparable projects**: Figma's canvas engine is C++/Emscripten with TS bindings; WASM cut load times >3x — the strongest production proof of a C++ wasm core in existence. — HIGH. [Figma is powered by WebAssembly](https://www.figma.com/blog/webassembly-cut-figmas-load-time-by-3x/)
- **Ecosystem**: cmake + vcpkg/conan is workable but far clunkier than cargo; cross-compilation setup is manual; C-ABI export trivial. — HIGH.

### Zig

- **WASM toolchain**: first-class `wasm32-freestanding` target, no Emscripten needed; "pretty much everything compiles out of the box"; tiny binaries (no runtime). **No bindgen equivalent** — you hand-write extern functions and the JS glue, and marshal strings/structs across the boundary manually. — HIGH. [Thinking About WebAssembly](https://medium.com/codex/thinking-about-webassembly-read-this-before-choosing-rust-or-zig-b22c30594563)
- **WASM SIMD**: `@Vector` lowers to simd128 with `-mcpu baseline+simd128`. **WASM threads**: no Emscripten/rayon-style runtime; browser threading means hand-rolling Workers + shared-memory glue yourself. — MEDIUM.
- **Native SIMD**: `@Vector` builtin is first-class portable SIMD in stable Zig (scalarizes where unsupported) — arguably the cleanest portable-SIMD syntax of the five. — HIGH.
- **Threads**: std.Thread is fine; no data-race protection. Note Zig 0.16 is **rearranging all I/O/sync into a new std.Io interface** — std-level churn directly touching threading code. — MEDIUM-HIGH.
- **Stability**: still pre-1.0; 0.15.1 had "extremely breaking changes"; 0.16 (2026) again overhauls std; community predicts 1.0 mid-to-late 2026 (unconfirmed). — HIGH that churn is real. [0.15.1 notes](https://ziglang.org/download/0.15.1/release-notes.html), [0.16 milestone](https://github.com/ziglang/zig/milestone/30)
- **Terminal**: libvaxis is the main Zig TUI lib; small ecosystem; Windows support less proven than crossterm/tcell. — LOW-MEDIUM.
- **Comparable projects**: [Ghostty 1.0](https://ghostty.org/docs/about) (Jan 2025) proves Zig ships production systems software — but macOS/Linux only, Windows "coming at some point" ([The Register](https://www.theregister.com/2025/01/08/ghostty_1/)). Mach engine remains pre-1.0, tracks "nominated" Zig **nightlies** ([docs](https://machengine.org/docs/zig-version/)); a community fork ("mach-next", modernized for 0.15.2) signals upstream version lag. — MEDIUM-HIGH.
- **Ecosystem**: `zig build` + package manager improving fast; best-in-class cross-compilation (bundled libcs); trivially exports C ABI (it IS a C compiler). But library ecosystem is thin: math, image loading, arg parsing often DIY. — HIGH.

### C

- Everything from C++ applies (Emscripten, intrinsics, pthreads) minus abstractions: no templates/generics for a maintainable vec/matrix math layer; manual memory management throughout. Terminal: ncurses (POSIX; Windows via PDCurses), termbox, [notcurses](https://github.com/dankamongmen/notcurses) (modern, Windows support newer/less battle-tested — MEDIUM). Build: make/cmake, no package manager. C ABI is the native tongue. Verdict: maximal portability, maximal foot-guns, lowest productivity for a renderer-sized codebase. — HIGH.

### Go

- **WASM toolchain**: standard Go wasm binaries ≈2+ MB minimum (runtime + GC ship in the binary). TinyGo shrinks 10–20x (KB-scale possible with `-no-debug -panic=trap -scheduler=none -gc=leaking`) but drops/limits reflection and stdlib and uses a slower collector. — HIGH. [TinyGo optimizing guide](https://tinygo.org/docs/guides/optimizing-binaries/)
- **WASM SIMD/threads**: the killer gap. Standard Go on wasm multiplexes goroutines on **one thread — no parallelism**; no simd128 path. TinyGo: cooperative scheduler, effectively GOMAXPROCS=1. A wasm software rasterizer in Go is scalar and single-threaded. — HIGH. [TinyGo wasm guide](https://tinygo.org/docs/guides/webassembly/)
- **Native SIMD**: historically assembly-only. Go 1.26 (Feb 2026) ships experimental `simd/archsimd` under GOEXPERIMENT=simd — **AMD64-only**, low-level; portable high-level package still future work (Go 1.27+). — HIGH. [golang/go#73787](https://github.com/golang/go/issues/73787), [Go 1.26](https://go.dev/blog/go1.26)
- **Threads**: goroutines make native tile-parallel rasterization easy; GC pauses are mostly fine at frame timescales but add jitter; race detector is dynamic-only. — HIGH.
- **Terminal**: excellent — tcell + Bubble Tea (Charm), strong Windows support, "fastest path to a polished TUI". — HIGH. [BubbleTea vs Ratatui](https://www.glukhov.org/developer-tools/comparisons/tui-frameworks-bubbletea-go-vs-ratatui-rust/)
- **Ecosystem**: go mod + trivial cross-compilation; but c-shared/C-ABI export drags the runtime along. — HIGH.

---

## Comparison table

| Criterion | Rust | C | C++ | Zig | Go |
|---|---|---|---|---|---|
| Native SIMD | std::arch stable; std::simd nightly; good crates (`wide`/`pulp`) | Intrinsics, mature | Intrinsics + ISPC, gold standard | `@Vector` first-class, stable | GOEXPERIMENT only, AMD64-only (Go 1.26) |
| WASM SIMD128 | Stable (`+simd128`, std::arch::wasm32) | Emscripten `-msimd128` | Emscripten `-msimd128` + intrinsic headers | `@Vector` → simd128 | **None** |
| Native threads | rayon; compile-time race-free | pthreads, manual safety | std::thread/TBB, manual safety | std.Thread (std.Io churn in 0.16) | goroutines + GC jitter |
| WASM threads | wasm-bindgen-rayon (nightly + std rebuild) | Emscripten pthreads (best) | Emscripten pthreads (best) | DIY Workers + SAB glue | **None (1 thread)** |
| WASM toolchain | wasm-bindgen/wasm-pack, excellent | Emscripten, mature/heavy | Emscripten, mature/heavy (Figma) | Freestanding, no bindgen, hand glue | Poor fit; TinyGo limited |
| WASM binary size | Tens of KB | Small | Small–medium | Smallest (no runtime) | 2+ MB (TinyGo: KBs w/ limits) |
| Terminal libs | crossterm/ratatui (best-in-class, Windows ✓) | ncurses/notcurses (Windows partial) | FTXUI (Windows ✓) | libvaxis (small, Windows ?) | tcell/bubbletea (excellent, Windows ✓) |
| Build/ecosystem | cargo (best) | make/cmake, no pkg mgr | cmake+vcpkg (clunky) | zig build (good, churning) | go mod (great) |
| Memory/race safety | Compile-time guaranteed | None | None (TSan/ASan only) | Better-than-C, not race-safe | GC + dynamic race detector |
| Language stability | Stable, 6-wk releases | Frozen | Stable | **Pre-1.0, breaking each release** | Stable |

## Recommendation (ranked)

1. **Rust** — the only candidate strong on every axis this project needs: stable wasm SIMD128, a proven wasm-threads path (wasm-bindgen-rayon), compile-time data-race-freedom for tile-parallel framebuffer writes (rayon `par_chunks_mut` is the exact shape of a tiled rasterizer), the best terminal stack (crossterm/ratatui), best build tooling, and direct evidence (tiny-skia/vello_cpu) that Rust software rasterizers land within ~2x of clang-tuned Skia while beating Cairo. Costs: nightly + std rebuild for wasm threads; std::simd not stabilized (use std::arch or `wide`/`pulp` on stable).
2. **C++** — choose if maximum native SIMD ceiling (ISPC) and the single most battle-tested wasm threads/SIMD pipeline (Emscripten pthreads; Figma-class proof) outweigh data-race risk and cmake/vcpkg friction.
3. **Zig** — smallest wasm binaries, elegant `@Vector`, trivial C-ABI/cross-compilation; rejected as primary because of pre-1.0 breakage every release (0.15→0.16 std overhaul; Mach pinned to nightlies), no wasm-threads runtime, and a thin terminal/library ecosystem.
4. **C** — viable everywhere but lowest productivity and safety for a renderer-sized codebase; only sensible as an FFI-friendly subset discipline within C++/Zig.
5. **Go** — eliminated by the browser requirement: no wasm threads, no wasm SIMD, multi-MB binaries (TinyGo trades away too much); native SIMD still experimental AMD64-only. Great TUI ecosystem can't compensate for a scalar single-threaded wasm core.

**Strongest counterargument to Rust**: Emscripten's C++ pthreads/SIMD pipeline is genuinely more turnkey and battle-tested for wasm than Rust's nightly-flagged atomics path — Figma ships it at massive scale, while Bevy still hasn't enabled wasm threading in core. If browser multithreaded performance were the single dominant requirement and team safety/tooling weighed nothing, C++ + Emscripten would win. The rebuttal: this project equally needs a native terminal app and long-term maintainability, where Rust's terminal stack, cargo, and race-freedom dominate, and tiny-skia shows the perf gap is a tolerable constant factor (and Skia's edge is clang-fragile anyway).

## Key sources

- https://web.dev/articles/webassembly-threads (COOP/COEP, Emscripten/Rust threads)
- https://github.com/RReverser/wasm-bindgen-rayon
- https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d
- https://github.com/golang/go/issues/73787 ; https://go.dev/blog/go1.26
- https://tinygo.org/docs/guides/webassembly/ ; https://tinygo.org/docs/guides/optimizing-binaries/
- https://github.com/linebender/tiny-skia ; https://linebender.org/blog/tmil-19/
- https://www.figma.com/blog/webassembly-cut-figmas-load-time-by-3x/ ; https://www.figma.com/blog/figma-rendering-powered-by-webgpu/
- https://github.com/bevyengine/bevy/issues/4078 ; https://thebevyflock.github.io/bevy_cli/cli/web/multi-threading.html
- https://ghostty.org/docs/about ; https://www.theregister.com/2025/01/08/ghostty_1/
- https://machengine.org/docs/zig-version/ ; https://github.com/hexops/mach/issues/1326
- https://ziglang.org/download/0.15.1/release-notes.html ; https://github.com/ziglang/zig/milestone/30
- https://ratatui.rs/concepts/backends/comparison/ ; https://github.com/ArthurSonzogni/FTXUI ; https://www.glukhov.org/developer-tools/comparisons/tui-frameworks-bubbletea-go-vs-ratatui-rust/
