# Rust Testing Best Practices — Research Notes

Project: from-scratch 3D software-rendering engine in Rust. Portable core lib
(math, scene model, software rasterizer, cell-buffer output) + native terminal
CLI frontend (ASCII/Unicode + ANSI color) + later browser/WASM frontend.
Owner is new to Rust. We are entering Phase 1 (terminal wireframe renderer).

Goal: unit + integration + functional end-to-end tests mapped to a requirement
spec / test plan, where e2e can verify rendered terminal frames (character grids).

Confidence legend: HIGH = official docs / canonical crate docs corroborated;
MEDIUM = single good source or widely-known practice; LOW = inferred / needs check.

> This file is written incrementally and is safe to resume after interruption.

---

## 1. Cargo test fundamentals — the three test kinds  (Confidence: HIGH)

Rust formally supports three kinds of tests, all run by `cargo test`:

**Unit tests** — live *inside* the source file, in a submodule annotated
`#[cfg(test)]` so the test code is compiled only under `cargo test`, never in
`cargo build`:

```rust
// src/math/vec3.rs
pub fn dot(a: Vec3, b: Vec3) -> f32 { /* ... */ }

#[cfg(test)]
mod tests {
    use super::*;           // brings parent (incl. PRIVATE items) into scope
    #[test]
    fn dot_of_orthogonal_is_zero() {
        assert_eq!(dot(Vec3::X, Vec3::Y), 0.0);
    }
}
```
Key property: unit tests **can test private functions** because they sit in a
child module with access to `super::*`.

**Integration tests** — live in a top-level `tests/` directory, a sibling of
`src/`. Each `.rs` file there is compiled as its **own separate crate / binary**
and links your library as an external dependency, so it can only exercise the
**public API** (black-box). No `#[cfg(test)]` needed — the whole file is test-only.

```
project/
├── src/lib.rs
└── tests/
    └── render_pipeline.rs   # one test binary
```

**Documentation tests (doctests)** — code in `///` doc comments is compiled and
run by `cargo test`. Great as living, guaranteed-correct examples of the public
API. Use fenced ```rust blocks; lines prefixed `#` are hidden from rendered docs
but still compiled; `no_run` / `ignore` / `compile_fail` modifiers exist.
Doctests run only for **library** targets.

### The test attributes/macros
- `#[test]` — marks a free function as a test.
- `assert!(cond)`, `assert!(cond, "msg {}", x)` — boolean assertion + custom msg.
- `assert_eq!(a, b)`, `assert_ne!(a, b)` — print both values on failure (require `PartialEq`+`Debug`).
- `#[should_panic]` / `#[should_panic(expected = "substring")]` — passes only if the body panics (and msg contains the substring).
- `Result<(), E>`-returning tests: a test fn may return `Result`; returning `Err`
  fails the test. Lets you use `?` inside tests. (Cannot combine with `#[should_panic]`.)
- `#[ignore]` — skip by default; run with `cargo test -- --ignored` or `--include-ignored`.

### Discovery & running
- `cargo test` builds and runs all unit + integration + doctests across the package.
- Tests run **in parallel by default** (separate threads). Force serial with
  `cargo test -- --test-threads=1` (needed when tests share global state, env vars, or a real terminal).
- Output of **passing** tests is captured/hidden; failing tests show their stdout.
  Force-show with `cargo test -- --nocapture` (or `--show-output` to show captured output of passing tests too).
- **Filtering:** `cargo test dot` runs every test whose name contains `dot`.
- The `--` separator passes everything after it to the **test binary** (libtest
  harness) rather than to cargo: `cargo test -- --test-threads=1 --nocapture`.

Sources:
- The Rust Book ch.11 (writing/running/organizing tests):
  https://doc.rust-lang.org/book/ch11-01-writing-tests.html ,
  https://doc.rust-lang.org/book/ch11-02-running-tests.html ,
  https://doc.rust-lang.org/book/ch11-03-test-organization.html
  (content confirmed via raw GitHub mirror rust-lang/book/src/ch11-*)
- The Cargo Book — `cargo test`: https://doc.rust-lang.org/cargo/commands/cargo-test.html

---

## 2. Workspace test organization (multi-crate)  (Confidence: HIGH)

