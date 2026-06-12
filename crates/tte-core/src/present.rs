//! Presenters: a [`Framebuffer`] → terminal output (FR-2.6–2.8).
//!
//! These are the swappable output backends from the project brief (D4). All are
//! pure functions producing values (a `CellBuffer` or a `String`), so they are
//! snapshot- and byte-level testable without a terminal. ANSI escape *emission*
//! to a real TTY stays in the CLI's presenter (tte-cli).

use crate::cell::CellBuffer;
use crate::color::Rgb;
use crate::framebuffer::Framebuffer;

/// Luminance ramp, dimmest → brightest (donut.c lineage,
/// docs/research/02-ascii-terminal-rendering.md). Index 0 is the background.
pub const ASCII_RAMP: &[u8] = b" .,-~:;=!*#$@";

/// FR-2.6 — map each pixel's luminance to a ramp glyph. Background pixels
/// (luminance 0) become spaces. Pure plain text: feeds the golden-frame path.
pub fn ascii_ramp(fb: &Framebuffer) -> CellBuffer {
    let mut buf = CellBuffer::new(fb.width(), fb.height());
    for (y, row) in fb.rows().enumerate() {
        for (x, &color) in row.iter().enumerate() {
            buf.put(x as i32, y as i32, ramp_glyph(color));
        }
    }
    buf
}

fn ramp_glyph(color: Rgb) -> char {
    let lum = color.luminance().clamp(0.0, 1.0);
    // Round to nearest ramp slot; luminance 0 → space.
    let idx = (lum * (ASCII_RAMP.len() - 1) as f32 + 0.5) as usize;
    ASCII_RAMP[idx.min(ASCII_RAMP.len() - 1)] as char
}

/// FR-2.7 — one full-block glyph per pixel, colored with a 24-bit foreground.
/// SGR codes are emitted only when the color changes (NFR-6 run-merging); each
/// row is newline-terminated and the stream ends with a reset.
pub fn truecolor(fb: &Framebuffer) -> String {
    let mut out = String::new();
    let mut pen: Option<Rgb> = None;
    for row in fb.rows() {
        for &color in row {
            set_fg(&mut out, &mut pen, color);
            out.push('█');
        }
        out.push('\n');
    }
    out.push_str("\x1b[0m");
    out
}

/// FR-2.8 — two vertical sub-pixels per cell via the upper-half block `▀`
/// (foreground = upper pixel, background = lower pixel). Doubles vertical
/// resolution at full per-sub-pixel color; the framebuffer must have an even
/// height (the caller renders at 2× cell rows). An odd final row pairs against
/// the background.
pub fn half_block(fb: &Framebuffer) -> String {
    let rows: Vec<&[Rgb]> = fb.rows().collect();
    let mut out = String::new();
    let (mut fg, mut bg): (Option<Rgb>, Option<Rgb>) = (None, None);
    for pair in rows.chunks(2) {
        let top = pair[0];
        let bottom = pair.get(1).copied();
        for x in 0..top.len() {
            let upper = top[x];
            let lower = bottom.map_or(fb.background(), |b| b[x]);
            set_fg(&mut out, &mut fg, upper);
            set_bg(&mut out, &mut bg, lower);
            out.push('▀');
        }
        out.push('\n');
    }
    out.push_str("\x1b[0m");
    out
}

fn set_fg(out: &mut String, pen: &mut Option<Rgb>, color: Rgb) {
    if *pen != Some(color) {
        out.push_str(&format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b));
        *pen = Some(color);
    }
}

fn set_bg(out: &mut String, pen: &mut Option<Rgb>, color: Rgb) {
    if *pen != Some(color) {
        out.push_str(&format!("\x1b[48;2;{};{};{}m", color.r, color.g, color.b));
        *pen = Some(color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_fb(w: u16, h: u16, color: Rgb) -> Framebuffer {
        let mut fb = Framebuffer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                fb.plot(i32::from(x), i32::from(y), 0.0, color);
            }
        }
        fb
    }

    #[test]
    fn fr2_6_ascii_ramp_maps_brightness_to_glyphs() {
        assert_eq!(ramp_glyph(Rgb::BLACK), ' ');
        assert_eq!(ramp_glyph(Rgb::WHITE), '@');
        // A mid-gray lands somewhere in the middle of the ramp.
        let mid = ramp_glyph(Rgb::new(128, 128, 128));
        assert!(ASCII_RAMP.contains(&(mid as u8)) && mid != ' ' && mid != '@');
    }

    #[test]
    fn fr2_6_ascii_ramp_shape_matches_framebuffer() {
        let fb = solid_fb(4, 2, Rgb::WHITE);
        assert_eq!(ascii_ramp(&fb).to_string(), "@@@@\n@@@@\n");
    }

    #[test]
    fn fr2_7_truecolor_emits_fg_then_block_and_resets() {
        let out = truecolor(&solid_fb(2, 1, Rgb::new(10, 20, 30)));
        assert!(out.starts_with("\x1b[38;2;10;20;30m█"), "got {out:?}");
        assert!(out.ends_with("\x1b[0m"));
    }

    #[test]
    fn nfr6_truecolor_run_merges_identical_colors() {
        // Three identical pixels → exactly one SGR foreground code.
        let out = truecolor(&solid_fb(3, 1, Rgb::new(1, 2, 3)));
        assert_eq!(
            out.matches("\x1b[38;2;").count(),
            1,
            "should not re-emit unchanged fg"
        );
        assert_eq!(out.matches('█').count(), 3);
    }

    #[test]
    fn fr2_8_half_block_pairs_two_rows_into_one() {
        // 2 framebuffer rows → 1 output line of '▀' cells.
        let mut fb = Framebuffer::new(2, 2);
        for x in 0..2 {
            fb.plot(x, 0, 0.0, Rgb::new(255, 0, 0)); // upper → fg
            fb.plot(x, 1, 0.0, Rgb::new(0, 0, 255)); // lower → bg
        }
        let out = half_block(&fb);
        assert_eq!(out.matches('▀').count(), 2);
        assert_eq!(out.matches('\n').count(), 1, "two fb rows → one cell row");
        assert!(out.contains("\x1b[38;2;255;0;0m"), "upper as fg: {out:?}");
        assert!(out.contains("\x1b[48;2;0;0;255m"), "lower as bg: {out:?}");
    }

    #[test]
    fn fr2_8_odd_height_pairs_last_row_against_background() {
        let fb = solid_fb(1, 1, Rgb::WHITE); // single row, bg black
        let out = half_block(&fb);
        assert_eq!(out.matches('▀').count(), 1);
        assert!(
            out.contains("\x1b[48;2;0;0;0m"),
            "background as bg: {out:?}"
        );
    }
}
