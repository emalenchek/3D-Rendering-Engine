# Test Harness Guide

How testing works in this repo — written for contributors new to Rust testing.
The "why" behind each choice is researched and cited in
[docs/research/06-rust-testing-best-practices.md](research/06-rust-testing-best-practices.md).

## 1. The one command

```sh
cargo test            # runs EVERYTHING: unit + integration/e2e + doctests, whole workspace
```

No test framework to install — the test runner is built into cargo. Useful variants:

```sh
cargo test -p tte-core        # one crate only
cargo test fr1_4              # only tests whose name contains "fr1_4" (requirement filter)
cargo test -- --nocapture     # show println! output from passing tests
cargo test -- --test-threads=1  # serial (tests run in parallel threads by default)
```

## 2. The three built-in test kinds (and where they live)

| Kind | Where | Sees private code? | Purpose here |
|---|---|---|---|
| **Unit** | `#[cfg(test)] mod tests` at the bottom of each `src/*.rs` | Yes | math internals, rasterizer steps, parsers |
| **Integration** | `crates/*/tests/*.rs` — each file is its own test binary linking the crate as a library | No (public API only) | rendering paths, OBJ fixtures, e2e CLI |
| **Doctest** | code blocks in `///` doc comments | No | guaranteed-correct API examples |

Two repo-specific notes:
- `tte-cli` keeps all logic in `src/lib.rs` with a 5-line `src/main.rs` shim. This is
  deliberate: a binary-only crate *cannot have integration tests* (nothing to link), and
  the split makes CLI logic unit-testable.
- Shared helpers for one crate's integration tests go in `tests/common/mod.rs`
  (a subdirectory — `tests/common.rs` would itself become a test binary).

## 3. Golden-frame snapshot tests (`insta`) — our e2e workhorse

Rendered frames are asserted by **snapshot**: the first run records the output, you
review and approve it, and any later drift fails the test with a diff.

```rust
let frame = render_headless("tests/data/cube.obj", 80, 24, /*frame*/ 0);
insta::assert_snapshot!("cube_front_80x24", frame);
```

Workflow:

1. Write the test and run `cargo test` → it **fails** and writes
   `snapshots/<name>.snap.new` (pending).
2. Inspect: `cargo insta review` (interactive accept/reject; install once with
   `cargo install cargo-insta`) — or eyeball the `.snap.new` file and rename/accept with
   `cargo insta accept`. Without the CLI: `INSTA_UPDATE=always cargo test` accepts everything (local only).
3. Commit the `.snap` file. **Golden frames are code-reviewed in PRs** — a frame diff in
   a PR is a visible, reviewable rendering change.
4. CI sets `INSTA_UPDATE=no`: drift always fails there, never silently re-records.

`.snap.new` files are gitignored; never commit them.

## 4. E2E tests of the real binary (`assert_cmd`)

```rust
use assert_cmd::Command;
use predicates::prelude::*;

Command::cargo_bin("tte").unwrap()
    .args(["view", "--headless", "--size", "80x24", "tests/data/cube.obj"])
    .assert()
    .success()
    .stdout(predicate::str::contains("…"));
```

`cargo_bin("tte")` builds and locates the actual release of our binary — these tests cover
the full path: arg parsing → scene load → render → stdout. Combined with snapshots
(capture stdout, `assert_snapshot!` it) this is the **functional e2e** pattern from the
test plan in [docs/01-requirements-spec.md](01-requirements-spec.md) §4.

**Design rule that makes this possible:** rendering produces a pure cell-buffer value;
terminal escape emission is a separate layer; the CLI has a `--headless` plain-text mode.
~90% of e2e coverage therefore needs no terminal/PTY. The few genuinely interactive tests
(Phase 3+) will use `expectrl` through a pseudo-terminal and be marked `#[ignore]`.

## 5. Math testing: floats and properties

- **Never `assert_eq!` floats** (rounding: `0.1 + 0.2 != 0.3`). Use the `approx` crate:
  `assert_relative_eq!(a, b, epsilon = 1e-6)`. We implement `approx`'s traits on
  `Vec3`/`Mat4` so whole-vector asserts read cleanly.
- **Property tests** (`proptest`) state invariants over thousands of generated inputs and
  shrink failures to minimal counterexamples:

```rust
proptest! {
    #[test]
    fn fr1_1_normalize_yields_unit_length(v in nonzero_vec3()) {
        prop_assert!((v.normalize().length() - 1.0).abs() < 1e-5);
    }
}
```

Used for matrix algebra invariants and, later, DSL/OBJ parse↔serialize round-trips.

## 6. Benchmarks (performance is a headline feature)

`criterion` benches live in `crates/tte-core/benches/` and run with `cargo bench`
(the built-in `#[bench]` is nightly-only; criterion is the stable-Rust standard, with
statistical regression detection and HTML reports under `target/criterion/`).
The first bench target lands with FR-1.4 (rasterization), per NFR-3. Bench numbers from
shared CI runners are treated as trend signal, not a hard gate.

## 7. CI gates (.github/workflows/ci.yml)

Ordered fastest-first; all must pass to merge:

| Gate | Command | Job |
|---|---|---|
| Format | `cargo fmt --all -- --check` | lint |
| Lints as errors | `cargo clippy --workspace --all-targets -- -D warnings` | lint |
| Tests | `cargo nextest run --workspace` | test (ubuntu+macos+windows matrix) |
| Doctests | `cargo test --workspace --doc` (nextest skips doctests) | test |
| Dependency vetting | `cargo deny check` (advisories, licenses, bans, sources — config in `deny.toml`) | deny |

Run the whole sequence locally before pushing:

```sh
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test
```

(`cargo-nextest` locally is optional — `cargo test` runs the same tests; nextest is a
faster runner with per-test process isolation.)

## 8. Coverage (optional, on demand)

```sh
cargo install cargo-llvm-cov   # once
cargo llvm-cov --workspace --html   # report in target/llvm-cov/html/
```

Source-based LLVM coverage; cross-platform. Treated as guidance — golden-frame and
property coverage matter more than the line number.

## 9. Tool inventory

| Tool | Kind | Required? | Install |
|---|---|---|---|
| `cargo test` | runner | built-in | — |
| `approx`, `proptest`, `insta`, `assert_cmd`, `predicates` | dev-dependencies | auto via cargo | — |
| `cargo-insta` | snapshot review CLI | recommended | `cargo install cargo-insta` |
| `cargo-nextest` | fast runner | optional locally, used in CI | `cargo install cargo-nextest` |
| `cargo-deny` | dependency vetting | optional locally, gates CI | `cargo install cargo-deny` |
| `cargo-llvm-cov` | coverage | optional | `cargo install cargo-llvm-cov` |
| `criterion` | benches | dev-dependency (from FR-1.4) | — |
| `expectrl` | PTY e2e | dev-dependency (Phase 3) | — |
| `cargo-fuzz` | parser fuzzing | Phase 4, nightly toolchain | `cargo install cargo-fuzz` |
