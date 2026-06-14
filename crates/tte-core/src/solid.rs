//! Solid shaded rendering: mesh → shaded, depth-tested triangles (FR-2.5).
//!
//! Pipeline (project brief D3): model → view → projection → near-plane cull →
//! perspective divide → viewport → shade → edge-function fill with z-buffer.
//! Produces a [`Framebuffer`]; a presenter (FR-2.6–2.8) turns that into output.

use crate::camera::Camera;
use crate::color::{Material, Rgb};
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec3, Vec4};
use crate::mesh::Mesh;
use crate::primitives;
use crate::scene::{Geometry, Scene};
use crate::shading::{DirectionalLight, ShadingMode};
use crate::triangle::{self, Vertex};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A screen-space triangle ready to rasterize: three shaded vertices, a base
/// color, and its pixel-row span (precomputed so a parallel band can skip
/// triangles it doesn't overlap — FR-6.1).
#[derive(Debug, Clone, Copy)]
struct DrawTri {
    v: [Vertex; 3],
    color: Rgb,
    y_min: i32,
    y_max: i32,
}

/// Below this triangle count the parallel path's overhead isn't worth it
/// (research report 09, small-working-set caveat) — render sequentially.
#[cfg(feature = "parallel")]
const PARALLEL_MIN_TRIS: usize = 256;

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
    render_mesh_into(
        &mut fb,
        mesh,
        model,
        camera,
        &opts.light,
        opts.shading,
        opts.material,
    );
    fb
}

/// Rasterize one mesh into an existing framebuffer (FR-4.5 building block). Lets
/// a whole scene accumulate into one shared z-buffer. The geometry stage builds
/// a screen-space triangle list (preserving mesh order), then the rasterizer
/// draws it sequentially or, with the `parallel` feature, across row bands —
/// byte-identically either way (FR-6.1, FR-6.3).
pub fn render_mesh_into(
    fb: &mut Framebuffer,
    mesh: &Mesh,
    model: Mat4,
    camera: &Camera,
    light: &DirectionalLight,
    shading: ShadingMode,
    material: Material,
) {
    let tris = prepare_mesh(
        mesh,
        model,
        camera,
        light,
        shading,
        material,
        fb.width(),
        fb.height(),
    );
    rasterize(fb, &tris);
}

/// Geometry stage: transform, shade, near-plane-cull, and project a mesh into a
/// screen-space triangle list (mesh order preserved). Separated from rasterization
/// so both rasterizer paths can be driven from the same input (FR-6.3 tests).
#[allow(clippy::too_many_arguments)]
fn prepare_mesh(
    mesh: &Mesh,
    model: Mat4,
    camera: &Camera,
    light: &DirectionalLight,
    shading: ShadingMode,
    material: Material,
    width: u16,
    height: u16,
) -> Vec<DrawTri> {
    let view_proj = camera.projection_matrix(width, height) * camera.view_matrix();

    // World-space positions and normals. The model's linear part transforms
    // normals correctly for rotation/uniform scale; non-uniform scale would
    // need the inverse-transpose — deferred with the material system.
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

    let mut tris: Vec<DrawTri> = Vec::with_capacity(mesh.triangles.len());
    for &[ia, ib, ic] in &mesh.triangles {
        let (ia, ib, ic) = (ia as usize, ib as usize, ic as usize);
        let (ca, cb, cc) = (clip[ia], clip[ib], clip[ic]);

        // Near-plane cull (FR-1.3 approach): drop any triangle with a vertex at
        // or behind the near plane rather than splitting it (Phase 5 hardening).
        if behind_near(ca) || behind_near(cb) || behind_near(cc) {
            continue;
        }

        let intensity = |vertex_normal: Vec3, face_normal: Vec3| match shading {
            ShadingMode::Flat => light.intensity(face_normal),
            ShadingMode::Gouraud => light.intensity(vertex_normal),
        };
        let face_normal = face_normal(world_pos[ia], world_pos[ib], world_pos[ic]);

        let v = [
            screen_vertex(ca, intensity(world_nrm[ia], face_normal), width, height),
            screen_vertex(cb, intensity(world_nrm[ib], face_normal), width, height),
            screen_vertex(cc, intensity(world_nrm[ic], face_normal), width, height),
        ];
        let y_min = v.iter().map(|p| p.y.floor() as i32).min().unwrap();
        let y_max = v.iter().map(|p| p.y.ceil() as i32).max().unwrap();
        tris.push(DrawTri {
            v,
            color: material.base_color,
            y_min,
            y_max,
        });
    }
    tris
}

