// Browser app (FR-5.3): wires the WASM Renderer to the canvas, pointer-event
// orbit (mouse + touch), wheel/pinch zoom, mode switching, and a live scene
// editor. The WASM module exposes only data; all DOM/canvas work lives here.
//
// FR-10.4: rendering runs in a Web Worker via OffscreenCanvas when available
// (the worker owns the WASM Renderer + presenter, off the UI thread), with an
// automatic main-thread fallback — `?noworker` forces it, and the worker path
// also self-falls-back if it can't produce a frame (e.g. no font in the worker).

import init, { Renderer } from "./pkg/tte_wasm.js";
import { makePresenter } from "./presenter.js";
import { nextScale } from "./adaptive.js";

// A minimal `() -> v128` module: `WebAssembly.validate` returns true only where
// fixed-width wasm SIMD is supported. (The canonical wasm-feature-detect probe.)
const SIMD_PROBE = new Uint8Array([
  0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8,
  0, 65, 0, 253, 15, 253, 98, 11,
]);

// Pick the SIMD module where supported, else the scalar fallback — a `+simd128`
// module fails to instantiate on iOS Safari < 16.4 (research 14 §1 / FR-9.2).
// `?nosimd` forces the fallback so CI can exercise it (FR-9.3).
function wasmUrl() {
  const forced = new URLSearchParams(location.search).has("nosimd");
  let simd = false;
  try {
    simd = !forced && WebAssembly.validate(SIMD_PROBE);
  } catch {
    simd = false;
  }
  const file = simd ? "./pkg/tte_wasm_bg.wasm" : "./pkg/tte_wasm_bg.scalar.wasm";
  return new URL(file, import.meta.url);
}

const PRESETS = {
  Cube: { kind: "obj", text: cubeObj() },
  Sphere: { kind: "scene", text: "sphere rings=20 segments=32" },
  Scene: {
    kind: "scene",
    text: `// edit me — re-renders live
material "red" base-color=(0.85 0.2 0.2)
material "blue" base-color=(0.2 0.4 0.9)
light direction=(-1 -2 -1) intensity=1.2 ambient=0.2
sphere "ball" translate=(-1.2 0 0) material="red"
cube "box" translate=(1.2 0 0) rotate=(0 35 0) material="blue"
plane translate=(0 -1 0) scale=(8 1 8)`,
  },
};

const COLS = 120, ROWS = 60;
const CELL = { cellW: 8, cellH: 16, font: "15px monospace" };
const mobileFrameMs = () => (matchMedia("(pointer: coarse)").matches ? 1000 / 30 : 0);

async function main() {
  const params = new URLSearchParams(location.search);
  const forceCanvas = params.has("nogl");
  const status = document.getElementById("status");
  let canvas = document.getElementById("screen");

  const canWorker = !params.has("noworker")
    && typeof Worker !== "undefined"
    && typeof canvas.transferControlToOffscreen === "function";

  let controller = null;
  if (canWorker) {
    try {
      controller = await startWorker(canvas, { forceCanvas, status });
    } catch (e) {
      console.warn("tte: worker render path unavailable, using main thread —", e.message);
      canvas = replaceCanvas(canvas); // transferControlToOffscreen is irreversible
    }
  }
  if (!controller) {
    controller = await startMainThread(canvas, { forceCanvas, status });
  }
  wireUI(canvas, controller);
}

// A transferred canvas can't be reclaimed; clone a fresh element to draw on.
function replaceCanvas(old) {
  const fresh = old.cloneNode(false);
  old.replaceWith(fresh);
  return fresh;
}

// Render in a worker (OffscreenCanvas). Resolves once the worker produces its
// first frame; rejects (→ caller falls back) on worker error or timeout.
async function startWorker(canvas, { forceCanvas, status }) {
  const offscreen = canvas.transferControlToOffscreen();
  // The worker can't size the displayed element; do it here (mobile pins via max-width).
  canvas.style.width = `${COLS * CELL.cellW}px`;
  canvas.style.height = `${ROWS * CELL.cellH}px`;
  const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
  const errBox = document.getElementById("error");

  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("worker produced no frame in time")), 8000);
    worker.onmessage = (e) => {
      const m = e.data;
      if (m.type === "status") {
        status.textContent = m.text;
        clearTimeout(timer);
        resolve();
      } else if (m.type === "loaderror") {
        errBox.textContent = m.message;
      } else if (m.type === "loadok") {
        errBox.textContent = "";
      } else if (m.type === "error") {
        clearTimeout(timer);
        reject(new Error(m.message));
      }
    };
    worker.onerror = (ev) => {
      clearTimeout(timer);
      reject(new Error(ev.message || "worker error"));
    };
    worker.postMessage(
      {
        type: "init",
        canvas: offscreen,
        cols: COLS,
        rows: ROWS,
        cellW: CELL.cellW,
        cellH: CELL.cellH,
        font: CELL.font,
        dpr: window.devicePixelRatio || 1,
        minFrameMs: mobileFrameMs(),
        wasmUrl: wasmUrl().href,
        forceCanvas,
        yaw: 0.6,
        pitch: 0.4,
        radius: 6.0,
      },
      [offscreen],
    );
  });

  document.addEventListener("visibilitychange", () =>
    worker.postMessage({ type: "visibility", hidden: document.hidden }),
  );
  return {
    setOrbit: (y, p, r) => worker.postMessage({ type: "orbit", yaw: y, pitch: p, radius: r }),
    setMode: (m) => worker.postMessage({ type: "mode", mode: m }),
    load: (kind, text) => worker.postMessage({ type: "load", kind, text }),
  };
}

