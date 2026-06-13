//! WASM-boundary smoke test (FR-5.5): runs the real compiled module in Node via
//! `wasm-bindgen-test-runner`, exercising the `Renderer` exactly as the browser
//! would. The heavy logic is covered by native `tte-core`/`tte-wasm` unit tests;
//! this proves the wasm build constructs, loads, renders, and marshals frames.
//!
//! Run: `cargo test -p tte-wasm --target wasm32-unknown-unknown`

#![cfg(target_arch = "wasm32")]

use tte_wasm::Renderer;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn renderer_constructs_and_renders_default_cube() {
    let r = Renderer::new(40, 20);
    assert_eq!(r.width(), 40);
    assert_eq!(r.height(), 20);
    assert_eq!(r.glyphs().len(), 40 * 20);
    assert_eq!(r.fg().len(), 40 * 20 * 3);
    assert_eq!(r.bg().len(), 40 * 20 * 3);
    assert!(
        r.glyphs().iter().any(|&g| g != u16::from(b' ')),
        "cube should be visible"
    );
}

#[wasm_bindgen_test]
fn renderer_loads_scene_and_orbits() {
    let mut r = Renderer::new(60, 30);
    r.load_scene("sphere\nplane translate=(0 -1 0) scale=4")
        .expect("scene parses");
    r.set_orbit(0.8, 0.3, 7.0);
    r.set_mode("truecolor").expect("mode");
    r.render();
    assert!(r.glyphs().contains(&0x2588), "truecolor uses full block");
}

#[wasm_bindgen_test]
fn parse_errors_surface_as_messages() {
    let mut r = Renderer::new(10, 10);
    let err = r.load_scene("teapot").unwrap_err();
    assert!(
        err.contains("teapot") || err.contains("unknown"),
        "got: {err}"
    );
}
