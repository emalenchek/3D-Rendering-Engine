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

## 4. CLI / terminal e2e testing  (Confidence: HIGH)

**Design principle first (most important).** Make the renderer testable by
**separating render-to-buffer from emit-to-terminal**. The core library should
produce a `Frame`/cell-buffer (a pure value: grid of chars + colors) that has a
deterministic `Display`/`to_string()`. Terminal escape emission is a thin outer
layer. Then **the vast majority of e2e frame tests need NO real TTY** — they call
the buffer API directly (or via a headless CLI mode) and snapshot the string.

Concretely, give the CLI a **headless frame-dump mode** (e.g.
`render --headless --frames 1 --size 40x20 scene.obj` printing plain frame text
to stdout, no ANSI/raw-mode). This is the single most valuable testability
investment: it lets `assert_cmd` + `insta` cover the real end-to-end path
(arg parsing → scene load → rasterize → frame) without a pseudo-terminal.

### `assert_cmd` (+ `predicates`) — run the built binary, assert on I/O
- `Command::cargo_bin("render-cli")` locates and runs your compiled binary.
- Chain `.arg()/.args()`, then `.assert()` then `.success()` / `.failure()` /
  `.code(2)`, `.stdout(...)`, `.stderr(...)`.
- Pairs with the **`predicates`** crate for flexible matching:
  `predicate::str::contains("...")`, `is_match(regex)`, `starts_with`, etc.
- Same `assert-rs` family: `assert_cmd`, `predicates`, `assert_fs` (temp dirs/files).

```rust
// tests/cli.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn headless_dump_of_cube_matches_golden() {
    let out = Command::cargo_bin("render-cli").unwrap()
        .args(["--headless", "--size", "40x20", "--scene", "tests/data/cube.obj"])
        .assert().success()
        .get_output().stdout.clone();
    insta::assert_snapshot!(String::from_utf8(out).unwrap());  // golden frame
}

#[test]
fn bad_scene_path_errors() {
    Command::cargo_bin("render-cli").unwrap()
        .args(["--scene", "/nope.obj"])
        .assert().failure().code(2)
        .stderr(predicate::str::contains("could not open scene"));
}
```

### `trycmd` / `snapbox` — declarative CLI snapshot tests (assert-rs family)
- `snapbox` is a snapshot toolbox for CLI output (file + inline, auto-update via
  `SNAPSHOTS=overwrite` / `TRYCMD=overwrite`).
- `trycmd` runs whole suites of CLI cases written as `.trycmd` / `.toml` / `.md`
  files asserting stdout/stderr/exit code — great for many "command → expected
  output" cases (e.g. `--help`, error messages, several headless renders) without
  writing Rust per case. Good complement to the per-frame `insta` snapshots.

### When you DO need a real TTY (raw mode, alt-screen, cursor) — PTY testing
If a test must exercise the actual interactive terminal path (raw mode, resize,
keypress handling), drive the binary through a pseudo-terminal:
- **`expectrl`** (recommended, actively maintained) — spawn a child in a PTY,
  `expect(pattern)`, `send()/send_line()`; **cross-platform** (Linux/macOS/Windows),
  async optional. Successor-in-spirit to rexpect/pexpect.
