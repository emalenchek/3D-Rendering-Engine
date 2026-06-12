//! Golden-frame tests of the wireframe pipeline (FR-1.3, FR-1.4).
//!
//! These render the cube fixture at fixed cameras/sizes and snapshot the
//! frames with insta. Review changes with `cargo insta review`; an approved
//! frame diff in a PR is a deliberate rendering change (docs/02-test-harness.md §3).

use tte_core::{Camera, Mat4, Mesh, parse_obj, render_wireframe};

fn cube() -> Mesh {
    parse_obj(include_str!("data/cube.obj")).expect("fixture parses")
}

#[test]
fn fr1_3_cube_front_view_80x24_matches_golden() {
    let frame = render_wireframe(&cube(), Mat4::IDENTITY, &Camera::default(), 80, 24);
    insta::assert_snapshot!("cube_default_80x24", frame.to_string());
}

#[test]
fn fr1_4_cube_rotated_40x20_matches_golden() {
    // A rotation that exercises diagonal Bresenham strokes in all octants.
    let model = Mat4::rotation_y(0.65) * Mat4::rotation_x(0.35);
    let frame = render_wireframe(&cube(), model, &Camera::default(), 40, 20);
    insta::assert_snapshot!("cube_rotated_40x20", frame.to_string());
}

#[test]
fn fr1_3_camera_inside_cube_culls_cleanly() {
    // Eye at the origin, inside the cube: every edge either crosses or sits
    // behind the near plane region — must not panic or smear (FR-1.3 cull).
    let camera = Camera {
        eye: tte_core::Vec3::new(0.0, 0.0, 0.0001),
        ..Camera::default()
    };
    let frame = render_wireframe(&cube(), Mat4::IDENTITY, &camera, 40, 20);
    assert_eq!(frame.to_string().lines().count(), 20);
}

#[test]
fn nfr1_cube_render_is_deterministic_across_calls() {
    let model = Mat4::rotation_y(1.1);
    let a = render_wireframe(&cube(), model, &Camera::default(), 200, 50);
    let b = render_wireframe(&cube(), model, &Camera::default(), 200, 50);
    assert_eq!(a, b);
}
