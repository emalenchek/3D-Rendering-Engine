// Tiny graphics helpers shared by the presenters and the render worker.
//
// The presenters run both on the main thread (real `<canvas>` + `document`) and
// inside the render worker (OffscreenCanvas, no `document`/`window`). These keep
// the presenter code agnostic to which context it's in.

// Create an offscreen drawing surface in either context.
export function makeCanvas(w, h) {
  if (typeof document !== "undefined") {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    return c;
  }
  return new OffscreenCanvas(w, h);
}

// Effective device-pixel ratio: an explicit `opts.dpr` (the worker passes the
// main thread's value, since `devicePixelRatio` isn't exposed to workers), else
// the ambient one, else 1.
export function pickDpr(opts) {
  if (opts && typeof opts.dpr === "number") return opts.dpr;
  return typeof devicePixelRatio !== "undefined" ? devicePixelRatio : 1;
}

// True if a 2D atlas canvas rendered no ink (e.g. a worker without the font) —
// used to bail out to the main-thread path rather than show blank glyphs.
export function atlasIsBlank(ctx, w, h) {
  const { data } = ctx.getImageData(0, 0, w, h);
  for (let i = 3; i < data.length; i += 4) {
    if (data[i] !== 0) return false;
  }
  return true;
}
