//! Wireframe rendering: mesh edges → projected segments → cell buffer (FR-1.3).
//!
//! Pipeline stages (project brief D3; clip-space rules per
//! docs/research/01-engine-architectures.md Q4):
//!   model → view → projection → near-plane cull → perspective divide
//!   → viewport → Bresenham.

use crate::camera::Camera;
use crate::cell::CellBuffer;
use crate::math::{Mat4, Vec4};
use crate::mesh::Mesh;
use crate::raster::{self, STROKE};

/// Render `mesh` (transformed by `model`) seen from `camera` into a fresh
/// `width`×`height` cell buffer. Pure: identical inputs → identical frames (NFR-1).
pub fn render_wireframe(
    mesh: &Mesh,
    model: Mat4,
    camera: &Camera,
    width: u16,
    height: u16,
) -> CellBuffer {
    let mut buf = CellBuffer::new(width, height);
    let mvp = camera.projection_matrix(width, height) * camera.view_matrix() * model;

    // Transform every vertex to clip space once; edges then index the result.
    let clip: Vec<Vec4> = mesh
        .positions
        .iter()
        .map(|&p| mvp * p.extend(1.0))
        .collect();

    for (a, b) in mesh.edges() {
        let (ca, cb) = (clip[a as usize], clip[b as usize]);
        // Near-plane cull (FR-1.3): drop edges with any endpoint at or behind
        // the near plane. In clip space the near plane is z = -w; points with
        // w <= 0 are behind the camera. True clipping (splitting the edge at
        // the plane) is Phase 5 hardening — see FR-1.3 note in the spec.
        if ca.w <= 0.0 || cb.w <= 0.0 || ca.z < -ca.w || cb.z < -cb.w {
            continue;
        }
        let (x0, y0) = to_cell(ca, width, height);
        let (x1, y1) = to_cell(cb, width, height);
        raster::draw_line(&mut buf, x0, y0, x1, y1, STROKE);
    }
    buf
}

/// Perspective divide + viewport transform: clip space → integer cell coords.
/// NDC y points up, cell y points down, hence the flip.
fn to_cell(clip: Vec4, width: u16, height: u16) -> (i32, i32) {
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    let x = (ndc_x * 0.5 + 0.5) * f32::from(width.saturating_sub(1));
    let y = (0.5 - ndc_y * 0.5) * f32::from(height.saturating_sub(1));
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    fn segment_mesh(a: Vec3, b: Vec3) -> Mesh {
        // Degenerate triangle (a, b, b) yields the single edge (a, b).
        Mesh {
            positions: vec![a, b],
            normals: vec![Vec3::Z; 2],
            triangles: vec![[0, 1, 1]],
        }
    }

    #[test]
    fn fr1_3_origin_projects_to_buffer_center() {
        let mesh = segment_mesh(Vec3::ZERO, Vec3::ZERO);
        let camera = Camera {
            eye: Vec3::new(0.0, 0.0, 5.0),
            ..Camera::default()
        };
        let buf = render_wireframe(&mesh, Mat4::IDENTITY, &camera, 41, 21);
        assert_eq!(buf.get(20, 10), Some(STROKE), "frame:\n{buf}");
    }

    #[test]
    fn fr1_3_edge_behind_camera_is_culled() {
        // Both endpoints behind the eye → nothing drawn.
        let mesh = segment_mesh(Vec3::new(0.0, 0.0, 10.0), Vec3::new(1.0, 0.0, 10.0));
        let camera = Camera {
            eye: Vec3::new(0.0, 0.0, 5.0),
            ..Camera::default()
        };
        let buf = render_wireframe(&mesh, Mat4::IDENTITY, &camera, 20, 10);
        assert_eq!(buf.to_string().trim(), "", "expected empty frame");
    }

    #[test]
    fn fr1_3_edge_crossing_near_plane_is_culled_not_distorted() {
        // One endpoint in front, one behind the eye: must be culled entirely
        // (drawing it would wrap through infinity — the classic artifact).
        let mesh = segment_mesh(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 10.0));
        let camera = Camera {
            eye: Vec3::new(0.0, 0.0, 5.0),
            ..Camera::default()
        };
        let buf = render_wireframe(&mesh, Mat4::IDENTITY, &camera, 20, 10);
        assert_eq!(buf.to_string().trim(), "", "expected empty frame");
    }

    #[test]
    fn nfr1_double_render_is_identical() {
        let mesh = segment_mesh(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 0.0));
        let camera = Camera::default();
        let model = Mat4::rotation_y(0.7) * Mat4::rotation_x(0.3);
        let a = render_wireframe(&mesh, model, &camera, 80, 24);
        let b = render_wireframe(&mesh, model, &camera, 80, 24);
        assert_eq!(a, b);
    }
}
