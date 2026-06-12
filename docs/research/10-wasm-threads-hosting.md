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
