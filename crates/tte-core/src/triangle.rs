//! Edge-function triangle rasterization with a z-buffer (FR-2.3, FR-2.4, FR-6.0).
//!
//! Coverage uses **integer edge functions** on sub-pixel-snapped coordinates
//! (Giesen-style fixed point) with a top-left fill rule. Two payoffs, per
//! research report 09:
//! - **Exact / deterministic**: integer `orient2d` has no rounding, so a SIMD or
//!   multi-threaded path can reproduce the scalar coverage bit-for-bit (FR-6.3).
//! - **Watertight**: the top-left rule gives every interior pixel exactly one
//!   covering triangle — no seams, no double-draw (replaces the old inclusive
//!   `barycentric ≥ 0` workaround).
//!
//! Attribute interpolation (depth, intensity) stays floating point but in a
//! fixed operation order (no FMA contraction) so it is reproducible too.

use crate::color::Rgb;
use crate::framebuffer::RasterTarget;

/// Sub-pixel precision: coordinates are snapped to a 1/16-pixel grid before the
/// integer edge functions are evaluated. 4 bits is plenty at terminal/browser
/// cell resolutions and keeps the `i64` edge products comfortably in range.
const SUBPIXEL_BITS: u32 = 4;
const SUBPIXEL: i64 = 1 << SUBPIXEL_BITS;

/// A triangle vertex in screen space, carrying the attributes interpolated
/// across the face: sub-pixel position, NDC depth, and shading intensity.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub intensity: f32,
}

/// A vertex snapped to the sub-pixel integer grid (position only).
#[derive(Clone, Copy)]
struct Snapped {
    x: i64,
    y: i64,
}

fn snap(v: &Vertex) -> Snapped {
    Snapped {
        x: (f64::from(v.x) * SUBPIXEL as f64).round() as i64,
        y: (f64::from(v.y) * SUBPIXEL as f64).round() as i64,
    }
}

/// Twice the signed area of `(a, b, c)` in sub-pixel integer space — exact, so
/// coverage is deterministic. Sign encodes winding.
fn orient2d(a: Snapped, b: Snapped, c: Snapped) -> i64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Top-left rule: a pixel lying exactly on an edge belongs to the triangle only
/// if that edge is a left edge (points upward, dy < 0) or a top edge (horizontal
/// and pointing left). Opposite windings on a shared edge make exactly one of
/// the two triangles include it.
fn is_top_left(s: Snapped, e: Snapped) -> bool {
    let (dx, dy) = (e.x - s.x, e.y - s.y);
    dy < 0 || (dy == 0 && dx < 0)
}

/// Rasterize `(v0, v1, v2)`, modulating `base_color` by the per-pixel
/// interpolated intensity and depth-testing each fragment.
///
/// `cull_back`: skip triangles whose screen-space winding faces away from the
/// camera. In our y-down screen space a front face (CCW in the y-up NDC the
/// projection produces) has **negative** signed area.
pub fn fill_triangle<T: RasterTarget>(
    fb: &mut T,
    v0: Vertex,
    v1: Vertex,
    v2: Vertex,
    base_color: Rgb,
    cull_back: bool,
) -> bool {
    let p0 = snap(&v0);
    let mut p1 = snap(&v1);
    let mut p2 = snap(&v2);
    let a0 = v0;
    let mut a1 = v1;
    let mut a2 = v2;

    let area = orient2d(p0, p1, p2);
    if area == 0 {
        return false; // degenerate / zero-area
    }
    let front_facing = area < 0;
    if cull_back && !front_facing {
        return false;
    }
    // Normalize to positive-area winding so the edge functions are ≥ 0 inside;
    // swapping two vertices carries their attributes along.
    let area = if area < 0 {
        std::mem::swap(&mut p1, &mut p2);
        std::mem::swap(&mut a1, &mut a2);
        -area
    } else {
        area
    };
    let area_f = area as f32;

    let (min_x, max_x, min_y, max_y) = bounds(fb.width(), fb.y_start(), fb.y_end(), [p0, p1, p2]);
    if min_x > max_x || min_y > max_y {
        return false;
    }

    // Top-left bias per edge: edge0 = p1→p2, edge1 = p2→p0, edge2 = p0→p1.
    let tl0 = is_top_left(p1, p2);
    let tl1 = is_top_left(p2, p0);
    let tl2 = is_top_left(p0, p1);

    // Per-pixel increments of each edge function (16 sub-pixels per pixel).
    let step_x = |s: Snapped, e: Snapped| -(e.y - s.y) * SUBPIXEL;
    let step_y = |s: Snapped, e: Snapped| (e.x - s.x) * SUBPIXEL;
    let (sx0, sy0) = (step_x(p1, p2), step_y(p1, p2));
    let (sx1, sy1) = (step_x(p2, p0), step_y(p2, p0));
    let (sx2, sy2) = (step_x(p0, p1), step_y(p0, p1));

    // Edge values at the center of the top-left pixel of the bounding box.
    let half = SUBPIXEL / 2;
    let origin = Snapped {
        x: i64::from(min_x) * SUBPIXEL + half,
        y: i64::from(min_y) * SUBPIXEL + half,
    };
    let mut w0_row = orient2d(p1, p2, origin);
    let mut w1_row = orient2d(p2, p0, origin);
    let mut w2_row = orient2d(p0, p1, origin);

    let mut drew = false;
    for y in min_y..=max_y {
        let (mut w0, mut w1, mut w2) = (w0_row, w1_row, w2_row);
        for x in min_x..=max_x {
            // Inside if every edge is positive, or zero on a top-left edge.
            let inside = (w0 > 0 || (w0 == 0 && tl0))
                && (w1 > 0 || (w1 == 0 && tl1))
                && (w2 > 0 || (w2 == 0 && tl2));
            if inside {
                let b0 = w0 as f32 / area_f;
                let b1 = w1 as f32 / area_f;
                let b2 = w2 as f32 / area_f;
                let depth = b0 * a0.depth + b1 * a1.depth + b2 * a2.depth;
                let intensity = b0 * a0.intensity + b1 * a1.intensity + b2 * a2.intensity;
                drew |= fb.plot(x, y, depth, base_color.scaled(intensity));
            }
            w0 += sx0;
            w1 += sx1;
            w2 += sx2;
        }
        w0_row += sy0;
        w1_row += sy1;
        w2_row += sy2;
    }
    drew
}

