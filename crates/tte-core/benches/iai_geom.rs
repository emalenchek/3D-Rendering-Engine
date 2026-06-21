//! FR-7.5 / NFR-16: a deterministic instruction-count gate for the render
//! pipeline, via iai-callgrind (callgrind). Unlike the wall-clock criterion
//! benches, callgrind instruction counts are exact and reproducible, so they
//! make a stable CI regression gate — e.g. the scalar path must not silently
//! get heavier.
//!
//! Run with an explicit feature config (scalar is the most stable):
//! ```sh
//! cargo bench -p tte-core --no-default-features --bench iai_geom   # scalar
//! cargo bench -p tte-core --no-default-features --features simd --bench iai_geom
//! ```
//! Requires `valgrind` and a matching `iai-callgrind-runner` on PATH.

use std::hint::black_box;

use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use tte_core::{Camera, Framebuffer, Mat4, Mesh, ShadeOptions, primitives, render_solid};

/// Build the workload outside the measured region: a ~2.3k-triangle sphere and a
/// rotation. Big enough to exercise the geometry + raster stages, small enough to
/// stay quick under callgrind's ~50× slowdown.
fn scene() -> (Mesh, Mat4) {
    let model = Mat4::rotation_y(0.6) * Mat4::rotation_x(0.4);
    (primitives::sphere(24, 48), model)
}

#[library_benchmark]
#[bench::sphere_2k(setup = scene)]
fn render_frame((mesh, model): (Mesh, Mat4)) -> Framebuffer {
    let camera = Camera::default();
    black_box(render_solid(
        black_box(&mesh),
        black_box(model),
        &camera,
        200,
        120,
        ShadeOptions::default(),
    ))
}

library_benchmark_group!(name = geometry; benchmarks = render_frame);
main!(library_benchmark_groups = geometry);