A **workspace** shares one `Cargo.lock` and one `target/` dir across member
crates. Root `Cargo.toml` has a `[workspace]` table and **no `[package]`**:

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "3"
members = ["crates/render-core", "crates/render-cli"]
```

Recommended layout for this project:
```
render-core/   # library: math, scene, rasterizer, cell-buffer  -> has lib.rs
render-cli/    # binary: terminal frontend                       -> main.rs (thin)
render-wasm/   # later: cdylib wasm frontend
```

- **Unit tests** live inside each crate's `src/**` modules (`#[cfg(test)] mod tests`).
- **Integration tests** live in each crate's own `tests/` dir and see only its public API.
- **Binary-crate caveat:** a crate with only `main.rs` cannot have integration
  tests (nothing public to link). Fix: put logic in `render-cli`'s `lib.rs` (or
  better in `render-core`) and keep `main.rs` a thin shim. This also makes CLI logic unit-testable.
- **Shared test helpers:** put them in `tests/common/mod.rs` (a *subdirectory*,
  not `tests/common.rs`) so the helper file is NOT compiled as its own test
  binary / doesn't show up as an empty test set. Import with `mod common;`.
  For helpers shared across *crates*, create a dedicated `[dev-dependencies]`
  helper crate (e.g. `render-testkit`) inside the workspace.
- **`[dev-dependencies]`** are available to tests/benches/examples only, never in
  the shipped build — this is where `insta`, `assert_cmd`, `proptest`, `criterion`, etc. go.
- Run everything: `cargo test` (whole workspace). One crate: `cargo test -p render-core`.

Sources:
- The Rust Book ch.14.3 Cargo Workspaces: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
- The Rust Book ch.11.3 (binary-crate integration-test limitation; tests/common pattern).

---
## 3. Snapshot / golden-file testing for rendered text frames  (Confidence: HIGH)

This is **the** key technique for asserting ASCII/Unicode frame output. Rather
than hand-writing the expected 40×20 grid of characters in a string literal, you
let the test record the rendered frame the first time, you eyeball+approve it,
and thereafter the test fails if the output drifts.

### `insta` (recommended) — current version 1.47.x (Mar 2026)
- For text frames use `insta::assert_snapshot!(frame_string)` (string/text
  snapshot — ideal for multi-line ASCII art; the `.snap` stores it verbatim and
  diffs are line-by-line and readable). Also `assert_debug_snapshot!`,
  `assert_json_snapshot!` / `assert_yaml_snapshot!` (need `serde`).
- **File snapshots:** stored as `.snap` files under a `snapshots/` directory next
  to the test. On first run (or when output changes) insta writes a pending
  `.snap.new` file and the test *fails* (so CI never silently accepts new output).
- **Inline snapshots:** `assert_snapshot!(value, @"...")` embeds the expected
  value directly in the source; `cargo insta` rewrites the `@"..."` in place.
- **Review workflow:** install the companion CLI `cargo install cargo-insta`.
  - `cargo insta test` — runs tests, collecting all snapshot changes.
  - `cargo insta review` — interactive diff viewer; press `a` accept, `r` reject,
    `s` skip, per snapshot. `cargo insta accept` / `reject` are non-interactive.
  - Env override: `INSTA_UPDATE=always cargo test` auto-accepts (use locally, never in CI).
  - There is a VS Code extension for inline review.
- **Redactions:** for nondeterministic fields (timestamps, frame counters, pointer
  addresses) use `assert_json_snapshot!(value, { ".timestamp" => "[ts]" })` style
  redactions so noise doesn't cause spurious diffs. For pure text frames you
  usually want determinism instead (fixed seed, fixed camera) rather than redaction.
- Pros: best-in-class review UX, file+inline, workspace-aware, huge adoption.
  Cons: extra dev-dep + a CLI to learn; `.snap` files must be committed and reviewed in PRs.

```rust
// tests/wireframe_cube.rs  (integration test)
#[test]
fn renders_unit_cube_front_view() {
    let frame = render_test_scene("cube", Camera::front(), 40, 20); // -> String grid
    insta::assert_snapshot!(frame);   // first run writes snapshots/...snap.new
}
```

### `expect-test` (rust-analyzer's, the lightweight alternative)
- Macros `expect![[ "..." ]]` / `expect_file!`; assert with `.assert_eq(&actual)`.
- Update with `UPDATE_EXPECT=1 cargo test` (rewrites the inline literal / file).
- Intentionally minimal: no review TUI, no redactions, no serialization formats —
  "a small addition over `assert_eq!` that can auto-update". Good when you want
  zero ceremony and inline expectations; insta is the fuller-featured choice and
  is what rust-analyzer-scale and most projects pick for golden frames.

Sources:
- insta docs/site: https://insta.rs/docs/ , https://docs.rs/insta/latest/insta/
- insta + cargo-insta README (mitsuhiko/insta): https://github.com/mitsuhiko/insta
- cargo-insta version 1.47.x: https://crates.io/crates/cargo-insta
- expect-test: https://docs.rs/expect-test/latest/expect_test/ , https://github.com/rust-analyzer/expect-test

---

<!-- MORE TOPICS APPENDED BELOW AS RESEARCH CONTINUES -->
