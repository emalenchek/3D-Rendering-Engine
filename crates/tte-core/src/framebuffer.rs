//! Color + depth framebuffer with a z-buffer (FR-2.2).
//!
//! The shaded counterpart to [`crate::cell::CellBuffer`]: the rasterizer writes
//! depth-tested colored fragments here, and a presenter (FR-2.6–2.8) turns it
//! into characters/ANSI. Keeping color separate from glyphs is the swappable
//! output seam from the project brief (D4).

use crate::color::Rgb;

/// A width×height grid of colored pixels, each with a depth for visibility.
/// Depth uses NDC z convention: smaller is nearer; cleared to `+∞`.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    width: u16,
    height: u16,
    color: Vec<Rgb>,
    depth: Vec<f32>,
    background: Rgb,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self::with_background(width, height, Rgb::BLACK)
    }

    pub fn with_background(width: u16, height: u16, background: Rgb) -> Self {
        let len = usize::from(width) * usize::from(height);
        Self {
            width,
            height,
            color: vec![background; len],
            depth: vec![f32::INFINITY; len],
            background,
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn background(&self) -> Rgb {
        self.background
    }

    /// Color at `(x, y)`, or `None` out of bounds.
    pub fn color_at(&self, x: u16, y: u16) -> Option<Rgb> {
        self.index(i32::from(x), i32::from(y))
            .map(|i| self.color[i])
    }

    /// Depth at `(x, y)`, or `None` out of bounds.
    pub fn depth_at(&self, x: u16, y: u16) -> Option<f32> {
        self.index(i32::from(x), i32::from(y))
            .map(|i| self.depth[i])
    }

    /// Depth-tested write: sets the pixel to `color` only if `depth` is nearer
    /// (strictly less) than the stored depth. Out-of-bounds writes are ignored.
    /// Returns whether the pixel was written (useful in tests).
    pub fn plot(&mut self, x: i32, y: i32, depth: f32, color: Rgb) -> bool {
        match self.index(x, y) {
            Some(i) if depth < self.depth[i] => {
                self.depth[i] = depth;
                self.color[i] = color;
                true
            }
            _ => false,
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < i32::from(self.width) && y < i32::from(self.height))
            .then(|| y as usize * usize::from(self.width) + x as usize)
    }

    /// Rows of colors, top to bottom — the access pattern presenters use.
    pub fn rows(&self) -> impl Iterator<Item = &[Rgb]> {
        self.color.chunks(usize::from(self.width).max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr2_2_nearer_fragment_wins_regardless_of_draw_order() {
        let mut fb = Framebuffer::new(1, 1);
        assert!(fb.plot(0, 0, 5.0, Rgb::WHITE), "first write always passes");
        assert!(fb.plot(0, 0, 2.0, Rgb::new(255, 0, 0)), "nearer overwrites");
        assert!(
            !fb.plot(0, 0, 9.0, Rgb::new(0, 255, 0)),
            "farther is rejected"
        );
        assert_eq!(fb.color_at(0, 0), Some(Rgb::new(255, 0, 0)));
        assert_eq!(fb.depth_at(0, 0), Some(2.0));
    }

    #[test]
    fn fr2_2_equal_depth_does_not_overwrite() {
        let mut fb = Framebuffer::new(1, 1);
        fb.plot(0, 0, 3.0, Rgb::WHITE);
        assert!(!fb.plot(0, 0, 3.0, Rgb::BLACK), "strict less-than test");
    }

    #[test]
    fn fr2_2_out_of_bounds_is_ignored() {
        let mut fb = Framebuffer::new(2, 2);
        assert!(!fb.plot(-1, 0, 0.0, Rgb::WHITE));
        assert!(!fb.plot(2, 2, 0.0, Rgb::WHITE));
        assert_eq!(fb.color_at(0, 0), Some(Rgb::BLACK));
    }
}
