//! Functional e2e tests of the headless render path (FR-1.8, NFR-1).
//!
//! This is the template e2e shape from docs/01-requirements-spec.md §4:
//! fixture scene → real binary in --headless → plain-text frames on stdout
//! → insta golden snapshot.

use assert_cmd::Command;
use predicates::prelude::*;

fn cube_path() -> String {
    format!("{}/tests/data/cube.obj", env!("CARGO_MANIFEST_DIR"))
}

fn tte() -> Command {
    Command::cargo_bin("tte").expect("tte binary should build")
}

#[test]
fn fr1_8_headless_cube_frame_matches_golden() {
    let out = tte()
        .args([
            "view",
            "--headless",
            "--size",
            "80x24",
            "--frames",
            "1",
            &cube_path(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    insta::assert_snapshot!(
        "headless_cube_80x24_frame0",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn fr1_8_headless_output_contains_no_ansi_escapes() {
    let out = tte()
        .args([
            "view",
            "--headless",
            "--size",
            "40x12",
            "--frames",
            "3",
            &cube_path(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        !text.contains('\x1b'),
        "headless output must be escape-free"
    );
    // 3 frames × 12 lines each.
    assert_eq!(text.lines().count(), 36);
}

#[test]
fn nfr1_headless_render_is_deterministic_across_runs() {
    let run = || {
        tte()
            .args([
                "view",
                "--headless",
                "--size",
                "60x20",
                "--frames",
                "4",
                &cube_path(),
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone()
    };
    assert_eq!(run(), run(), "same invocation must produce identical bytes");
}

#[test]
fn fr1_8_missing_scene_file_fails_with_message() {
    tte()
        .args(["view", "--headless", "/definitely/not/a/file.obj"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("could not open scene"));
}

#[test]
fn fr1_8_invalid_size_fails_with_usage_message() {
    tte()
        .args(["view", "--headless", "--size", "banana", &cube_path()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--size expects WxH"));
}

/// FR-1.7 PTY smoke test: drive the real interactive path through a
/// pseudo-terminal — spawn, let it render, press 'q', expect a clean exit.
/// `#[ignore]`d: needs a PTY, excluded from the default/CI run
/// (docs/02-test-harness.md §4). Run with: cargo test -- --ignored
#[cfg(unix)]
#[test]
#[ignore = "requires a PTY; run explicitly with --ignored"]
fn fr1_7_interactive_quits_on_q() {
    let bin = assert_cmd::cargo::cargo_bin("tte");
    let mut session =
        expectrl::spawn(format!("{} view {}", bin.display(), cube_path())).expect("spawn in PTY");
    std::thread::sleep(std::time::Duration::from_millis(400)); // let it draw
    session.send("q").expect("send quit key");
    session
        .expect(expectrl::Eof)
        .expect("process should exit after 'q'");
}
