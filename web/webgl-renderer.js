// WebGL2 grid presenter (FR-11.1) — an ADDITIVE alternative to the Canvas2D
// `GridRenderer`, with the identical interface (`resize`, `draw`). It offloads
// the per-cell glyph compositing to the GPU: the cell grid (glyph index, fg, bg)
// is uploaded as textures each frame and a single fullscreen shader samples a
// glyph-atlas texture and blends fg over bg per pixel. The CPU software
// rasterizer is unchanged — only the presenter differs.
//
// `app.js` selects this when WebGL2 is available and falls back to Canvas2D
// otherwise (NFR-23). Construction throws if WebGL2 is unavailable so the caller
// can fall back.

import { atlasIsBlank, makeCanvas, pickDpr } from "./gfx-util.js";

const ATLAS_GLYPHS =
  " .,-~:;=!*#$@█▀"; // ASCII ramp + full block + upper-half block (matches GridRenderer)

const NO_GLYPH = 255; // cell index sentinel: space / unknown → no coverage
const MAX_CANVAS_AREA = 16_777_216; // iOS pre-18 canvas cap (FR-10.2)

const VERT = `#version 300 es
in vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }
`;

const FRAG = `#version 300 es
precision highp float;
uniform sampler2D uAtlas; // white glyphs, alpha = coverage
uniform sampler2D uGlyph; // R8: per-cell atlas index / 255
uniform sampler2D uFg;    // RGB8 per-cell foreground
uniform sampler2D uBg;    // RGB8 per-cell background
uniform float uCols, uRows, uGw, uGh, uCanvasH, uAtlasW, uN;
out vec4 fragColor;
void main() {
  float px = gl_FragCoord.x;
  float py = uCanvasH - gl_FragCoord.y;            // flip to top-left origin
  float cellX = floor(px / uGw);
  float cellY = floor(py / uGh);
  vec2 cellUV = vec2((cellX + 0.5) / uCols, (cellY + 0.5) / uRows);
  float idx = floor(texture(uGlyph, cellUV).r * 255.0 + 0.5);
  float ix = px - cellX * uGw;                     // intra-cell pixel (centre-based)
  float iy = py - cellY * uGh;
  float cov = idx >= uN ? 0.0
            : texture(uAtlas, vec2((idx * uGw + ix) / uAtlasW, iy / uGh)).a;
  vec3 fg = texture(uFg, cellUV).rgb;
  vec3 bg = texture(uBg, cellUV).rgb;
  fragColor = vec4(mix(bg, fg, cov), 1.0);
}
`;

