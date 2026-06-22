//! `tte-wasm` — browser frontend for the text-encoded 3D rendering engine.
//!
//! A thin `wasm-bindgen` `Renderer` over `tte-core` (FR-5.1): construct with a
//! cell grid size, load an OBJ model or a DSL scene, orbit the camera, and pull
//! a per-cell `{glyph, fg, bg}` frame out as typed arrays. All real work lives in
//! the (natively-tested) core; this crate only marshals data across the JS
//! boundary — `web-sys` is intentionally absent (decision V4a), so the page's JS
//! owns the canvas.
//!
//! The `#[wasm_bindgen]` API also compiles natively (the crate is an `rlib` too),
//! so the logic is unit-tested with plain `cargo test`; `wasm-bindgen-test`
//! covers the wasm boundary (FR-5.5).
#![allow(unsafe_code)] // wasm-bindgen's generated glue.

use tte_core::{
    Mat4, Mesh, Scene, ShadeOptions, ShadingMode, WebFrame, WebMode, parse_obj, parse_scene,
    primitives, render_scene, render_solid, web_frame,
};
use tte_core::{PITCH_LIMIT, RADIUS_MAX, RADIUS_MIN};
use wasm_bindgen::prelude::*;

/// What the renderer is currently showing.
#[derive(Debug, Clone)]
enum Subject {
    Mesh(Mesh),
    Scene(Scene),
}

/// The browser-facing renderer. Holds the subject, an orbit camera, the output
/// mode, and the last rendered frame; `render()` refreshes the frame and the
/// `glyphs()`/`fg()`/`bg()` getters expose it as typed arrays.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Renderer {
    width: u16,
    height: u16,
    subject: Subject,
    orbit: tte_core::OrbitCamera,
    mode: WebMode,
    frame: WebFrame,
}

#[wasm_bindgen]
impl Renderer {
    /// Create a renderer for a `width`×`height` **cell** grid, showing a unit
    /// cube by default. Pre-renders one frame so the getters are valid
    /// immediately (FR-5.6: buffers exist from construction).
    #[wasm_bindgen(constructor)]
    pub fn new(width: u16, height: u16) -> Renderer {
        let width = width.max(1);
        let height = height.max(1);
        let mut r = Renderer {
            width,
            height,
            subject: Subject::Mesh(primitives::cube()),
            orbit: tte_core::OrbitCamera::default(),
            mode: WebMode::Ascii,
            frame: web_frame(
                &render_solid(
                    &primitives::cube(),
                    Mat4::IDENTITY,
                    &tte_core::OrbitCamera::default().to_camera(),
                    width,
                    height,
                    ShadeOptions::default(),
                ),
                WebMode::Ascii,
            ),
        };
        r.render();
        r
    }

    /// Replace the subject with an OBJ model. Parse errors become JS exceptions
    /// carrying the line-numbered message (FR-5.1).
    pub fn load_obj(&mut self, text: &str) -> Result<(), String> {
        self.subject = Subject::Mesh(parse_obj(text).map_err(|e| e.to_string())?);
        Ok(())
    }

    /// Replace the subject with a DSL scene (errors → JS exceptions, FR-5.1).
    /// External mesh references are unsupported in the browser (no filesystem)
    /// and render as nothing.
    pub fn load_scene(&mut self, text: &str) -> Result<(), String> {
        self.subject = Subject::Scene(parse_scene(text).map_err(|e| e.to_string())?);
        Ok(())
    }

    /// Set the orbit camera (yaw/pitch radians, radius world units); clamped to
    /// the engine's safe ranges.
    pub fn set_orbit(&mut self, yaw: f32, pitch: f32, radius: f32) {
        self.orbit.yaw = yaw;
        self.orbit.pitch = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.orbit.radius = radius.clamp(RADIUS_MIN, RADIUS_MAX);
    }

    /// Choose the output mode: `"ascii"`, `"truecolor"`, or `"halfblock"`.
    pub fn set_mode(&mut self, mode: &str) -> Result<(), String> {
        self.mode = match mode {
            "ascii" => WebMode::Ascii,
            "truecolor" => WebMode::Truecolor,
            "halfblock" => WebMode::HalfBlock,
            other => return Err(format!("unknown mode '{other}'")),
        };
        Ok(())
    }

