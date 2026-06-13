// Glyph-atlas canvas renderer (decision V8, research report 07).
//
// The WASM `Renderer` hands us three flat typed arrays per frame (glyphs as
// codepoints, fg and bg as RGB triplets). We pre-render every glyph we might
// draw, once, into an offscreen atlas, then blit one `drawImage` per cell each
// frame — far faster than per-cell `fillText`. Full redraw every frame (no
// dirty-diffing; worthless under full-frame 3D animation, report 07 Q6).

const ATLAS_GLYPHS =
  " .,-~:;=!*#$@█▀"; // ASCII ramp + full block + upper-half block

export class GridRenderer {
  // `cellW`/`cellH` are device pixels per character cell.
  constructor(canvas, { cellW = 9, cellH = 18, font = "16px monospace" } = {}) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d", { alpha: false });
    this.cellW = cellW;
    this.cellH = cellH;
    this.font = font;
    this.cols = 0;
    this.rows = 0;
    // Per-foreground-color atlases are built lazily and cached; for colored
    // glyphs we instead tint a white-glyph atlas via per-cell fillStyle on a
    // scratch canvas. Simpler and fast enough at our scale: we draw glyphs in
    // white into the atlas and recolor with globalCompositeOperation.
    this._buildAtlas();
  }

  // Render one white-on-transparent copy of each glyph into an offscreen atlas.
  _buildAtlas() {
    const dpr = window.devicePixelRatio || 1;
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
    // Scratch canvas for tinting a glyph to a target color.
    this.tint = document.createElement("canvas");
    this.tint.width = w;
    this.tint.height = h;
    this.tintCtx = this.tint.getContext("2d");
  }

  _scaledFont(dpr) {
    // "16px monospace" → "32px monospace" at dpr 2.
    return this.font.replace(/(\d+)px/, (_, n) => `${Math.round(n * dpr)}px`);
  }

  // Size the canvas to a cols×rows grid (device-pixel aware).
  resize(cols, rows) {
    this.cols = cols;
    this.rows = rows;
    const dpr = this.dpr;
    this.canvas.width = cols * this.glyphPxW;
    this.canvas.height = rows * this.glyphPxH;
    this.canvas.style.width = `${cols * this.cellW}px`;
    this.canvas.style.height = `${rows * this.cellH}px`;
  }

  // Draw one frame from the WASM renderer's typed arrays.
  draw(cols, rows, glyphs, fg, bg) {
    if (cols !== this.cols || rows !== this.rows) this.resize(cols, rows);
    const { ctx, glyphPxW: gw, glyphPxH: gh } = this;

    // Background: fill per-cell rects, batching runs of equal color.
    let i = 0;
    for (let y = 0; y < rows; y++) {
      let x = 0;
      while (x < cols) {
        const r = bg[i * 3], g = bg[i * 3 + 1], b = bg[i * 3 + 2];
        let run = 1;
        while (
          x + run < cols &&
          bg[(i + run) * 3] === r &&
          bg[(i + run) * 3 + 1] === g &&
          bg[(i + run) * 3 + 2] === b
        ) run++;
        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(x * gw, y * gh, run * gw, gh);
        x += run;
        i += run;
      }
    }

    // Foreground glyphs: tint the atlas glyph to the cell's fg, blit per cell.
    // Spaces and full-block-with-matching-bg are skipped (nothing to draw).
    i = 0;
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
    // White glyph shape → multiply by solid color = colored glyph.
    t.globalCompositeOperation = "source-over";
    t.drawImage(this.atlas, idx * gw, 0, gw, gh, 0, 0, gw, gh);
    t.globalCompositeOperation = "source-in";
    t.fillStyle = `rgb(${r},${g},${b})`;
    t.fillRect(0, 0, gw, gh);
  }
}
