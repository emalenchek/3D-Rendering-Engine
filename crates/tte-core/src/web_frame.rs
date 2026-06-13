//! Browser-frame export (FR-5.2): a [`Framebuffer`] → a compact per-cell
//! `{glyph, fg, bg}` grid the browser frontend draws with a glyph atlas.
//!
//! This is the structured counterpart to the terminal presenters in
//! [`crate::present`]: instead of ANSI strings it produces three flat arrays the
//! WASM layer hands to JS as typed arrays (no per-cell boundary calls — research
//! report 08). Pure core code, so native and WASM produce identical frames
//! (NFR-9) and it is unit-tested without a browser.

use crate::framebuffer::Framebuffer;
use crate::present::ASCII_RAMP;

/// Full block `█` (U+2588) — one solid colored pixel per cell.
pub const GLYPH_FULL_BLOCK: u16 = 0x2588;
/// Upper half block `▀` (U+2580) — fg = upper sub-pixel, bg = lower (half-block mode).
pub const GLYPH_UPPER_HALF: u16 = 0x2580;

/// Which character a cell draws, mirroring the terminal output modes (FR-2.6–2.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebMode {
    /// Luminance-ramp glyph, colored by the cell — colored ASCII art.
    #[default]
    Ascii,
    /// Full block per cell, colored by the cell — solid "pixels".
    Truecolor,
    /// Upper-half block; two framebuffer rows pack into one cell row (2× vertical
    /// resolution). The framebuffer must have been rendered at double height.
    HalfBlock,
}

/// A browser frame as three parallel, flat arrays indexed by `y * width + x`:
/// `glyphs` (one Unicode scalar per cell), and `fg`/`bg` (3 bytes RGB per cell).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebFrame {
    width: u16,
    height: u16,
    glyphs: Vec<u16>,
    fg: Vec<u8>,
    bg: Vec<u8>,
}

impl WebFrame {
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// One Unicode scalar (codepoint) per cell, `width * height` entries.
    pub fn glyphs(&self) -> &[u16] {
        &self.glyphs
    }

    /// Foreground RGB, 3 bytes per cell (`r, g, b` interleaved).
    pub fn fg(&self) -> &[u8] {
        &self.fg
    }

    /// Background RGB, 3 bytes per cell.
    pub fn bg(&self) -> &[u8] {
        &self.bg
    }

    /// `(glyph char, fg, bg)` at a cell — for tests/inspection.
    pub fn cell(&self, x: u16, y: u16) -> Option<(char, crate::Rgb, crate::Rgb)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = usize::from(y) * usize::from(self.width) + usize::from(x);
        let glyph = char::from_u32(u32::from(self.glyphs[i])).unwrap_or('\u{FFFD}');
        let rgb = |buf: &[u8]| crate::Rgb::new(buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);
        Some((glyph, rgb(&self.fg), rgb(&self.bg)))
    }
}

/// Export a framebuffer for the browser in the given mode (FR-5.2).
pub fn web_frame(fb: &Framebuffer, mode: WebMode) -> WebFrame {
    match mode {
        WebMode::Ascii => glyph_per_pixel(fb, ramp_glyph_code),
        WebMode::Truecolor => glyph_per_pixel(fb, |_| GLYPH_FULL_BLOCK),
        WebMode::HalfBlock => half_block_frame(fb),
    }
}

/// One cell per framebuffer pixel; the cell's glyph is chosen by `glyph_of`, its
/// fg is the pixel color, its bg is the framebuffer background.
fn glyph_per_pixel(fb: &Framebuffer, glyph_of: impl Fn(crate::Rgb) -> u16) -> WebFrame {
    let (w, h) = (fb.width(), fb.height());
    let count = usize::from(w) * usize::from(h);
    let mut glyphs = Vec::with_capacity(count);
    let mut fg = Vec::with_capacity(count * 3);
    let mut bg = Vec::with_capacity(count * 3);
    let bgc = fb.background();
    for row in fb.rows() {
        for &color in row {
            glyphs.push(glyph_of(color));
            fg.extend_from_slice(&[color.r, color.g, color.b]);
            bg.extend_from_slice(&[bgc.r, bgc.g, bgc.b]);
        }
    }
    WebFrame {
        width: w,
        height: h,
        glyphs,
        fg,
        bg,
    }
}