- **`rexpect`** — older, **Unix-only** (won't compile off *nix/WSL); fine if Linux-only.
- **`portable-pty`** (wezterm) / **`ptyprocess`** — lower-level PTY primitives if you
  need to build your own harness or capture OSC/escape sequences.
Keep PTY tests few and mark slow ones `#[ignore]` (run in a dedicated CI job);
rely on the headless + snapshot path for the bulk of coverage.

Sources:
- assert_cmd: https://docs.rs/assert_cmd/latest/assert_cmd/ , https://github.com/assert-rs/assert_cmd
- predicates: https://docs.rs/predicates/latest/predicates/
- snapbox + trycmd (assert-rs): https://github.com/assert-rs/snapbox , https://docs.rs/trycmd/
- expectrl: https://github.com/zhiburt/expectrl ; rexpect: https://docs.rs/rexpect/ ; portable-pty: https://docs.rs/portable-pty/

---

## 5. Property-based & fuzz testing  (Confidence: HIGH)

**Property tests** generate many random inputs and assert *invariants* hold,
auto-**shrinking** any failure to a minimal counterexample. They run inside the
normal `cargo test` suite (no extra tooling) — ideal for the **math layer**:
- matrix invariants: `M * M.inverse() ≈ I`; `(A*B)*v ≈ A*(B*v)`; transpose-of-transpose; identity is neutral.
- vector invariants: `normalize(v).length() ≈ 1`; dot/cross relationships.
- parser round-trips: `parse(serialize(scene)) == scene` for the scene DSL / OBJ subset.

- **`proptest`** (recommended) — uses explicit `Strategy` objects, so you can
  constrain generation (e.g. nonzero vectors, valid matrices) and compose
  strategies; more flexible than quickcheck. Idiom: `proptest! { #[test] fn p(v in any::<[f32;3]>()) { ... } }`.
- **`quickcheck`** — simpler, generates/shrinks by type alone (one impl per type);
  fine for quick checks, less flexible for constrained domains.

**Fuzzing** (`cargo-fuzz`, libFuzzer; coverage-guided, mutation-based) is the
right tool for **untrusted byte input parsers** — your OBJ loader and scene DSL
parser. A fuzz target asserts "never panics / never UB on arbitrary bytes".
Worth it once those parsers exist and accept external files; run continuously /
nightly in CI, not on every PR. Use `arbitrary` to derive structured inputs.
Rule of thumb: **proptest for logical invariants you can state; cargo-fuzz to
hunt crashes in byte-level parsers.** They complement each other.

Sources:
- proptest: https://lib.rs/crates/proptest , https://proptest-rs.github.io/proptest/
- quickcheck: https://github.com/BurntSushi/quickcheck
- cargo-fuzz / Rust Fuzz Book: https://rust-fuzz.github.io/book/ ; structured fuzzing: https://fitzgen.com/2020/01/16/better-support-for-fuzzing-structured-inputs-in-rust.html
- Rust Project Primer (property testing): https://rustprojectprimer.com/testing/property.html

---

## 6. Benchmarking & performance regression  (Confidence: HIGH)

Performance is a project headline, so wire benchmarks in from Phase 1.
Built-in `#[bench]` is **nightly-only and now fully de-stabilized on stable**
(hard error), so use a third-party harness on stable:

- **`criterion`** (recommended, v0.5/0.6+; "the de-facto standard / gold standard")
  — statistically rigorous, detects regressions vs the previous run, generates
  HTML reports + plots, works on stable. Benches live in `benches/*.rs`, run via
  `cargo bench`; declared with `[[bench]] harness = false` in `Cargo.toml`.
- **`divan`** — modern, lighter-weight, very ergonomic (attribute-based
  `#[divan::bench]`), faster to write; less statistical depth than criterion.
  Good alternative; pick one. (Either is fine; criterion if you want rigorous
  regression stats, divan if you want minimal ceremony.)
- **CI perf tracking:** run `cargo bench` in a dedicated job; criterion stores a
  baseline in `target/criterion` and reports % change. For gating, tools like
  `critcmp` (compare saved baselines) or `bencher.dev` / `codspeed` (hosted
  continuous benchmarking that comments on PRs) are common. Note: bench numbers
  are noisy on shared CI runners — treat as trend signal, not a hard gate.

Sources:
- criterion: https://github.com/bheisler/criterion.rs , https://bheisler.github.io/criterion.rs/book/
- divan: https://github.com/nvzqz/divan , https://nikolaivazquez.com/blog/divan/
- The Rust Performance Book (benchmarking): https://nnethercote.github.io/perf-book/benchmarking.html

---

## 9. Float comparison in tests (rasterizer math)  (Confidence: HIGH)

Never use `assert_eq!` on `f32`/`f64` results of arithmetic. Floating-point ops
round to the nearest representable value, so `0.1 + 0.2 != 0.3` exactly and
chained matrix math accumulates rounding error. Use approximate comparison:

- **`approx`** (recommended for this project) — `assert_relative_eq!(a, b, epsilon = 1e-6)`
  and `assert_ulps_eq!`, plus `relative_eq!`/`abs_diff_eq!`. Provides derivable
  traits (`AbsDiffEq`, `RelativeEq`, `UlpsEq`) you can implement for `Vec3`/`Mat4`
  so you can write `assert_relative_eq!(got_vec, expected_vec)` directly. Widely
  used by graphics/math crates (e.g. nalgebra ecosystem).
- **`float-cmp`** — `approx_eq!` with `F32Margin { epsilon, ulps }`; lets you set
  both an absolute epsilon and a ULPs tolerance (use ulps 1–5, epsilon a small
  multiple of `f32::EPSILON`).

Idiom: epsilon (relative) tolerance for "close enough" math; ULPs when you care
about last-bit precision. For rasterizer geometry, `assert_relative_eq!` with an
explicit small epsilon is the everyday tool; implement `approx` traits on your
math types so vector/matrix asserts read cleanly.

Sources:
- approx: https://docs.rs/approx , https://lib.rs/crates/approx
- float-cmp: https://github.com/mikedilger/float-cmp , https://docs.rs/float-cmp

---

## 7. Coverage  (Confidence: HIGH)

- **`cargo-llvm-cov`** (recommended) — uses LLVM **source-based** instrumentation,
  region-level accuracy, works on Linux/macOS/Windows. Outputs LCOV / HTML /
  Cobertura / Codecov formats: `cargo llvm-cov --workspace --lcov --output-path lcov.info`
  or `--html`. Integrates with Codecov/Coveralls and VS Code Coverage Gutters.
- **`cargo-tarpaulin`** — older Rust-specific tool; ptrace backend is Linux-x86_64
  only (also has an LLVM backend now). Fine if Linux-only, but llvm-cov is the
  better default for a portable/cross-platform project like this.
- Optional **coverage gate** in CI (e.g. fail under N%), but treat as guidance, not
  dogma — golden-frame and property tests matter more than a line-coverage number.

Sources:
- cargo-llvm-cov: https://github.com/taiki-e/cargo-llvm-cov
- Rust Project Primer (coverage): https://rustprojectprimer.com/measure/coverage.html
- cargo-tarpaulin: https://crates.io/crates/cargo-tarpaulin

---

## 8. Lint / format / CI gates & faster runner  (Confidence: HIGH)

Treat the test harness as a *pipeline of gates*, fastest/cheapest first:

- **`cargo fmt --all -- --check`** — fails if code isn't rustfmt-clean (no rewrite in CI).
- **`cargo clippy --all-targets --all-features -- -D warnings`** — lint, with
  warnings promoted to errors so lint debt can't land.
- **`cargo test --workspace`** (or nextest below) — unit + integration + doctests.
- **`cargo deny check`** and/or **`cargo audit`** — dependency vetting:
  - `cargo audit` checks deps against the RustSec advisory DB (vulnerabilities).
  - `cargo deny` is broader: advisories **plus** license policy, banned/duplicate
    crates, allowed sources — and subsumes audit's advisory check. Prefer `cargo deny`.
- **`cargo-nextest`** (recommended runner) — drop-in faster `cargo test`
  replacement; process-per-test isolation, up to ~3x faster, clean per-test
  output buffering, JUnit XML for CI UIs, retries/partitioning for sharding.
  **Caveat: nextest does NOT run doctests** — run `cargo test --doc` separately in
  CI so doctests still execute. Invoke as `cargo nextest run --workspace`.
- **GitHub Actions:** matrix over `{stable, beta}` × `{ubuntu, macos, windows}`
  (relevant since core must stay portable + WASM later). Use a Rust setup action
  with component caching; add a `wasm32-unknown-unknown` build/test job for the
  WASM frontend; run fmt/clippy once on stable/ubuntu; run benches + fuzz on a
  scheduled (nightly) job, not per-PR.

Sources:
- cargo-nextest: https://nexte.st/ (incl. "nextest does not run doctests")
- clippy/fmt in CI: https://rustprojectprimer.com/checks/ ; https://doc.rust-lang.org/clippy/
- cargo-deny: https://embarkstudios.github.io/cargo-deny/ ; cargo-audit/RustSec: https://rustsec.org/
- Rust Project Primer (test runners): https://rustprojectprimer.com/testing/runners.html

---

## 10. Mapping a requirement spec / test plan to tests  (Confidence: MEDIUM — convention, not spec'd by any official source)

Rust has no built-in requirements-traceability tooling; the working convention is
naming + structure + review discipline:

- **Requirement IDs in test names.** Give every requirement a stable ID
  (`FR-1.3`, `NFR-2`) in the spec, and embed it in the test function name:
  `fn fr1_3_wireframe_cube_front_view()`. `cargo test fr1_3` then runs exactly
  the tests for that requirement (libtest substring filtering), and a grep of
  the codebase answers "which tests cover FR-1.3?".
- **One integration-test file per functional area**, mirroring the spec's
  sections (`tests/e2e_render.rs`, `tests/e2e_cli.rs`, `tests/obj_loader.rs`),
  with a doc comment at the top linking back to the spec section.
- **The test plan is a table in the spec**: requirement ID → verification method
  (unit / property / golden-frame e2e / PTY) → test name(s) → status. Tests are
  the source of truth; the table is reviewed whenever the spec changes.
- **What a functional e2e test concretely is here**: render a known scene
  (e.g. unit cube, fixed camera, fixed size 80×24) through the *real* public
  path (CLI in headless frame-dump mode, or the library's render API), and
  snapshot the resulting character frame with `insta`. Deterministic inputs →
  byte-identical frames → meaningful golden diffs in PR review.

---

## Recommended test stack for this project (synthesis)

| Layer | Tool | Used for |
|---|---|---|
| Unit tests | built-in `#[cfg(test)]` | math, rasterizer internals (private fns OK) |
| Float asserts | `approx` | `assert_relative_eq!` on Vec3/Mat4 results |
| Property tests | `proptest` | math invariants; parser round-trips (DSL/OBJ) |
| Integration tests | built-in `tests/` dirs | public-API rendering paths per crate |
| Golden frames | `insta` + `cargo-insta` | snapshot ASCII frames; review/accept workflow |
| CLI e2e | `assert_cmd` + `predicates` | run real `tte` binary; exit codes, stderr |
| Bulk CLI cases | `trycmd` (optional, later) | many cmd→output cases as .md/.toml files |
| PTY e2e (few) | `expectrl` (later, Phase 3) | raw-mode/interactive path only |
| Fuzzing | `cargo-fuzz` (later, Phase 4) | OBJ/DSL parsers on arbitrary bytes |
| Benchmarks | `criterion` | raster perf regression; wired when math lands |
| Test runner | `cargo-nextest` (+ `cargo test --doc`) | faster CI runs; doctests run separately |
| Coverage | `cargo-llvm-cov` | LCOV/HTML; guidance metric, not a hard gate |
| Dependency vetting | `cargo-deny` | advisories + licenses + bans (subsumes audit) |

**Key architectural enabler:** separate render-to-buffer from emit-to-terminal,
and give the CLI a headless frame-dump mode. This makes ~90% of e2e coverage
PTY-free, deterministic, and snapshot-able.

## CI gate sequence (synthesis)

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo nextest run --workspace` (or `cargo test --workspace`)
4. `cargo test --workspace --doc` (nextest skips doctests)
5. `cargo deny check` (advisories, licenses)
6. Scheduled/nightly jobs only: benches (criterion), fuzzing, coverage upload.

