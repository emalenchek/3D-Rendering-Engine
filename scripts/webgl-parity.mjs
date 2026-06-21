// FR-11.2 / NFR-23: the WebGL2 presenter must render the same frame as the
// Canvas2D presenter (within tolerance — GPU sampling need not be bit-identical)
// and never be blank. Renders one synthetic frame through both presenters in a
// real browser and compares. If the runner has no WebGL2 at all, the test skips
// (the Canvas2D fallback path is what matters there). Run with the demo up:
//   node scripts/webgl-parity.mjs
import { chromium } from "playwright";

const URL = process.env.SMOKE_URL ?? "http://localhost:8000/";

const browser = await chromium.launch();
const page = await browser.newPage();
let code = 0;
try {
  page.on("pageerror", (e) => {
    console.error("pageerror:", e.message);
    code = 1;
  });
  await page.goto(URL, { waitUntil: "domcontentloaded", timeout: 30000 });

  const hasWebGL2 = await page.evaluate(
    () => !!document.createElement("canvas").getContext("webgl2"),
  );
  if (!hasWebGL2) {
    console.log("· runner has no WebGL2 — skipping parity (Canvas2D fallback covers this)");
  } else {
    const res = await page.evaluate(async () => {
      const [{ GridRenderer }, { WebGLGridRenderer }] = await Promise.all([
        import("./renderer.js"),
        import("./webgl-renderer.js"),
      ]);
      const cols = 48, rows = 24, n = cols * rows;
      const glyphs = new Uint32Array(n);
      const fg = new Uint8Array(n * 3);
      const bg = new Uint8Array(n * 3);
      const cps = [..." .,-~:;=!*#$@█▀"].map((c) => c.codePointAt(0));
      for (let i = 0; i < n; i++) {
        glyphs[i] = cps[(i * 7) % cps.length];
        fg[i * 3] = (i * 53) % 256; fg[i * 3 + 1] = (i * 97) % 256; fg[i * 3 + 2] = (i * 29) % 256;
        bg[i * 3] = (i * 13) % 256; bg[i * 3 + 1] = (i * 31) % 256; bg[i * 3 + 2] = (i * 71) % 256;
      }
      const opts = { cellW: 8, cellH: 16, font: "15px monospace" };

      const c2 = new GridRenderer(document.createElement("canvas"), opts);
      c2.draw(cols, rows, glyphs, fg, bg);

      const gl = new WebGLGridRenderer(document.createElement("canvas"), opts);
      gl.draw(cols, rows, glyphs, fg, bg);

      const W = c2.canvas.width, H = c2.canvas.height;
      // Read the WebGL canvas back through a 2D context.
      const rb = document.createElement("canvas");
      rb.width = W; rb.height = H;
      rb.getContext("2d").drawImage(gl.canvas, 0, 0);

      const a = c2.ctx.getImageData(0, 0, W, H).data;
      const b = rb.getContext("2d").getImageData(0, 0, W, H).data;
      let sum = 0, perceptible = 0, nonbg = 0;
      const first = [b[0], b[1], b[2]];
      let blank = true;
      for (let k = 0; k < a.length; k += 4) {
        for (let c = 0; c < 3; c++) {
          const d = Math.abs(a[k + c] - b[k + c]);
          sum += d;
          if (d > 32) perceptible++;
        }
        if (b[k] !== first[0] || b[k + 1] !== first[1] || b[k + 2] !== first[2]) blank = false;
      }
      const channels = (a.length / 4) * 3;
      return { W, H, mean: sum / channels, perceptibleFrac: perceptible / channels, blank };
    });
    if (res.error) throw new Error(res.error);
    console.log(
      `webgl-parity: ${res.W}×${res.H} — mean Δ ${res.mean.toFixed(2)}, ` +
        `perceptible ${(res.perceptibleFrac * 100).toFixed(2)}%, blank=${res.blank}`,
    );
    if (res.blank) { console.error("✗ WebGL output is blank"); code = 1; }
    else if (res.mean > 4 || res.perceptibleFrac > 0.01) {
      console.error("✗ WebGL output diverges from the Canvas2D presenter beyond tolerance");
      code = 1;
    } else {
      console.log("✓ WebGL presenter matches the Canvas2D presenter within tolerance");
    }
  }
} catch (err) {
  console.error("✗ webgl-parity test error:", err.message);
  code = 1;
} finally {
  await browser.close();
}
process.exit(code);