// Render on the main thread (the fallback, and where OffscreenCanvas is absent).
async function startMainThread(canvas, { forceCanvas, status }) {
  await init({ module_or_path: wasmUrl() });
  const renderer = new Renderer(COLS, ROWS);
  renderer.set_orbit(0.6, 0.4, 6.0);
  const grid = makePresenter(canvas, { ...CELL, forceCanvas });
  const errBox = document.getElementById("error");

  const minFrameMs = mobileFrameMs();
  let lastFrameAt = 0, hidden = document.hidden, gridScale = 1;
  let frames = 0, lastFpsAt = performance.now(), renderMs = 0, presentMs = 0;
  document.addEventListener("visibilitychange", () => {
    hidden = document.hidden;
    if (!hidden) {
      lastFpsAt = performance.now();
      frames = renderMs = presentMs = 0;
    }
  });

  function frame(now) {
    requestAnimationFrame(frame);
    if (hidden || now - lastFrameAt < minFrameMs) return;
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
      status.textContent =
        `${renderer.width()}×${renderer.height()} · ${fps} FPS · render ${r}ms · present ${p}ms`;
      frames = renderMs = presentMs = 0;
      lastFpsAt = t2;
      const ns = nextScale(gridScale, fps);
      if (ns !== gridScale) {
        gridScale = ns;
        renderer.resize(Math.max(2, Math.round(COLS * ns)), Math.max(2, Math.round(ROWS * ns)));
      }
    }
  }
  requestAnimationFrame(frame);

  return {
    setOrbit: (y, p, r) => renderer.set_orbit(y, p, r),
    setMode: (m) => {
      try {
        renderer.set_mode(m);
      } catch { /* unknown mode: ignore */ }
    },
    load: (kind, text) => {
      try {
        if (kind === "obj") renderer.load_obj(text);
        else renderer.load_scene(text);
        errBox.textContent = "";
      } catch (e) {
        errBox.textContent = String(e);
      }
    },
  };
}

// UI wiring — identical for both render paths, driving the `controller`.
function wireUI(canvas, controller) {
  let yaw = 0.6, pitch = 0.4, radius = 6.0;
  const applyOrbit = () => controller.setOrbit(yaw, pitch, radius);

  setupPointer(canvas, {
    onOrbit(dx, dy) {
      yaw -= dx * 0.01;
      pitch = clamp(pitch + dy * 0.01, -1.55, 1.55);
      applyOrbit();
    },
    onZoom(factor) {
      radius = clamp(radius * factor, 1.5, 50);
      applyOrbit();
    },
  });

  document.getElementById("mode").addEventListener("change", (e) => controller.setMode(e.target.value));

  const editor = document.getElementById("editor");
  let editorKind = "scene";
  editor.addEventListener("input", () => controller.load(editorKind, editor.value));

  const presetSel = document.getElementById("preset");
  for (const name of Object.keys(PRESETS)) {
    const opt = document.createElement("option");
    opt.value = opt.textContent = name;
    presetSel.appendChild(opt);
  }
  function selectPreset(name) {
    const p = PRESETS[name];
    editorKind = p.kind;
    editor.value = p.text;
    controller.load(p.kind, p.text);
  }
  presetSel.addEventListener("change", (e) => selectPreset(e.target.value));
  selectPreset("Scene");
}

// Unified mouse + touch input via Pointer Events (FR-5.3): one-pointer drag
// orbits; wheel or two-finger pinch zooms.
function setupPointer(el, { onOrbit, onZoom }) {
  const pointers = new Map();
  let lastPinch = 0;

  el.addEventListener("pointerdown", (e) => {
    el.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
  });
  el.addEventListener("pointermove", (e) => {
    const prev = pointers.get(e.pointerId);
    if (!prev) return;
    if (pointers.size === 1) {
      onOrbit(e.clientX - prev.x, e.clientY - prev.y);
    }
    prev.x = e.clientX;
    prev.y = e.clientY;
    if (pointers.size === 2) {
      const pts = [...pointers.values()];
      const dist = Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y);
      if (lastPinch) onZoom(lastPinch / dist);
      lastPinch = dist;
    }
  });
  const end = (e) => {
    pointers.delete(e.pointerId);
    if (pointers.size < 2) lastPinch = 0;
  };
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
  el.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      onZoom(e.deltaY > 0 ? 1.1 : 0.9);
    },
    { passive: false },
  );
  // Prevent the page from scrolling while dragging on touch.
  el.style.touchAction = "none";
}

const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

// A minimal cube OBJ so the "Cube" preset needs no external file.
function cubeObj() {
  return `v -1 -1 -1
v 1 -1 -1
v 1 1 -1
v -1 1 -1
v -1 -1 1
v 1 -1 1
v 1 1 1
v -1 1 1
f 1 2 3 4
f 6 5 8 7
f 5 1 4 8
f 2 6 7 3
f 4 3 7 8
f 5 6 2 1`;
}

main();
