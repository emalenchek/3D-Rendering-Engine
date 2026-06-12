//! Terminal presentation of cell buffers (FR-1.6).
//!
//! All emission goes through a generic [`std::io::Write`] sink so unit tests
//! can assert the exact byte stream without a PTY (docs/02-test-harness.md §4).
//! Frames are drawn with cursor-home + per-row overwrite — never `Clear` —
//! and one buffered flush per frame (docs/research/02-ascii-terminal-rendering.md §5).

use crossterm::{cursor, queue, style::Print, terminal};
use std::io::Write;
use tte_core::CellBuffer;

/// Enter render mode: alternate screen + hidden cursor.
/// (Raw mode is a process-global toggle, handled by [`super::interactive`].)
pub fn enter(out: &mut impl Write) -> std::io::Result<()> {
    queue!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    out.flush()
}

/// Leave render mode: restore cursor + main screen. Safe to call on a sink
/// that never entered (escape codes are idempotent for our purposes).
pub fn leave(out: &mut impl Write) -> std::io::Result<()> {
    queue!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
    out.flush()
}

/// Draw one frame: cursor-home, overwrite every row in place, single flush.
/// In raw mode `\n` does not return the carriage, so each row is addressed
/// with an explicit `MoveTo`.
pub fn present_frame(out: &mut impl Write, frame: &CellBuffer) -> std::io::Result<()> {
    queue!(out, cursor::MoveTo(0, 0))?;
    for (y, row) in frame.rows().enumerate() {
        queue!(out, cursor::MoveTo(0, y as u16), Print(row))?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr1_6_enter_emits_alt_screen_and_hides_cursor() {
        let mut sink: Vec<u8> = Vec::new();
        enter(&mut sink).unwrap();
        let s = String::from_utf8(sink).unwrap();
        assert!(
            s.contains("\x1b[?1049h"),
            "missing EnterAlternateScreen: {s:?}"
        );
        assert!(s.contains("\x1b[?25l"), "missing cursor Hide: {s:?}");
    }

    #[test]
    fn fr1_6_leave_restores_cursor_and_screen() {
        let mut sink: Vec<u8> = Vec::new();
        leave(&mut sink).unwrap();
        let s = String::from_utf8(sink).unwrap();
        assert!(s.contains("\x1b[?25h"), "missing cursor Show: {s:?}");
        assert!(
            s.contains("\x1b[?1049l"),
            "missing LeaveAlternateScreen: {s:?}"
        );
    }

    #[test]
    fn fr1_6_frame_uses_cursor_home_not_clear() {
        let mut frame = CellBuffer::new(3, 2);
        frame.put(1, 0, '#');
        let mut sink: Vec<u8> = Vec::new();
        present_frame(&mut sink, &frame).unwrap();
        let s = String::from_utf8(sink).unwrap();
        assert!(
            s.starts_with("\x1b[1;1H"),
            "must home the cursor first: {s:?}"
        );
        assert!(s.contains(" # "), "row content missing: {s:?}");
        assert!(!s.contains("\x1b[2J"), "must overwrite, never Clear: {s:?}");
    }

    #[test]
    fn fr1_6_every_row_is_explicitly_addressed() {
        // Raw mode: \n does not imply carriage return, so rows are MoveTo'd.
        let frame = CellBuffer::new(2, 3);
        let mut sink: Vec<u8> = Vec::new();
        present_frame(&mut sink, &frame).unwrap();
        let s = String::from_utf8(sink).unwrap();
        for row in 1..=3 {
            assert!(
                s.contains(&format!("\x1b[{row};1H")),
                "row {row} not addressed: {s:?}"
            );
        }
    }
}
