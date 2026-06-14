//! Performance benchmarks (NFR-3): headless wireframe render at terminal
//! resolution. Run with `cargo bench`; reports in target/criterion/.
//!
//! NFR-3 target: ≤5 ms/frame for a ≤1k-triangle model at 200×50. Tracked as a
//! trend (criterion compares against the previous run), not a hard CI gate.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tte_core::{Camera, Mat4, Mesh, ShadeOptions, Vec3, render_solid, render_wireframe};

/// Procedural UV-sphere: `rings`×`segments` quads → 2·rings·segments triangles.
/// (16×32 ≈ 1024 triangles — the NFR-3 reference size.)
fn uv_sphere(rings: u32, segments: u32) -> Mesh {
    let mut positions = Vec::new();
    for r in 0..=rings {
        let phi = std::f32::consts::PI * r as f32 / rings as f32;
        for s in 0..segments {
            let theta = std::f32::consts::TAU * s as f32 / segments as f32;
            positions.push(Vec3::new(
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            ));
        }
    }
    let mut triangles = Vec::new();
    let idx = |r: u32, s: u32| r * segments + (s % segments);
    for r in 0..rings {
        for s in 0..segments {
            let (a, b) = (idx(r, s), idx(r, s + 1));
            let (c, d) = (idx(r + 1, s), idx(r + 1, s + 1));
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    let normals = positions.clone(); // unit sphere: normal == position
    Mesh {
        positions,
        normals,
        triangles,
    }
}

fn bench_wireframe(c: &mut Criterion) {
    let camera = Camera::default();
    let model = Mat4::rotation_y(0.6) * Mat4::rotation_x(0.4);

    let cube = tte_core::parse_obj(include_str!("../tests/data/cube.obj")).unwrap();
    c.bench_function("wireframe_cube_80x24", |b| {
        b.iter(|| render_wireframe(black_box(&cube), model, &camera, 80, 24))
    });

    let sphere = uv_sphere(16, 32);
    c.bench_function("wireframe_1k_tri_sphere_200x50", |b| {
        b.iter(|| render_wireframe(black_box(&sphere), model, &camera, 200, 50))
    });
}

/// NFR-3 (ext): solid shaded render of a ~1k-triangle model at 200×50.
fn bench_solid(c: &mut Criterion) {
    let camera = Camera::default();
    let model = Mat4::rotation_y(0.6) * Mat4::rotation_x(0.4);
    let sphere = uv_sphere(16, 32);
    let opts = ShadeOptions::default();
    c.bench_function("solid_1k_tri_sphere_200x50", |b| {
        b.iter(|| render_solid(black_box(&sphere), model, &camera, 200, 50, opts))
    });
}

/// FR-6.4 / NFR-10: the large-mesh, high-resolution workload the parallel and
/// SIMD speedups are measured against (~100k triangles at 400×200). Compare
/// `cargo bench` (parallel feature on) vs `cargo bench --no-default-features`
/// (scalar) to read the NFR-10 (≥3×) speedup; small working sets don't show it,
/// which is why the target is defined on this bench (report 09).
fn bench_solid_large(c: &mut Criterion) {
    let camera = Camera::default();
    let model = Mat4::rotation_y(0.6) * Mat4::rotation_x(0.4);
    let sphere = uv_sphere(160, 320); // 2·160·320 = 102_400 triangles
    let opts = ShadeOptions::default();
    let mut group = c.benchmark_group("solid_100k_tri");
    group.sample_size(20); // a heavy frame; fewer samples keeps the run quick
    group.bench_function("sphere_400x200", |b| {
        b.iter(|| render_solid(black_box(&sphere), model, &camera, 400, 200, opts))
    });
    group.finish();
}

criterion_group!(benches, bench_wireframe, bench_solid, bench_solid_large);
criterion_main!(benches);
