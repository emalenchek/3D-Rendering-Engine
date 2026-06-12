//! Integration tests for `tte-core`'s public API (black-box: this file is
//! compiled as its own crate and can only see what the library exports).
//!
//! Currently a smoke test of the staging API; Phase 1 adds files per
//! functional area (math, obj loader, pipeline) with golden-frame tests of
//! the cell buffer — see docs/01-requirements-spec.md for the test plan.

#[test]
fn version_is_semver_shaped() {
    let v = tte_core::version();
    assert_eq!(
        v.split('.').count(),
        3,
        "expected MAJOR.MINOR.PATCH, got '{v}'"
    );
}
