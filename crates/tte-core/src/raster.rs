//! Line rasterization into a cell buffer (FR-1.4).

use crate::cell::CellBuffer;

/// Glyph used for wireframe strokes in Phase 1. Becomes a parameter when
/// shading modes arrive (Phase 2).
pub const STROKE: char = '#';

/// Draw a line segment from `(x0,y0)` to `(x1,y1)` in cell coordinates using
/// Bresenham's algorithm (integer-only, all octants). Endpoints outside the
/// buffer are fine — `CellBuffer::put` bounds-checks each cell.
pub fn draw_line(buf: &mut CellBuffer, x0: i32, y0: i32, x1: i32, y1: i32, ch: char) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let (mut x, mut y) = (x0, y0);
    let mut error = dx + dy;
    loop {
        buf.put(x, y, ch);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * error;
        if e2 >= dy {
            error += dy;
            x += sx;
        }
        if e2 <= dx {
            error += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(w: u16, h: u16, segment: (i32, i32, i32, i32)) -> String {
        let mut buf = CellBuffer::new(w, h);
        let (x0, y0, x1, y1) = segment;
        draw_line(&mut buf, x0, y0, x1, y1, STROKE);
        buf.to_string()
    }

    #[test]
    fn fr1_4_horizontal_line() {
        assert_eq!(grid(5, 3, (0, 1, 4, 1)), "     \n#####\n     \n");
    }

    #[test]
    fn fr1_4_vertical_line() {
        assert_eq!(grid(3, 3, (1, 0, 1, 2)), " # \n # \n # \n");
    }

    #[test]
    fn fr1_4_perfect_diagonal() {
        assert_eq!(grid(3, 3, (0, 0, 2, 2)), "#  \n # \n  #\n");
    }

    #[test]
    fn fr1_4_steep_line_has_no_gaps() {
        // dy > dx: every row between the endpoints must be touched.
        let mut buf = CellBuffer::new(3, 6);
        draw_line(&mut buf, 0, 0, 2, 5, STROKE);
        for (y, row) in buf.rows().enumerate() {
            assert!(row.contains(STROKE), "row {y} empty: {row:?}");
        }
    }

    #[test]
    fn fr1_4_single_point_line() {
        assert_eq!(grid(3, 3, (1, 1, 1, 1)), "   \n # \n   \n");
    }

    #[test]
    fn fr1_4_partially_out_of_bounds_line_is_clipped() {
        // From inside to far outside: draws the in-bounds prefix, no panic.
        let mut buf = CellBuffer::new(3, 3);
        draw_line(&mut buf, 1, 1, 10, 1, STROKE);
        assert_eq!(buf.to_string(), "   \n ##\n   \n");
    }

    #[test]
    fn fr1_4_reversed_line_hits_same_endpoints_and_cell_count() {
        // NOTE: Bresenham is NOT cell-for-cell symmetric under endpoint swap
        // (error tie-breaks depend on direction); the guaranteed invariants
        // are: both endpoints plotted, and the same number of cells touched.
        let count = |s: &str| s.matches(STROKE).count();
        let fwd = grid(5, 5, (0, 0, 4, 2));
        let rev = grid(5, 5, (4, 2, 0, 0));
        assert_eq!(count(&fwd), count(&rev));
        for g in [&fwd, &rev] {
            let rows: Vec<&str> = g.lines().collect();
            assert_eq!(rows[0].as_bytes()[0], STROKE as u8, "missing (0,0):\n{g}");
            assert_eq!(rows[2].as_bytes()[4], STROKE as u8, "missing (4,2):\n{g}");
        }
    }
}
