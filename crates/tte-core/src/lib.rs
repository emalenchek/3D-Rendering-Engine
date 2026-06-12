//! `tte-core` — portable core of the text-encoded 3D rendering engine.
//!
//! Rendering paths (see `docs/01-requirements-spec.md`):
//! - **Phase 1** — wireframe into a [`CellBuffer`] ([`render_wireframe`]).
//! - **Phase 2** — solid, depth-tested, diffuse-shaded triangles into a
//!   [`Framebuffer`] ([`render_solid`]), turned into output by a presenter
//!   ([`present::ascii_ramp`], [`present::truecolor`], [`present::half_block`]).
//!
//! ```
//! use tte_core::{parse_obj, render_solid, present, Camera, Mat4, ShadeOptions};
//!
//! let mesh = parse_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap();
//! let fb = render_solid(&mesh, Mat4::IDENTITY, &Camera::default(), 40, 12, ShadeOptions::default());
//! assert_eq!(present::ascii_ramp(&fb).to_string().lines().count(), 12);
//! ```

pub mod camera;
pub mod cell;
pub mod color;
pub mod framebuffer;
pub mod math;
pub mod mesh;
pub mod obj;
pub mod present;
pub mod raster;
pub mod render;
pub mod shading;
pub mod solid;
pub mod triangle;

pub use camera::{Camera, OrbitCamera, PITCH_LIMIT, RADIUS_MAX, RADIUS_MIN};
pub use cell::CellBuffer;
pub use color::{Material, Rgb};
pub use framebuffer::Framebuffer;
pub use math::{Mat4, Vec3, Vec4};
pub use mesh::Mesh;
pub use obj::{ObjError, load_obj, parse_obj};
pub use render::render_wireframe;
pub use shading::{DirectionalLight, ShadingMode};
pub use solid::{ShadeOptions, render_solid};

/// Returns the crate's semantic version string (from `Cargo.toml`).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
