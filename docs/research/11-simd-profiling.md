# Research 11: Profile-Guided SIMD for tte v2.1.0

Scope: confirm the geometry-vs-fill split with a profiler, pick a stable-Rust SIMD
crate that yields deterministic (ideally byte-identical) output, vectorize the hot
stage (likely Mat4xVec4 vertex transforms), and set honest NFRs.

Status: IN PROGRESS (incremental). Date: 2026-06-20.

---

## Q1. Profiling Rust release builds (locate the hot stage)

- **Symbols in release.** Profiles are meaningless without debug info. Set in the
  benchmarked profile: `[profile.release] debug = true` (or `debug = "line-tables-only"`
  for smaller symbols), or env `CARGO_PROFILE_RELEASE_DEBUG=true`. Keep optimizations on.
  Confidence: HIGH. Source: Rust Performance Book — Profiling
  (https://nnethercote.github.io/perf-book/profiling.html).
- **Frame pointers.** `perf`/flamegraph stack unwinding is much more reliable with frame
  pointers; build with `RUSTFLAGS="-C force-frame-pointers=yes"` (or use DWARF call-graph
  `perf record --call-graph dwarf`, slower/larger). Confidence: HIGH.
- **cargo flamegraph** (flamegraph-rs): wraps perf (Linux) / dtrace (macOS/BSD), emits an
  interactive SVG. Best for an at-a-glance "which function dominates" view to confirm the
  geometry-vs-fill split visually. Sampling (statistical), not deterministic.
  Source: https://github.com/flamegraph-rs/flamegraph. Confidence: HIGH.
- **samply**: cross-platform sampling profiler (Linux/macOS/Windows), opens results in the
  Firefox Profiler UI with a good call-tree + inverted ("self time") view — better than
  flamegraph for attributing self-time to `transform`/`orient2d`/fill. `samply record
  cargo run --release …`. Source: https://github.com/mstange/samply, Rust Perf Book.
  Confidence: HIGH.
- **perf stat / perf record** (Linux): `perf stat` gives cycles, instructions, IPC,
  cache-miss counters — use to learn whether the geometry stage is compute-bound or
  memory/bandwidth-bound (key for SIMD ceiling, Q6). Confidence: HIGH.
- **iai-callgrind** (now also maintained as "gungraun"): runs benches under Valgrind
  Callgrind → **instruction counts / cache sim / estimated cycles**. Deterministic and
  noise-free, so it is the **CI-friendly** way to track per-stage cost and catch
  regressions; pairs with `cargo bench`. Caveat: counts instructions, not wall-clock, and
  Valgrind serializes execution (no real parallelism), so it measures the *scalar/geometry
  cost per call*, not the rayon end-to-end time. Install `iai-callgrind-runner` matching the
  dev-dep version. Sources: https://github.com/iai-callgrind/iai-callgrind,
  https://lib.rs/crates/iai-callgrind. Confidence: HIGH.

**Recommendation for Q1:** Use **samply** (or cargo flamegraph) once to *confirm* the
geometry-bound hypothesis at 100k tris @ 400x200 (look for Mat4xVec4 transform +
edge-function setup dominating over the fill loop), use `perf stat` to check the
compute-vs-bandwidth nature, then add **iai-callgrind** benches per stage (transform /
setup / fill) as the durable CI regression guard. criterion stays for wall-clock speedup
numbers in the NFRs.

---

## Q2. SIMD crate choice on stable Rust (2025/2026)

- **`std::simd` (portable_simd)** is **still nightly-only** as of 2025/2026 — not usable on
  the stable toolchain. Rules it out for a stable release. Confidence: HIGH (tracking
  issue still open; portable-simd repo). Source: https://github.com/rust-lang/portable-simd.
- **`wide`** (Lokathor): stable, provides `f32x4`, `f32x8`, `f64x2`, `f64x4`, `i32x4`,
  `i32x8`, etc. Fixed lane counts independent of host → deterministic lane structure.
  Plain `Mul`/`Add` operators are *separate* multiply and add (NOT auto-fused, see Q4), so
  cross-platform results are stable **unless you call `mul_add`** (which DOES use hardware
  FMA where available → platform-dependent rounding). Sources:
  https://github.com/Lokathor/wide, https://docs.rs/wide. Confidence: HIGH.
- **`pulp`** (Sarah/`faer` author): safe abstraction over SIMD with **runtime feature
  dispatch** — you write one generic kernel, pulp picks SSE/AVX2/AVX512/NEON at runtime.
  Stable. Good when you want a single source compiled for multiple ISAs with a safe API.
  Confidence: MEDIUM (need to confirm determinism guarantees w/ a fetch).
- **`multiversion`** crate + `#[target_feature]`: function multiversioning + runtime
  detection on stable; clones a function per target-feature set and dispatches. Confidence:
  HIGH (to verify with a fetch).
- **Glam**: uses **horizontal** 128-bit SIMD (one Vec4/Mat4 at a time) on x86/x86_64/wasm32;
  battle-tested, fast in mathbench. Adopting glam replaces the hand-rolled Mat4/Vec but does
  NOT batch 4-8 vertices vertically. ultraviolet / nalgebra offer **vertical** AoSoA
  (`f32x4`/`f32x8`-backed) types that transform 4/8 vertices at once — the better fit for a
  transform-throughput-bound workload. Sources: https://github.com/bitshifter/glam-rs,
  https://github.com/bitshifter/mathbench-rs,
  https://www.rustsim.org/blog/2020/03/23/simd-aosoa-in-nalgebra/. Confidence: HIGH.

---

## Q4. Determinism under SIMD float math (partial — verified)

- **Rust does NOT contract `a*b + c` into an FMA by default.** Rust/LLVM lowers `mul_add`
  to `llvm.fma` (always fused, single rounding) but lowers separate `*` then `+` to two
  separately-rounded ops — Rust never enables `-ffp-contract=fast`, so the optimizer will
  not silently fuse them. This means: if you write transforms with plain `*`/`+` (no
  `mul_add`), the result is the **same on FMA and non-FMA hardware**. The moment you call
  `mul_add` / `simd_fma`, you get hardware FMA where available → **different rounding on
  AVX2-FMA vs an SSE2-only box** → breaks byte-identical parity.
  Sources: Rust RFC 3514 float-semantics
  (https://rust-lang.github.io/rfcs/3514-float-semantics.html);
  rust-lang/libs-team #712; siboehm "Inlining, FMA and FP consistency"
  (https://siboehm.com/articles/23/Inlining-FMA-FP-consistency). Confidence: HIGH
  (two+ independent sources agree).
- **Implication for `wide`:** its plain `f32x4 * + ` operators are safe/deterministic; its
  `mul_add` is NOT. So a `wide`-based transform that avoids `mul_add` can stay
  byte-identical to the scalar path *provided the scalar path also avoids `mul_add` and
  uses the same operation order* (see reduction-ordering hazard below — still to expand).

(To expand: reduction/dot-product ordering, denormals/FTZ, x87-vs-SSE, ULP-tolerance
recommendation — pending more sources.)

---

---

## Q3. Vectorizing the geometry stage (Mat4xVec4)

- **AoS vs SoA/AoSoA.** AoS (interleaved x,y,z,w per vertex) wastes SIMD lanes for a
  "transform many vertices" loop. Convert positions to **SoA / AoSoA** (one lane per
  vertex): hold `f32x4`/`f32x8` of xs, ys, zs, ws, multiply by *broadcast* matrix elements.
  Literature shows **2-4x** for AoS->SoA at small/medium sizes, tapering as the working set
  exceeds cache. Source: HAL "Data layout and SIMD abstraction layers"
  (https://hal.science/hal-01915529/document); nadavrot matmul gist
  (https://gist.github.com/nadavrot/5b35d44e8ba3dd718e595e40184d03f0). Confidence: MEDIUM.
- **Two ways to vectorize a Mat4xVec4:**
  - *Horizontal* (glam-style): one vertex per 128-bit register, 4 dot products / shuffles.
    Easy drop-in, ~Vec4 width, no batching. glam's Vec4/Mat4 already do this on
    x86/x86_64/wasm32. Source: https://github.com/bitshifter/glam-rs. Confidence: HIGH.
  - *Vertical* (AoSoA, ultraviolet/nalgebra-style): transform 4 (f32x4) or 8 (f32x8)
    vertices simultaneously; each output component = sum of 4 broadcast-mul terms. Best
    throughput for the "100k tiny tris" transform-bound workload; uses plain mul+add (16
    muls + 12 adds per batch), no horizontal shuffles. Sources: rustsim AoSoA blog,
    mathbench-rs. Confidence: HIGH.
- **glam adoption vs hand-rolled.** glam is fast and correct but **horizontal** — it does
  not batch vertices, so it would not exploit the transform-throughput headroom as well as a
  `wide`-backed AoSoA kernel. Also, switching the engine's exact integer rasterizer math to
  glam risks changing the float results that feed `orient2d` setup. Recommendation: keep the
  hand-rolled scalar path as the golden reference and add a **`wide` f32x8 AoSoA transform
  kernel** as the vectorized path, rather than wholesale-adopting glam. Confidence: MEDIUM.
- **Cache/bandwidth.** A vertex buffer of 100k verts x 16 B (xyzw f32) = 1.6 MB > L2; the
  transform loop streams memory, so it is partly **bandwidth-bound** — expect SIMD speedup
  below the theoretical 8x. Confirm with `perf stat` (Q1) before promising numbers.

## Q5. target_feature + runtime detection on stable

- **Baseline.** x86-64 guarantees **SSE2**; `wide` always compiles to at least SSE2 with no
  runtime check needed. aarch64 always has **NEON**. wasm needs explicit `+simd128`.
  Source: wide README (https://github.com/Lokathor/wide), rustc wasm32 platform docs
  (https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html). Conf: HIGH.
- **Beyond baseline (AVX2):** two stable options —
  1. **Compile the whole binary for AVX2** via `RUSTFLAGS="-C target-feature=+avx2,+fma"`
     (or `target-cpu=native` locally). Simple, but the binary then *requires* AVX2 → not
     distributable to old CPUs. Source: alexheretic "Getting rustc to use AVX2"
     (https://alexheretic.github.io/posts/auto-avx2/). Confidence: HIGH.
  2. **Runtime dispatch**: `is_x86_feature_detected!("avx2")` + `#[target_feature(enable=
     "avx2")]` (unsafe fn), or the **`multiversion`** crate (safe macro that clones the fn
     per feature set and dispatches at runtime), or **`pulp`** (generic-over-ISA kernel,
     built-in multiversioning, powers `faer`; limited to native SIMD width and NEON/AVX2/
     AVX512). Sources: docs.rs/pulp, docs.rs/multiversion, Shnatsel "State of SIMD in Rust
     2025" (https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d).
     Confidence: MEDIUM-HIGH.
- **For this project:** `wide` over a fixed `f32x8` already covers SSE2 (split into 2x
  SSE), AVX2 (1 register) and NEON/wasm transparently at compile time. If a runtime AVX2
  uplift is wanted on a stock SSE2 binary, wrap the `wide` kernel in `multiversion`.
  Recommendation: ship `wide`-baseline first; add `multiversion` only if profiling shows the
  AVX2 path is worth a second codegen.

## Q7. wasm-simd128 for tte-wasm

- `wide` lowers its types to wasm `v128` SIMD **automatically** when built with
  `-C target-feature=+simd128` (it special-cases wasm32 internally; otherwise falls back to
  scalar `[f32; N]`). No code change needed in the kernel. Source: wide README. Conf: HIGH.
- Browser support for wasm SIMD is universal in modern browsers (Chrome 91+, Firefox 89+,
  Safari 16.4+, Edge 91+). Source: testmuai wasm-simd hub. Confidence: HIGH.
- **Determinism caveat for wasm:** the *standard* wasm SIMD `f32x4.mul`/`add` are
  IEEE-deterministic; only the **relaxed-simd** `fma`/`madd` ops are nondeterministic across
  engines. As long as the kernel avoids `mul_add`/relaxed-fma, wasm output matches native
  (modulo the FMA rule in Q4). Source: WebAssembly/relaxed-simd Overview
  (https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md).
  Confidence: HIGH.

(Q4 expansion on reductions/denormals/x87, Q6 speedup numbers, and the
Recommendation + NFR sections still pending.)
