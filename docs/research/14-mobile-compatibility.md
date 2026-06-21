# Research 14 — Mobile (iOS Safari) compatibility & performance

Date: 2026-06. Confidence: [H]igh / [M]edium / [L]ow.
Motivation: the live demo (`web/`) is sluggish on mobile iOS Safari. Goal: improve
mobile performance **and** compatibility **without losing any current functionality
or requirement**.

## 0. What the demo does today (the constraint surface)

The pipeline (see `web/app.js`, `web/renderer.js`, `crates/tte-wasm`):

1. `Renderer.render()` — the **CPU software rasterizer** (`tte-core`, WASM) fills a fixed
   `COLS=120 × ROWS=60` = **7,200-cell** framebuffer. Single-threaded (no `parallel` on
   wasm). Now built with `+simd128` (W5).
2. `Renderer.glyphs()/fg()/bg()` — three typed arrays copied out of wasm linear memory
   each frame (data-only boundary; **no `web-sys`**, decision V4a).
3. `GridRenderer.draw()` — a **Canvas2D** presenter blits the cell grid: per-run `fillRect`
   backgrounds, then **per non-space cell** a tinted-glyph blit.

**Requirements that must survive any change** (so we don't regress):

- **R1** Byte-identical core output — golden frames + the FR-6.3/FR-7.4 parity tests. Any
  change to what the *rasterizer* emits is out of bounds; presentation may change *how*
  pixels reach the screen but not the cell data.
- **R2** Data-only WASM boundary, no `web-sys` in the module (V4a).
- **R3** The cell/glyph model (ASCII / truecolor / half-block presenters), live editor, and
  presets (FR-5.x).
- **R4** Deploys on GitHub Pages via the existing artifact flow; **no custom HTTP headers**
  available (D1/D2); NFR-7 wasm size budget (≤250 KB).
- **R5** Runs on the current browser baseline the demo already supported.

---

## 1. ⚠️ Compatibility regression introduced by W5 (`+simd128`) — [H]

A wasm module built **only** with `-msimd128` **fails to instantiate** on Safari/iOS
**3.2–16.3** (they silently lack fixed-width SIMD; the module is rejected outright, not
run scalar). Fixed-width SIMD shipped in **Safari 16.4** (27 Mar 2023). So the W5 change
means: **on iOS < 16.4 the demo now shows a blank page instead of running** — a straight
loss of R5 vs the pre-W5 build, which loaded everywhere.

**Fix (P0, cheap):** ship a **scalar fallback** and feature-detect. Build two artifacts
(`tte_wasm_bg.wasm` simd + `tte_wasm_bg.scalar.wasm` without `+simd128`), probe support at
load with a tiny `WebAssembly.validate(<simd probe bytes>)`, and `init()` the right one.
Restores universal load; keeps the SIMD speedup where supported. Adds ~one extra wasm to
the artifact (each well under NFR-7).

Sources: [WebKit/Safari 16.4 SIMD](https://webkit.org/blog/13966/webkit-features-in-safari-16-4/),
[web-features: wasm SIMD](https://web-platform-dx.github.io/web-features-explorer/features/wasm-simd/),
[platform.uno on 16.4 fixed-width SIMD](https://platform.uno/blog/safari-16-4-support-for-webassembly-fixed-width-simd-how-to-use-it-with-c/).

---

## 2. Profile first (mirror the FR-7.1 discipline) — [H]

Before optimizing, measure on-device which of {WASM raster, Canvas2D present, DPR fill}
dominates. The loop already times frames; add a split timer around `render()` vs
`draw()` and read it via Safari **Web Inspector** remote debugging (Mac + cabled iPhone →
Develop → device → Timelines). Hypothesis below is strong but unverified per-device; the
gate is "don't rewrite the presenter if the raster dominates," and vice-versa.

---

## 3. The Canvas2D presenter is the prime mobile suspect — [H]

`GridRenderer._blitTinted` runs **once per non-space cell, every frame** and switches
`globalCompositeOperation` to `"source-in"` (plus `clearRect` + 2× `drawImage`/`fillRect`)
— i.e. **up to ~7,200 compositing-mode state changes + tiny `drawImage`s per frame**. This
is precisely the Canvas2D slow path MDN's *Optimizing canvas* warns against: avoid
unnecessary state changes, batch draws, use integer coordinates, cache scaled glyphs.
Desktop GPUs hide it; **iOS Safari's Canvas2D is far more sensitive to per-op state changes
and many small blits** (each can force a tile flush / readback). Strong candidate for the
mobile-specific slowness.

**Fix A — batch the compositing to ONE whole-frame op (exact, preserves R1):**
the per-cell `source-in` exists only to mask a white glyph by that cell's fg colour. Restructure:
1. blit every glyph shape (white) to an offscreen *ink* canvas in one pass — `source-over`,
   **no per-cell state change**;
2. paint the per-cell fg colours into a *colour* canvas (the existing run-batched `fillRect`);
3. composite `ink × colour` with `source-in` **once** over the whole canvas;
4. draw the bg field behind.
This turns ~7,200 composite switches into **one**, with **bit-identical output** (white mask
× exact per-cell colour). Pure presentation-layer; R1–R4 untouched.

**Fix B — integer coords + cached glyph metrics** (MDN): floor all blit coordinates; already
mostly integer here, but enforce it.

Sources: [MDN Optimizing canvas](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Optimizing_canvas),
[MDN globalCompositeOperation](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/globalCompositeOperation).

---

## 4. Cap devicePixelRatio + respect the iOS canvas-area cap — [H]

`renderer.js` sizes the backing store at `cellW·dpr × …`. On a `dpr=3` iPhone the 120×60
grid backs **2880×2880 ≈ 8.3 M px** and every tint scales up too — ~3× the fill/bandwidth
of a dpr-1 desktop for the *same* scene. **Cap the render DPR at ~1.5–2** (decouple backing
resolution from the device's full retina factor); near-free, large bandwidth cut, only
slightly softer glyphs. **Guardrail:** keep backing area under iOS Safari's canvas cap —
**16,777,216 px (4096²)** on iOS < 18, raised to **8192² (67 M)** in iOS 18; exceeding it
makes Safari refuse to render. The current 8.3 M is safe, but an uncapped DPR × larger grid
could trip it.

Sources: [PQINA canvas area limit](https://pqina.nl/blog/canvas-area-exceeds-the-maximum-limit/),
[iOS 18 raises canvas to 8192²](https://lionpuro.com/posts/canvas-is-finally-usable-on-safari/).

---

## 5. Adaptive resolution + frame-rate cap — [M]

The grid is hard-coded `120×60`. Add an **adaptive scale**: lower the internal render
resolution (fewer cells, or a sub-cell render scale) when measured FPS stays below a
threshold, and **cap `requestAnimationFrame` to ~30 fps on mobile** (skip alternate frames)
to cut sustained CPU and **thermal throttling** (mobile SoCs downclock within seconds, which
also caps the SIMD win). Pause on `visibilitychange`. All within the existing cell model
(R3 intact) — fewer cells is a quality knob, not a feature loss.

---

## 6. Move render+present off the main thread: OffscreenCanvas + Worker — [M]

Run the wasm `Renderer` and the Canvas2D presenter in a **Web Worker** via
**`OffscreenCanvas`** (`canvas.transferControlToOffscreen()`), posting only orbit/scene
updates from the UI thread. Keeps input responsive and removes main-thread jank; preserves
the data-only boundary (the worker owns wasm + canvas). **iOS support: Safari 16.4+**
(2D OffscreenCanvas), solid by 2026. Needs a main-thread fallback for older iOS (and pairs
naturally with the §1 scalar-fallback gating). Improves *smoothness*, not raw raster speed.

Sources: [caniuse OffscreenCanvas](https://caniuse.com/offscreencanvas),
[Safari 16.4 OffscreenCanvas](https://forum.babylonjs.com/t/safari-16-4-web-push-and-offscreen-canvas-yay/38319).

---

## 7. GPU presenter (WebGL2) — biggest win, larger change — [M-H]

Offload the cell compositing to the GPU while **keeping the CPU software rasterizer**: upload
the per-cell glyph index + fg/bg as small textures each frame and draw with **one
instanced/fullscreen WebGL2 shader** that samples a glyph-atlas texture. This deletes the
Canvas2D bottleneck (§3) entirely; the engine's identity — *software rasterizer producing a
cell grid* — is unchanged, only the **presenter** swaps. Ship it as an **alternate presenter
alongside** the Canvas2D one (auto-select, Canvas2D fallback) so **nothing is lost** (R1–R3
intact; `web_frame` data boundary preserved — uploads are JS-side).

- **WebGL2**: universally supported on iOS **15+** → the portable GPU target.
- **WebGPU**: Metal-backed, but **iOS 26+ only** — too new to depend on; a nice future
  upgrade behind the same presenter interface.

Sources: [web.dev WebGPU in browsers](https://web.dev/blog/webgpu-supported-major-browsers),
[WebGPU in iOS 26](https://appdevelopermagazine.com/webgpu-in-ios-26/), [caniuse WebGPU](https://caniuse.com/webgpu).

---

## 8. WASM threads (parallel rasterizer on wasm) — deferred — [M-L]

The `parallel` feature already exists but is native-only. Bringing it to wasm needs
SharedArrayBuffer → **cross-origin isolation** (COOP `same-origin` + COEP
`require-corp`/`credentialless`). **GitHub Pages cannot set these headers** (R4). The only
path is a **`coi-serviceworker` shim** (injects COEP `credentialless` client-side), plus the
nightly `wasm-bindgen-rayon` toolchain already costed in **research report 10** (FR-6.5).
Caveats: first-load needs a reload, COEP can block cross-origin assets, iOS SAB quirks, and
mobile has only ~2 performance cores so the win is modest. **Verdict: defer** — §3/§4/§7
deliver more for less and keep the deploy simple.

Sources: [GitHub community: COOP/COEP on Pages](https://github.com/orgs/community/discussions/13309),
[coi-serviceworker approach](https://blog.tomayac.com/2025/03/08/setting-coop-coep-headers-on-static-hosting-like-github-pages/),
[web.dev cross-origin isolation](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cross-Origin-Embedder-Policy),
report `10-wasm-threads-hosting.md`.

---

## 9. Requirements-preservation matrix

| Optimization | Touches the rasterizer output (R1)? | Boundary (R2) | Model/UX (R3) | Deploy/size (R4) | Baseline (R5) |
|---|---|---|---|---|---|
| §1 scalar fallback | no | no | no | +1 wasm (under budget) | **restores** old iOS |
| §3 batch compositing | **no — bit-identical** | no | no | no | no |
| §4 DPR cap / area guard | no (presentation only) | no | slightly softer | no | safer |
| §5 adaptive res / 30 fps | no (fewer cells = quality knob) | no | no | no | no |
| §6 OffscreenCanvas+Worker | no | no | no | no | iOS 16.4+ (needs fallback) |
| §7 WebGL2 presenter (additive) | no | no | no | +JS only | iOS 15+ (Canvas2D fallback) |
| §8 wasm threads | no (byte-identical, like native) | no | no | **needs SW + COEP** | iOS SAB-gated |
| ~~colour-quantized glyph cache~~ | **yes — changes output** | — | — | — | — |

Only the rejected colour-quantization shortcut would alter output; everything recommended is
output-preserving.

---

## 10. Recommended ordering (for a v2.2 scope)

1. **§1 scalar fallback** — P0, fixes the iOS<16.4 *regression* (correctness/compat).
2. **§3 batch the per-cell compositing** + **§4 DPR cap/area guard** — biggest cheap perf
   win, exact output, presentation-only.
3. **§5 adaptive resolution + 30 fps mobile cap** — smooths thermal throttling.
4. **§6 OffscreenCanvas + Worker** (iOS 16.4+, main-thread fallback) — responsiveness.
5. **§7 optional WebGL2 presenter** (additive, Canvas2D fallback) — the structural mobile win.
6. **§8 wasm threads** — deferred (hosting cost; report 10).

Profile (§2) gates 2 vs 7: if on-device profiling shows the *raster* (not the presenter)
dominates even after §3/§4, prioritize §7 (or revisit §8); if the presenter dominated (the
hypothesis), §3 alone may suffice for a smooth demo.

## 11. Open questions for scoping

- Verify (§2) the render-vs-present split on a real mid-range iPhone before committing to §7.
- Decide whether §7 WebGL2 is in-scope for v2.2 or a v2.3 follow-up (it's the largest item).
- Confirm the scalar-fallback artifact naming/loader fits `web/build.sh` + the deploy job.
