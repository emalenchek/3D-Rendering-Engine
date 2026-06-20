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

---

## Q4 (expanded). Determinism hazards & the byte-identical question

Four hazards that can make SIMD != scalar bit-for-bit:

1. **FMA contraction (the main one).** `mul_add`/`simd_fma` -> hardware FMA (single
   rounding) where available, plain mul/add elsewhere -> *different result on AVX2-FMA vs
   SSE2-only / different NEON parts*. **Verified by two independent primary sources** that
   `mul_add` lowers to `llvm.fma` (always-fused, deterministic-per-op) while Rust does NOT
   auto-contract `a*b + c` (it uses `llvm.fmuladd`/relaxed only for the explicit
   nondeterministic path, and never enables `-ffp-contract=fast` on `*`/`+`). Sources:
   rust-lang/libs-team #712 (https://github.com/rust-lang/libs-team/issues/712); RFC 3514
   float-semantics; rust-lang/portable-simd #102. **Mitigation: never call `mul_add` in the
   transform kernel** — write the matrix multiply as explicit `*` then `+`. Confidence: HIGH.
2. **Reduction/operation ordering.** Float `+`/`*` are not associative, so a tree/SIMD-lane
   reduction can differ from a left-to-right scalar sum. For Mat4xVec4 each output component
   is a *fixed* sum of exactly 4 products — if the SIMD kernel performs the additions in the
   **same order** as the scalar code (e.g. `((m0*x + m1*y) + m2*z) + m3*w`), results are
   identical. The hazard only appears in horizontal reductions/dot-products with a different
   accumulation order. **Mitigation: match the scalar add order in the kernel.** Source:
   Gaffer-On-Games "Floating Point Determinism"; general IEEE-754 non-associativity. Conf: HIGH.
3. **x87 vs SSE2.** Only a problem on **32-bit x86 without SSE2** (80-bit excess precision,
   inconsistent rounding). All 64-bit x86 and aarch64/wasm use IEEE single-precision SSE2/
   NEON/v128. `wide` guarantees >= SSE2 on x86. So for the targets that matter this is a
   non-issue; just don't build a no-SSE2 i686 target. Source: rust-lang/rust #114479,
   Intel FP docs. Confidence: HIGH.
4. **Denormals / FTZ-DAZ.** Differs only if MXCSR flush-to-zero is toggled. **Rust does not
   change MXCSR by default**, and `wide` does not enable FTZ — so denormals behave the same
   in scalar and SIMD. Avoid third-party code that flips FTZ. Source: Intel denorm docs,
   rust-lang/rust discussions. Confidence: HIGH.

**Verdict on byte-identical:** YES, byte-identical scalar==SIMD is realistic for f32
Mat4xVec4 across SSE2/AVX2/NEON/wasm **iff** the kernel (a) never uses `mul_add`/FMA, (b)
uses the same operation/accumulation order as the scalar reference, (c) targets are SSE2+
(no x87). The basic ops `+ - * /` are IEEE-754 correctly-rounded and identical across these
ISAs. The existing golden-frame + parity test then keeps it honest. If at some future point
an FMA fast-path is wanted for speed, switch the parity test to an explicit small-ULP
tolerance and document it — but that is **not** needed for v2.1.

## Q6. Realistic speedup & NFR calibration

- The geometry stage is partly **memory-bandwidth-bound** (100k verts stream through L2/L3),
  so SIMD will land well under the 8x (f32x8) compute peak. Literature: AoS->SoA gives
  ~2-4x at small/medium sizes, tapering as the set exceeds cache. Source: HAL data-layout
  paper. Confidence: MEDIUM.
- Concrete Rust math data (mathbench): **horizontal** SIMD (glam) is ~1.5-2.1x over
  nalgebra-scalar and ~2.7-3.8x over cgmath for Mat4 transform-point/vector. **Vertical**
  AoSoA SIMD (ultraviolet `wide` f32x4) shows ~**1.2-1.7x throughput** over scalar in the
  batched bench. So for *this* transform-throughput workload a realistic, honest target is
  **~1.5x** end-to-end on the geometry stage, not 2x+. Source:
  https://github.com/bitshifter/mathbench-rs. Confidence: MEDIUM-HIGH.
- Old fill-stage NFR (>=2x) does NOT transfer: the frame is setup/geometry-bound here, and
  the transform is bandwidth-limited, so a >=2x geometry claim would be dishonest.

---

## Recommended approach for v2.1

1. **Confirm the hot stage first (gate the whole effort).** Run **samply** (or cargo
   flamegraph) on the 100k-tri @ 400x200 scene with `CARGO_PROFILE_RELEASE_DEBUG=true` +
   `-C force-frame-pointers=yes`. Verify Mat4xVec4 transform + edge-function setup dominate
   over the fill loop. Run `perf stat` to confirm whether the transform is compute- or
   bandwidth-bound. If the profile does NOT show geometry dominance, stop and re-scope.
