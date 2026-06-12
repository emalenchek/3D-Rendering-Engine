//! `tte-cli` — native terminal frontend for the text-encoded 3D rendering engine.
//!
//! Library + thin `main.rs` shim layout (binary-only crates can't have
//! integration tests; see docs/02-test-harness.md §2).
//!
//! Surface (docs/01-requirements-spec.md §3, §3a):
//! `tte view model.obj` — interactive spinning model (FR-1.7)
//! `tte view --headless --size WxH --frames N model.obj` — deterministic
//! frame dump for tests and pipelines (FR-1.8)
//! `--render solid|wireframe  --shading flat|gouraud  --mode ascii|truecolor|halfblock`
//! select the Phase 2 rendering path (FR-2.9).

pub mod frame;
pub mod interactive;
pub mod present;

use frame::{ColorMode, FrameSpec, RenderKind};
use std::path::PathBuf;
use std::process::ExitCode;
use tte_core::{Camera, OrbitCamera, PITCH_LIMIT, RADIUS_MAX, RADIUS_MIN, ShadingMode};

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
    pub kind: RenderKind,
    pub shading: ShadingMode,
    pub color: ColorMode,
    /// Starting orbit view; `None` means the canonical default framing (and, in
    /// headless mode, the unchanged Phase 1/2 spinning-model dump). `Some` is set
    /// when any of `--yaw/--pitch/--radius` is given (FR-3.3).
    pub orbit: Option<OrbitCamera>,
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
    let mut kind = RenderKind::default();
    let mut shading = ShadingMode::default();
    let mut color = ColorMode::default();
    // Lazily created the first time an orbit flag appears (FR-3.3).
    let mut orbit: Option<OrbitCamera> = None;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--yaw" => match args.next().and_then(|v| v.parse::<f32>().ok()) {
                Some(deg) => orbit.get_or_insert_with(OrbitCamera::default).yaw = deg.to_radians(),
                None => return usage("--yaw expects a number in degrees"),
            },
            "--pitch" => match args.next().and_then(|v| v.parse::<f32>().ok()) {
                Some(deg) => {
                    let p = deg.to_radians().clamp(-PITCH_LIMIT, PITCH_LIMIT);
                    orbit.get_or_insert_with(OrbitCamera::default).pitch = p;
                }
                None => return usage("--pitch expects a number in degrees"),
            },
            "--radius" => match args.next().and_then(|v| v.parse::<f32>().ok()) {
                Some(r) if r > 0.0 => {
                    let r = r.clamp(RADIUS_MIN, RADIUS_MAX);
                    orbit.get_or_insert_with(OrbitCamera::default).radius = r;
                }
                _ => return usage("--radius expects a positive number"),
            },
            "--size" => match args.next().as_deref().map(parse_size) {
                Some(Some(parsed)) => size = Some(parsed),
                _ => return usage("--size expects WxH, e.g. --size 80x24"),
            },
            "--frames" => match args.next().and_then(|n| n.parse().ok()) {
                Some(n) if n > 0 => frames = n,
                _ => return usage("--frames expects a positive integer"),
            },
            "--render" => match args.next().as_deref() {
                Some("solid") => kind = RenderKind::Solid,
                Some("wireframe") => kind = RenderKind::Wireframe,
                _ => return usage("--render expects 'solid' or 'wireframe'"),
            },
            "--shading" => match args.next().as_deref() {
                Some("flat") => shading = ShadingMode::Flat,
                Some("gouraud") => shading = ShadingMode::Gouraud,
                _ => return usage("--shading expects 'flat' or 'gouraud'"),
            },
            "--mode" => match args.next().as_deref() {
                Some("ascii") => color = ColorMode::Ascii,
                Some("truecolor") => color = ColorMode::Truecolor,
                Some("halfblock") => color = ColorMode::HalfBlock,
                _ => return usage("--mode expects 'ascii', 'truecolor', or 'halfblock'"),
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
            kind,
            shading,
            color,
            orbit,
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

/// Build the [`FrameSpec`] for a view, given the resolved cell-grid size.
/// With no orbit flags the camera is the canonical default (so the Phase 1/2
/// headless dumps are byte-for-byte unchanged); `--yaw/--pitch/--radius` switch
/// to the requested orbit view (FR-3.3).
fn frame_spec(opts: &ViewOptions, width: u16, height: u16) -> FrameSpec {
    let camera = opts.orbit.map_or_else(Camera::default, |o| o.to_camera());
    FrameSpec {
        kind: opts.kind,
        shading: opts.shading,
        color: opts.color,
        camera,
        width,
        height,
    }
}

fn view(opts: &ViewOptions) -> Result<(), Box<dyn std::error::Error>> {
    let mesh = tte_core::load_obj(&opts.scene)?;
    if opts.headless {
        // FR-1.8 / FR-2.9 / FR-3.3: deterministic frame dump. Frame i uses the
        // same rotation step as interactive frame i; the camera is fixed.
        let (width, height) = opts.size.unwrap_or((80, 24));
        let spec = frame_spec(opts, width, height);
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        use std::io::Write;
        for i in 0..u64::from(opts.frames) {
            let rendered = frame::render(&mesh, interactive::step_rotation(i), spec);
            write!(out, "{}", rendered.headless_text())?;
        }
        out.flush()?;
        Ok(())
    } else {
        Ok(interactive::run(&mesh, opts)?)
    }
}

/// Help text as a pure value so tests can snapshot it (see tests/e2e_cli.rs).
pub fn help_text() -> String {
    "tte — text-encoded 3D rendering engine\n\
     \n\
     USAGE:\n\
     \x20   tte view [OPTIONS] <scene.obj>    Orbit a model interactively\n\
     \x20   tte [OPTIONS]\n\
     \n\
     VIEW OPTIONS:\n\
     \x20   --render KIND    solid (default) or wireframe\n\
     \x20   --shading MODE   flat (default) or gouraud (solid only)\n\
     \x20   --mode OUTPUT    ascii (default), truecolor, or halfblock (solid only)\n\
     \x20   --yaw DEG        Initial orbit azimuth in degrees\n\
     \x20   --pitch DEG      Initial orbit elevation in degrees\n\
     \x20   --radius F       Initial orbit distance from the model\n\
     \x20   --headless       Dump frames to stdout (no terminal control)\n\
     \x20   --size WxH       Cell grid size (default: terminal size, or 80x24 headless)\n\
     \x20   --frames N       Number of frames to dump in headless mode (default: 1)\n\
     \n\
     KEYS (interactive):\n\
     \x20   arrows / hjkl    Orbit the camera\n\
     \x20   + = i  /  - o    Zoom in / out\n\
     \x20   space            Toggle auto-orbit\n\
     \x20   r                Reset the view\n\
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
            "--render",
            "solid",
            "--shading",
            "gouraud",
            "--mode",
            "truecolor",
            "cube.obj",
        ]);
        assert_eq!(
            inv,
            Invocation::View(ViewOptions {
                scene: PathBuf::from("cube.obj"),
                headless: true,
                size: Some((100, 30)),
                frames: 5,
                kind: RenderKind::Solid,
                shading: ShadingMode::Gouraud,
                color: ColorMode::Truecolor,
                orbit: None,
            })
        );
    }

    #[test]
    fn fr3_3_orbit_flags_build_an_orbit_camera() {
        let inv = parse(&[
            "view", "--yaw", "90", "--pitch", "30", "--radius", "8", "cube.obj",
        ]);
        let Invocation::View(opts) = inv else {
            panic!("expected view")
        };
        let orbit = opts.orbit.expect("orbit flags should populate orbit");
        assert!((orbit.yaw - 90f32.to_radians()).abs() < 1e-5);
        assert!((orbit.pitch - 30f32.to_radians()).abs() < 1e-5);
        assert!((orbit.radius - 8.0).abs() < 1e-5);
    }

    #[test]
    fn fr3_3_pitch_flag_is_clamped() {
        let Invocation::View(opts) = parse(&["view", "--pitch", "200", "cube.obj"]) else {
            panic!("expected view")
        };
        assert!((opts.orbit.unwrap().pitch - PITCH_LIMIT).abs() < 1e-5);
    }

    #[test]
    fn fr3_3_bad_orbit_values_are_usage_errors() {
        for bad in [
            &["view", "--yaw", "left", "c.obj"][..],
            &["view", "--pitch", "up", "c.obj"][..],
            &["view", "--radius", "-3", "c.obj"][..],
            &["view", "--radius", "0", "c.obj"][..],
        ] {
            assert!(
                matches!(parse(bad), Invocation::Usage(_)),
                "expected usage error for {bad:?}"
            );
        }
    }

    #[test]
    fn fr2_9_view_defaults_are_interactive_solid_flat_ascii() {
        let inv = parse(&["view", "cube.obj"]);
        assert_eq!(
            inv,
            Invocation::View(ViewOptions {
                scene: PathBuf::from("cube.obj"),
                headless: false,
                size: None,
                frames: 1,
                kind: RenderKind::Solid,
                shading: ShadingMode::Flat,
                color: ColorMode::Ascii,
                orbit: None,
            })
        );
    }

    #[test]
    fn fr2_9_bad_enum_values_are_usage_errors() {
        for bad in [
            &["view", "--render", "fancy", "c.obj"][..],
            &["view", "--shading", "phong", "c.obj"][..],
            &["view", "--mode", "sixel", "c.obj"][..],
        ] {
            assert!(
                matches!(parse(bad), Invocation::Usage(_)),
                "expected usage error for {bad:?}"
            );
        }
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
