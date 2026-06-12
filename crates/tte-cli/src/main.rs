//! Thin binary shim over `tte-cli`'s library — see lib.rs for why.

use std::process::ExitCode;

fn main() -> ExitCode {
    tte_cli::run(tte_cli::parse_args(std::env::args().skip(1)))
}
