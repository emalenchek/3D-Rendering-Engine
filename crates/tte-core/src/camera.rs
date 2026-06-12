//! Perspective camera (FR-1.3) and orbit controls (FR-3.1, FR-3.2).

use crate::math::{Mat4, Vec3};

/// Width:height of one terminal cell. Cells are roughly twice as tall as they
/// are wide in typical monospace fonts; folding this into the projection
/// aspect keeps circles circular on screen (project brief D3, stage 7).
pub const DEFAULT_CELL_ASPECT: f32 = 0.5;

/// Pitch is clamped to just under ±90° so the view never reaches the pole,
/// where the eye and up vectors would become parallel and `look_at` degenerate.
pub const PITCH_LIMIT: f32 = 1.553_343; // 89° in radians
/// Dolly (zoom) range, in world units from the target.
pub const RADIUS_MIN: f32 = 1.5;
pub const RADIUS_MAX: f32 = 50.0;

/// A perspective camera: position + orientation + lens.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Vertical field of view, radians.
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    /// Width:height ratio of one output cell (see [`DEFAULT_CELL_ASPECT`]).
    pub cell_aspect: f32,
}

impl Default for Camera {
    /// The canonical test/demo camera: slightly above and to the right of a
    /// unit-cube-at-origin scene. Fixed values → deterministic golden frames.
    fn default() -> Self {
        Self {
            eye: Vec3::new(3.0, 2.2, 4.5),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: 50f32.to_radians(),
            near: 0.1,
            far: 100.0,
            cell_aspect: DEFAULT_CELL_ASPECT,
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.eye, self.target, self.up)
    }

    /// Projection matrix for an output grid of `width`×`height` cells, with
    /// the cell aspect ratio folded in.
    pub fn projection_matrix(&self, width: u16, height: u16) -> Mat4 {
        let aspect = (f32::from(width.max(1)) * self.cell_aspect) / f32::from(height.max(1));
        Mat4::perspective(self.fov_y, aspect, self.near, self.far)
    }
}

/// A camera parameterized as a point orbiting a target on a sphere (FR-3.1):
/// `yaw` around +Y, `pitch` above/below the horizon, at distance `radius`.
/// This is the interactive control model; it produces a plain [`Camera`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    /// Azimuth around +Y, radians.
    pub yaw: f32,
    /// Elevation above the horizon, radians (clamped to ±[`PITCH_LIMIT`]).
    pub pitch: f32,
    /// Distance from `target` (clamped to [`RADIUS_MIN`]..=[`RADIUS_MAX`]).
    pub radius: f32,
    pub target: Vec3,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub cell_aspect: f32,
}

impl Default for OrbitCamera {
    /// Reproduces [`Camera::default`]'s viewpoint (eye ≈ (3, 2.2, 4.5)) in
    /// spherical form, so an orbit view with no offsets matches the canonical
    /// demo framing.
    fn default() -> Self {
        Self {
            yaw: 0.588,
            pitch: 0.386,
            radius: 5.84,
            target: Vec3::ZERO,
            fov_y: 50f32.to_radians(),
            near: 0.1,
            far: 100.0,
            cell_aspect: DEFAULT_CELL_ASPECT,
        }
    }
}

impl OrbitCamera {
    /// Eye position from the spherical coordinates. At yaw = pitch = 0 the eye
    /// sits on +Z looking toward the target (research 05 §7 formula).
    pub fn eye(&self) -> Vec3 {
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.radius
    }

    /// The equivalent perspective [`Camera`].
    pub fn to_camera(&self) -> Camera {
        Camera {
            eye: self.eye(),
            target: self.target,
            up: Vec3::Y,
            fov_y: self.fov_y,
            near: self.near,
            far: self.far,
            cell_aspect: self.cell_aspect,
        }
    }

    /// Rotate the camera around the target; pitch is clamped (FR-3.2).
    pub fn orbit(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw += dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Multiply the orbit radius by `factor` (zoom), clamped (FR-3.2).
    pub fn dolly(&mut self, factor: f32) {
        self.radius = (self.radius * factor).clamp(RADIUS_MIN, RADIUS_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn fr3_1_eye_at_origin_angles_sits_on_positive_z() {
        let orbit = OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            radius: 4.0,
            ..Default::default()
        };
        assert_relative_eq!(orbit.eye(), Vec3::new(0.0, 0.0, 4.0), epsilon = 1e-5);
    }

    #[test]
    fn fr3_1_eye_is_always_radius_from_target() {
        let mut orbit = OrbitCamera {
            target: Vec3::new(1.0, 2.0, -3.0),
            ..Default::default()
        };
        for (dy, dp) in [(0.3, 0.2), (-1.4, 0.9), (2.0, -1.2)] {
            orbit.orbit(dy, dp);
            assert_relative_eq!(
                (orbit.eye() - orbit.target).length(),
                orbit.radius,
                epsilon = 1e-4
            );
        }
    }

    #[test]
    fn fr3_2_pitch_is_clamped_below_the_pole() {
        let mut orbit = OrbitCamera::default();
        orbit.orbit(0.0, 100.0);
        assert_relative_eq!(orbit.pitch, PITCH_LIMIT);
        orbit.orbit(0.0, -100.0);
        assert_relative_eq!(orbit.pitch, -PITCH_LIMIT);
    }

    #[test]
    fn fr3_2_dolly_clamps_radius() {
        let mut orbit = OrbitCamera::default();
        for _ in 0..100 {
            orbit.dolly(0.5);
        }
        assert_relative_eq!(orbit.radius, RADIUS_MIN);
        for _ in 0..100 {
            orbit.dolly(2.0);
        }
        assert_relative_eq!(orbit.radius, RADIUS_MAX);
    }

    #[test]
    fn fr3_1_default_matches_canonical_camera_framing() {
        // Within ~0.05 world units of the hand-picked Camera::default() eye.
        let eye = OrbitCamera::default().eye();
        assert_relative_eq!(eye, Camera::default().eye, epsilon = 0.05);
    }
}
