//! FR-7.1 profiling aid: render N frames of the `solid_100k_tri @ 400×200`
//! workload so a profiler can attribute cost to functions. Pairs with the
//! `prof_stage_split` wall-clock test. Run under callgrind for a deterministic,
//! instruction-level breakdown:
//!
//! ```sh
//! cargo build --release -p tte-core --no-default-features --example profile_frame
//! valgrind --tool=callgrind --callgrind-out-file=cg.out \
//!     target/release/examples/profile_frame 1
//! callgrind_annotate cg.out | head -40
//! ```
//!
//! The argument is the frame count (default 1; keep it small under callgrind).

use tte_core::{Camera, Mat4, ShadeOptions, primitives, render_solid};

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Matches benches/raster.rs `solid_100k_tri`: 2·160·320 = 102_400 triangles.
    let sphere = primitives::sphere(160, 320);
    let model = Mat4::rotation_y(0.6) * Mat4::rotation_x(0.4);
    let camera = Camera::default();
    let opts = ShadeOptions::default();

    let mut acc = 0u64;
    for _ in 0..frames {
        let fb = render_solid(&sphere, model, &camera, 400, 200, opts);
        // Touch the result so nothing is optimized away.
        acc = acc.wrapping_add(fb.width() as u64 + fb.height() as u64);
    }
    std::hint::black_box(acc);
}
