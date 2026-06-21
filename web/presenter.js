// Presenter selection, shared by the main thread (app.js) and the render worker
// (worker.js). Prefers the WebGL2 GPU presenter, falls back to Canvas2D.
//
// Throws if the chosen presenter can't actually draw glyphs — specifically a
// **blank atlas**, which happens when a worker's OffscreenCanvas has no font.
// Callers treat that as "this rendering context won't work, fall back" (the
// worker path falls back to the main thread, where document fonts are present).

import { GridRenderer } from "./renderer.js";
import { WebGLGridRenderer } from "./webgl-renderer.js";

export function makePresenter(canvas, opts = {}) {
  const { forceCanvas = false, ...rest } = opts;

  if (!forceCanvas) {
    let gl = null;
    try {
      gl = new WebGLGridRenderer(canvas, rest);
    } catch {
      gl = null; // no WebGL2 here — try Canvas2D below (canvas not yet locked)
    }
    if (gl) {
      if (gl.atlasBlank) throw new Error("blank glyph atlas (font unavailable)");
      return gl;
    }
  }

  const c2 = new GridRenderer(canvas, rest);
  if (c2.atlasBlank) throw new Error("blank glyph atlas (font unavailable)");
  return c2;
}
