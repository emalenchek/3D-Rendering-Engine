# Development Environment Setup

Everything needed to build, run, and test this project from a fresh machine.

## 1. Prerequisites

The only hard prerequisite is **rustup** (the Rust toolchain installer):

```sh
# Linux / macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Windows: download rustup-init.exe from https://rustup.rs
```

That's it for required tooling. The repo pins its compiler in
[`rust-toolchain.toml`](../rust-toolchain.toml) (stable channel + `rustfmt` + `clippy`
components): the **first `cargo` command you run in the repo auto-installs the right
toolchain** — no manual version management.

- Minimum supported Rust version (MSRV): **1.94** (also pinned as `rust-version` in `Cargo.toml`).
- A C compiler/linker is needed by some platforms (`build-essential` on Debian/Ubuntu,
  Xcode Command Line Tools on macOS, MSVC Build Tools on Windows) — rustup will tell you if it's missing.
- No GPU, no graphics SDK, no system libraries: the renderer is pure-software by design.

## 2. Build & run

```sh
git clone https://github.com/emalenchek/3d-rendering-engine
cd 3d-rendering-engine
cargo build            # builds the whole workspace (debug)
cargo run -p tte-cli -- --help    # run the `tte` binary
cargo build --release  # optimized build: target/release/tte
```

First build downloads and compiles dependencies; afterwards builds are incremental.
All dependency versions are locked by the committed `Cargo.lock`.

## 3. Repository layout

```
├── Cargo.toml             # workspace root: members, shared metadata, lints, dep versions
├── Cargo.lock             # locked dependency versions (committed: we ship a binary)
├── rust-toolchain.toml    # pinned toolchain (auto-installed by rustup)
├── rustfmt.toml           # formatter config (cargo fmt)
├── deny.toml              # dependency vetting policy (cargo deny)
├── .github/workflows/ci.yml   # CI gates: fmt, clippy, tests (3 OSes), doctests, deny
├── crates/
│   ├── tte-core/          # engine library: math, scene, rasterizer, cell buffer
│   │   ├── src/           #   (unit tests live inside these files)
│   │   └── tests/         #   integration tests incl. golden frames + snapshots/
│   └── tte-cli/           # terminal frontend
│       ├── src/lib.rs     #   all CLI logic (testable)
│       ├── src/main.rs    #   thin shim
│       └── tests/         #   e2e tests of the real binary + snapshots/
└── docs/                  # 00 brief · 01 requirements+test plan · 02 test harness · 03 this file · research/
```

## 4. Dependencies and what they're for

Runtime dependencies are currently **zero** (scaffolding stage); Phase 1 adds terminal
handling (planned: `crossterm`). Dev-dependencies (test-only, never shipped):

| Crate | Crate(s) using it | Purpose |
|---|---|---|
| `approx` | tte-core | float-tolerant assertions (`assert_relative_eq!`) |
| `proptest` | tte-core | property-based tests for math/parsers |
| `insta` | tte-core, tte-cli | golden-frame snapshot tests |
| `assert_cmd` + `predicates` | tte-cli | e2e tests running the real `tte` binary |

Versions are declared once (workspace root or crate manifests) and locked in `Cargo.lock`.
Adding a dependency: add it to the relevant `Cargo.toml`, run `cargo build`, and note it
here if it's load-bearing. `cargo deny check` must stay green (license/advisory policy in
`deny.toml`).

## 5. Test harness setup

Nothing to install — `cargo test` runs the full suite (see
[docs/02-test-harness.md](02-test-harness.md) for the complete guide). Recommended
quality-of-life CLIs, installed once into `~/.cargo/bin`:

```sh
cargo install cargo-insta     # interactive snapshot review:  cargo insta review
cargo install cargo-nextest   # faster test runner (what CI uses):  cargo nextest run
cargo install cargo-deny      # run the CI dependency gate locally:  cargo deny check
cargo install cargo-llvm-cov  # coverage reports:  cargo llvm-cov --html
```

## 6. Pre-push checklist (mirrors CI)

```sh
cargo fmt --all                                          # format
cargo clippy --workspace --all-targets -- -D warnings    # lint (CI fails on warnings)
cargo test                                               # full suite incl. doctests
cargo deny check                                         # if you changed dependencies
```

## 7. Editor setup (optional)

- **VS Code**: install *rust-analyzer* (official). Suggested settings: enable
  `rust-analyzer.check.command = "clippy"` so editor diagnostics match CI.
- **Any editor**: rust-analyzer is an LSP server and works with Neovim, Helix, Zed, JetBrains (via RustRover), etc.
- The *insta* VS Code extension renders snapshot diffs inline (optional).

## 8. Known environment notes

- CI runs on ubuntu/macos/windows with the same pinned stable toolchain; if it passes
  locally on any OS it should pass everywhere — portability failures are CI's job to catch (NFR-2).
- WASM targets (`wasm32-unknown-unknown`) are **not** needed yet; Phase 5 will add a
  `tte-wasm` crate plus target/toolchain instructions when it lands.
