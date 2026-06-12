//! Built-in primitive meshes for the scene DSL (FR-4.4).
//!
//! All primitives are unit-sized and centered at the origin; the scene's
//! per-node transform scales/positions them. Normals are per-vertex so both
//! flat and Gouraud shading look right.

use crate::math::Vec3;
use crate::mesh::Mesh;

/// Axis-aligned cube spanning ±0.5 on each axis (unit edge length).
/// Each face has its own 4 vertices so face normals are exact (no smoothing
/// across the hard edges).
pub fn cube() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();

    // (normal, two in-plane axes) for each of the 6 faces.
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::new(-1.0, 0.0, 0.0), Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::new(0.0, -1.0, 0.0), Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::new(0.0, 0.0, -1.0), Vec3::Y, Vec3::X),
    ];
    for (n, u, v) in faces {
        let base = positions.len() as u32;
        let center = n * 0.5;
        for (su, sv) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            positions.push(center + u * su + v * sv);
            normals.push(n);
        }
        triangles.push([base, base + 1, base + 2]);
        triangles.push([base, base + 2, base + 3]);
    }
    Mesh {
        positions,
        normals,
        triangles,
    }
}

/// Unit-radius UV sphere (`rings` latitude bands × `segments` longitude). For
/// a sphere centered at the origin the normal at a vertex equals its position.
pub fn sphere(rings: u32, segments: u32) -> Mesh {
    let rings = rings.max(2);
    let segments = segments.max(3);
    let mut positions = Vec::new();
    for r in 0..=rings {
        let phi = std::f32::consts::PI * r as f32 / rings as f32;
        let (sp, cp) = phi.sin_cos();
        for s in 0..segments {
            let theta = std::f32::consts::TAU * s as f32 / segments as f32;
            let (st, ct) = theta.sin_cos();
            positions.push(Vec3::new(sp * ct, cp, sp * st));
        }
    }
    let idx = |r: u32, s: u32| r * segments + (s % segments);
    let mut triangles = Vec::new();
    for r in 0..rings {
        for s in 0..segments {
            let (a, b) = (idx(r, s), idx(r, s + 1));
            let (c, d) = (idx(r + 1, s), idx(r + 1, s + 1));
            triangles.push([a, b, c]);
            triangles.push([b, d, c]);
        }
    }
    let normals = positions.clone();
    Mesh {
        positions,
        normals,
        triangles,
    }
}

/// Unit square in the XZ plane (normal +Y), spanning ±0.5.
pub fn plane() -> Mesh {
    let positions = vec![
        Vec3::new(-0.5, 0.0, -0.5),
        Vec3::new(0.5, 0.0, -0.5),
        Vec3::new(0.5, 0.0, 0.5),
        Vec3::new(-0.5, 0.0, 0.5),
    ];
    let normals = vec![Vec3::Y; 4];
    let triangles = vec![[0, 1, 2], [0, 2, 3]];
    Mesh {
        positions,
        normals,
        triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn fr4_4_cube_has_12_triangles_and_unit_extent() {
        let m = cube();
        assert_eq!(m.triangles.len(), 12);
        for p in &m.positions {
            assert!(p.x.abs() <= 0.5 + 1e-6 && p.y.abs() <= 0.5 + 1e-6 && p.z.abs() <= 0.5 + 1e-6);
        }
    }

    #[test]
    fn fr4_4_sphere_vertices_are_unit_length() {
        for p in &sphere(8, 12).positions {
            assert_relative_eq!(p.length(), 1.0, epsilon = 1e-5);
        }
    }

    #[test]
    fn fr4_4_plane_faces_up() {
        let m = plane();
        assert_eq!(m.triangles.len(), 2);
        assert!(m.normals.iter().all(|n| *n == Vec3::Y));
    }
}
