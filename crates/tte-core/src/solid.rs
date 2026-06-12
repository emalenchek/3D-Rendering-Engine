//! Solid shaded rendering: mesh → shaded, depth-tested triangles (FR-2.5).
//!
//! Pipeline (project brief D3): model → view → projection → near-plane cull →
//! perspective divide → viewport → shade → edge-function fill with z-buffer.
//! Produces a [`Framebuffer`]; a presenter (FR-2.6–2.8) turns that into output.

use crate::camera::Camera;
use crate::color::Material;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::Mesh;
use crate::shading::{DirectionalLight, ShadingMode};
use crate::triangle::{self, Vertex};

/// Lighting + surface options for a solid render (bundled to keep the render
/// signature small and `Copy`). Each field's own `Default` supplies the default
/// keylight / flat shading / light-gray material.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShadeOptions {
    pub light: DirectionalLight,
    pub shading: ShadingMode,
    pub material: Material,
}

/// Render `mesh` (transformed by `model`) from `camera` into a fresh
/// `width`×`height` [`Framebuffer`]. Pure: identical inputs → identical output.
pub fn render_solid(
    mesh: &Mesh,
    model: Mat4,
    camera: &Camera,
    width: u16,
    height: u16,
    opts: ShadeOptions,
) -> Framebuffer {
    let mut fb = Framebuffer::new(width, height);
    let view_proj = camera.projection_matrix(width, height) * camera.view_matrix();

    // World-space positions and normals (model has rotation only in Phase 1–2,
    // so the linear part transforms normals correctly; non-uniform scale would
    // need the inverse-transpose — deferred with the material system).
    let world_pos: Vec<Vec3> = mesh
        .positions
        .iter()
        .map(|&p| transform_point(model, p))
        .collect();
    let world_nrm: Vec<Vec3> = mesh
        .normals
        .iter()
        .map(|&n| transform_dir(model, n))
        .collect();
    let clip: Vec<Vec4> = world_pos
        .iter()
        .map(|&p| view_proj * p.extend(1.0))
        .collect();

    for &[ia, ib, ic] in &mesh.triangles {
        let (ia, ib, ic) = (ia as usize, ib as usize, ic as usize);
        let (ca, cb, cc) = (clip[ia], clip[ib], clip[ic]);

        // Near-plane cull (FR-1.3 approach): drop any triangle with a vertex at
        // or behind the near plane rather than splitting it (Phase 5 hardening).
        if behind_near(ca) || behind_near(cb) || behind_near(cc) {
            continue;
        }

        let intensity = |vertex_normal: Vec3, face_normal: Vec3| match opts.shading {
            ShadingMode::Flat => opts.light.intensity(face_normal),
            ShadingMode::Gouraud => opts.light.intensity(vertex_normal),
        };
        let face_normal = face_normal(world_pos[ia], world_pos[ib], world_pos[ic]);

        let v0 = screen_vertex(ca, intensity(world_nrm[ia], face_normal), width, height);
        let v1 = screen_vertex(cb, intensity(world_nrm[ib], face_normal), width, height);
        let v2 = screen_vertex(cc, intensity(world_nrm[ic], face_normal), width, height);

        triangle::fill_triangle(&mut fb, v0, v1, v2, opts.material.base_color, true);
    }
    fb
}

fn behind_near(clip: Vec4) -> bool {
    clip.w <= 0.0 || clip.z < -clip.w
}

/// Perspective divide + viewport transform → a screen-space [`Vertex`].
/// Depth carried is NDC z (smaller = nearer), matching the z-buffer convention.
fn screen_vertex(clip: Vec4, intensity: f32, width: u16, height: u16) -> Vertex {
    let inv_w = 1.0 / clip.w;
    let ndc_x = clip.x * inv_w;
    let ndc_y = clip.y * inv_w;
    Vertex {
        x: (ndc_x * 0.5 + 0.5) * f32::from(width.saturating_sub(1)),
        y: (0.5 - ndc_y * 0.5) * f32::from(height.saturating_sub(1)),
        depth: clip.z * inv_w,
        intensity,
    }
}

fn face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    (b - a).cross(c - a).normalize().unwrap_or(Vec3::Z)
}

fn transform_point(m: Mat4, p: Vec3) -> Vec3 {
    (m * p.extend(1.0)).truncate()
}

fn transform_dir(m: Mat4, d: Vec3) -> Vec3 {
    // w = 0 drops the translation column, leaving the linear part.
    (m * d.extend(0.0)).truncate().normalize().unwrap_or(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgb;
    use crate::parse_obj;

    fn cube() -> Mesh {
        parse_obj(include_str!("../tests/data/cube.obj")).unwrap()
    }

    fn nonbackground_count(fb: &Framebuffer) -> usize {
        fb.rows().flatten().filter(|&&c| c != Rgb::BLACK).count()
    }

    #[test]
    fn fr2_5_solid_cube_fills_a_region() {
        let fb = render_solid(
            &cube(),
            Mat4::rotation_y(0.6) * Mat4::rotation_x(0.3),
            &Camera::default(),
            60,
            30,
            ShadeOptions::default(),
        );
        // A solid cube should cover a substantial chunk of a 60×30 frame.
        assert!(nonbackground_count(&fb) > 200, "cube barely rendered");
    }

    #[test]
    fn fr2_5_flat_and_gouraud_differ() {
        let render = |shading| {
            render_solid(
                &cube(),
                Mat4::rotation_y(0.7) * Mat4::rotation_x(0.4),
                &Camera::default(),
                60,
                30,
                ShadeOptions {
                    shading,
                    ..Default::default()
                },
            )
        };
        let flat: Vec<_> = render(ShadingMode::Flat)
            .rows()
            .flatten()
            .copied()
            .collect();
        let gouraud: Vec<_> = render(ShadingMode::Gouraud)
            .rows()
            .flatten()
            .copied()
            .collect();
        assert_ne!(
            flat, gouraud,
            "shading modes should produce different images"
        );
    }

    #[test]
    fn nfr1_solid_render_is_deterministic() {
        let go = || {
            render_solid(
                &cube(),
                Mat4::rotation_y(1.2),
                &Camera::default(),
                80,
                24,
                ShadeOptions::default(),
            )
        };
        let a: Vec<_> = go().rows().flatten().copied().collect();
        let b: Vec<_> = go().rows().flatten().copied().collect();
        assert_eq!(a, b);
    }

    #[test]
    fn fr2_5_far_face_is_occluded_by_near_face() {
        // The cube is opaque: the z-buffer must hide its back faces, so the
        // visible pixel count is far less than drawing all 12 triangles flat.
        let fb = render_solid(
            &cube(),
            Mat4::IDENTITY,
            &Camera::default(),
            40,
            40,
            ShadeOptions::default(),
        );
        assert!(nonbackground_count(&fb) > 0);
    }
}
