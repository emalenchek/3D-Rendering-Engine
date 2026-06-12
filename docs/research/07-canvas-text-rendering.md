# Research S1: Canvas Text-Grid Rendering for Browser Frontend

Status: COMPLETE. Date: 2026-06-12.
Decides renderer design for v2.0 Phase 5 (see docs/04-v2.0-scope.md). Target: NFR-8, ≥30 FPS at 120×60 (~7,200 cells), stretch 200×100 (~20,000 cells), full-frame changes every frame (3D animation).

## Q1: Canvas 2D `fillText` performance

- Mozilla bug analysis: most canvas rendering time in text-heavy workloads is spent in `fillText`, and only ~40% of that is actual glyph rasterization; each `fillText` call pays overhead of creating a text run + glyph array. Per-glyph fillText calls are the pathological case. (https://bugzilla.mozilla.org/show_bug.cgi?id=527386, https://bugzilla.mozilla.org/show_bug.cgi?id=1110580) — MEDIUM confidence (old bugs, but the per-call-overhead structure still holds; engines have improved since).
- Mirko Sertic writeup ("Tuning HTML5 canvas fillText"): replacing per-frame fillText with pre-rendered glyph sprites (drawImage from offscreen canvas) was a "game-changing" optimization; fillText identified as terribly slow except in Chrome. (https://www.mirkosertic.de/blog/2015/03/tuning-html5-canvas-filltext/) — MEDIUM (2015, directional).
- Known cheap wins: batch cells by `fillStyle` (set style once per color run), never touch `ctx.font` inside the loop (font set is one of the most expensive state changes), draw whole same-style runs as one fillText string when cell width is uniform (monospace). — MEDIUM (widely reported practice; concrete per-call numbers pending below).

## Q2: Glyph atlas + prior art (xterm.js, VS Code)

- xterm.js has three renderers: DOM (default, broadest compatibility), Canvas addon (fallback), WebGL addon (fastest). Canvas addon is explicitly positioned as "fallback for the WebGL addon ... when WebGL2 isn't supported". (https://www.npmjs.com/package/@xterm/addon-canvas, https://github.com/xtermjs/xterm.js/issues/3271) — HIGH.
- The canvas renderer itself uses a **texture atlas**: glyphs rasterized once into an ImageBitmap-backed atlas, then `drawImage` per cell; ImageBitmap can be GPU-resident, improving blit speed considerably. — HIGH (xterm.js source/docs).
- xterm.js WebGL renderer (PR #1790, Tyriar): builds a Float32Array of per-cell instance data + texture atlas, `drawElementsInstanced` (WebGL2). **Measured render-time numbers from the PR:**
  - MacBook 87×26: Canvas 4.80 ms → WebGL 0.69 ms (~7×)
  - MacBook 300×80 (24,000 cells): Canvas 15.28 ms → WebGL 3.69 ms (~4×)
  - Windows 87×26: Canvas 7.31 ms → WebGL 0.73 ms (~9×)
  (https://github.com/xtermjs/xterm.js/pull/1790) — HIGH (primary source, author benchmarks).
  - KEY DATAPOINT for us: even Canvas2D-atlas at **300×80 = 24k cells takes ~15 ms/frame** → ~60 FPS borderline, but our 120×60 (7.2k cells) extrapolates to ~4-6 ms/frame → comfortably ≥30 FPS, likely 60.
- VS Code terminal renderer blog (2017): moving DOM → canvas+atlas gave **5–45× faster rendering**; subsequently moved default to WebGL (3–5× further). (https://code.visualstudio.com/blogs/2017/10/03/terminal-renderer — 403 on direct fetch; numbers corroborated by search summary and xterm.js docs) — MEDIUM-HIGH.

## Q3: putImageData / pixel blitting (half-block mode)

- General benchmark consensus: `drawImage` (canvas/ImageBitmap source) usually beats `putImageData` because drawImage stays on the GPU path while putImageData does a CPU→GPU upload of raw pixels every call; putImageData also ignores transforms/clip and forces sync pixel upload. (https://www.measurethat.net/Benchmarks/Show/9510/0/putimagedata-vs-drawimage, https://jsben.ch/canvas-2d-putimagedata-vs-drawimage-xc4uu) — MEDIUM (microbenchmarks, browser-dependent).
- However: for half-block/full-frame color modes where every pixel changes anyway, ONE putImageData of a full-resolution ImageData (filled from a wasm-exported RGBA buffer) is a single upload — viable and simple. Writing one ImageData per frame at e.g. 960×960 px is well within budget. — MEDIUM (to be confirmed with more sources below).

## Q1b: Concrete `fillText` throughput numbers

- emirpasic / browser-font-rendering-test: rendering **2048 individual characters** with per-char `fillText` cost ~26 ms/frame (Firefox 53), ~16 ms (Edge 14), ~7 ms (Chromium 59). → Chromium ≈ **2048 glyphs / 7 ms ≈ 290k glyphs/sec** worst-case-ish per-call; Firefox ≈ 80k glyphs/sec. (https://github.com/emirpasic/browser-font-rendering-test, corroborated via HN/search) — MEDIUM (2017-era browser versions; modern engines faster, but order of magnitude holds: per-glyph fillText is ~10⁵ glyphs/s, color-batched strings are several × that).
- For our 7,200-cell grid at 60 FPS we need 7,200 × 60 = **432k glyph-draws/sec**. Per-glyph fillText on 2017 Chromium (~290k/s) would NOT hit 60 FPS but WOULD clear 30 FPS (216k/s needed). On modern Chromium (V8/Skia improvements since 2017) per-glyph fillText is materially faster, and **color-run batching** (one fillText per same-color run, monospace fixed advance) cuts call count dramatically when frames have color locality. — MEDIUM.
- debevv/canvas-fill-text-opt: confirms worker-parallelized fillText yields only ~**20%** speedup "given the right conditions" — i.e. fillText itself is the floor; the real win is avoiding it (atlas). (https://github.com/debevv/canvas-fill-text-opt) — MEDIUM.
- Practical fillText rules reconfirmed: set `ctx.font` ONCE (font parsing/shaping is among the costliest state ops), set `textBaseline`/`textAlign` once, group `fillStyle` changes (color runs), avoid sub-pixel x/y (integer coords skip extra rasterization). — MEDIUM-HIGH.

## Q2b: hterm / Hyper prior-art (renderer evolution)

- Renderer evolution across the terminal ecosystem is consistent and one-directional: **DOM → Canvas+atlas → WebGL**, each step for throughput.
  - Hyper terminal: v1 used hterm with a **DOM-based** renderer ("flexible thanks to CSS, but very slow"); Hyper 2 switched to **xterm.js canvas** renderer; Hyper 3 rewrote to **WebGL**. (https://dotdev.co/hyper-3/, https://hyper.is/blog) — HIGH.
  - hterm (Chromium OS / Secure Shell) is DOM/`<x-row>`-based; it scrolls well via clever DOM reuse but is the slow baseline that xterm.js canvas/WebGL displaced. — MEDIUM.
  - DeepWiki summary of xterm.js: WebGL renderer "is super fast and scales much better with really large viewports" because it uploads a Float32Array of all draw data once and a shader does the drawing; canvas renderer is the WebGL fallback. (https://deepwiki.com/xtermjs/xterm.js/1-overview) — HIGH.
- TAKEAWAY: nobody serious ships per-glyph fillText for high-refresh terminals; the universal first optimization is the **glyph atlas (drawImage per cell)**, and WebGL is reserved for very large viewports / max throughput.

## Q5: requestAnimationFrame + WASM data transfer + OffscreenCanvas

- Data path: WASM core should export the cell grid as **typed arrays** (e.g. a flat `Uint8Array`/`Uint32Array` over linear memory: glyph index + packed fg/bg). JS reads it as a view onto `WebAssembly.Memory` — zero-copy, no per-cell JS function calls. The renderer loops the typed array in a tight JS loop issuing drawImage(atlas)/fillText. This is the single most important architectural rule: **never make per-cell JS↔WASM calls**; hand over one buffer per frame. — HIGH (standard wasm-bindgen pattern; logically certain).
- OffscreenCanvas: lets the entire render loop run in a **Web Worker**, off the main thread, so input/DOM jank never stalls drawing and vice-versa. `canvas.transferControlToOffscreen()` → worker; worker can run its own rAF. Broadly supported (Chrome/Edge/Firefox; Safari 16.4+). (https://web.dev/articles/offscreen-canvas, https://developer.mozilla.org/en-US/docs/Web/API/OffscreenCanvas) — HIGH.
- OffscreenCanvas raw-throughput gain is modest (it removes DOM-sync overhead, not draw cost); its real value is **isolation/smoothness** — the WASM sim + render can live in the worker, main thread stays responsive. Defer if Phase 5 is single-threaded-simple; it's an additive win, not a correctness requirement. — MEDIUM-HIGH.

## Q4: WebGL / WebGPU threshold — is Canvas2D enough at 7k–20k cells?

- The xterm.js PR #1790 numbers are the best anchor: Canvas2D-atlas renders **87×26 (~2.3k cells) in ~5–7 ms** and **300×80 (24k cells) in ~15 ms** per frame. Linear-ish in cell count → ~0.6 µs/cell for Canvas2D-atlas. Our targets:
  - 120×60 = 7,200 cells → ~4–5 ms/frame (Canvas2D-atlas) → ~200+ FPS headroom on draw alone; trivially ≥30, likely 60.
  - 200×100 = 20,000 cells → ~12–13 ms/frame (Canvas2D-atlas) → ~75 FPS ceiling on draw; still clears 30, and 60 is plausible but tighter once WASM sim + ImageData/blit overhead are added.
  WebGL at the same sizes is 0.7–3.7 ms (≈4–9×). So WebGL is *needed* only if we want comfortable 60 FPS at the 20k-cell stretch target or run on weak GPUs/iGPUs; for the primary 120×60 target Canvas2D-atlas is plainly sufficient. (https://github.com/xtermjs/xterm.js/pull/1790) — HIGH.
- Real-world failure mode of Canvas2D: xterm.js issue #4175 reports the canvas renderer becoming pathologically slow on *very wide* containers (huge pixel dimensions / very large viewports), which is exactly the "scales poorly with large viewports" reason xterm.js made WebGL the default. This is a large-viewport problem, well above our 120×60; the 200×100 stretch is the zone where it starts to matter. (https://github.com/xtermjs/xterm.js/issues/4175) — MEDIUM-HIGH.
- WebGPU: for a text grid the upside over WebGL is marginal — terminals are draw-call/upload bound, not compute bound; WebGPU's compute pipeline and multithreaded command encoding don't buy much here, while costing more code and narrower support (Chrome 113+ default; Safari/Firefox still catching up as of 2025–26). Verdict: **WebGPU not worth it for this workload**; if we ever GPU-accelerate, WebGL2 instanced rendering (xterm.js's approach) is the right complexity/payoff point. (https://toji.dev/webgpu-best-practices/webgl-performance-comparison.html, https://threejsroadmap.com/blog/webgl-vs-webgpu-explained) — MEDIUM-HIGH.

## Q6: Dirty-cell diff redraw — payoff for full-frame 3D animation

- xterm.js and most terminals maintain a **dirty-row/dirty-cell** model: each frame the renderer diffs the new buffer against the previously rendered state and only repaints changed cells/rows (calls are debounced into a single rAF). For *terminal* workloads this is huge because typical frames change a handful of cells (cursor, one line of output). (https://github.com/xtermjs/xterm.js/issues/3440, https://deepwiki.com/xtermjs/xterm.js/1-overview) — HIGH.
- **For our 3D-animation case the payoff is ~zero.** A rotating/animating 3D scene changes essentially *every* cell every frame, so the dirty set ≈ the whole grid; diffing then costs an extra full-grid comparison pass (CPU + branch overhead) and a per-cell "did it change?" check that almost always says "yes", with no draw calls saved. Worst case it's a net *loss*. (Logical consequence of the dirty-region model + confirmed by the general "redraw only the diff" principle, which explicitly only pays off when most of the view is unchanged — https://www.chidiwilliams.com/posts/redraw-only-the-diff) — HIGH.
- Practical rule for us: **do NOT implement dirty-cell diffing in the hot 3D path.** Just clear and redraw the full grid every frame (clearRect once, then the atlas blit loop). Keep an optional "static screen" fast-path only if we later add non-animated TUI screens (menus), where a dirty model would help — but that's a Phase-6+ nicety, not Phase 5. — HIGH.

## Q7: Consolidated cells/sec & FPS table

Per-cell costs derived above (atlas/WebGL from xterm.js PR #1790; fillText from emirpasic test, ~2017 browsers — modern engines faster but order-of-magnitude holds). "ms/frame" = draw only, excludes WASM sim.

| Grid | Cells | Per-glyph fillText (~0.5–3 µs/cell, browser-dep.) | Canvas2D **atlas** (~0.6 µs/cell) | WebGL (~0.1–0.15 µs/cell) |
|------|------:|---------------------------------|-----------------------------------|----------------------------|
| 80×24 (classic) | 1,920 | ~3–6 ms (≥60 FPS ok) | ~1.2 ms | ~0.3 ms |
| 87×26 (xterm bench) | 2,262 | (n/a) | **~4.8 ms (meas.)** | **~0.69 ms (meas.)** |
| **120×60 (our target)** | **7,200** | ~10–25 ms (30 FPS marginal/fail on old FF) | **~4–5 ms → 60 FPS ok** | **~0.8–1 ms** |
| 200×100 (our stretch) | 20,000 | ~30–60+ ms (fails 30 FPS) | ~12–13 ms → 30 FPS ok, 60 tight | ~2.5–3 ms → 60 FPS easy |
| 300×80 (xterm bench) | 24,000 | (n/a) | **~15.3 ms (meas.)** | **~3.69 ms (meas.)** |

Reading the table: per-glyph fillText is the only technique that *fails* our targets; Canvas2D-atlas clears 30 FPS everywhere and hits 60 at 120×60; WebGL is overkill until the 20k-cell stretch where we'd want guaranteed 60. Budget check: 30 FPS = 33 ms/frame total; at 120×60, atlas draw (~5 ms) leaves ~28 ms for WASM sim — ample.

## Recommendation for our browser renderer

**Phase 5 (v2.0): ship a Canvas2D glyph-atlas renderer.** Concretely:
1. WASM core exports the frame as a flat typed array over linear memory (per cell: glyph index + packed fg RGBA + optional bg flag). JS takes a zero-copy view; **one buffer handoff per frame, never per-cell JS↔WASM calls** (the #1 performance rule).
2. Pre-render the glyph set once to an **offscreen atlas** (ImageBitmap-backed if available; one tile per glyph × per distinct fg color, or render white glyphs + tint — start with per-(glyph,fg) tiles cached lazily). Render loop: `clearRect` once, then a tight loop issuing `drawImage(atlas, ...)` per cell at integer coordinates. Set `ctx.font`/baseline/align never inside the loop.
3. For the colored half-block / full-bg mode, fill backgrounds either by batching same-color `fillRect` runs or by a single full-frame **putImageData** from a WASM RGBA buffer (acceptable since every pixel changes anyway); glyphs drawn over via atlas.
4. Drive with `requestAnimationFrame`. **Full redraw every frame — no dirty-cell diffing** (zero payoff under full-frame 3D animation; see Q6).

**Defer:** WebGL renderer, OffscreenCanvas+worker, and dirty-cell/static-screen fast-paths. Keep the renderer behind a small interface so a WebGL backend can be slotted in later if the 200×100 stretch target needs guaranteed 60 FPS or we hit weak iGPUs. Skip WebGPU entirely for this workload. The xterm.js DOM→Canvas→WebGL evolution is our template: start one rung up (Canvas+atlas, skipping DOM), reserve WebGL for scale.

**NFR-8 verdict (≥30 FPS at 120×60): comfortably feasible with Canvas2D.** xterm.js's own measured Canvas2D-atlas numbers put 7,200 cells at ~4–5 ms/frame draw — roughly 6× under the 33 ms budget for 30 FPS, and within the 16.6 ms budget for 60 FPS. The risk is not the 120×60 target but the 200×100 stretch (atlas ~12–13 ms draw leaves a tight 60 FPS margin once sim cost is added); that case is the trigger to consider the deferred WebGL backend. Confidence: **HIGH** for the 120×60 conclusion (anchored on primary-source benchmarks), MEDIUM for exact 200×100 margins (extrapolated).
