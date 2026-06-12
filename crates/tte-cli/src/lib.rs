//! `tte-cli` — native terminal frontend for the text-encoded 3D rendering engine.
//!
//! Library + thin `main.rs` shim layout (binary-only crates can't have
//! integration tests; see docs/02-test-harness.md §2).
//!
//! Phase 1 surface (docs/01-requirements-spec.md §3):
//! `tte view model.obj` — interactive spinning wireframe (FR-1.7)
//! `tte view --headless --size WxH --frames N model.obj` — deterministic
//! plain-text frame dump for tests and pipelines (FR-1.8).

pub mod interactive;
pub mod present;

use std::path::PathBuf;
use std::process::ExitCode;
use tte_core::{Camera, render_wireframe};

/// Model spin per frame: 3° (a full turn every 120 frames / 4 s at 30 FPS).
/// Shared by interactive and headless modes so frame N is the same picture in
/// both (NFR-1).
pub const ROTATION_STEP_RAD: f32 = std::f32::consts::TAU / 120.0;

/// Parsed result of a CLI invocation, as a pure value (no I/O until `run`).
#[derive(Debug, PartialEq)]
pub enum Invocation {
    Version,
    Help,
    View(ViewOptions),
    /// Unusable command line, with a message for stderr.
    Usage(String),
}

#[derive(Debug, PartialEq)]
pub struct ViewOptions {
    pub scene: PathBuf,
    pub headless: bool,
    /// Cell grid size; `None` in interactive mode means "follow the terminal".
    pub size: Option<(u16, u16)>,
    pub frames: u32,
}

/// Decide what an argument list means. Pure function: unit-testable.
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Invocation {
    let mut args = args.into_iter().peekable();
    match args.next().as_deref() {
        Some("--version" | "-V") => Invocation::Version,
        Some("--help" | "-h") | None => Invocation::Help,
        Some("view") => parse_view_args(args),
        Some(other) => Invocation::Usage(format!("unknown argument '{other}'")),
    }
}

fn parse_view_args(args: impl Iterator<Item = String>) -> Invocation {
    let mut headless = false;
    let mut size: Option<(u16, u16)> = None;
    let mut frames: u32 = 1;
    let mut scene: Option<PathBuf> = None;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--size" => match args.next().as_deref().map(parse_size) {
                Some(Some(parsed)) => size = Some(parsed),
                _ => return usage("--size expects WxH, e.g. --size 80x24"),
            },
            "--frames" => match args.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => frames = n,
                _ => return usage("--frames expects a positive integer"),
            },
            flag if flag.starts_with('-') => {
                return usage(format!("unknown argument '{flag}'"));
            }
            path if scene.is_none() => scene = Some(PathBuf::from(path)),
            extra => return usage(format!("unexpected extra argument '{extra}'")),
        }
    }

    match scene {
        Some(scene) => Invocation::View(ViewOptions {
            scene,
            headless,
            size,
            frames,
        }),
        None => usage("view: missing path to an .obj scene"),
    }
}

fn usage(message: impl Into<String>) -> Invocation {
    Invocation::Usage(message.into())
}

/// Parse `"80x24"` → `(80, 24)`. Zero dimensions are rejected.
pub fn parse_size(s: &str) -> Option<(u16, u16)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    let (w, h) = (w.parse().ok()?, h.parse().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// Execute an invocation: print to stdout/stderr and return the exit code.
pub fn run(invocation: Invocation) -> ExitCode {
    match invocation {
        Invocation::Version => {
            println!("tte {}", tte_core::version());
            ExitCode::SUCCESS
        }
        Invocation::Help => {
            println!("{}", help_text());
            ExitCode::SUCCESS
        }
        Invocation::Usage(message) => {
            eprintln!("tte: {message}");
            eprintln!("try 'tte --help'");
            ExitCode::FAILURE
        }
        Invocation::View(opts) => match view(&opts) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("tte: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn view(opts: &ViewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mesh = tte_core::load_obj(&opts.scene)?;
    if opts.headless {
        // FR-1.8: plain text, no ANSI, deterministic. Frame i uses the same
        // rotation step as interactive frame i.
        let (width, height) = opts.size.unwrap_or((80, 24));
        let camera = Camera::default();
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        for i in 0..u64::from(opts.frames) {
            let frame =
                render_wireframe(&mesh, interactive::step_rotation(i), &camera, width, height);
            use std::io::Write;
            write!(out, "{frame}")?;
        }
        use std::io::Write;
        out.flush()?;
        Ok(())
    } else {
        Ok(interactive::run(&mesh)?)
    }
}

/// Help text as a pure value so tests can snapshot it (see tests/e2e_cli.rs).
pub fn help_text() -> String {
    "tte — text-encoded 3D rendering engine\n\
     \n\
     USAGE:\n\
     \x20   tte view [OPTIONS] <scene.obj>    View a model as a spinning wireframe\n\
     \x20   tte [OPTIONS]\n\
     \n\
     VIEW OPTIONS:\n\
     \x20   --headless       Dump frames as plain text to stdout (no terminal control)\n\
     \x20   --size WxH       Cell grid size (default: terminal size, or 80x24 headless)\n\
     \x20   --frames N       Number of frames to dump in headless mode (default: 1)\n\
     \n\
     KEYS (interactive):\n\
     \x20   q, Esc, Ctrl-C   Quit\n\
     \n\
     OPTIONS:\n\
     \x20   -V, --version    Print version information\n\
     \x20   -h, --help       Print this help"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Invocation {
        parse_args(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_args_means_help() {
        assert_eq!(parse(&[]), Invocation::Help);
    }

    #[test]
    fn version_flags_parse() {
        for flag in ["--version", "-V"] {
            assert_eq!(parse(&[flag]), Invocation::Version);
        }
    }

    #[test]
    fn unknown_arg_is_reported() {
        assert!(matches!(parse(&["--bogus"]), Invocation::Usage(m) if m.contains("--bogus")));
    }

    #[test]
    fn fr1_8_view_headless_parses_all_options() {
        let inv = parse(&[
            "view",
            "--headless",
            "--size",
            "100x30",
            "--frames",
            "5",
            "cube.obj",
        ]);
        assert_eq!(
            inv,
            Invocation::View(ViewOptions {
                scene: PathBuf::from("cube.obj"),
                headless: true,
                size: Some((100, 30)),
                frames: 5,
            })
        );
    }

    #[test]
    fn fr1_8_view_defaults_are_interactive_one_frame_terminal_size() {
        let inv = parse(&["view", "cube.obj"]);
        assert_eq!(
            inv,
            Invocation::View(ViewOptions {
                scene: PathBuf::from("cube.obj"),
                headless: false,
                size: None,
                frames: 1,
            })
        );
    }

    #[test]
    fn fr1_8_view_without_scene_is_usage_error() {
        assert!(matches!(
            parse(&["view", "--headless"]),
            Invocation::Usage(_)
        ));
    }

    #[test]
    fn fr1_8_size_parser_accepts_wxh_rejects_junk() {
        assert_eq!(parse_size("80x24"), Some((80, 24)));
        assert_eq!(parse_size("200X50"), Some((200, 50)));
        for bad in ["80", "x24", "80x", "0x10", "10x0", "80x24x2", "axb"] {
            assert_eq!(parse_size(bad), None, "should reject {bad:?}");
        }
    }
}