/// Integer pixel bounding box of the triangle, clamped to the target's columns
/// (`0..width`) and its owned row band (`y_start..y_end`) — so a parallel band
/// only iterates the rows it owns (FR-6.1).
fn bounds(width: u16, y_start: i32, y_end: i32, vs: [Snapped; 3]) -> (i32, i32, i32, i32) {
    let min = |sel: fn(Snapped) -> i64| vs.iter().map(|&v| sel(v)).min().unwrap();
    let max = |sel: fn(Snapped) -> i64| vs.iter().map(|&v| sel(v)).max().unwrap();
    // Floor to whole pixels via Euclidean division on the sub-pixel grid.
    let px = |sub: i64| sub.div_euclid(SUBPIXEL) as i32;
    (
        px(min(|v| v.x)).max(0),
        px(max(|v| v.x)).min(i32::from(width) - 1),
        px(min(|v| v.y)).max(y_start),
        px(max(|v| v.y)).min(y_end - 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;

    fn vert(x: f32, y: f32, depth: f32, intensity: f32) -> Vertex {
        Vertex {
            x,
            y,
            depth,
            intensity,
        }
    }

    /// A front-facing (CCW in y-up NDC → negative screen area) flat triangle.
    fn front_tri(depth: f32, intensity: f32) -> [Vertex; 3] {
        [
            vert(1.0, 8.0, depth, intensity),
            vert(8.0, 8.0, depth, intensity),
            vert(4.0, 1.0, depth, intensity),
        ]
    }

    fn count_filled(fb: &Framebuffer) -> usize {
        fb.rows()
            .flatten()
            .filter(|&&c| c != fb.background())
            .count()
    }

    #[test]
    fn fr2_3_fills_interior_pixels() {
        let mut fb = Framebuffer::new(10, 10);
        let [a, b, c] = front_tri(0.0, 1.0);
        assert!(fill_triangle(&mut fb, a, b, c, Rgb::WHITE, true));
        assert!(count_filled(&fb) > 10, "expected a filled area");
        assert_eq!(fb.color_at(4, 6), Some(Rgb::WHITE));
    }

    #[test]
    fn fr2_3_back_facing_triangle_is_culled() {
        let mut fb = Framebuffer::new(10, 10);
        let [a, b, c] = front_tri(0.0, 1.0);
        assert!(!fill_triangle(&mut fb, a, c, b, Rgb::WHITE, true));
        assert_eq!(count_filled(&fb), 0);
    }

    #[test]
    fn fr2_3_back_face_drawn_when_culling_disabled() {
        let mut fb = Framebuffer::new(10, 10);
        let [a, b, c] = front_tri(0.0, 1.0);
        assert!(fill_triangle(&mut fb, a, c, b, Rgb::WHITE, false));
        assert!(count_filled(&fb) > 10);
    }

    #[test]
    fn fr2_3_degenerate_triangle_draws_nothing() {
        let mut fb = Framebuffer::new(10, 10);
        let v = vert(2.0, 2.0, 0.0, 1.0);
        let collinear = vert(4.0, 4.0, 0.0, 1.0);
        assert!(!fill_triangle(
            &mut fb,
            v,
            collinear,
            vert(6.0, 6.0, 0.0, 1.0),
            Rgb::WHITE,
            false
        ));
    }

    #[test]
    fn fr2_3_two_triangles_sharing_an_edge_leave_no_gap() {
        let mut fb = Framebuffer::new(12, 12);
        let tl = vert(2.0, 2.0, 0.0, 1.0);
        let tr = vert(9.0, 2.0, 0.0, 1.0);
        let bl = vert(2.0, 9.0, 0.0, 1.0);
        let br = vert(9.0, 9.0, 0.0, 1.0);
        fill_triangle(&mut fb, tl, bl, tr, Rgb::WHITE, false);
        fill_triangle(&mut fb, tr, bl, br, Rgb::WHITE, false);
        for (x, y) in [(3, 3), (8, 8), (5, 5), (3, 8), (8, 3)] {
            assert_eq!(fb.color_at(x, y), Some(Rgb::WHITE), "gap at ({x},{y})");
        }
    }

    /// FR-6.0: the top-left rule covers a shared edge exactly once. Two tris
    /// meeting at a vertical seam must not double-write any column.
    #[test]
    fn fr6_0_shared_edge_is_covered_exactly_once() {
        // Count how many times each pixel is written using a counting buffer:
        // render each triangle into its own fb, then check no pixel is set in both.
        let tri = |verts: [Vertex; 3]| {
            let mut fb = Framebuffer::new(16, 16);
            fill_triangle(&mut fb, verts[0], verts[1], verts[2], Rgb::WHITE, false);
            fb
        };
        // Quad (2,2)-(12,12) split along the diagonal (2,2)->(12,12).
        let a = vert(2.0, 2.0, 0.0, 1.0);
        let b = vert(12.0, 2.0, 0.0, 1.0);
        let c = vert(2.0, 12.0, 0.0, 1.0);
        let d = vert(12.0, 12.0, 0.0, 1.0);
        let left = tri([a, c, d]);
        let right = tri([a, d, b]);
        let mut overlaps = 0;
        for y in 0..16 {
            for x in 0..16 {
                let lit = |fb: &Framebuffer| fb.color_at(x, y) == Some(Rgb::WHITE);
                if lit(&left) && lit(&right) {
                    overlaps += 1;
                }
            }
        }
        assert_eq!(
            overlaps, 0,
            "shared edge double-covered on {overlaps} pixels"
        );
    }

    #[test]
    fn fr2_4_nearer_triangle_occludes_farther_regardless_of_order() {
        let draw = |first_depth: f32, second_depth: f32| {
            let mut fb = Framebuffer::new(10, 10);
            let [a, b, c] = front_tri(first_depth, 1.0);
            fill_triangle(&mut fb, a, b, c, Rgb::WHITE, false);
            let [a, b, c] = front_tri(second_depth, 1.0);
            fill_triangle(&mut fb, a, b, c, Rgb::new(100, 100, 100), false);
            fb.color_at(4, 6).unwrap()
        };
        assert_eq!(draw(0.8, 0.2), Rgb::new(100, 100, 100), "second nearer");
        assert_eq!(draw(0.2, 0.8), Rgb::WHITE, "first nearer");
    }

    #[test]
    fn fr2_4_depth_interpolates_across_the_face() {
        let mut fb = Framebuffer::new(20, 20);
        let a = vert(2.0, 18.0, 0.0, 1.0);
        let b = vert(18.0, 18.0, 1.0, 1.0);
        let c = vert(10.0, 2.0, 0.5, 1.0);
        fill_triangle(&mut fb, a, b, c, Rgb::WHITE, false);
        let d = fb.depth_at(10, 15).unwrap();
        assert!(
            d > 0.0 && d < 1.0,
            "interpolated depth {d} not strictly interior"
        );
    }

    /// FR-6.3 precondition: integer coverage is fully deterministic.
    #[test]
    fn fr6_0_rasterization_is_deterministic() {
        let render = || {
            let mut fb = Framebuffer::new(40, 40);
            let [a, b, c] = front_tri(0.3, 0.8);
            fill_triangle(&mut fb, a, b, c, Rgb::new(200, 150, 100), false);
            fb.rows().flatten().copied().collect::<Vec<_>>()
        };
        assert_eq!(render(), render());
    }
}
