//! Property-based invariants for the math layer (FR-1.1).
//! Spec: docs/01-requirements-spec.md §4. proptest generates thousands of
//! inputs per invariant and shrinks failures to minimal counterexamples.

use approx::{abs_diff_eq, relative_eq};
use proptest::prelude::*;
use tte_core::{Mat4, Vec3};

/// Finite, moderately-sized floats: keeps relative-epsilon comparisons
/// meaningful (f32 falls apart near its extremes, which is not what these
/// invariants are about).
fn coord() -> impl Strategy<Value = f32> {
    -100.0f32..100.0
}

fn vec3() -> impl Strategy<Value = Vec3> {
    (coord(), coord(), coord()).prop_map(|(x, y, z)| Vec3::new(x, y, z))
}

fn nonzero_vec3() -> impl Strategy<Value = Vec3> {
    vec3().prop_filter("vector must not be near zero", |v| v.length() > 1e-3)
}

fn angle() -> impl Strategy<Value = f32> {
    -std::f32::consts::TAU..std::f32::consts::TAU
}

proptest! {
    #[test]
    fn fr1_1_normalize_yields_unit_length(v in nonzero_vec3()) {
        prop_assert!(abs_diff_eq!(v.normalize().unwrap().length(), 1.0, epsilon = 1e-4));
    }

    #[test]
    fn fr1_1_cross_is_orthogonal_to_both_inputs(a in nonzero_vec3(), b in nonzero_vec3()) {
        let c = a.cross(b);
        // Orthogonality scales with the magnitudes involved.
        let scale = a.length() * b.length() * c.length();
        prop_assume!(c.length() > 1e-3); // skip near-parallel inputs
        prop_assert!(c.dot(a).abs() <= scale * 1e-5);
        prop_assert!(c.dot(b).abs() <= scale * 1e-5);
    }

    #[test]
    fn fr1_1_matrix_mul_is_associative_over_vectors(
        a in angle(), b in angle(), v in vec3()
    ) {
        let (ra, rb) = (Mat4::rotation_y(a), Mat4::rotation_x(b));
        let combined = (ra * rb) * v.extend(1.0);
        let sequential = ra * (rb * v.extend(1.0));
        prop_assert!(relative_eq!(
            combined.truncate(), sequential.truncate(),
            epsilon = 1e-3, max_relative = 1e-3
        ));
    }

    #[test]
    fn fr1_1_rotation_composed_with_inverse_is_identity(a in angle(), v in vec3()) {
        // rotation_y(a) * rotation_y(-a) == I, applied to an arbitrary point.
        let round_trip = (Mat4::rotation_y(a) * Mat4::rotation_y(-a)) * v.extend(1.0);
        prop_assert!(relative_eq!(
            round_trip.truncate(), v,
            epsilon = 1e-3, max_relative = 1e-3
        ));
    }

    #[test]
    fn fr1_1_rotation_preserves_length(a in angle(), v in vec3()) {
        let rotated = (Mat4::rotation_y(a) * v.extend(1.0)).truncate();
        prop_assert!(relative_eq!(rotated.length(), v.length(), epsilon = 1e-3, max_relative = 1e-3));
    }
}
