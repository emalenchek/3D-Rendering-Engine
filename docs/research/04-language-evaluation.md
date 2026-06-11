# Language Evaluation: High-Performance Portable 3D Software Renderer (Native Terminal + WASM)

> Status: RESEARCH IN PROGRESS — incremental notes, will be polished into final format.
> Date: 2026-06-11. Candidates: Rust, C, C++, Zig, Go.

## Raw findings (incremental)

### Native SIMD

- **Rust**: `std::simd` (portable SIMD) is still **nightly-only** as of late 2025; stabilization blocked on mask-type and swizzle API questions. `std::arch` intrinsics (SSE/AVX/NEON) ARE stable; safe usage requires `is_x86_feature_detected!` + `#[target_feature]`. Ecosystem crates (`wide`, `pulp`, `multiversion`) give portable SIMD on stable. Ergonomic SIMD multiversioning is a 2025 Rust project goal. Confidence: HIGH.
  - https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d
  - https://doc.rust-lang.org/std/simd/index.html
  - https://rust-lang.github.io/rust-project-goals/2025h1/simd-multiversioning.html
- **Go**: Historically NO SIMD intrinsics (assembly only). Movement in 2025: proposal #67520 + #73787 (multi-level approach); a low-level AMD64-only `simd/archsimd` package ships in **Go 1.26 under GOEXPERIMENT=simd** (Feb 2026). High-level portable `simd` package reserved for the future; ARM/WASM not covered yet. Still experimental, AMD64-only. Confidence: HIGH.
  - https://github.com/golang/go/issues/73787
  - https://github.com/golang/go/issues/67520
  - https://go.dev/blog/go1.26
- **Zig**: `@Vector` builtin is a first-class portable SIMD type in stable Zig; compiles to native SIMD where available, scalarizes otherwise. Works for wasm32 with `-mcpu baseline+simd128`. Confidence: HIGH.
  - https://ziglang.org/documentation/ (langref @Vector)
- **C/C++**: Mature intrinsics (`<immintrin.h>`, NEON), compiler auto-vectorization, OpenMP SIMD pragmas, ISPC as dedicated SPMD compiler. Gold standard. Confidence: HIGH (well-established).

### WASM threads + SIMD128

- All browser WASM threading requires **SharedArrayBuffer**, which requires cross-origin isolation headers: `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp`. Applies to every language. Confidence: HIGH.
  - https://web.dev/articles/webassembly-threads
- **Rust**: `wasm-bindgen-rayon` adapts Rayon to Web Workers + SharedArrayBuffer; requires **nightly** + rebuilding std with atomics (`-C target-feature=+atomics,+bulk-memory`), wasm-bindgen `--target web`. Working in production (e.g., used by Squoosh-style apps). SIMD128: stable via `-C target-feature=+simd128`, `std::arch::wasm32` intrinsics are stable; portable_simd also lowers to simd128. Confidence: HIGH.
  - https://github.com/RReverser/wasm-bindgen-rayon
  - https://web.dev/articles/webassembly-threads
- **C/C++ (Emscripten)**: pthreads support is mature (`-pthread`), maps pthreads to Web Workers + SAB; SIMD128 via `-msimd128`, including intrinsic header `wasm_simd128.h` and auto-vectorization; can even compile SSE/NEON intrinsics to simd128. Most mature WASM threads story. Confidence: HIGH.
  - https://web.dev/articles/webassembly-threads
  - https://emscripten.org/docs/porting/pthreads.html
- **Zig**: wasm32 SIMD128 works (`-mcpu baseline+simd128`, @Vector lowers to simd128). Threads on wasm: `wasm32-wasi` has limited shared-memory thread support (wasi-threads experimental); **browser-target threading must be hand-rolled** (no Emscripten-style runtime; you write the Worker + shared memory glue yourself). Confidence: MEDIUM.
  - https://vexcess.github.io/blog/zig-for-webassembly-guide.html
- **Go**: Standard Go wasm: goroutines multiplex on a single thread — **no real WASM thread parallelism**; no SIMD on wasm. TinyGo: goroutines via Binaryen Asyncify, effectively GOMAXPROCS=1, cooperative scheduler. No simd128 intrinsics path. Confidence: HIGH.
  - https://tinygo.org/docs/guides/webassembly/
  - https://dev.to/alanwest/why-your-go-binary-is-too-fat-for-webassembly-and-how-tinygo-fixes-it-24l

### WASM toolchain maturity + binary size

- **Rust**: wasm32-unknown-unknown tier-2 target; wasm-bindgen/wasm-pack generate JS/TS bindings; wasm-opt integration. Small libraries typically tens of KB (e.g., "hello"-class modules ~1–20 KB after wasm-opt with `opt-level="z"`, lto). Confidence: HIGH.
- **C/C++**: Emscripten is the oldest, most complete toolchain (full POSIX shims, OpenGL→WebGL, filesystem emulation); used by Figma, Google Earth, Photoshop web, Unity. Binary sizes small with `-Os`/closure. Confidence: HIGH.
- **Zig**: first-class `wasm32-freestanding` target (bare module, no JS bindgen — you write extern functions and JS glue by hand); very small binaries since no runtime. Some open issues remain with wasm32-freestanding in 0.15-dev (e.g., build errors #23867). No equivalent of wasm-bindgen. Confidence: HIGH for "no bindgen", MEDIUM for issue severity.
  - https://github.com/ziglang/zig/issues/23867
- **Go**: standard Go wasm binaries ~2+ MB minimum (runtime + GC included); TinyGo 10–20x smaller (tens of KB possible: 93K → 1.6K with -no-debug -panic=trap -scheduler=none -gc=leaking), but TinyGo drops/limits reflection, some stdlib, and uses slower GC. Confidence: HIGH.
  - https://tinygo.org/docs/guides/optimizing-binaries/
  - https://www.fermyon.com/blog/optimizing-tinygo-wasm

### Comparable projects

- **Mach (Zig game engine)**: still alive (machengine.org, hexops/mach), but tracks **nominated Zig nightly versions** rather than stable releases — illustrates Zig's pre-1.0 churn; engine roadmap still early/experimental. Confidence: MEDIUM (need direct check of 2025/2026 status).
  - https://machengine.org/docs/nominated-zig/
  - https://machengine.org/engine/roadmap/

(More findings to be appended: tiny-skia perf, Figma WASM, Bevy-in-browser, Ghostty, terminal ecosystem, build systems.)
