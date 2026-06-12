//! End-to-end tests of the `tte` binary surface (requirement area: CLI).
//!
//! Spec traceability: test names embed requirement IDs from
//! docs/01-requirements-spec.md (`fr0_*` = FR-0.x, staging requirements).
//! Run just these: `cargo test -p tte-cli --test e2e_cli`
//! Run one requirement's tests everywhere: `cargo test fr0_2`
//!
//! Patterns demonstrated here (the templates for all later golden-frame tests):
//! - `assert_cmd`: build + run the real binary, assert exit code / stdout / stderr
//! - `insta::assert_snapshot!`: golden-text snapshot with review workflow
//!   (first run writes `snapshots/*.snap.new`; review with `cargo insta review`)

use assert_cmd::Command;
use predicates::prelude::*;

fn tte() -> Command {
    Command::cargo_bin("tte").expect("tte binary should build")
}

#[test]
fn fr0_1_version_reports_crate_version() {
    tte()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("tte "));
}

#[test]
fn fr0_2_help_output_matches_golden() {
    let out = tte()
        .arg("--help")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    // Golden-snapshot pattern: identical to how rendered frames are asserted
    // from Phase 1 on (headless frame dump -> assert_snapshot!).
    insta::assert_snapshot!("help_output", String::from_utf8(out).unwrap());
}

#[test]
fn fr0_3_unknown_argument_fails_with_message() {
    tte()
        .arg("--definitely-not-a-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown argument"))
        .stderr(predicate::str::contains("--help"));
}
