# Research 08: WASM Toolchain & Binary Size (Spike S2 validation)

Date: 2026-06-12. Validates spike S2 in docs/04-v2.0-scope.md. Target: `tte-wasm` cdylib, wasm-bindgen, ≤250 KB wasm (NFR-7).

## Q1. Toolchain state 2025–2026 (wasm-pack / wasm-bindgen / rustwasm org)

- **rustwasm GitHub org sunset/archived September 2025.** Official announcement (Inside Rust blog, 2025-07-21): the org saw drastically reduced activity since 2019; most projects in maintenance mode ~5 years. https://blog.rust-lang.org/inside-rust/2025/07/21/sunsetting-the-rustwasm-github-org/ — HIGH
- **wasm-bindgen survived and is actively maintained** under a new dedicated org: `wasm-bindgen/wasm-bindgen`, with new additional maintainers. Latest release **0.2.122 (2026-05-22)**; active changelog (e.g. new `--target emscripten`, panic=unwind hooks). https://github.com/wasm-bindgen/wasm-bindgen , https://github.com/wasm-bindgen/wasm-bindgen/blob/main/CHANGELOG.md — HIGH
- **wasm-pack also survived**: transferred first to original maintainer `drager`, then into the `wasm-bindgen` org (`wasm-bindgen/wasm-pack`). Latest release **v0.15.0** (adds wasm64 target, `--panic-unwind` flag, vendored template). Maintenance is real but low-cadence (roughly one minor release/year; dependency/security updates ongoing). https://github.com/wasm-bindgen/wasm-pack/releases — HIGH (existence/ownership), MEDIUM (exact dates)
- Community sentiment (nickb.dev "Life after wasm-pack", users.rust-lang.org "Future of Rust WASM" threads): many projects moved to **calling `wasm-bindgen` CLI directly** (cargo build + wasm-bindgen + wasm-opt), or to **trunk** for app-style projects. wasm-pack remains fine for library-style `--target web` output but is no longer the assumed default. https://nickb.dev/blog/life-after-wasm-pack-an-opinionated-deconstruction/ , https://users.rust-lang.org/t/future-of-rust-wasm/133089 — MEDIUM
- Synthesis: **wasm-bindgen is the safe long-term dependency; wasm-pack is alive (same org) but treat it as a convenience wrapper you can drop.** Pin the `wasm-bindgen` crate version and matching CLI version; the CLI version must exactly match the crate version.

## Q2. Recommended build pipeline (library wasm + hand-written JS, no bundler)

Two viable paths; both produce a `--target web` ES module loadable via `<script type="module">`.

**Path A — wasm-pack (convenience wrapper, still works):**
```
wasm-pack build --target web --release
# emits pkg/: tte_wasm_bg.wasm, tte_wasm.js (glue + JS init()), tte_wasm.d.ts
```
wasm-pack runs cargo + wasm-bindgen + wasm-opt for you (default wasm-opt `-O`; configure via `[package.metadata.wasm-pack.profile.release]` `wasm-opt = ["-Oz"]` in Cargo.toml). Good when you want one command.

**Path B — wasm-bindgen CLI directly (recommended for control / CI reproducibility):**
```
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/tte_wasm.wasm \
    --target web --out-dir pkg --no-typescript   # or keep .d.ts
wasm-opt -Oz -o pkg/tte_wasm_bg.wasm pkg/tte_wasm_bg.wasm
```
- The wasm-bindgen CLI version MUST exactly equal the `wasm-bindgen` crate version (pin both; install via `cargo install wasm-bindgen-cli --version =0.2.x` or use `cargo binstall`).
- wasm-opt ships in `binaryen`; install separately (not bundled with the CLI). `-Oz` = optimize aggressively for size.
- Path B is what most post-wasm-pack projects use (nickb.dev "Life after wasm-pack"); it's a 3-line Makefile/justfile and removes the wasm-pack dependency. https://rustwasm.github.io/docs/wasm-bindgen/reference/deployment.html — HIGH
- For app-style projects `trunk` is the popular alternative, but for a *library* module + hand-written JS, trunk's HTML-pipeline model adds little; Path A or B is the better fit. — MEDIUM

## Q3. Size reality — is ≤250 KB (≤150 KB gz) realistic?

Recommended release profile (Cargo.toml):
```toml
[profile.release]
opt-level = "z"      # size; measure vs "s" — "s" sometimes smaller/faster
lto = true
codegen-units = 1
panic = "abort"
strip = true
```
- **panic=abort × wasm-bindgen:** historically wasm-bindgen needed `panic=unwind`-ish behavior, but `panic="abort"` works fine for `--target web` libraries and removes landing-pad/unwind tables. wasm-bindgen 0.2.12x added explicit panic=unwind *hooks* (CHANGELOG) but abort remains the size-optimal default and is fully supported. Removing panic infra + the formatting (`core::fmt`) machinery is the single biggest win. — HIGH (profile), MEDIUM (exact panic interaction nuance)
- **What dominates:** `core::fmt` / formatting + panic message/backtrace infrastructure, then std collections monomorphization. `fmt` is pulled in by `panic!`, `unwrap`/`expect` messages, `{:?}` derives, and `format!`. Minimizing these (avoid `Debug` formatting in hot/large paths, prefer abort) is where most bytes go.
- **Data points:** Game-of-life reference (wasm-bindgen + opt-level=z + lto + wasm-opt -Oz) = **17.3 KB raw / 9.0 KB gz**. A minimal wasm-bindgen "hello" lands well under 20 KB. The baseline wasm-bindgen glue + panic/fmt floor is roughly **20–40 KB** before your code. https://rustwasm.github.io/docs/book/game-of-life/code-size.html — HIGH
- **Verdict for ~3–4 KLOC pure-compute geometry + glue:** ≤250 KB raw is comfortably realistic; ≤150 KB gz is very likely. Pure-compute f32 math/OBJ parsing adds modest code; the risk is heavy generics, `format!`-based error strings, or pulling a large parser/serde stack. Keep deps lean (avoid serde+serde_json for scene if a hand parser suffices; or use a minimal parser). Profile with **`twiggy top pkg/tte_wasm_bg.wasm`** and `twiggy dominators` to find bloat; `wasm-snip` to strip unreachable panic paths. Nightly `build-std` + `panic_immediate_abort` gives a further 10–20% if ever needed (not required for budget). https://rustwasm.github.io/docs/book/game-of-life/code-size.html — HIGH

(Sections Q4–Q8 follow as research proceeds.)
