//! The cell buffer: a frame as a pure value (FR-1.5).
//!
//! This is the seam the whole test strategy hangs on (docs/02-test-harness.md
//! §4): rendering produces a `CellBuffer`, presentation (ANSI, WASM canvas)
//! is someone else's job, so frames are assertable without any terminal.

/// A width×height grid of characters. `(0,0)` is the top-left cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellBuffer {
    width: u16,
    height: u16,
    cells: Vec<char>,
}

impl CellBuffer {
    /// A buffer filled with spaces.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; usize::from(width) * usize::from(height)],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Set a cell; out-of-bounds writes are ignored (the rasterizer clips by
    /// bounds-checking, per docs/research/01-engine-architectures.md Q4).
    pub fn put(&mut self, x: i32, y: i32, ch: char) {
        if x >= 0 && y >= 0 && x < i32::from(self.width) && y < i32::from(self.height) {
            self.cells[y as usize * usize::from(self.width) + x as usize] = ch;
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<char> {
        (x < self.width && y < self.height)
            .then(|| self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)])
    }

    /// Rows of the grid, top to bottom, as `String`s.
    pub fn rows(&self) -> impl Iterator<Item = String> + '_ {
        self.cells
            .chunks(usize::from(self.width).max(1))
            .map(|row| row.iter().collect())
    }
}

/// Deterministic plain-text rendering: `height` lines of exactly `width`
/// chars, each terminated by `\n` (FR-1.5; this exact format is what golden
/// snapshots and the headless mode emit).
impl std::fmt::Display for CellBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.rows() {
            f.write_str(&row)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr1_5_display_is_height_lines_of_width_chars() {
        let buf = CellBuffer::new(3, 2);
        assert_eq!(buf.to_string(), "   \n   \n");
    }

    #[test]
    fn fr1_5_put_and_get_round_trip() {
        let mut buf = CellBuffer::new(4, 3);
        buf.put(2, 1, '#');
        assert_eq!(buf.get(2, 1), Some('#'));
        assert_eq!(buf.to_string(), "    \n  # \n    \n");
    }

    #[test]
    fn fr1_5_out_of_bounds_put_is_ignored() {
        let mut buf = CellBuffer::new(2, 2);
        buf.put(-1, 0, '#');
        buf.put(0, -5, '#');
        buf.put(2, 0, '#');
        buf.put(0, 2, '#');
        assert_eq!(buf.to_string(), "  \n  \n");
    }
}
