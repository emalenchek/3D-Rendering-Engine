//! Minimal linear algebra for the renderer (FR-1.1).
//!
//! Conventions (fixed project-wide, see docs/00-project-brief.md D3):
//! - Right-handed coordinates, +Y up, camera looks down −Z in view space.
//! - `Mat4` is row-major: `m[row][col]`; vectors are columns (`M * v`).
//! - Projection maps to OpenGL-style NDC: x,y,z ∈ [−1, 1] inside the frustum.

use std::ops::{Add, Mul, Neg, Sub};

/// 3-component vector of `f32`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const X: Self = Self::new(1.0, 0.0, 0.0);
    pub const Y: Self = Self::new(0.0, 1.0, 0.0);
    pub const Z: Self = Self::new(0.0, 0.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// Unit vector in this direction. Returns `None` for (near-)zero vectors
    /// instead of producing NaNs.
    pub fn normalize(self) -> Option<Self> {
        let len = self.length();
        (len > f32::EPSILON).then(|| self * (1.0 / len))
    }

    pub const fn extend(self, w: f32) -> Vec4 {
        Vec4::new(self.x, self.y, self.z, w)
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// 4-component homogeneous vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    pub const fn truncate(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

/// 4×4 matrix, row-major (`m[row][col]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        m: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    /// Rotation about the +X axis by `angle` radians (right-hand rule).
    pub fn rotation_x(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut out = Self::IDENTITY;
        out.m[1][1] = c;
        out.m[1][2] = -s;
        out.m[2][1] = s;
        out.m[2][2] = c;
        out
    }

    /// Rotation about the +Y axis by `angle` radians (right-hand rule).
    pub fn rotation_y(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut out = Self::IDENTITY;
        out.m[0][0] = c;
        out.m[0][2] = s;
        out.m[2][0] = -s;
        out.m[2][2] = c;
        out
    }

    /// Rotation about the +Z axis by `angle` radians (right-hand rule).
    pub fn rotation_z(angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        let mut out = Self::IDENTITY;
        out.m[0][0] = c;
        out.m[0][1] = -s;
        out.m[1][0] = s;
        out.m[1][1] = c;
        out
    }

    /// Translation by `t`.
    pub fn translation(t: Vec3) -> Self {
        let mut out = Self::IDENTITY;
        out.m[0][3] = t.x;
        out.m[1][3] = t.y;
        out.m[2][3] = t.z;
        out
    }

    /// Non-uniform scale.
    pub fn scale(s: Vec3) -> Self {
        let mut out = Self::IDENTITY;
        out.m[0][0] = s.x;
        out.m[1][1] = s.y;
        out.m[2][2] = s.z;
        out
    }

    /// Right-handed view matrix: world space → view space, camera at `eye`
    /// looking at `target`, looking down −Z.
    ///
    /// Panics if `eye == target` or `up` is parallel to the view direction —
    /// both are caller bugs, caught early by the orbit camera's pitch clamp.
    pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = (target - eye).normalize().expect("look_at: eye == target");
        let s = f
            .cross(up)
            .normalize()
            .expect("look_at: up parallel to view direction");
        let u = s.cross(f);
        Self {
            m: [
                [s.x, s.y, s.z, -s.dot(eye)],
                [u.x, u.y, u.z, -u.dot(eye)],
                [-f.x, -f.y, -f.z, f.dot(eye)],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Right-handed perspective projection to OpenGL-style NDC (z ∈ [−1, 1]).
    /// `fov_y` is the vertical field of view in radians.
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        let f = 1.0 / (fov_y * 0.5).tan();
        let mut out = Self { m: [[0.0; 4]; 4] };
        out.m[0][0] = f / aspect;
        out.m[1][1] = f;
        out.m[2][2] = (far + near) / (near - far);
        out.m[2][3] = (2.0 * far * near) / (near - far);
        out.m[3][2] = -1.0;
        out
    }
}

impl Mul for Mat4 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let mut out = Self { m: [[0.0; 4]; 4] };
        for (r, row) in out.m.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.m[r][k] * rhs.m[k][c]).sum();
            }
        }
        out
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        let dot = |r: usize| {
            self.m[r][0] * v.x + self.m[r][1] * v.y + self.m[r][2] * v.z + self.m[r][3] * v.w
        };
        Vec4::new(dot(0), dot(1), dot(2), dot(3))
    }
}

