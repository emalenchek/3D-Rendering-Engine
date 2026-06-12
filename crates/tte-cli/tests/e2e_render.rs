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

fn stdout_of(args: &[&str]) -> String {
    let out = tte()
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).expect("stdout is utf-8")
}

#[test]
fn fr1_8_headless_wireframe_frame_matches_golden() {
    // Wireframe is no longer the default (solid is) — request it explicitly.
    let text = stdout_of(&[
        "view",
        "--headless",
        "--render",
        "wireframe",
        "--size",
        "80x24",
        "--frames",
        "1",
        &cube_path(),
    ]);
    insta::assert_snapshot!("headless_cube_80x24_frame0", text);
}

#[test]
fn fr2_9_headless_solid_ascii_frame_matches_golden() {
    let text = stdout_of(&[
        "view",
        "--headless",
        "--render",
        "solid",
        "--shading",
        "flat",
        "--mode",
        "ascii",
        "--size",
        "80x24",
        "--frames",
        "1",
        &cube_path(),
    ]);
    insta::assert_snapshot!("headless_solid_flat_ascii_80x24", text);
}

#[test]
fn fr2_9_headless_solid_gouraud_frame_matches_golden() {
    let text = stdout_of(&[
        "view",
        "--headless",
        "--shading",
        "gouraud",
        "--size",
        "80x24",
        "--frames",
        "1",
        &cube_path(),
    ]);
    insta::assert_snapshot!("headless_solid_gouraud_ascii_80x24", text);
}

#[test]
fn fr2_9_truecolor_output_has_ansi_fg_and_final_reset() {
    let text = stdout_of(&[
        "view",
        "--headless",
        "--mode",
        "truecolor",
        "--size",
        "40x12",
        &cube_path(),
    ]);
    assert!(text.contains("\x1b[38;2;"), "expected 24-bit fg codes");
    assert!(text.contains('█'), "expected block glyphs");
    assert!(text.ends_with("\x1b[0m"), "must end with SGR reset");
}

#[test]
fn fr2_9_halfblock_output_uses_upper_half_block_and_bg() {
    let text = stdout_of(&[
        "view",
        "--headless",
        "--mode",
        "halfblock",
        "--size",
        "40x12",
        &cube_path(),
    ]);
    assert!(text.contains('▀'), "expected upper-half-block glyphs");
    assert!(
        text.contains("\x1b[48;2;"),
        "expected 24-bit background codes"
    );
    // 12 cell rows even though rendered at 24 pixel rows.
    assert_eq!(
        text.trim_end_matches("\x1b[0m").trim_end().lines().count(),
        12
    );
}

#[test]
fn fr1_8_headless_default_output_contains_no_ansi_escapes() {
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
