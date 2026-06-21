// Browser app (FR-5.3): wires the WASM Renderer to the canvas, pointer-event
// orbit (mouse + touch), wheel/pinch zoom, mode switching, and a live scene
// editor. The WASM module exposes only data; all DOM/canvas work lives here.

import init, { Renderer } from "./pkg/tte_wasm.js";
import { GridRenderer } from "./renderer.js";

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

async function main() {
  await init({ module_or_path: wasmUrl() });
  const renderer = new Renderer(COLS, ROWS);

  const canvas = document.getElementById("screen");
  const grid = new GridRenderer(canvas, { cellW: 8, cellH: 16, font: "15px monospace" });

  // Orbit state, mirrored into the WASM camera.
  let yaw = 0.6, pitch = 0.4, radius = 6.0;
  const apply = () => renderer.set_orbit(yaw, pitch, radius);
  apply();

  const status = document.getElementById("status");
  let frames = 0, lastFpsAt = performance.now(), fps = 0;

  function frame() {
    renderer.render();
    grid.draw(
      renderer.width(),
      renderer.height(),
      renderer.glyphs(),
      renderer.fg(),
      renderer.bg(),
    );
    frames++;
    const now = performance.now();
    if (now - lastFpsAt >= 500) {
      fps = Math.round((frames * 1000) / (now - lastFpsAt));
      frames = 0;
      lastFpsAt = now;
      status.textContent = `${renderer.width()}×${renderer.height()} · ${fps} FPS`;
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);

  setupPointer(canvas, {
    onOrbit(dx, dy) {
      yaw -= dx * 0.01;
      pitch = clamp(pitch + dy * 0.01, -1.55, 1.55);
      apply();
    },
    onZoom(factor) {
      radius = clamp(radius * factor, 1.5, 50);
      apply();
    },
  });

  // Output mode.
  document.getElementById("mode").addEventListener("change", (e) => {
    renderer.set_mode(e.target.value);
  });

  // Live scene editor: re-load on every edit; keep the last good subject on error.
  const editor = document.getElementById("editor");
  const errBox = document.getElementById("error");
  function loadFromEditor(kind) {
    try {
      if (kind === "obj") renderer.load_obj(editor.value);
      else renderer.load_scene(editor.value);
      errBox.textContent = "";
    } catch (e) {
      errBox.textContent = String(e);
    }
  }
  let editorKind = "scene";
  editor.addEventListener("input", () => loadFromEditor(editorKind));

  // Presets populate the editor and load.
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
    loadFromEditor(p.kind);
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
