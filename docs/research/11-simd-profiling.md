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

(Q3, Q5, Q6, Q7 and the Recommendation/NFR sections pending.)
