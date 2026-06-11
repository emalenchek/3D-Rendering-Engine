//! `tte-core` — portable core of the text-encoded 3D rendering engine.
//!
//! This crate is **environment-staging scaffolding** for Phase 1. The real
//! modules (math, scene model, software rasterizer, cell-buffer output) land as
//! Phase 1 progresses; see `docs/00-project-brief.md` and
//! `docs/01-requirements-spec.md`.
//!
//! It currently exposes only a version helper so the workspace builds and the
//! test harness (unit + doctest + integration + snapshot) has something real to
//! exercise end to end.

/// Returns the crate's semantic version string (from `Cargo.toml`).
///
/// # Examples
///
/// ```
/// // Doctest: compiled and run by `cargo test`.
/// assert!(!tte_core::version().is_empty());
/// ```
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    //! Unit tests: live in-module, can reach private items, run with `cargo test`.
    use super::*;

    #[test]
    fn version_is_reported() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