/// Pack two framebuffer rows into one cell row using the upper-half block:
/// fg = upper pixel, bg = lower pixel. An odd final row pairs against background.
fn half_block_frame(fb: &Framebuffer) -> WebFrame {
    let w = fb.width();
    let cell_rows = fb.height().div_ceil(2);
    let count = usize::from(w) * usize::from(cell_rows);
    let mut glyphs = vec![GLYPH_UPPER_HALF; count];
    let mut fg = Vec::with_capacity(count * 3);
    let mut bg = Vec::with_capacity(count * 3);
    let bgc = fb.background();
    for cy in 0..cell_rows {
        for x in 0..w {
            let upper = fb.color_at(x, cy * 2).unwrap_or(bgc);
            let lower = fb.color_at(x, cy * 2 + 1).unwrap_or(bgc);
            fg.extend_from_slice(&[upper.r, upper.g, upper.b]);
            bg.extend_from_slice(&[lower.r, lower.g, lower.b]);
        }
    }
    // glyphs filled with GLYPH_UPPER_HALF; replace the trailing logic if blank
    // cells (both bg) should differ — not needed: a ▀ with fg==bg reads solid.
    glyphs.truncate(count);
    WebFrame {
        width: w,
        height: cell_rows,
        glyphs,
        fg,
        bg,
    }
}

fn ramp_glyph_code(color: crate::Rgb) -> u16 {
    let lum = color.luminance().clamp(0.0, 1.0);
    let idx = (lum * (ASCII_RAMP.len() - 1) as f32 + 0.5) as usize;
    u16::from(ASCII_RAMP[idx.min(ASCII_RAMP.len() - 1)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rgb;

    fn solid_fb(w: u16, h: u16, color: Rgb, bg: Rgb) -> Framebuffer {
        let mut fb = Framebuffer::with_background(w, h, bg);
        for y in 0..h {
            for x in 0..w {
                fb.plot(i32::from(x), i32::from(y), 0.0, color);
            }
        }
        fb
    }

    #[test]
    fn fr5_2_array_lengths_match_grid() {
        let fb = solid_fb(4, 3, Rgb::WHITE, Rgb::BLACK);
        let frame = web_frame(&fb, WebMode::Truecolor);
        assert_eq!(frame.width(), 4);
        assert_eq!(frame.height(), 3);
        assert_eq!(frame.glyphs().len(), 12);
        assert_eq!(frame.fg().len(), 12 * 3);
        assert_eq!(frame.bg().len(), 12 * 3);
    }

    #[test]
    fn fr5_2_truecolor_is_full_block_with_pixel_fg() {
        let fb = solid_fb(2, 1, Rgb::new(10, 20, 30), Rgb::new(1, 2, 3));
        let frame = web_frame(&fb, WebMode::Truecolor);
        let (glyph, fg, bg) = frame.cell(0, 0).unwrap();
        assert_eq!(glyph, '█');
        assert_eq!(fg, Rgb::new(10, 20, 30));
        assert_eq!(bg, Rgb::new(1, 2, 3));
    }

    #[test]
    fn fr5_2_ascii_picks_ramp_glyph_by_luminance() {
        let white = web_frame(&solid_fb(1, 1, Rgb::WHITE, Rgb::BLACK), WebMode::Ascii);
        assert_eq!(white.cell(0, 0).unwrap().0, '@');
        let black = web_frame(&solid_fb(1, 1, Rgb::BLACK, Rgb::BLACK), WebMode::Ascii);
        assert_eq!(black.cell(0, 0).unwrap().0, ' ');
    }

    #[test]
    fn fr5_2_half_block_pairs_rows_into_half_height() {
        let mut fb = Framebuffer::new(2, 4);
        for x in 0..2 {
            for y in 0..4 {
                let c = if y % 2 == 0 {
                    Rgb::new(255, 0, 0)
                } else {
                    Rgb::new(0, 0, 255)
                };
                fb.plot(x, y, 0.0, c);
            }
        }
        let frame = web_frame(&fb, WebMode::HalfBlock);
        assert_eq!(frame.height(), 2, "4 pixel rows → 2 cell rows");
        let (glyph, fg, bg) = frame.cell(0, 0).unwrap();
        assert_eq!(glyph, '▀');
        assert_eq!(fg, Rgb::new(255, 0, 0), "upper pixel → fg");
        assert_eq!(bg, Rgb::new(0, 0, 255), "lower pixel → bg");
    }

    #[test]
    fn fr5_2_half_block_odd_height_pairs_last_against_background() {
        let frame = web_frame(
            &solid_fb(1, 1, Rgb::WHITE, Rgb::new(5, 5, 5)),
            WebMode::HalfBlock,
        );
        assert_eq!(frame.height(), 1);
        let (_, fg, bg) = frame.cell(0, 0).unwrap();
        assert_eq!(fg, Rgb::WHITE);
        assert_eq!(bg, Rgb::new(5, 5, 5), "missing lower row → background");
    }
}
