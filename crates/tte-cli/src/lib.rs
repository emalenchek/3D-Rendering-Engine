//! `tte-cli` — native terminal frontend for the text-encoded 3D rendering engine.
//!
//! The crate is split into this library (all logic, unit-testable and linkable
//! from integration tests) and a thin `main.rs` shim — the standard Rust layout
//! for testable binaries (a binary-only crate cannot have integration tests;
//! see docs/02-test-harness.md).
//!
//! Phase 1 staging: today the CLI only resolves `--version`/`--help`, which
//! gives the e2e harness (`assert_cmd` + `insta`) a real, deterministic surface.
//! The render loop, scene loading, and orbit camera arrive during Phase 1, and
//! with them the `--headless` frame-dump mode the golden-frame e2e tests use.

use std::process::ExitCode;

/// Parsed result of a CLI invocation: what to print, and the exit code.
///
/// Kept as a pure value (no I/O) so unit tests can assert on behavior without
/// spawning a process; `run()` performs the actual printing.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    Version,
    Help,
    UnknownArg(String),
}

/// Decide what a given argument list means. Pure function: trivially testable.
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Invocation {
    match args.into_iter().next().as_deref() {
        Some("--version" | "-V") => Invocation::Version,
        Some("--help" | "-h") | None => Invocation::Help,
        Some(other) => Invocation::UnknownArg(other.to_string()),
    }
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
        Invocation::UnknownArg(arg) => {
            eprintln!("tte: unknown argument '{arg}'");
            eprintln!("try 'tte --help'");
            ExitCode::FAILURE
        }
    }
}

/// Help text as a pure value so tests can snapshot it (see tests/e2e_cli.rs).
pub fn help_text() -> String {
    "tte — text-encoded 3D rendering engine (Phase 1 scaffolding)\n\
     \n\
     USAGE:\n\
     \x20   tte [OPTIONS]\n\
     \n\
     OPTIONS:\n\
     \x20   -V, --version    Print version information\n\
     \x20   -h, --help       Print this help"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_args_means_help() {
        assert_eq!(parse_args(Vec::new()), Invocation::Help);
    }

    #[test]
    fn version_flags_parse() {
        for flag in ["--version", "-V"] {
            assert_eq!(parse_args(vec![flag.to_string()]), Invocation::Version);
        }
    }

    #[test]
    fn unknown_arg_is_reported() {
        assert_eq!(
            parse_args(vec!["--bogus".to_string()]),
            Invocation::UnknownArg("--bogus".to_string())
        );
    }
}