export class WebGLGridRenderer {
  constructor(
    canvas,
    { cellW = 9, cellH = 18, font = "16px monospace", maxDpr = 2, dpr } = {},
  ) {
    const gl = canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
    });
    if (!gl) throw new Error("WebGL2 unavailable");
    this.canvas = canvas;
    this.gl = gl;
    this.cellW = cellW;
    this.cellH = cellH;
    this.font = font;
    this.maxDpr = maxDpr;
    this.cols = 0;
    this.rows = 0;
    this.dpr = Math.min(pickDpr({ dpr }), maxDpr);
    this.glyphPxW = Math.ceil(cellW * this.dpr);
    this.glyphPxH = Math.ceil(cellH * this.dpr);

    this._buildProgram();
    this._buildQuad();
    this._buildAtlasTexture();
    this.glyphTex = this._dataTexture(gl.R8, gl.RED);
    this.fgTex = this._dataTexture(gl.RGB8, gl.RGB);
    this.bgTex = this._dataTexture(gl.RGB8, gl.RGB);
    this._glyphBuf = null; // reused per-frame Uint8Array(cols*rows)
  }

  _compile(type, src) {
    const gl = this.gl;
    const s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error("shader compile: " + gl.getShaderInfoLog(s));
    }
    return s;
  }

  _buildProgram() {
    const gl = this.gl;
    const p = gl.createProgram();
    gl.attachShader(p, this._compile(gl.VERTEX_SHADER, VERT));
    gl.attachShader(p, this._compile(gl.FRAGMENT_SHADER, FRAG));
    gl.bindAttribLocation(p, 0, "aPos");
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
      throw new Error("program link: " + gl.getProgramInfoLog(p));
    }
    this.program = p;
    this.u = {};
    for (const name of ["uAtlas", "uGlyph", "uFg", "uBg", "uCols", "uRows", "uGw", "uGh", "uCanvasH", "uAtlasW", "uN"]) {
      this.u[name] = gl.getUniformLocation(p, name);
    }
  }

  _buildQuad() {
    const gl = this.gl;
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    // One oversized triangle covering the clip-space viewport.
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
  }

  // Render the white-glyph atlas on a 2D canvas (same as GridRenderer) and
  // upload it as a texture; build the codepoint → tile-index map.
  _buildAtlasTexture() {
    const gl = this.gl;
    const w = this.glyphPxW, h = this.glyphPxH;
    this.atlasPxW = w * ATLAS_GLYPHS.length;
    this.atlasIndex = new Map();
    const atlas = makeCanvas(this.atlasPxW, h);
    const a = atlas.getContext("2d");
    a.font = this.font.replace(/(\d+)px/, (_, n) => `${Math.round(n * this.dpr)}px`);
    a.textBaseline = "top";
    a.textAlign = "left";
    a.fillStyle = "#fff";
    let i = 0;
    for (const ch of ATLAS_GLYPHS) {
      this.atlasIndex.set(ch.codePointAt(0), i);
      a.fillText(ch, i * w, 0);
      i++;
    }
    // No ink rendered (e.g. a worker without the font) → caller falls back.
    this.atlasBlank = atlasIsBlank(a, this.atlasPxW, h);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, atlas);
    this._nearestClamp();
    this.atlasTex = tex;
  }

  _dataTexture(internalFormat, format) {
    const gl = this.gl;
    const tex = gl.createTexture();
    tex._internalFormat = internalFormat;
    tex._format = format;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    this._nearestClamp();
    return tex;
  }

  _nearestClamp() {
    const gl = this.gl;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  resize(cols, rows) {
    const gl = this.gl;
    this.cols = cols;
    this.rows = rows;
    this.canvas.width = cols * this.glyphPxW;
    this.canvas.height = rows * this.glyphPxH;
    if (this.canvas.style) {
      this.canvas.style.width = `${cols * this.cellW}px`;
      this.canvas.style.height = `${rows * this.cellH}px`;
    }
    if (this.canvas.width * this.canvas.height > MAX_CANVAS_AREA) {
      console.warn(
        `tte: canvas backing ${this.canvas.width}×${this.canvas.height} exceeds the ` +
          `iOS Safari area cap (${MAX_CANVAS_AREA}px) — lower maxDpr or the grid size.`,
      );
    }
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    // Allocate the per-cell textures at the new grid size.
    for (const tex of [this.glyphTex, this.fgTex, this.bgTex]) {
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, tex._internalFormat, cols, rows, 0, tex._format, gl.UNSIGNED_BYTE, null);
    }
    this._glyphBuf = new Uint8Array(cols * rows);
  }

  draw(cols, rows, glyphs, fg, bg) {
    if (cols !== this.cols || rows !== this.rows) this.resize(cols, rows);
    const gl = this.gl;

    // Per-cell glyph index (atlas tile, or NO_GLYPH for space/unknown).
    const gbuf = this._glyphBuf;
    const space = 0x20;
    for (let i = 0; i < gbuf.length; i++) {
      const cp = glyphs[i];
      const idx = cp === space ? undefined : this.atlasIndex.get(cp);
      gbuf[i] = idx === undefined ? NO_GLYPH : idx;
    }

    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    this._upload(this.glyphTex, gl.RED, cols, rows, gbuf);
    this._upload(this.fgTex, gl.RGB, cols, rows, fg);
    this._upload(this.bgTex, gl.RGB, cols, rows, bg);

    gl.useProgram(this.program);
    gl.bindVertexArray(this.vao);
    this._bindUnit(0, this.atlasTex, this.u.uAtlas);
    this._bindUnit(1, this.glyphTex, this.u.uGlyph);
    this._bindUnit(2, this.fgTex, this.u.uFg);
    this._bindUnit(3, this.bgTex, this.u.uBg);
    gl.uniform1f(this.u.uCols, cols);
    gl.uniform1f(this.u.uRows, rows);
    gl.uniform1f(this.u.uGw, this.glyphPxW);
    gl.uniform1f(this.u.uGh, this.glyphPxH);
    gl.uniform1f(this.u.uCanvasH, this.canvas.height);
    gl.uniform1f(this.u.uAtlasW, this.atlasPxW);
    gl.uniform1f(this.u.uN, ATLAS_GLYPHS.length);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.bindVertexArray(null);
  }

  _upload(tex, format, cols, rows, data) {
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, cols, rows, format, gl.UNSIGNED_BYTE, data);
  }

  _bindUnit(unit, tex, loc) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.uniform1i(loc, unit);
  }
}
