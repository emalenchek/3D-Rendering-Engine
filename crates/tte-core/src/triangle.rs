//! Edge-function triangle rasterization with a z-buffer (FR-2.3, FR-2.4).

use crate::color::Rgb;
use crate::framebuffer::Framebuffer;

/// A triangle vertex in screen space, carrying the attributes interpolated
/// across the face: sub-pixel position, NDC depth, and shading intensity.
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub intensity: f32,
}

/// Twice the signed area of triangle `(a, b, c)` in screen space (the standard
/// orientation predicate). Sign encodes winding; magnitude is 2·area.
fn orient2d(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
}

/// Rasterize `(v0, v1, v2)`, modulating `base_color` by the per-pixel
/// interpolated intensity and depth-testing each fragment (FR-2.4).
///
/// `cull_back`: when set, triangles whose screen-space winding faces away from
/// the camera are skipped. In our y-down screen space a front face (CCW in the
/// y-up NDC the projection produces) has **negative** signed area.
///
/// Coverage is inclusive (`barycentric ≥ 0`), so shared edges never leave gaps;
/// exact single-coverage (top-left rule) is a deferred refinement, harmless
/// here because the z-buffer makes seam double-writes idempotent.
pub fn fill_triangle(
    fb: &mut Framebuffer,
    v0: Vertex,
    v1: Vertex,
    v2: Vertex,
    base_color: Rgb,
    cull_back: bool,
) -> bool {
    let area = orient2d(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
    if area.abs() < 1e-6 {
        return false; // degenerate / zero-area
    }
    let front_facing = area < 0.0;
    if cull_back && !front_facing {
        return false;
    }

    // Normalize to a positive-area winding so barycentric weights are ≥ 0
    // inside; swapping two vertices carries their attributes along.
    let (v1, v2, area) = if area < 0.0 {
        (v2, v1, -area)
    } else {
        (v1, v2, area)
    };

    let (min_x, max_x, min_y, max_y) = bounds(fb, &[v0, v1, v2]);
    let inv_area = 1.0 / area;
    let mut drew = false;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Barycentric weight of each vertex = normalized opposite-edge area.
            let b0 = orient2d(v1.x, v1.y, v2.x, v2.y, px, py) * inv_area;
            let b1 = orient2d(v2.x, v2.y, v0.x, v0.y, px, py) * inv_area;
            let b2 = orient2d(v0.x, v0.y, v1.x, v1.y, px, py) * inv_area;
            if b0 < 0.0 || b1 < 0.0 || b2 < 0.0 {
                continue;
            }
            let depth = b0 * v0.depth + b1 * v1.depth + b2 * v2.depth;
            let intensity = b0 * v0.intensity + b1 * v1.intensity + b2 * v2.intensity;
            drew |= fb.plot(x, y, depth, base_color.scaled(intensity));
        }
    }
    drew
}

/// Integer bounding box of the triangle, clamped to the framebuffer.
fn bounds(fb: &Framebuffer, vs: &[Vertex; 3]) -> (i32, i32, i32, i32) {
    let xs = vs.iter().map(|v| v.x);
    let ys = vs.iter().map(|v| v.y);
    let min_x = xs.clone().fold(f32::INFINITY, f32::min).floor() as i32;
    let max_x = xs.fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
    let min_y = ys.clone().fold(f32::INFINITY, f32::min).floor() as i32;
    let max_y = ys.fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
    (
        min_x.max(0),
        max_x.min(i32::from(fb.width()) - 1),
        min_y.max(0),
        max_y.min(i32::from(fb.height()) - 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // A point clearly inside should be the base color at full intensity.
        assert_eq!(fb.color_at(4, 6), Some(Rgb::WHITE));
    }

    #[test]
    fn fr2_3_back_facing_triangle_is_culled() {
        let mut fb = Framebuffer::new(10, 10);
        // Reverse winding of front_tri → back-facing.
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
        // Quad split into two tris: every pixel of the rectangle is covered.
        let mut fb = Framebuffer::new(12, 12);
        let tl = vert(2.0, 2.0, 0.0, 1.0);
        let tr = vert(9.0, 2.0, 0.0, 1.0);
        let bl = vert(2.0, 9.0, 0.0, 1.0);
        let br = vert(9.0, 9.0, 0.0, 1.0);
        fill_triangle(&mut fb, tl, bl, tr, Rgb::WHITE, false);
        fill_triangle(&mut fb, tr, bl, br, Rgb::WHITE, false);
        // Interior sample on the shared diagonal region is covered.
        for (x, y) in [(3, 3), (8, 8), (5, 5), (3, 8), (8, 3)] {
            assert_eq!(fb.color_at(x, y), Some(Rgb::WHITE), "gap at ({x},{y})");
        }
    }

    #[test]
    fn fr2_4_nearer_triangle_occludes_farther_regardless_of_order() {
        // Full intensity on both so each pixel's color is exactly the winning
        // triangle's base color, isolating the depth test from shading.
        let draw = |first_depth: f32, second_depth: f32| {
            let mut fb = Framebuffer::new(10, 10);
            let [a, b, c] = front_tri(first_depth, 1.0);
            fill_triangle(&mut fb, a, b, c, Rgb::WHITE, false);
            let [a, b, c] = front_tri(second_depth, 1.0);
            fill_triangle(&mut fb, a, b, c, Rgb::new(100, 100, 100), false);
            fb.color_at(4, 6).unwrap()
        };
        // Whichever triangle is nearer (smaller depth) wins both ways.
        assert_eq!(draw(0.8, 0.2), Rgb::new(100, 100, 100), "second nearer");
        assert_eq!(draw(0.2, 0.8), Rgb::WHITE, "first nearer");
    }

    #[test]
    fn fr2_4_depth_interpolates_across_the_face() {
        let mut fb = Framebuffer::new(20, 20);
        // Depth ramps 0 → 1 along the triangle; interior depths land between.
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
}
