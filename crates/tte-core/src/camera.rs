//! Perspective camera (FR-1.3).

use crate::math::{Mat4, Vec3};

/// Width:height of one terminal cell. Cells are roughly twice as tall as they
/// are wide in typical monospace fonts; folding this into the projection
/// aspect keeps circles circular on screen (project brief D3, stage 7).
pub const DEFAULT_CELL_ASPECT: f32 = 0.5;

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
