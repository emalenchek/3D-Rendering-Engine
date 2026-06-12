//! Integration + property tests for the scene DSL (FR-4.3, FR-4.5, FR-4.7).

use proptest::prelude::*;
use tte_core::{Camera, Rgb, ShadingMode, parse_scene, render_scene, serialize_scene};

#[test]
fn fr4_5_renders_a_multi_object_scene() {
    // Two spheres at different positions over a plane → many lit pixels.
    let scene = parse_scene(
        "scene {\n\
         background color=(0 0 0)\n\
         material \"red\" base-color=(1 0 0)\n\
         sphere translate=(-1 0 0) material=red\n\
         sphere translate=(1 0 0)\n\
         plane translate=(0 -1 0) scale=(6 1 6)\n\
         }",
    )
    .unwrap();
    let fb = render_scene(
        &scene,
        &Camera::default(),
        80,
        40,
        ShadingMode::Flat,
        |_| None,
    );
    let lit = fb.rows().flatten().filter(|&&c| c != Rgb::BLACK).count();
    assert!(lit > 200, "expected a populated scene, got {lit} lit cells");
}

#[test]
fn fr4_5_mesh_refs_use_the_loader() {
    use std::cell::Cell;
    let scene = parse_scene("mesh src=\"teapot.obj\"\nmesh src=\"other.obj\"").unwrap();
    let calls = Cell::new(0);
    let _ = render_scene(
        &scene,
        &Camera::default(),
        20,
        10,
        ShadingMode::Flat,
        |path| {
            calls.set(calls.get() + 1);
            assert!(path.ends_with(".obj"));
            None // pretend the asset is unavailable
        },
    );
    assert_eq!(calls.get(), 2, "loader should be called once per mesh ref");
}

// --- Property tests ---------------------------------------------------------

prop_compose! {
    fn arb_vec3()(x in -50.0f32..50.0, y in -50.0f32..50.0, z in -50.0f32..50.0)
        -> (f32, f32, f32) { (x, y, z) }
}

// Render a small DSL document from generated parts and assert the round-trip
// property (FR-4.7): `parse(serialize(parse(src))) == parse(src)`.
proptest! {
    #[test]
    fn fr4_7_round_trip_is_stable(
        (tx, ty, tz) in arb_vec3(),
        rings in 2u32..40,
        segments in 3u32..40,
        fov in 20.0f32..120.0,
    ) {
        let src = format!(
            "scene {{\n\
             camera position=({tx} {ty} {tz}) look-at=(0 0 0) fov={fov}\n\
             material \"m\" base-color=(0.5 0.25 0.125)\n\
             sphere rings={rings} segments={segments} translate=({tx} {ty} {tz}) material=m\n\
             cube rotate=(0 90 0) scale=(2 1 1)\n\
             }}"
        );
        let scene = parse_scene(&src).unwrap();
        let reparsed = parse_scene(&serialize_scene(&scene)).unwrap();
        prop_assert_eq!(scene, reparsed);
    }
}

// Fuzz-style robustness (FR-4.7): the parser must never panic on arbitrary
// input — it returns `Ok` or a clean `Err`, but never crashes or hangs.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]
    #[test]
    fn fr4_7_parser_never_panics_on_arbitrary_text(s in ".{0,200}") {
        let _ = parse_scene(&s);
    }
}

// Robustness on inputs built from real DSL tokens (more likely to reach deep
// parser paths than random bytes).
proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]
    #[test]
    fn fr4_7_parser_never_panics_on_token_soup(
        tokens in proptest::collection::vec(
            prop_oneof![
                Just("scene"), Just("cube"), Just("sphere"), Just("node"), Just("mesh"),
                Just("material"), Just("camera"), Just("light"), Just("plane"),
                Just("{"), Just("}"), Just("("), Just(")"), Just("="),
                Just("translate"), Just("scale"), Just("(1 2 3)"), Just("\"x\""),
                Just("1.5"), Just("\n"), Just(";"), Just("//c\n"),
            ],
            0..40,
        ),
    ) {
        let src = tokens.join(" ");
        let _ = parse_scene(&src);
    }
}
