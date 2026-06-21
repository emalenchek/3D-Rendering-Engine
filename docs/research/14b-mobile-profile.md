# Research 14b — M2 device-profile gate (render vs present)

Status: **harness ready, awaiting on-device numbers.** This is the v2.2 analogue of
`11b` (the SIMD profile gate): measure where the mobile frame time actually goes
**before** committing to the largest item (Phase 11, the WebGL2 presenter). Decision M2.

## What to measure

The split between the two per-frame costs:

- **render** — `Renderer.render()` (the WASM software rasterizer), and
- **present** — `GridRenderer.draw()` (the Canvas2D presenter).

## How (no Web Inspector needed)

The demo is now instrumented (`web/app.js`, M2): the status line shows the live split,
averaged over a 500 ms window:

```
120×60 · 30 FPS · render 4.1ms · present 12.8ms
```

Procedure:

1. Open the live demo on the target device: <https://emalenchek.github.io/3D-Rendering-Engine/>.
2. Let it settle ~5 s (mobile SoCs throttle within seconds — read the *sustained* numbers,
   not the first frame).
3. Record `FPS`, `render ms`, `present ms` for each preset (Scene / Sphere / Cube) and each
   output mode (ASCII / truecolor / half-block).
4. Repeat in **low-power mode** (mobile throttles harder) to capture the worst case.
5. Optional cross-check: Safari **Web Inspector → Timelines** (Mac + cabled iPhone) to confirm
   the on-screen split matches the profiler.

This re-uses the FR-7.1 discipline: the on-screen instrumentation *is* the gate, and these
results decide M2 below.

## Results (fill in)

| Device / iOS | Preset | Mode | FPS | render ms | present ms | present share |
|---|---|---|---|---|---|---|
| _e.g._ iPhone 13 / 18.x | Scene | truecolor | | | | |
| | Sphere | ascii | | | | |
| (low-power mode) | Scene | truecolor | | | | |

(Capture at least one mid-range device. A second, older device near the iOS 16.4 floor is
useful for the compatibility story — it now loads via the Phase 9 scalar fallback.)

## Decision rule (M2)

- **present ≫ render** (the working hypothesis, research 14 §3): the Canvas2D presenter is the
  bottleneck. The Phase 10 batched composite (FR-10.1) + DPR cap should help materially; if it
  still dominates after Phase 10, **Phase 11 (WebGL2 presenter) earns its place in v2.2**.
- **render ≫ present**: the software rasterizer dominates on-device. Then the presenter work
  matters less; revisit the (deferred) wasm-threads path (report 10) or accept the cost, and
  **defer Phase 11 to v2.3**.
- **comparable**: ship Phase 10 (it's cheap and exact), re-measure, then decide Phase 11.

## Status of the surrounding work

- Phase 9 (scalar fallback) — merged; restores load on iOS < 16.4.
- FR-10.1 (batched one-composite presenter) — implemented, pixel-identical (NFR-22 test).
- This gate (M2) — **the one step that needs a physical device**; everything else in Phase 10
  is measurable in CI / on desktop.
