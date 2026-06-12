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
- **panic=abort × wasm-bindgen:** on `wasm32-unknown-unknown` panics already translate to aborts (no real unwinding), so `panic="abort"` gives the same runtime behavior *without* the unwinding/landing-pad code bloat — it is the size-optimal, recommended setting and works fine for `--target web` libraries. The Leptos/yew/rustwasm size guides all recommend `panic="abort"`. **Nuance:** wasm-bindgen's newer *hard-abort detection / abort-recovery handler* API (see Cloudflare 2026 "Making Rust Workers reliable") currently requires `panic="unwind"`; that matters only for long-lived workers that must recover from panics — NOT for a render demo, so keep `panic="abort"`. Removing panic infra + `core::fmt` is the single biggest size win. https://book.leptos.dev/deployment/binary_size.html , https://blog.cloudflare.com/making-rust-workers-reliable/ — HIGH
- **What dominates:** `core::fmt` / formatting + panic message/backtrace infrastructure, then std collections monomorphization. `fmt` is pulled in by `panic!`, `unwrap`/`expect` messages, `{:?}` derives, and `format!`. Minimizing these (avoid `Debug` formatting in hot/large paths, prefer abort) is where most bytes go.
- **Data points:** Game-of-life reference (wasm-bindgen + opt-level=z + lto + wasm-opt -Oz) = **17.3 KB raw / 9.0 KB gz**. A minimal wasm-bindgen "hello" lands well under 20 KB. The baseline wasm-bindgen glue + panic/fmt floor is roughly **20–40 KB** before your code. https://rustwasm.github.io/docs/book/game-of-life/code-size.html — HIGH
- **Verdict for ~3–4 KLOC pure-compute geometry + glue:** ≤250 KB raw is comfortably realistic; ≤150 KB gz is very likely. Pure-compute f32 math/OBJ parsing adds modest code; the risk is heavy generics, `format!`-based error strings, or pulling a large parser/serde stack. Keep deps lean (avoid serde+serde_json for scene if a hand parser suffices; or use a minimal parser). Profile with **`twiggy top pkg/tte_wasm_bg.wasm`** and `twiggy dominators` to find bloat; `wasm-snip` to strip unreachable panic paths. Nightly `build-std` + `panic_immediate_abort` gives a further 10–20% if ever needed (not required for budget). https://rustwasm.github.io/docs/book/game-of-life/code-size.html — HIGH

## Q4. Returning per-frame cell buffers efficiently (view vs copy)

- **wasm-bindgen returns typed arrays as a COPY by default.** When a Rust `fn` returns `Vec<u8>` / `Box<[u8]>`, the generated glue copies the bytes out of linear memory into a fresh JS `Uint8Array` (then frees the Rust allocation). This is safe and self-contained — the returned array is independent of wasm memory. — HIGH
- **Zero-copy "view" (`Uint8Array::view`) is possible but fragile.** `js_sys::Uint8Array::view(&[u8])` / `view_mut_raw(ptr, len)` create a JS view directly over wasm linear memory (no copy). **Danger:** the view is only valid until wasm memory grows. Any allocation (Box::new, Vec push, malloc) can trigger `memory.grow`, which **detaches the old ArrayBuffer and vends a new one**, silently invalidating every outstanding view. So a view must be consumed immediately, before any further wasm calls/allocations. https://github.com/wasm-bindgen/wasm-bindgen/issues/1643 , https://docs.rs/js-sys/latest/js_sys/struct.Uint8Array.html — HIGH
- **Boundary cost at 60 fps:** the call boundary itself is cheap (~tens of ns per call). The real cost is the memcpy of the cell buffer. For a character-cell frame (e.g. 200×60 = 12 000 cells × a few typed arrays of glyph/fg/bg = tens of KB), a per-frame copy is a few tens of KB memcpy — negligible at 60 fps (sub-millisecond). — MEDIUM
- **Recommended pattern:** keep a persistent output buffer **owned by the wasm struct**; expose a stable pointer + length getter, and have JS construct a `Uint8Array` view ONCE over `wasm.memory.buffer` at the right offset, **re-creating the view only after a `memory.grow`** (detect via `buffer.byteLength` change or after any call that might allocate). For one demo this is over-engineering: returning copies (`Vec<u8>` → fresh `Uint8Array`, or `Uint8ClampedArray` for canvas `ImageData`) is simplest and fast enough. If you pre-allocate the frame buffers once at `new()`/`set_size()` and never grow during steady-state `render()`, a stable view is safe and truly zero-copy. — HIGH (mechanism), MEDIUM (the 60fps verdict)
- **Practical rule:** allocate all per-frame buffers up front; avoid heap growth inside `render()`; then either (a) return copies (simple, safe) or (b) expose `frame_ptr()`/`frame_len()` and view once (zero-copy). Both meet 60 fps for character-cell frames.

