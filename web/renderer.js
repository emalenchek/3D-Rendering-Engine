// Glyph-atlas canvas renderer (decision V8, research report 07).
//
// The WASM `Renderer` hands us three flat typed arrays per frame (glyphs as
// codepoints, fg and bg as RGB triplets). We pre-render every glyph we might
// draw, once, into an offscreen atlas. Full redraw every frame (no dirty-diffing;
// worthless under full-frame 3D animation, report 07 Q6).
//
// Presentation cost is the mobile bottleneck (research 14 §3): the old path
// switched `globalCompositeOperation` to "source-in" *per cell* to tint each
// glyph — thousands of compositing state-changes per frame, which iOS Safari
// handles very poorly. `draw()` now batches that into **one** whole-frame
// composite (FR-10.1), bit-identical to the per-cell path (kept as `drawCompat`
// for the NFR-22 pixel-parity test).

const ATLAS_GLYPHS =
  " .,-~:;=!*#$@█▀"; // ASCII ramp + full block + upper-half block

// iOS Safari refuses to render a canvas whose backing store exceeds this area
// (4096² px, pre-iOS-18; raised to 8192² in iOS 18). Stay under it (FR-10.2).
const MAX_CANVAS_AREA = 16_777_216;

export class GridRenderer {
  // `cellW`/`cellH` are device pixels per character cell. `maxDpr` caps the
  // backing-store scale (FR-10.2): retina phones report devicePixelRatio 2–3,
  // which squares the fill cost for no visible gain at these cell sizes.
  constructor(
    canvas,
    { cellW = 9, cellH = 18, font = "16px monospace", maxDpr = 2 } = {},
  ) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d", { alpha: false });
    this.cellW = cellW;
    this.cellH = cellH;
    this.font = font;
    this.maxDpr = maxDpr;
    this.cols = 0;
    this.rows = 0;
    this._buildAtlas();
  }

  // Render one white-on-transparent copy of each glyph into an offscreen atlas.
  _buildAtlas() {
    const dpr = Math.min(window.devicePixelRatio || 1, this.maxDpr);
    this.dpr = dpr;
    const w = Math.ceil(this.cellW * dpr);
    const h = Math.ceil(this.cellH * dpr);
    this.glyphPxW = w;
    this.glyphPxH = h;
    this.atlasIndex = new Map();
    const atlas = document.createElement("canvas");
    atlas.width = w * ATLAS_GLYPHS.length;
    atlas.height = h;
    const a = atlas.getContext("2d");
    a.font = this._scaledFont(dpr);
    a.textBaseline = "top";
    a.textAlign = "left";
    a.fillStyle = "#fff";
    let i = 0;
    for (const ch of ATLAS_GLYPHS) {
      this.atlasIndex.set(ch.codePointAt(0), i);
      a.fillText(ch, i * w, 0);
      i++;
    }
    this.atlas = atlas;
    // Scratch canvas for the per-cell tint in `drawCompat` (reference path only).
    this.tint = document.createElement("canvas");
    this.tint.width = w;
    this.tint.height = h;
    this.tintCtx = this.tint.getContext("2d");
  }

  _scaledFont(dpr) {
    // "16px monospace" → "32px monospace" at dpr 2.
    return this.font.replace(/(\d+)px/, (_, n) => `${Math.round(n * dpr)}px`);
  }

  // Size the canvas (and the offscreen composite layers) to a cols×rows grid.
  resize(cols, rows) {
    this.cols = cols;
    this.rows = rows;
    this.canvas.width = cols * this.glyphPxW;
    this.canvas.height = rows * this.glyphPxH;
    this.canvas.style.width = `${cols * this.cellW}px`;
    this.canvas.style.height = `${rows * this.cellH}px`;
    if (this.canvas.width * this.canvas.height > MAX_CANVAS_AREA) {
      console.warn(
        `tte: canvas backing ${this.canvas.width}×${this.canvas.height} exceeds the ` +
          `iOS Safari area cap (${MAX_CANVAS_AREA}px) — lower maxDpr or the grid size.`,
      );
    }

    // Offscreen layers for the batched composite (FR-10.1): an `ink` layer
    // (transparent, holds the white glyph shapes) and a `color` layer (opaque,
    // holds the per-cell fg colour field).
    if (!this.ink) {
      this.ink = document.createElement("canvas");
      this.inkCtx = this.ink.getContext("2d"); // alpha: true (default)
      this.color = document.createElement("canvas");
      this.colorCtx = this.color.getContext("2d", { alpha: false });
    }
    this.ink.width = this.color.width = this.canvas.width;
    this.ink.height = this.color.height = this.canvas.height;
  }

  // Run-batched solid fill of a cols×rows colour field into context `c`.
  _fillField(c, cols, rows, field) {
    const { glyphPxW: gw, glyphPxH: gh } = this;
    let i = 0;
    for (let y = 0; y < rows; y++) {
      let x = 0;
      while (x < cols) {
        const r = field[i * 3], g = field[i * 3 + 1], b = field[i * 3 + 2];
        let run = 1;
        while (
          x + run < cols &&
          field[(i + run) * 3] === r &&
          field[(i + run) * 3 + 1] === g &&
          field[(i + run) * 3 + 2] === b
        ) run++;
        c.fillStyle = `rgb(${r},${g},${b})`;
        c.fillRect(x * gw, y * gh, run * gw, gh);
        x += run;
        i += run;
      }
    }
  }

  // Draw one frame — ONE whole-frame composite (FR-10.1).
  draw(cols, rows, glyphs, fg, bg) {
    if (cols !== this.cols || rows !== this.rows) this.resize(cols, rows);
    const { ctx, inkCtx: ink, glyphPxW: gw, glyphPxH: gh } = this;
    const W = this.canvas.width, H = this.canvas.height;

    // 1) Background straight onto the visible canvas.
    this._fillField(ctx, cols, rows, bg);

    // 2) Ink layer: blit every glyph shape (white) in one pass — `source-over`,
    //    no per-cell compositing state change.
    ink.globalCompositeOperation = "source-over";
    ink.clearRect(0, 0, W, H);
    let i = 0;
    const space = 0x20;
    for (let y = 0; y < rows; y++) {
      for (let x = 0; x < cols; x++, i++) {
        const cp = glyphs[i];
        if (cp === space) continue;
        const idx = this.atlasIndex.get(cp);
        if (idx === undefined) continue;
        ink.drawImage(this.atlas, idx * gw, 0, gw, gh, x * gw, y * gh, gw, gh);
      }
    }

    // 3) Colour field on its own layer.
    this._fillField(this.colorCtx, cols, rows, fg);

    // 4) ONE `source-in`: clip the colour field to the glyph ink → coloured
    //    glyphs on transparent. (Per-pixel identical to tinting each cell.)
    ink.globalCompositeOperation = "source-in";
    ink.drawImage(this.color, 0, 0);

    // 5) Composite the coloured glyphs over the background.
    ctx.drawImage(this.ink, 0, 0);
  }

  // Reference presenter: the original per-cell `source-in` tint (one composite
  // state change per non-space cell). Kept only to validate `draw()` is
  // pixel-identical (NFR-22); not used by the live demo.
  drawCompat(cols, rows, glyphs, fg, bg) {
    if (cols !== this.cols || rows !== this.rows) this.resize(cols, rows);
    const { ctx, glyphPxW: gw, glyphPxH: gh } = this;
    this._fillField(ctx, cols, rows, bg);
    let i = 0;
    const space = 0x20;
    for (let y = 0; y < rows; y++) {
      for (let x = 0; x < cols; x++, i++) {
        const cp = glyphs[i];
        if (cp === space) continue;
        const idx = this.atlasIndex.get(cp);
        if (idx === undefined) continue;
        const r = fg[i * 3], g = fg[i * 3 + 1], b = fg[i * 3 + 2];
        this._blitTinted(idx, r, g, b);
        ctx.drawImage(this.tint, x * gw, y * gh);
      }
    }
  }

  // Produce a colored copy of atlas glyph `idx` on the scratch canvas.
  _blitTinted(idx, r, g, b) {
    const t = this.tintCtx;
    const { glyphPxW: gw, glyphPxH: gh } = this;
    t.clearRect(0, 0, gw, gh);
    t.globalCompositeOperation = "source-over";
    t.drawImage(this.atlas, idx * gw, 0, gw, gh, 0, 0, gw, gh);
    t.globalCompositeOperation = "source-in";
    t.fillStyle = `rgb(${r},${g},${b})`;
    t.fillRect(0, 0, gw, gh);
  }
}
