# FR-6.5 — Threaded WASM build (wasm-bindgen-rayon) — toolchain + hosting cost

Research for v2.0 Phase 6 STRETCH goal. Validates spike S4 in docs/04-v2.0-scope.md.
Question: does a threaded wasm rasterizer variant earn its keep vs. staying single-threaded?
Date: 2026-06-12. Confidence legend: [H]igh / [M]edium / [L]ow.

---

## 1. wasm-bindgen-rayon: version, maintenance, toolchain

- **Project**: `RReverser/wasm-bindgen-rayon` (originally GoogleChromeLabs; same codebase, repo moved to maintainer's personal org). Still actively maintained by RReverser (Ingvar Stepanyan). [H]
- **Current version**: 1.2 / 1.3.0 on crates.io (docs.rs shows 1.3.0 latest). [H]
- **Toolchain — STILL NIGHTLY**: requires a fixed nightly (`nightly-2025-11-15` per current README), `rust-src` component, `wasm32-unknown-unknown` target, and `-Z build-std=panic_abort,std` to rebuild std with atomics. No move to stable; threads remain nightly-only. [H]
- **RUSTFLAGS** (exact):
  ```
  -C target-feature=+atomics,+bulk-memory
  -C link-arg=--shared-memory
  -C link-arg=--max-memory=1073741824   # 1 GiB preset — MUST match real need
  -C link-arg=--import-memory
  -C link-arg=--export=__wasm_init_tls
  -C link-arg=--export=__tls_size
  -C link-arg=--export=__tls_align
  -C link-arg=--export=__tls_base
  ```
  (Note: `mutable-globals` is implied/enabled by the atomics+bulk-memory toolchain for TLS; older docs listed it explicitly.) [H]
- **Build recipe**:
  ```
  rustup run nightly-2025-11-15 wasm-pack build --target web -- -Z build-std=panic_abort,std
  ```
  Only `--target web` is supported (not bundler/no-modules). [H]
- **Pitfall — max-memory preset**: shared memory needs a fixed `--max-memory`. The 1 GiB default over-reserves address space; a software rasterizer with large framebuffers must tune this or risk OOM / wasted reservation. [H]
- **Worker pool**: JS-side `await initThreadPool(navigator.hardwareConcurrency)` after instantiation; spins up a Web Worker per core, each re-instantiating the module over shared memory. [H]

Sources:
- https://github.com/RReverser/wasm-bindgen-rayon
- https://github.com/GoogleChromeLabs/wasm-bindgen-rayon
- https://docs.rs/crate/wasm-bindgen-rayon/latest
- https://crates.io/crates/wasm-bindgen-rayon/versions

---

## 2. SharedArrayBuffer / cross-origin isolation requirements

- SAB (and therefore wasm threads + shared memory) is gated behind **cross-origin isolation** in all engines since Chrome 92 (2021). Page must send both:
  - `Cross-Origin-Opener-Policy: same-origin`
  - `Cross-Origin-Embedder-Policy: require-corp` (or `credentialless` on Chromium). [H]
- `self.crossOriginIsolated === true` is the runtime gate; if false, `SharedArrayBuffer`/`initThreadPool` will fail. Must feature-detect and fall back to the single-threaded build. [H]
- **What breaks under isolation**: every cross-origin subresource (images, scripts, fonts, iframes, analytics) must serve `Cross-Origin-Resource-Policy` (CORP) or be CORS-fetched, else it is blocked. Third-party iframes/widgets lacking CORP simply won't load. For a self-contained renderer demo this is low risk; for a page embedding third-party content it is a real constraint. `credentialless` COEP relaxes this for no-credential subresources (Chromium only; not Safari). [H]

Sources:
- https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cross-Origin-Embedder-Policy
- https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer

---

## 3. Hosting: GitHub Pages vs header-capable hosts

- **GitHub Pages**: cannot set custom response headers — no way to send COOP/COEP directly. [H]
- **coi-serviceworker** (`gzuidhof/coi-serviceworker`) is the standard workaround: a service worker re-fetches responses and injects COOP/COEP, making static hosts cross-origin-isolated. Caveats: [H]
  - **First-load reload**: the SW isn't controlling the page on the very first visit, so it forces one page reload to take effect (visible flash / extra load).
  - Must be served from your own origin, in its own file, **not** from a CDN, not bundled.
  - HTTPS or localhost only.
  - Adds a service-worker caching layer you now own (cache-busting, update lifecycle).
  - Safari historically the flakiest target for SW-injected isolation; reliability is "good enough for demos," not bulletproof. Third-party CORP-less subresources still break exactly as with real headers.
- **Header-capable hosts (preferred if threading is kept)**:
  - **Netlify**: `_headers` file or `netlify.toml [[headers]]` — native, reliable.
  - **Cloudflare Pages**: `_headers` file (same syntax as Netlify).
  - **Vercel**: `vercel.json` `headers` array.
  These set real headers, no service-worker hack, no first-load reload. [H]

Sources:
- https://github.com/gzuidhof/coi-serviceworker
- https://blog.tomayac.com/2025/03/08/setting-coop-coep-headers-on-static-hosting-like-github-pages/
- https://docs.wasmer.io/sdk/wasmer-js/how-to/coop-coep-headers/
- https://webcontainers.io/guides/configuring-headers

---

## 4. Browser / threading snapshot 2026

- WASM threads + atomics + SAB: **Chrome 74+, Firefox 79+, Safari 14.1+, Edge 79+**; global support ~95%, all major engines ship it on desktop + mobile. [H]
- **iOS**: all iOS browsers (incl. "Chrome"/"Firefox" for iOS) are WebKit, so behavior == Safari. SAB works but only under cross-origin isolation; same COOP/COEP requirement. Safari does **not** support COEP `credentialless`, so you must use full `require-corp` (stricter CORP requirement) for iOS. [H]
- `navigator.hardwareConcurrency`: used to size the pool. Safari historically **caps/under-reports** core count (and on some iOS it returns conservative values), so expected speedup on Apple devices is lower than core count suggests. [M]
- Safari SAB gotchas: stricter isolation, no `credentialless`, more conservative `hardwareConcurrency`, and historically slower to honor SW-injected headers — Apple platforms are the weakest link for a threaded build. [M]

Sources:
- https://caniuse.com/wasm-threads
- https://platform.uno/blog/the-state-of-webassembly-2025-2026/
- https://reintech.io/blog/webassembly-browser-support-2026-compatibility-guide

---

## 4a. Stable-tracking status (added 2026-06-12)

- WebAssembly atomics tracking issue **rust-lang/rust#77839 still OPEN**;
  `target_feature = "atomics"` remains **nightly-only**. No stabilization, and
  **no precompiled std-with-atomics** ships — `-Z build-std=panic_abort,std` is
  still mandatory to rebuild std. [H]
- It is actively *moving but not converging*: fresh breakage reports through
  Aug 2025 (e.g. #145101 "build wasm32-unknown-unknown failed with atomics",
  const-fn `Mutex::new`/`Condvar::new` issues, cargo #13035) — reinforces the
  "pin a known-good nightly" rule; a floating nightly will eventually break the
  threaded job. [H]

Sources:
- https://github.com/rust-lang/rust/issues/77839
- https://github.com/rust-lang/rust/issues/145101
- https://github.com/rust-lang/cargo/issues/13035

> Maintenance note: **GoogleChromeLabs/wasm-bindgen-rayon was ARCHIVED
> 2024-07-17** (author left Google); the live fork is
> **RReverser/wasm-bindgen-rayon** (the one this report tracks). So "maintained"
> = one-person personal fork, not an org-backed project. Bus-factor = 1. [H]

---

## 5. Realistic payoff

- **Mandelbrot demo (RReverser official):** single-thread **273 ms** ->
  multi-thread **87 ms** ~= **3.1x** on that machine (embarrassingly parallel,
  near-best case). [H]
- **Real apps (Squoosh image codecs):** consistent **1.5x-3x** from threads
  alone, more when combined with SIMD. A tile-based rasterizer is closer to the
  embarrassingly-parallel end, so expect **upper half of that range on desktop
  with 4-8 real cores**; far less on mobile/Safari. [M]
- **Worker-pool spin-up overhead:** `initThreadPool(N)` **greedily** instantiates
  N Web Workers, each re-instantiating the wasm module over shared memory — a
  one-time cost of tens-to-hundreds of ms at startup, *not* per-frame. For a
  long-lived interactive renderer this amortizes; for a one-shot render it can
  erase the win. Pick N from `navigator.hardwareConcurrency`; **over-provisioning
  spins up idle workers** (unlike native rayon's lazy pool). [H]
- **max-memory preset pitfall:** shared `WebAssembly.Memory` cannot grow past the
  link-time `--max-memory`. Default 1 GiB over-reserves; but a software rasterizer
  with large/multiple framebuffers can *also* hit the ceiling if set too low.
  Must be tuned to real framebuffer + scene footprint — a per-build constant you
  now own. [H]

Sources:
- https://rreverser.com/wasm-bindgen-rayon-demo/
- https://web.dev/articles/webassembly-threads
- https://github.com/RReverser/wasm-bindgen-rayon

---

## 6. Long-term ops burden for a threaded variant

If FR-6.5 is built, the project permanently carries:
1. **A nightly CI job** pinned to a specific nightly (+ `rust-src`) that rebuilds
   std via `-Z build-std`. Slower builds; breaks on nightly churn; needs periodic
   re-pinning. Separate from the stable matrix. [H]
2. **Dual wasm artifacts** (single-thread stable default + threaded nightly),
   doubling build/test/size-budget surface and golden-frame parity checks across
   both. [H]
3. **Header-aware hosting**: either move the demo off GitHub Pages to a
   header-capable host (Netlify/Cloudflare Pages/Vercel `_headers`), or ship and
   maintain **coi-serviceworker** with its first-load reload + SW cache lifecycle.
   Plus the cross-origin-isolation tax: every third-party subresource needs CORP. [H]
4. **Runtime fallback detection**: JS must check `self.crossOriginIsolated` (and
   SAB availability) and **load the single-thread build when isolation fails** —
   so the single-thread path must stay first-class anyway. The threaded build is
   strictly *additive*, never a replacement. [H]

Net: threaded variant is a **separate toolchain + separate artifact + separate
hosting story + a fallback path**, maintained around a bus-factor-1 dependency on
a nightly compiler feature with no stabilization date.

---

## Verdict for FR-6.5 (stretch): KEEP-AS-STRETCH (do NOT promote, do NOT drop)

- **Don't promote**: nightly-only + build-std + dual artifacts + header hosting +
  bus-factor-1 dep is too much standing cost to make it a committed deliverable,
  for a 1.5-3x win that **only lands on cross-origin-isolated desktop** and is
  weakest exactly where most users are (mobile/Safari).
- **Don't drop**: the payoff is real (~3x on a parallel rasterizer demo), the
  recipe is well-trodden, and the single-thread fallback (already required by
  V5/V7) means the threaded build is purely additive and risk-isolated. It's a
  legitimate, time-boxed showcase. Keep it gated behind a feature + separate CI
  job, host the demo on a header-capable host, and only spend the hours if Phase 6
  finishes early.

### Minimal viable recipe (if kept)
```toml
# rust-toolchain.toml
[toolchain]
channel = "nightly-2025-11-15"     # PIN; bump deliberately
components = ["rust-src"]
targets   = ["wasm32-unknown-unknown"]
```
```bash
# build (threaded variant only)
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory' \
  wasm-pack build --target web -- -Z build-std=panic_abort,std
```
```js
// runtime: feature-detect, else load the stable single-thread build
import init, { initThreadPool } from './pkg-threaded/renderer.js';
if (self.crossOriginIsolated) {
  await init();
  await initThreadPool(navigator.hardwareConcurrency);
} else {
  await import('./pkg-single/renderer.js').then(m => m.default());
}
```
- **Hosting:** prefer **Cloudflare Pages / Netlify `_headers`** with
  `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy:
  require-corp`. Use **coi-serviceworker only** if the demo must stay on GitHub
  Pages (accept first-load reload + Safari flakiness).
- **CI:** one extra nightly job producing `pkg-threaded/`; stable matrix
  unchanged; golden frames assert byte-parity between both wasm builds.
