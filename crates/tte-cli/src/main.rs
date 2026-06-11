//! `tte` — native terminal frontend for the text-encoded 3D rendering engine.
//!
//! Environment-staging scaffolding for Phase 1. Today it only resolves a
//! `--version` request and prints a banner, which gives the end-to-end CLI
//! tests (`assert_cmd`) a deterministic surface to assert against. The render
//! loop, scene loading, and orbit camera arrive during Phase 1.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--version" | "-V") => {
            println!("tte {}", tte_core::version());
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("tte: unknown argument '{other}'");
            eprintln!("try 'tte --help'");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("tte — text-encoded 3D rendering engine (Phase 1 scaffolding)");
    println!();
    println!("USAGE:");
    println!("    tte [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -V, --version    Print version information");
    println!("    -h, --help       Print this help");
}