    /// Render the current subject from the current camera into the frame buffers.
    pub fn render(&mut self) {
        let camera = self.orbit.to_camera();
        // Half-block packs two pixel rows per cell row, so render at 2× height.
        let render_h = if self.mode == WebMode::HalfBlock {
            self.height.saturating_mul(2)
        } else {
            self.height
        };
        let fb = match &self.subject {
            Subject::Mesh(mesh) => render_solid(
                mesh,
                Mat4::IDENTITY,
                &camera,
                self.width,
                render_h,
                ShadeOptions::default(),
            ),
            Subject::Scene(scene) => render_scene(
                scene,
                &camera,
                self.width,
                render_h,
                ShadingMode::Flat,
                |_| None,
            ),
        };
        self.frame = web_frame(&fb, self.mode);
    }

    /// Resize the cell grid (FR-10.3 adaptive resolution). Keeps the current
    /// subject, camera, and mode — just re-renders at the new dimensions. The
    /// presenter follows the new `width()`/`height()` on its next draw.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.render();
    }

    /// Cell-grid width.
    pub fn width(&self) -> u16 {
        self.frame.width()
    }

    /// Cell-grid height (equals the constructor height in every mode).
    pub fn height(&self) -> u16 {
        self.frame.height()
    }

    /// One Unicode scalar per cell (`width*height` entries) → `Uint16Array`.
    pub fn glyphs(&self) -> Vec<u16> {
        self.frame.glyphs().to_vec()
    }

    /// Foreground RGB, 3 bytes/cell → `Uint8Array`.
    pub fn fg(&self) -> Vec<u8> {
        self.frame.fg().to_vec()
    }

    /// Background RGB, 3 bytes/cell → `Uint8Array`.
    pub fn bg(&self) -> Vec<u8> {
        self.frame.bg().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr5_1_new_renders_default_cube_frame() {
        let r = Renderer::new(40, 20);
        assert_eq!(r.width(), 40);
        assert_eq!(r.height(), 20);
        assert_eq!(r.glyphs().len(), 40 * 20);
        assert_eq!(r.fg().len(), 40 * 20 * 3);
        // A cube in view → at least one non-space glyph.
        assert!(
            r.glyphs().iter().any(|&g| g != u16::from(b' ')),
            "cube should be visible"
        );
    }

    #[test]
    fn fr5_1_load_obj_replaces_subject() {
        let mut r = Renderer::new(30, 15);
        r.load_obj("v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n")
            .unwrap();
        r.render();
        assert!(r.glyphs().iter().any(|&g| g != u16::from(b' ')));
    }

    #[test]
    fn fr5_1_load_obj_error_carries_line_number() {
        let mut r = Renderer::new(10, 10);
        let err = r.load_obj("v 0 0 0\nf 1 2 3\n").unwrap_err();
        assert!(err.contains("line 2"), "got: {err}");
    }

    #[test]
    fn fr5_1_load_scene_parses_dsl() {
        let mut r = Renderer::new(30, 15);
        r.load_scene("sphere\nplane translate=(0 -1 0) scale=4")
            .unwrap();
        r.render();
        assert!(r.glyphs().iter().any(|&g| g != u16::from(b' ')));
    }

    #[test]
    fn fr5_1_load_scene_error_is_reported() {
        let mut r = Renderer::new(10, 10);
        assert!(r.load_scene("teapot").is_err());
    }

    #[test]
    fn fr5_1_set_orbit_clamps() {
        let mut r = Renderer::new(10, 10);
        r.set_orbit(1.0, 100.0, 0.0); // pitch & radius out of range
        assert!((r.orbit.pitch - PITCH_LIMIT).abs() < 1e-5);
        assert!((r.orbit.radius - RADIUS_MIN).abs() < 1e-5);
    }

    #[test]
    fn fr5_1_halfblock_keeps_cell_height() {
        let mut r = Renderer::new(20, 12);
        r.set_mode("halfblock").unwrap();
        r.render();
        assert_eq!(
            r.height(),
            12,
            "cell height unchanged; render is 2× internally"
        );
        assert!(r.glyphs().contains(&0x2580), "should use ▀");
    }

    #[test]
    fn fr5_1_unknown_mode_errs() {
        let mut r = Renderer::new(10, 10);
        assert!(r.set_mode("sixel").is_err());
    }

    #[test]
    fn nfr9_render_is_deterministic() {
        let frame = || {
            let mut r = Renderer::new(60, 30);
            r.set_orbit(0.7, 0.4, 6.0);
            r.render();
            (r.glyphs(), r.fg(), r.bg())
        };
        assert_eq!(frame(), frame());
    }
}
