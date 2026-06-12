//! `tte-core` — portable core of the text-encoded 3D rendering engine.
//!
//! Phase 1 scope (see `docs/01-requirements-spec.md`): wireframe rendering of
//! OBJ meshes into a [`CellBuffer`] — a pure character-grid value that
//! frontends (terminal, later WASM) present however they like.
//!
//! ```
//! use tte_core::{parse_obj, render_wireframe, Camera, Mat4};
//!
//! let mesh = parse_obj("v -1 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
//! let frame = render_wireframe(&mesh, Mat4::IDENTITY, &Camera::default(), 40, 12);
//! assert_eq!(frame.to_string().lines().count(), 12);
//! ```

pub mod camera;
pub mod cell;
pub mod math;
pub mod mesh;
pub mod obj;
pub mod raster;
pub mod render;

pub use camera::Camera;
pub use cell::CellBuffer;
pub use math::{Mat4, Vec3, Vec4};
pub use mesh::Mesh;
pub use obj::{ObjError, load_obj, parse_obj};
pub use render::render_wireframe;

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
