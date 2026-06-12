//! Diffuse (Lambert) shading from a directional light (FR-2.5).

use crate::math::Vec3;

/// An infinitely-distant light: parallel rays, constant direction.
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    /// Direction the light travels (normalized on construction).
    direction: Vec3,
    /// Ambient floor in `0.0..=1.0`, so back faces aren't pure black.
    pub ambient: f32,
}

impl DirectionalLight {
    pub fn new(direction: Vec3, ambient: f32) -> Self {
        Self {
            direction: direction.normalize().unwrap_or(Vec3::new(0.0, 0.0, -1.0)),
            ambient: ambient.clamp(0.0, 1.0),
        }
    }

    /// Lambertian intensity for a surface with unit `normal`, in `0.0..=1.0`.
    /// `intensity = ambient + (1 - ambient) * max(0, N · L_toward_light)`.
    pub fn intensity(&self, normal: Vec3) -> f32 {
        let to_light = -self.direction;
        let diffuse = normal.dot(to_light).max(0.0);
        (self.ambient + (1.0 - self.ambient) * diffuse).clamp(0.0, 1.0)
    }
}

impl Default for DirectionalLight {
    /// Keylight from the upper-front-right of the default camera.
    fn default() -> Self {
        Self::new(Vec3::new(-0.5, -1.0, -0.8), 0.15)
    }
}

/// How surface normals are sampled across a triangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadingMode {
    /// One face normal → constant intensity per triangle (faceted look).
    #[default]
    Flat,
    /// Per-vertex normals → intensity interpolated across the triangle (smooth).
    Gouraud,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr2_5_face_lit_head_on_is_full_bright() {
        let light = DirectionalLight::new(Vec3::new(0.0, 0.0, -1.0), 0.0);
        // Normal points back along the incoming ray → fully lit.
        assert!((light.intensity(Vec3::new(0.0, 0.0, 1.0)) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fr2_5_back_face_gets_only_ambient() {
        let light = DirectionalLight::new(Vec3::new(0.0, 0.0, -1.0), 0.2);
        // Normal facing away from the light → diffuse term clamps to 0.
        assert!((light.intensity(Vec3::new(0.0, 0.0, -1.0)) - 0.2).abs() < 1e-6);
    }

    #[test]
    fn fr2_5_intensity_is_bounded() {
        let light = DirectionalLight::default();
        for n in [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::X, -Vec3::Y, -Vec3::Z] {
            let i = light.intensity(n);
            assert!(
                (0.0..=1.0).contains(&i),
                "intensity {i} out of range for {n:?}"
            );
        }
    }
}
