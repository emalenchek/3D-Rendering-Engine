# Research 11b — FR-7.1 Profile Gate Results (v2.1.0 SIMD)

Date: 2026-06. Decision gate for Phase 7 (W1): **measure before vectorizing.**
Workload: `solid_100k_tri @ 400×200` (102,400-triangle UV sphere, the bench in
`crates/tte-core/benches/raster.rs`), **scalar single-thread** (`--no-default-features`),
release. Host: x86-64, AVX2+FMA+SSE2, rustc 1.94.1.

Two independent measurements, both reproducible from the repo:

1. **Wall-clock stage split** — `crates/tte-core/src/solid.rs::tests::prof_stage_split`
   (`#[ignore]`d): `cargo test --release -p tte-core --no-default-features prof_stage_split -- --ignored --nocapture`.
2. **Instruction-level attribution** — `crates/tte-core/examples/profile_frame.rs` under
   callgrind: `valgrind --tool=callgrind … target/release/examples/profile_frame 1` then
   `callgrind_annotate`.

## Result 1 — stage split (wall-clock, 30 iters)

```
triangles: 102400 (102400 drawn after cull); vertices: 51520
geometry  (prepare_mesh): 5403.9 us/frame  (60.9%)
  └ vertex transforms:     872.4 us/frame  ( 9.8% of frame, 16.1% of geometry)
raster    (rasterize):    3464.3 us/frame  (39.1%)
total:                    8868.2 us/frame
```

## Result 2 — instruction attribution (callgrind, 1 frame, 112.4M Ir)

| Ir share | Function | Belongs to |
|---:|---|---|
| 35.8% | per-triangle closure (`prepare_mesh`'s `filter_map` body) | geometry: cull, `face_normal` (cross+`sqrt`), shading dots, `screen_vertex` (÷w) |
| 20.9% | `triangle::fill_triangle` | rasterizer |
| 13.1% | `round` (libm) | rasterizer — `snap()` (6 `f64::round` per triangle) |
| 6.6% | `floorf` (libm) | geometry — `y_min = p.y.floor()` |
| 6.0% | `ceilf` (libm) | geometry — `y_max = p.y.ceil()` |
| 4.9% | `Vec::from_iter` | geometry — the three `map_collect` allocations |
| **3.7%** | **`Mat4 × Vec4`** | **geometry — the vertex transform (FR-7.2's target)** |

## Verdict

**The geometry stage dominates (61% wall) — the stage-level W1 hypothesis holds.**
**But the originally-scoped kernel target is mis-aimed.** The per-vertex `Mat4 × Vec4`
transform that FR-7.2 set out to vectorize "8 vertices at a time" is only **3.7% of
instructions / 9.8% of frame time / 16% of the geometry stage**. By Amdahl, vectorizing
*only* the vertex transforms caps the frame at **≈1.11×** and the geometry **stage** at
**≈1.19×** — both below NFR-13's **≥1.5×** (NFR-13 is defined on the geometry/transform
*stage*).

The cost actually lives in the **per-triangle** work, which is scalar and divide/transcendental-heavy:

- the projection/shade closure (35.8%): three perspective divides (`1/clip.w`), a
  `face_normal` (`cross` + `sqrt` normalize), and shading dot products — **per triangle**;
- the libm rounding: `floor`/`ceil` for the row span (12.6%, geometry) and `round` in
  `snap()` (13.1%, rasterizer).

This is exactly the outcome the gate exists to catch (Risk #1): the prior insight "transforms
dominate" was a hypothesis; measured, the transforms are a rounding error and the
**per-triangle projection/shading** is the hot stage.

## Recommendation — re-scope FR-7.2

Keep the geometry stage as the target (it dominates) but **widen the kernel from "vertex
`Mat4 × Vec4` only" to the whole per-triangle geometry stage, laid out AoSoA and processed
8 triangles per `f32x8` lane**:

- batch the three perspective divides, `face_normal` (cross + reciprocal-sqrt), the shading
  dot/clamp, and the `screen_vertex` viewport map across 8 triangles;
- fold the cheap vertex `Mat4 × Vec4` transform (3.7%) and the `floor`/`ceil` row-span
  (12.6%, via `f32x8` floor/ceil) into the same pass;
- **leave the rasterizer (`fill_triangle`, integer `orient2d` coverage) untouched** — W3/W4
  byte-identical coverage is preserved.

This attacks ~52% of instructions (the whole geometry stage) instead of 3.7%, so **≥1.5× on
the geometry stage (NFR-13) is reachable**. Byte-identical parity (W4) is still achievable:
`wide` division and `sqrt` are exact IEEE-754, `f32x8` floor/ceil match libm, and the kernel
forbids `mul_add`/FMA and replays the scalar add order — the same three conditions W4 already
relies on.

**Cost note:** this is a larger kernel than the narrow transform batch (more surface for the
parity test to guard), but it is the only version of Phase 7 that meets NFR-13. If a smaller
effort is preferred, the honest fallback is to drop NFR-13 and ship the demo-only v2.1 (Phase
8, already merged), deferring a geometry-stage SIMD refactor.

## Reproduce

```sh
# stage split
cargo test --release -p tte-core --no-default-features prof_stage_split -- --ignored --nocapture
# instruction attribution
cargo build --release -p tte-core --no-default-features --example profile_frame
valgrind --tool=callgrind --callgrind-out-file=cg.out target/release/examples/profile_frame 1
callgrind_annotate cg.out | head -40
```