2. **SIMD crate: `wide`** (stable, fixed lane widths, SSE2/AVX2/NEON/wasm-simd128 with a
   scalar fallback, no auto-FMA). Use **`f32x8`** for the transform kernel (degrades cleanly
   to 2x SSE on baseline, 1x AVX2 register, NEON pairs, wasm). std::simd is still nightly →
   excluded. Defer `pulp`/`multiversion` unless profiling shows a runtime-AVX2 uplift is
   worth a second codegen on an SSE2-baseline binary.
3. **Vectorize the geometry stage first** (the proven hot path): convert vertex positions to
   **SoA/AoSoA**, batch the Mat4xVec4 transform 8 vertices at a time with `wide::f32x8`,
   broadcasting matrix elements. Keep the edge-function/`orient2d` integer setup exact; if
   profiling shows setup is also hot, vectorize the float pre-transform feeding it, not the
   i64 exact math. Do NOT wholesale-adopt glam (horizontal-only; risks perturbing the float
   inputs to the exact rasterizer).
4. **Determinism strategy: keep BYTE-IDENTICAL** (no tolerance). Achievable because (a) the
   kernel forbids `mul_add`/FMA, (b) it replays the scalar add order, (c) targets are SSE2+.
   Guard with the existing golden frames + parity test (now also run the parity test on the
   wasm build and, if CI allows, an aarch64 runner). Rationale: the project's identity is
   "exact/deterministic"; a documented-tolerance path is strictly worse here and unnecessary
   for an f32 affine transform.
5. **CI cost tracking:** add **iai-callgrind** benches per stage (transform / setup / fill)
   for deterministic instruction-count regression gating; keep **criterion** for the
   wall-clock speedup numbers that back the NFRs.

## Proposed NFRs (v2.1.0)

- **NFR-1 (speedup):** geometry/transform stage **>= 1.5x** faster than scalar at 100k tris
  @ 400x200, measured by criterion wall-clock on the project's x86-64 AVX2 CI host.
  (Stretch: >= 2x if `perf stat` shows the stage is compute- rather than bandwidth-bound.)
- **NFR-2 (parity):** SIMD output **byte-identical** to scalar — existing golden frames +
  parity test pass unchanged on x86-64 (SSE2 baseline AND AVX2), and on the wasm-simd128
  build; ideally also on aarch64 NEON in CI.
- **NFR-3 (stable toolchain):** builds and passes on stable Rust, no nightly features.
- **NFR-4 (portability/fallback):** correct results with `wide` scalar fallback (no SIMD
  target-feature), on SSE2 baseline, AVX2, NEON, and wasm `+simd128`.
- **NFR-5 (no regression):** iai-callgrind per-stage instruction counts do not regress for
  the scalar path; the rayon `parallel` parity test still proves byte-identity.

---

## Source list (confidence)

- Rust Performance Book — Profiling — HIGH — https://nnethercote.github.io/perf-book/profiling.html
- flamegraph-rs — HIGH — https://github.com/flamegraph-rs/flamegraph
- samply — HIGH — https://github.com/mstange/samply
- iai-callgrind / gungraun — HIGH — https://github.com/iai-callgrind/iai-callgrind , https://lib.rs/crates/iai-callgrind
- wide (README + docs) — HIGH — https://github.com/Lokathor/wide , https://docs.rs/wide
- rust-lang/portable-simd (#102, std::simd nightly) — HIGH — https://github.com/rust-lang/portable-simd
- rust-lang/libs-team #712 (mul_add = llvm.fma, deterministic) — HIGH — https://github.com/rust-lang/libs-team/issues/712
- RFC 3514 float-semantics — HIGH — https://rust-lang.github.io/rfcs/3514-float-semantics.html
- siboehm Inlining/FMA/FP-consistency — MEDIUM — https://siboehm.com/articles/23/Inlining-FMA-FP-consistency
- Gaffer On Games — Floating Point Determinism — MEDIUM — https://gafferongames.com/post/floating_point_determinism/
- pulp — MEDIUM — https://docs.rs/pulp
- multiversion (calebzulawski) — HIGH — https://crates.io/crates/multiversion , https://docs.rs/multiversion
- Shnatsel "State of SIMD in Rust 2025" — MEDIUM — https://shnatsel.medium.com/the-state-of-simd-in-rust-in-2025-32c263e5f53d
- alexheretic Getting rustc to use AVX2 — HIGH — https://alexheretic.github.io/posts/auto-avx2/
- glam-rs — HIGH — https://github.com/bitshifter/glam-rs
- mathbench-rs (concrete Mat4 transform numbers) — HIGH — https://github.com/bitshifter/mathbench-rs
- rustsim AoSoA SIMD blog — HIGH — https://www.rustsim.org/blog/2020/03/23/simd-aosoa-in-nalgebra/
- HAL data-layout/SIMD abstraction — MEDIUM — https://hal.science/hal-01915529/document
- WebAssembly/relaxed-simd Overview (relaxed-fma nondeterminism) — HIGH — https://github.com/WebAssembly/relaxed-simd/blob/main/proposals/relaxed-simd/Overview.md
- rustc wasm32 platform support — HIGH — https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html
- rust-lang/rust x87/SSE2 determinism (#114479) — HIGH
- testmuai wasm SIMD browser support — MEDIUM — https://www.testmuai.com/learning-hub/wasm-simd-browser-support/

