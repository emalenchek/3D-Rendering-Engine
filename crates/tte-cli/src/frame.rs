//! Turning render options into a displayable frame (FR-2.9).
//!
//! One place builds frames for both interactive and headless output, so the two
//! paths can never drift (NFR-1). The result is a list of row strings (possibly
//! carrying ANSI color) plus whether a trailing SGR reset is needed.

use tte_core::{
    Camera, Framebuffer, Mat4, Mesh, Scene, ShadeOptions, ShadingMode, present, render_scene,
    render_solid, render_wireframe,
};

/// Smallest cell grid we'll render into, so a tiny/odd terminal can't produce a
/// zero-sized frame (FR-3.4).
pub const MIN_DIM: u16 = 4;

/// Clamp a reported terminal size to something renderable (FR-3.4).
pub fn clamp_dims(width: u16, height: u16) -> (u16, u16) {
    (width.max(MIN_DIM), height.max(MIN_DIM))
}

/// Wireframe vs solid surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderKind {
    Wireframe,
    #[default]
    Solid,
}

/// Output fidelity for solid rendering (see project brief D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// Luminance ramp, plain text — the universal fallback.
    #[default]
    Ascii,
    /// 24-bit foreground, one block per cell.
    Truecolor,
    /// Two vertical sub-pixels per cell via `▀` (2× vertical resolution).
    HalfBlock,
}

/// Everything needed to render one frame, independent of where it's shown.
#[derive(Debug, Clone, Copy)]
pub struct FrameSpec {
    pub kind: RenderKind,
    pub shading: ShadingMode,
    pub color: ColorMode,
    pub camera: Camera,
    pub width: u16,
    pub height: u16,
}

/// A rendered frame as display-ready rows.
#[derive(Debug, Clone)]
pub struct RenderedFrame {
    pub lines: Vec<String>,
    pub reset: bool,
}

impl RenderedFrame {
    /// Plain-text form for headless output. Reproduces the core presenters'
    /// string form exactly (byte-identical golden frames).
    pub fn headless_text(&self) -> String {
        let mut s = self.lines.join("\n");
        s.push('\n');
        if self.reset {
            s.push_str("\x1b[0m");
        }
        s
    }
}

/// Render a single `mesh` under `model` into display-ready rows.
pub fn render(mesh: &Mesh, model: Mat4, spec: FrameSpec) -> RenderedFrame {
    if spec.kind == RenderKind::Wireframe {
        let cb = render_wireframe(mesh, model, &spec.camera, spec.width, spec.height);
        return RenderedFrame {
            lines: cb.rows().collect(),
            reset: false,
        };
    }
    let opts = ShadeOptions {
        shading: spec.shading,
        ..Default::default()
    };
    let fb = render_solid(
        mesh,
        model,
        &spec.camera,
        spec.width,
        render_height(spec),
        opts,
    );
    present_framebuffer(&fb, spec.color)
}

/// Render a whole [`Scene`] into display-ready rows (FR-4.5). Scenes are always
/// solid; `load_mesh` resolves external mesh references. The camera in `spec`
/// is used (the caller resolves the scene's own camera vs. an orbit override).
pub fn render_scene_frame<F>(scene: &Scene, spec: FrameSpec, load_mesh: F) -> RenderedFrame
where
    F: FnMut(&str) -> Option<Mesh>,
{
    let fb = render_scene(
        scene,
        &spec.camera,
        spec.width,
        render_height(spec),
        spec.shading,
        load_mesh,
    );
    present_framebuffer(&fb, spec.color)
}

/// Half-block packs two framebuffer rows per cell row, so it renders at double
/// height (research D4); other modes render one framebuffer row per cell.
fn render_height(spec: FrameSpec) -> u16 {
    match spec.color {
        ColorMode::HalfBlock => spec.height.saturating_mul(2),
        _ => spec.height,
    }
}

/// Turn a rendered [`Framebuffer`] into display rows via the chosen presenter.
fn present_framebuffer(fb: &Framebuffer, color: ColorMode) -> RenderedFrame {
    match color {
        ColorMode::Ascii => RenderedFrame {
            lines: present::ascii_ramp(fb).rows().collect(),
            reset: false,
        },
        ColorMode::Truecolor => RenderedFrame {
            lines: ansi_lines(present::truecolor(fb)),
            reset: true,
        },
        ColorMode::HalfBlock => RenderedFrame {
            lines: ansi_lines(present::half_block(fb)),
            reset: true,
        },
    }
}

/// Split a presenter string (`"row\nrow\n…\n\x1b[0m"`) into rows, dropping the
/// trailing reset (re-applied by the presenter/headless layer).
fn ansi_lines(s: String) -> Vec<String> {
    let body = s.strip_suffix("\x1b[0m").unwrap_or(&s);
    body.trim_end_matches('\n')
        .split('\n')
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tte_core::parse_obj;

    fn cube() -> tte_core::Mesh {
        parse_obj(include_str!("../tests/data/cube.obj")).unwrap()
    }

    fn spec(kind: RenderKind, color: ColorMode) -> FrameSpec {
        FrameSpec {
            kind,
            shading: ShadingMode::Flat,
            color,
            camera: Camera::default(),
            width: 40,
            height: 20,
        }
    }

    #[test]
    fn fr3_4_clamp_dims_enforces_minimum() {
        assert_eq!(clamp_dims(0, 0), (MIN_DIM, MIN_DIM));
        assert_eq!(clamp_dims(1, 2), (MIN_DIM, MIN_DIM));
        assert_eq!(clamp_dims(80, 24), (80, 24));
    }

    #[test]
    fn fr2_9_ascii_solid_is_plain_text_of_full_height() {
        let f = render(
            &cube(),
            Mat4::IDENTITY,
            spec(RenderKind::Solid, ColorMode::Ascii),
        );
        assert_eq!(f.lines.len(), 20);
        assert!(!f.reset);
        assert!(
            !f.headless_text().contains('\x1b'),
            "ascii must be escape-free"
        );
    }

    #[test]
    fn fr2_9_truecolor_has_ansi_and_reset() {
        let f = render(
            &cube(),
            Mat4::IDENTITY,
            spec(RenderKind::Solid, ColorMode::Truecolor),
        );
        assert_eq!(f.lines.len(), 20);
        assert!(f.reset);
        assert!(
            f.headless_text().contains("\x1b[38;2;"),
            "expected truecolor fg"
        );
        assert!(f.headless_text().ends_with("\x1b[0m"));
    }

    #[test]
    fn fr2_9_halfblock_renders_one_cell_row_per_two_pixels() {
        let f = render(
            &cube(),
            Mat4::IDENTITY,
            spec(RenderKind::Solid, ColorMode::HalfBlock),
        );
        // Rendered at 2× height, packed back to `height` cell rows.
        assert_eq!(f.lines.len(), 20);
        assert!(f.headless_text().contains('▀'));
    }

    #[test]
    fn fr2_9_wireframe_matches_phase1_text() {
        let f = render(
            &cube(),
            Mat4::IDENTITY,
            spec(RenderKind::Wireframe, ColorMode::Ascii),
        );
        let direct =
            render_wireframe(&cube(), Mat4::IDENTITY, &Camera::default(), 40, 20).to_string();
        assert_eq!(
            f.headless_text(),
            direct,
            "wireframe headless unchanged from Phase 1"
        );
    }
}
