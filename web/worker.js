// Render worker (FR-10.4): owns the WASM Renderer AND the presenter (drawing to
// a transferred OffscreenCanvas), so both the software rasterization and the
// canvas present run off the main/UI thread. The main thread (app.js) only
// forwards camera/scene/visibility messages and updates the status line.
//
// Workers have no requestAnimationFrame, so the loop is a self-scheduling
// setTimeout at the target frame interval. If anything here fails (e.g. no font
// for the glyph atlas in this worker), it posts an `error` and app.js falls back
// to the main-thread render path.

import init, { Renderer } from "./pkg/tte_wasm.js";
import { makePresenter } from "./presenter.js";
import { nextScale } from "./adaptive.js";

let renderer = null;
let grid = null;
let running = false;
let hidden = false;
let baseCols = 120, baseRows = 60, gridScale = 1;
let minFrameMs = 0, lastFrameAt = 0;
let frames = 0, renderMs = 0, presentMs = 0, lastFpsAt = 0;

self.onmessage = async (e) => {
  const m = e.data;
  try {
    switch (m.type) {
      case "init":
        await onInit(m);
        break;
      case "orbit":
        renderer?.set_orbit(m.yaw, m.pitch, m.radius);
        break;
      case "mode":
        try {
          renderer?.set_mode(m.mode);
        } catch { /* unknown mode: ignore */ }
        break;
      case "load":
        onLoad(m);
        break;
      case "visibility":
        hidden = m.hidden;
        if (!hidden) {
          lastFpsAt = performance.now();
          frames = renderMs = presentMs = 0;
        }
        break;
    }
  } catch (err) {
    self.postMessage({ type: "error", message: String((err && err.message) || err) });
  }
};

async function onInit(m) {
  baseCols = m.cols;
  baseRows = m.rows;
  minFrameMs = m.minFrameMs;
  await init({ module_or_path: m.wasmUrl });
  renderer = new Renderer(m.cols, m.rows);
  renderer.set_orbit(m.yaw, m.pitch, m.radius);
  // Throws on a blank atlas → caught above → main thread falls back.
  grid = makePresenter(m.canvas, {
    cellW: m.cellW,
    cellH: m.cellH,
    font: m.font,
    dpr: m.dpr,
    forceCanvas: m.forceCanvas,
  });
  running = true;
  lastFpsAt = performance.now();
  self.postMessage({ type: "ready" });
  loop();
}

function onLoad(m) {
  if (!renderer) return;
  try {
    if (m.kind === "obj") renderer.load_obj(m.text);
    else renderer.load_scene(m.text);
    self.postMessage({ type: "loadok" });
  } catch (err) {
    self.postMessage({ type: "loaderror", message: String((err && err.message) || err) });
  }
}

function loop() {
  if (!running) return;
  const now = performance.now();
  if (!hidden && now - lastFrameAt >= minFrameMs) {
    lastFrameAt = now;
    const t0 = performance.now();
    renderer.render();
    const t1 = performance.now();
    grid.draw(renderer.width(), renderer.height(), renderer.glyphs(), renderer.fg(), renderer.bg());
    const t2 = performance.now();
    renderMs += t1 - t0;
    presentMs += t2 - t1;
    frames++;
    if (t2 - lastFpsAt >= 500) {
      const fps = Math.round((frames * 1000) / (t2 - lastFpsAt));
      const r = (renderMs / frames).toFixed(1);
      const p = (presentMs / frames).toFixed(1);
      self.postMessage({
        type: "status",
        text: `${renderer.width()}×${renderer.height()} · ${fps} FPS · render ${r}ms · present ${p}ms`,
      });
      frames = renderMs = presentMs = 0;
      lastFpsAt = t2;
      const ns = nextScale(gridScale, fps);
      if (ns !== gridScale) {
        gridScale = ns;
        renderer.resize(Math.max(2, Math.round(baseCols * ns)), Math.max(2, Math.round(baseRows * ns)));
      }
    }
  }
  setTimeout(loop, minFrameMs > 0 ? minFrameMs : 8);
}
