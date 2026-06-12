//! Interactive render loop (FR-1.7): fixed-timestep spin, key handling,
//! guaranteed terminal restore.

use crate::ROTATION_STEP_RAD;
use crate::present;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::io::Write;
use std::time::{Duration, Instant};
use tte_core::{Camera, Mat4, Mesh, render_wireframe};

/// Target frame rate. 30 FPS is the portable floor identified in
/// docs/research/02-ascii-terminal-rendering.md §5.
const TARGET_FPS: u64 = 30;
const TILT_RAD: f32 = 0.35;

/// Restores the terminal even on panic/early return (RAII): raw mode off,
/// cursor + main screen back.
struct TerminalGuard;

impl TerminalGuard {
    fn enter(out: &mut impl Write) -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        present::enter(out)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let _ = present::leave(&mut stdout);
        let _ = terminal::disable_raw_mode();
    }
}

/// Advance the model rotation by one fixed timestep. Pure — unit-tested for
/// determinism (fr1_7); the loop below is exercised by the PTY smoke test.
pub fn step_rotation(frame_index: u64) -> Mat4 {
    Mat4::rotation_y(frame_index as f32 * ROTATION_STEP_RAD) * Mat4::rotation_x(TILT_RAD)
}

/// True if this key means "quit" (q, Esc, or Ctrl-C — FR-1.7).
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// Run the interactive viewer until the user quits. Frame size follows the
/// terminal; the camera is the canonical default.
pub fn run(mesh: &Mesh) -> std::io::Result<()> {
    let mut out = std::io::BufWriter::new(std::io::stdout());
    let _guard = TerminalGuard::enter(&mut out)?;
    let camera = Camera::default();
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);

    let mut frame_index: u64 = 0;
    loop {
        let frame_start = Instant::now();
        let (width, height) = terminal::size().unwrap_or((80, 24));
        let frame = render_wireframe(mesh, step_rotation(frame_index), &camera, width, height);
        present::present_frame(&mut out, &frame)?;
        frame_index += 1;

        // Spend the rest of the frame budget polling for input; this is also
        // the frame pacing (poll blocks up to the timeout).
        let elapsed = frame_start.elapsed();
        let wait = frame_budget.saturating_sub(elapsed);
        if event::poll(wait)?
            && let Event::Key(key) = event::read()?
            && is_quit_key(&key)
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr1_7_rotation_step_is_deterministic() {
        assert_eq!(step_rotation(17), step_rotation(17));
        assert_ne!(step_rotation(17), step_rotation(18));
    }

    #[test]
    fn fr1_7_quit_keys_are_q_esc_and_ctrl_c() {
        let plain = |code| KeyEvent::new(code, KeyModifiers::NONE);
        assert!(is_quit_key(&plain(KeyCode::Char('q'))));
        assert!(is_quit_key(&plain(KeyCode::Esc)));
        assert!(is_quit_key(&KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
        assert!(!is_quit_key(&plain(KeyCode::Char('c'))));
        assert!(!is_quit_key(&plain(KeyCode::Char('w'))));
    }
}
