//! Interactive orbit viewer (FR-3.4, FR-3.5): per-frame render at the current
//! terminal size, orbit-camera key controls, guaranteed terminal restore.

use crate::subject::{self, Subject};
use crate::{ROTATION_STEP_RAD, ViewOptions, frame, present};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};
use tte_core::{Mat4, OrbitCamera};

/// Watches a scene file's modification time and hot-reloads it on change
/// (FR-4.8). For non-scene subjects (`.obj`) or when the file can't be stat'd,
/// `poll` simply never reloads. A reload that fails to parse is ignored (the
/// previous good scene keeps showing).
struct ReloadWatch {
    path: PathBuf,
    enabled: bool,
    last_modified: Option<SystemTime>,
}

impl ReloadWatch {
    fn new(path: &Path, subject: &Subject) -> Self {
        let enabled = matches!(subject, Subject::Scene { .. });
        Self {
            path: path.to_path_buf(),
            enabled,
            last_modified: modified_time(path),
        }
    }

    /// Returns a freshly reloaded subject if the file changed and reparsed.
    fn poll(&mut self) -> Option<Subject> {
        if !self.enabled {
            return None;
        }
        let current = modified_time(&self.path);
        if current != self.last_modified {
            self.last_modified = current;
            return subject::load(&self.path).ok();
        }
        None
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Target frame rate. 30 FPS is the portable floor identified in
/// docs/research/02-ascii-terminal-rendering.md §5.
const TARGET_FPS: u64 = 30;
/// Fixed tilt applied to the headless frame-dump animation (FR-1.7 / FR-2.9).
const TILT_RAD: f32 = 0.35;
/// Radians per keypress (~7°) and per auto-orbit frame.
const ORBIT_STEP: f32 = 0.12;
const AUTO_ORBIT_STEP: f32 = 0.02;
/// Multiplicative zoom per keypress.
const DOLLY_IN: f32 = 0.9;
const DOLLY_OUT: f32 = 1.1;

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

/// Model rotation for headless frame `i` — the deterministic frame-dump
/// animation shared with Phase 1/2 golden frames (NFR-1).
pub fn step_rotation(frame_index: u64) -> Mat4 {
    Mat4::rotation_y(frame_index as f32 * ROTATION_STEP_RAD) * Mat4::rotation_x(TILT_RAD)
}

/// What a keypress means for the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    Quit,
    Continue,
}

/// True if this key means "quit" (q, Esc, or Ctrl-C).
pub fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// Apply a keypress to the orbit state (FR-3.5). Pure except for the `&mut`
/// state it mutates, so it is exhaustively unit-testable without a terminal.
/// `initial` is the view to restore on reset.
pub fn handle_key(
    orbit: &mut OrbitCamera,
    auto_orbit: &mut bool,
    initial: &OrbitCamera,
    key: &KeyEvent,
) -> InputAction {
    if is_quit_key(key) {
        return InputAction::Quit;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => orbit.orbit(0.0, ORBIT_STEP),
        KeyCode::Down | KeyCode::Char('j') => orbit.orbit(0.0, -ORBIT_STEP),
        KeyCode::Left | KeyCode::Char('h') => orbit.orbit(ORBIT_STEP, 0.0),
        KeyCode::Right | KeyCode::Char('l') => orbit.orbit(-ORBIT_STEP, 0.0),
        KeyCode::Char('+' | '=' | 'i') => orbit.dolly(DOLLY_IN),
        KeyCode::Char('-' | 'o') => orbit.dolly(DOLLY_OUT),
        KeyCode::Char(' ') => *auto_orbit = !*auto_orbit,
        KeyCode::Char('r') => *orbit = *initial,
        _ => {}
    }
    InputAction::Continue
}

/// Run the interactive viewer until the user quits. The subject is static and
/// the camera orbits it (auto-orbit on by default); the frame follows the
/// terminal size every iteration so resizes are reflected immediately (FR-3.4).
/// Scene files are watched for edits and hot-reloaded (FR-4.8).
pub fn run(subject: &Subject, opts: &ViewOptions) -> std::io::Result<()> {
    let mut out = std::io::BufWriter::new(std::io::stdout());
    let _guard = TerminalGuard::enter(&mut out)?;
    let frame_budget = Duration::from_micros(1_000_000 / TARGET_FPS);

    let initial = opts.orbit.unwrap_or_default();
    let mut orbit = initial;
    let mut auto_orbit = true;
    let mut reloaded;
    // Hot-reload state for scene files: poll mtime, reparse on change (FR-4.8).
    let mut watch = ReloadWatch::new(&opts.scene, subject);

    loop {
        let frame_start = Instant::now();
        reloaded = watch.poll();
        let subject = reloaded.as_ref().unwrap_or(subject);

        if auto_orbit {
            orbit.orbit(AUTO_ORBIT_STEP, 0.0);
        }
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let (width, height) = frame::clamp_dims(term_w, term_h);
        let spec = frame::FrameSpec {
            kind: opts.kind,
            shading: opts.shading,
            color: opts.color,
            camera: orbit.to_camera(),
            width,
            height,
        };
        // The orbit camera always drives interactively, so a scene can be
        // navigated even if it declares its own camera.
        let rendered = match subject {
            Subject::Mesh(mesh) => frame::render(mesh, Mat4::IDENTITY, spec),
            Subject::Scene { scene, assets } => {
                frame::render_scene_frame(scene, spec, |p| assets.get(p).cloned())
            }
        };
        present::present_lines(&mut out, &rendered.lines, rendered.reset)?;

        // Spend the rest of the frame budget waiting for input; this is also the
        // frame pacing. A resize just falls through and re-renders next frame.
        let wait = frame_budget.saturating_sub(frame_start.elapsed());
        if event::poll(wait)?
            && let Event::Key(key) = event::read()?
            && handle_key(&mut orbit, &mut auto_orbit, &initial, &key) == InputAction::Quit
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tte_core::{PITCH_LIMIT, RADIUS_MAX, RADIUS_MIN};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn fr1_7_rotation_step_is_deterministic() {
        assert_eq!(step_rotation(17), step_rotation(17));
        assert_ne!(step_rotation(17), step_rotation(18));
    }

    #[test]
    fn fr3_5_quit_keys_are_q_esc_and_ctrl_c() {
        let mut o = OrbitCamera::default();
        let mut a = true;
        let init = o;
        for k in [key(KeyCode::Char('q')), key(KeyCode::Esc)] {
            assert_eq!(handle_key(&mut o, &mut a, &init, &k), InputAction::Quit);
        }
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            handle_key(&mut o, &mut a, &init, &ctrl_c),
            InputAction::Quit
        );
        // A plain 'c' is not quit.
        assert_eq!(
            handle_key(&mut o, &mut a, &init, &key(KeyCode::Char('c'))),
            InputAction::Continue
        );
    }

    #[test]
    fn fr3_5_arrows_orbit_the_camera() {
        let mut o = OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        };
        let init = o;
        let mut a = false;
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Up));
        assert!(o.pitch > 0.0, "up should raise pitch");
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Left));
        assert!(o.yaw > 0.0, "left should change yaw");
        // hjkl mirror the arrows.
        let mut o2 = OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        };
        handle_key(&mut o2, &mut a, &init, &key(KeyCode::Char('j')));
        assert!(o2.pitch < 0.0, "j should lower pitch");
    }

    #[test]
    fn fr3_5_zoom_keys_change_radius_within_bounds() {
        let mut o = OrbitCamera::default();
        let init = o;
        let mut a = false;
        let start = o.radius;
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Char('+')));
        assert!(o.radius < start, "+ zooms in");
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Char('-')));
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Char('-')));
        assert!(o.radius > start, "- zooms out");
        assert!((RADIUS_MIN..=RADIUS_MAX).contains(&o.radius));
    }

    #[test]
    fn fr3_5_space_toggles_auto_orbit_and_r_resets() {
        let init = OrbitCamera::default();
        let mut o = init;
        let mut a = true;
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Char(' ')));
        assert!(!a, "space toggles auto-orbit off");
        // Move the camera, then reset.
        o.orbit(1.0, 1.0);
        o.dolly(2.0);
        assert_ne!(o, init);
        handle_key(&mut o, &mut a, &init, &key(KeyCode::Char('r')));
        assert_eq!(o, init, "r restores the initial view");
    }

    #[test]
    fn fr3_5_pitch_stays_clamped_under_repeated_input() {
        let mut o = OrbitCamera {
            pitch: 0.0,
            ..Default::default()
        };
        let init = o;
        let mut a = false;
        for _ in 0..100 {
            handle_key(&mut o, &mut a, &init, &key(KeyCode::Up));
        }
        assert!(o.pitch <= PITCH_LIMIT);
    }
}