// approx trait impls so tests can write `assert_relative_eq!(vec_a, vec_b)`.
// `approx` is a tiny, dependency-free crate; carrying it as a regular
// dependency keeps the impls available to integration tests (a #[cfg(test)]
// impl would be invisible to them — they link the lib as an external crate).
impl approx::AbsDiffEq for Vec3 {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        self.x.abs_diff_eq(&other.x, epsilon)
            && self.y.abs_diff_eq(&other.y, epsilon)
            && self.z.abs_diff_eq(&other.z, epsilon)
    }
}

impl approx::RelativeEq for Vec3 {
    fn default_max_relative() -> f32 {
        f32::default_max_relative()
    }
    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        self.x.relative_eq(&other.x, epsilon, max_relative)
            && self.y.relative_eq(&other.y, epsilon, max_relative)
            && self.z.relative_eq(&other.z, epsilon, max_relative)
    }
}

impl approx::AbsDiffEq for Mat4 {
    type Epsilon = f32;
    fn default_epsilon() -> f32 {
        f32::default_epsilon()
    }
    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        (0..4).all(|r| (0..4).all(|c| self.m[r][c].abs_diff_eq(&other.m[r][c], epsilon)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{assert_abs_diff_eq, assert_relative_eq};

    #[test]
    fn fr1_1_cross_follows_right_hand_rule() {
        assert_relative_eq!(Vec3::X.cross(Vec3::Y), Vec3::Z);
    }

    #[test]
    fn fr1_1_normalize_rejects_zero_vector() {
        assert_eq!(Vec3::ZERO.normalize(), None);
    }

    #[test]
    fn fr4_2_trs_compose_translate_rotate_scale() {
        // T*R*S applied to a point: scale, then rotate 90° about Y, then translate.
        let m = Mat4::translation(Vec3::new(10.0, 0.0, 0.0))
            * Mat4::rotation_y(std::f32::consts::FRAC_PI_2)
            * Mat4::scale(Vec3::new(2.0, 1.0, 1.0));
        let p = (m * Vec3::X.extend(1.0)).truncate();
        // X scaled to 2, rotated +90° about Y → (0,0,-2), translated +10x.
        assert_abs_diff_eq!(p, Vec3::new(10.0, 0.0, -2.0), epsilon = 1e-5);
    }

    #[test]
    fn fr4_2_rotation_z_turns_x_toward_y() {
        let p = (Mat4::rotation_z(std::f32::consts::FRAC_PI_2) * Vec3::X.extend(1.0)).truncate();
        assert_abs_diff_eq!(p, Vec3::Y, epsilon = 1e-5);
    }

    #[test]
    fn fr1_1_look_at_maps_eye_to_origin() {
        let eye = Vec3::new(3.0, 2.0, 5.0);
        let view = Mat4::look_at(eye, Vec3::ZERO, Vec3::Y);
        let mapped = (view * eye.extend(1.0)).truncate();
        assert_abs_diff_eq!(mapped, Vec3::ZERO, epsilon = 1e-5);
    }

    #[test]
    fn fr1_1_look_at_target_lands_on_negative_z() {
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let mapped = (view * Vec3::ZERO.extend(1.0)).truncate();
        assert_abs_diff_eq!(mapped, Vec3::new(0.0, 0.0, -5.0), epsilon = 1e-5);
    }

    #[test]
    fn fr1_1_perspective_maps_near_plane_to_ndc_minus_one() {
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let clip = proj * Vec3::new(0.0, 0.0, -0.1).extend(1.0);
        assert_abs_diff_eq!(clip.z / clip.w, -1.0, epsilon = 1e-4);
    }

    #[test]
    fn fr1_1_perspective_maps_far_plane_to_ndc_plus_one() {
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let clip = proj * Vec3::new(0.0, 0.0, -100.0).extend(1.0);
        assert_abs_diff_eq!(clip.z / clip.w, 1.0, epsilon = 1e-4);
    }
}