/// Draw a prepared triangle list into `fb`. Dispatches to the parallel band
/// rasterizer when the `parallel` feature is on and there's enough work.
fn rasterize(fb: &mut Framebuffer, tris: &[DrawTri]) {
    #[cfg(feature = "parallel")]
    if tris.len() >= PARALLEL_MIN_TRIS {
        rasterize_parallel(fb, tris);
        return;
    }
    rasterize_seq(fb, tris);
}

/// Sequential rasterization: triangles in list order into the whole frame.
fn rasterize_seq(fb: &mut Framebuffer, tris: &[DrawTri]) {
    for t in tris {
        triangle::fill_triangle(fb, t.v[0], t.v[1], t.v[2], t.color, true);
    }
}

/// Parallel rasterization (FR-6.1): partition the frame into disjoint row bands
/// and draw every triangle into each band it overlaps. Bands own non-overlapping
/// pixels (no atomics, no races), and each band applies triangles in the same
/// global list order, so the strict-`<` depth tie-break — and thus the output —
/// is byte-identical to [`rasterize_seq`] (FR-6.3).
#[cfg(feature = "parallel")]
fn rasterize_parallel(fb: &mut Framebuffer, tris: &[DrawTri]) {
    use crate::framebuffer::Band;

    let (width, height, color, depth) = fb.buffers_mut();
    if width == 0 || height == 0 {
        return;
    }
    let threads = rayon::current_num_threads().max(1);
    let band_rows = usize::from(height).div_ceil(threads * 4).max(1);
    let chunk = band_rows * usize::from(width);

    color
        .par_chunks_mut(chunk)
        .zip(depth.par_chunks_mut(chunk))
        .enumerate()
        .for_each(|(bi, (c, d))| {
            let y_start = (bi * band_rows) as i32;
            let y_end = y_start + (c.len() / usize::from(width)) as i32;
            let mut band = Band::new(width, y_start, c, d);
            for t in tris {
                if t.y_max < y_start || t.y_min >= y_end {
                    continue; // triangle doesn't touch this band
                }
                triangle::fill_triangle(&mut band, t.v[0], t.v[1], t.v[2], t.color, true);
            }
        });
}

/// Render a whole [`Scene`] into one framebuffer (FR-4.5). Built-in primitives
/// are baked here; external mesh references are resolved by `load_mesh` (so the
/// core stays free of filesystem access — the CLI supplies the loader).
pub fn render_scene<F>(
    scene: &Scene,
    camera: &Camera,
    width: u16,
    height: u16,
    shading: ShadingMode,
    mut load_mesh: F,
) -> Framebuffer
where
    F: FnMut(&str) -> Option<Mesh>,
{
    let light = scene.light.to_light();
    let mut fb = Framebuffer::with_background(width, height, scene.background);
    for drawable in scene.flatten() {
        let mesh = match &drawable.geometry {
            Geometry::Cube => Some(primitives::cube()),
            Geometry::Sphere { rings, segments } => Some(primitives::sphere(*rings, *segments)),
            Geometry::Plane => Some(primitives::plane()),
            Geometry::MeshRef(path) => load_mesh(path),
            Geometry::Group => None,
        };
        if let Some(mesh) = mesh {
            render_mesh_into(
                &mut fb,
                &mesh,
                drawable.world,
                camera,
                &light,
                shading,
                drawable.material,
            );
        }
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

    /// FR-6.3: the parallel band rasterizer must reproduce the sequential output
    /// bit-for-bit. Drives both from one prepared triangle list (a ~2k-tri sphere
    /// large enough to span many bands).
    #[cfg(feature = "parallel")]
    #[test]
    fn fr6_3_parallel_matches_sequential_byte_for_byte() {
        let sphere = primitives::sphere(24, 48); // ~2300 triangles
        let (w, h) = (200u16, 120u16);
        let tris = prepare_mesh(
            &sphere,
            Mat4::rotation_y(0.6) * Mat4::rotation_x(0.3),
            &Camera::default(),
            &DirectionalLight::default(),
            ShadingMode::Gouraud,
            Material::default(),
            w,
            h,
        );
        assert!(
            tris.len() >= PARALLEL_MIN_TRIS,
            "need enough tris to exercise bands"
        );

        let mut seq = Framebuffer::new(w, h);
        rasterize_seq(&mut seq, &tris);
        let mut par = Framebuffer::new(w, h);
        rasterize_parallel(&mut par, &tris);

        let seq_px: Vec<_> = seq.rows().flatten().copied().collect();
        let par_px: Vec<_> = par.rows().flatten().copied().collect();
        assert_eq!(seq_px, par_px, "parallel output diverged from sequential");
        assert!(
            nonbackground_count(&par) > 1000,
            "sphere should fill a region"
        );
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