## Q5. Passing OBJ / scene text in (`&str` cost)

- A wasm-bindgen `pub fn load_obj(&mut self, text: &str)` copies the JS string's UTF-8 bytes into wasm linear memory once per call (JS strings are UTF-16; the glue encodes to UTF-8 via `TextEncoder` and writes into a wasm allocation). For **one-shot loads** (load_obj/load_scene called once at startup, not per frame) this copy is completely fine — even a multi-MB OBJ is a single sub-ms-to-few-ms encode+copy, dwarfed by parsing. — HIGH (mechanism), MEDIUM (timing)
- Take `&str` (read-only, freed after call) rather than `String` to avoid an extra owning allocation. Don't pass text per-frame; parse once into an internal mesh and reuse. No optimization needed here for the spike.

## Q6. wasm-bindgen-test in 2025/2026 (runner, headless, CI)

- `wasm-bindgen-test` is the test harness; `#[wasm_bindgen_test]` replaces `#[test]`. **Default target is Node.js**; force browser with `wasm_bindgen_test_configure!(run_in_browser);` per-module, or set the env var to use a browser globally. Tests can also target dedicated/shared/service workers. https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/usage.html , https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html — HIGH
- **Two ways to run:**
  - `wasm-pack test --headless --chrome` (or `--firefox`/`--node`): wasm-pack installs the test runner + a matching WebDriver and wires up cargo's custom test runner. Easiest. https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html — HIGH
  - Pure wasm-bindgen CLI: install `wasm-bindgen-test-runner` (matching version), set `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`, run `cargo test --target wasm32-unknown-unknown`. Pick Chrome/Firefox via `WASM_BINDGEN_USE_BROWSER` / driver env vars. Works without wasm-pack (fits the Path-B toolchain). — MEDIUM
- **For a pure-compute geometry core, prefer `--target wasm32-unknown-unknown` + Node** (no DOM needed → no WebDriver, fastest CI). Reserve headless-browser tests for code that actually touches `web-sys`. — HIGH (reasoning)
- **GitHub Actions recipe (Node, no browser — recommended for tte-wasm):**
  ```yaml
  - uses: dtolnay/rust-toolchain@stable
    with: { targets: wasm32-unknown-unknown }
  - uses: jetli/wasm-pack-action@v0.4.0   # or cargo-binstall wasm-pack
  - run: wasm-pack test --node          # pure-compute tests, no browser
  ```
  For browser coverage add Chrome (`browser-actions/setup-chrome`) and use `wasm-pack test --headless --chrome`. GitHub-hosted Ubuntu runners ship Chrome+Firefox+chromedriver, so often no extra install is needed. — MEDIUM
- You can ALSO keep ordinary native `#[test]`s in the pure-Rust core crate (`cargo test` on the host) for the geometry math — far faster than wasm — and reserve wasm-bindgen-test for the boundary/glue. This is the recommended split. — HIGH

## Q7. web-sys in the wasm crate, or keep canvas/DOM in JS?

- **wasm-bindgen only emits glue for the JS imports you actually use**, and `web-sys` is feature-gated per interface — so pulling canvas-2D in (`features = ["Window","Document","HtmlCanvasElement","CanvasRenderingContext2d","ImageData"]`) adds glue but not enormous binary bloat. A reference WASM canvas demo was ~43.6 KB wasm + 4.8 KB JS. https://rustwasm.github.io/docs/wasm-bindgen/examples/2d-canvas.html — MEDIUM
- **But the consensus 2025/2026 pattern is hybrid: compute in wasm, DOM/canvas in JS.** Multiple sources recommend wasm for the heavy compute and JS for UI/canvas drawing to minimize binary bloat and keep glue simple. https://users.rust-lang.org/t/where-to-do-things-between-wasm-and-javascript/84171 , https://www.iloveblogs.blog/post/web-assembly-javascript-performance-2026 — MEDIUM
- **Verdict for tte-wasm: keep web-sys OUT of the wasm crate.** The API already returns per-cell typed arrays — pure data. Let hand-written JS own the canvas/`<pre>` and paint cells from those arrays. Benefits: smallest binary (no web-sys/`js_sys` DOM features, fewer imports), simplest glue, easiest testing (Node target, no headless browser), clean compute/UI separation, and the wasm core stays reusable (e.g. native/CLI). Only reach for web-sys if you later want wasm to drive the canvas directly for perf — not needed when the per-frame data is small character-cell buffers. — HIGH (recommendation)

## Q8. Local serving + GitHub Pages (application/wasm MIME)

- **`<script type="module">` + `init()` (the `--target web` glue) uses `WebAssembly.instantiateStreaming`, which REQUIRES the `application/wasm` MIME type** or it errors (some browsers fall back to non-streaming, but don't rely on it). So the server must serve `.wasm` with the right content type. https://rustwasm.github.io/book/reference/deploying-to-production.html — HIGH
- **GitHub Pages serves `.wasm` as `application/wasm` correctly in production** (the well-known type is in its mapping). The classic failure is **local Jekyll/`jekyll serve`**, which can serve the wrong content type; production Pages is fine. Use a `.nojekyll` file to skip Jekyll processing for a plain static demo. https://github.com/github/pages-gem/issues/695 , https://community.latenode.com/t/wasm-file-serving-with-incorrect-content-type-on-github-pages/32127 — HIGH
- **Local dev servers that set `application/wasm` out of the box:** `python3 -m http.server` (modern Python maps .wasm correctly), `npx serve`, `miniserve`, `basic-http-server` (rustwasm's own), or trunk's dev server. Avoid opening via `file://` (module + streaming fetch won't work; needs http). — HIGH
- **Gotchas:** (1) cross-origin isolation headers (COOP/COEP) are only needed for threads/SharedArrayBuffer — NOT for a single-threaded demo, so no special headers required on Pages. (2) Set correct relative paths for the `.wasm` (the `--target web` glue fetches `<name>_bg.wasm` next to the JS by default; keep them together). (3) Add `.nojekyll` so files starting with `_` aren't stripped. — HIGH

## Recommended build pipeline + API-boundary design

**Crate layout:** pure-compute core crate (`tte-core`, no wasm deps, native `#[test]`s) + thin `tte-wasm` cdylib that only wraps it with `#[wasm_bindgen]`. **No `web-sys`/DOM in wasm** — the API returns per-cell typed arrays; JS owns the canvas/`<pre>`.

**API (data-only boundary):**
- `new() -> Renderer` (constructor); `load_obj(&mut self, text: &str)`; `load_scene(&mut self, text: &str)` — one-shot, `&str` copy is fine.
- `set_orbit(&mut self, yaw: f32, pitch: f32, radius: f32)`.
- `render(&mut self)` writing into pre-allocated internal buffers; expose either copies (`-> Uint8Array`/`Vec<u8>`, simplest) or `frame_ptr()`/`frame_len()` getters for a zero-copy view. Pre-allocate all frame buffers at `new()`/`set_size()` so steady-state `render()` never grows memory (keeps views valid).

**Build (Path B — wasm-bindgen CLI directly, pinned & reproducible; wasm-pack Path A is the convenience equivalent):**
```
cargo build --release --target wasm32-unknown-unknown -p tte-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/tte_wasm.wasm \
    --target web --out-dir pkg
wasm-opt -Oz -o pkg/tte_wasm_bg.wasm pkg/tte_wasm_bg.wasm
twiggy top pkg/tte_wasm_bg.wasm   # profile bloat
```
Pin `wasm-bindgen` crate == `wasm-bindgen-cli` version (install via cargo-binstall). `binaryen`/`wasm-opt` installed separately. Release profile: `opt-level="z"`, `lto=true`, `codegen-units=1`, `panic="abort"`, `strip=true`.

**Testing:** native `cargo test` on `tte-core` for geometry math; `wasm-pack test --node` (or `wasm-bindgen-test-runner` + Node) for the boundary — no headless browser needed since no DOM in wasm.

**Serve/deploy:** local `python3 -m http.server` (serves `application/wasm`); GitHub Pages with a `.nojekyll` file (production Pages serves `.wasm` correctly; no COOP/COEP needed for single-threaded). Keep `tte_wasm.js` + `tte_wasm_bg.wasm` co-located; load via `<script type="module">` calling `init()`.

**Size verdict (NFR-7):** ≤250 KB raw is **comfortably achievable** and ≤150 KB gz **very likely** for ~3–4 KLOC of pure-compute f32 geometry + OBJ parsing + glue, GIVEN: keep deps lean (avoid serde/serde_json — hand-parse OBJ/scene), keep DOM out of wasm, panic=abort + opt-level=z + lto + wasm-opt -Oz, and avoid `format!`/`Debug` in large/hot paths. Baseline floor (glue + panic/fmt) ≈ 20–40 KB; a Game-of-Life-class app is ~17 KB raw / ~9 KB gz. The geometry core leaves ample headroom under 250 KB. Profile with `twiggy`; if ever tight, nightly `build-std` + `panic_immediate_abort` reclaims another 10–20%. **NFR-7: PASS (high confidence).**

