// FR-10.1 / NFR-22: prove the batched `GridRenderer.draw()` is **pixel-identical**
// to the original per-cell `drawCompat()` reference. Runs both paths in one real
// browser (same font backend, same atlas) on a synthetic frame and compares the
// canvases byte-for-byte. Run with the demo server already up:
//   node scripts/pixel-parity.mjs
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

  const res = await page.evaluate(async () => {
    const { GridRenderer } = await import("./renderer.js");
    const cols = 48, rows = 24, n = cols * rows;
    const glyphs = new Uint32Array(n);
    const fg = new Uint8Array(n * 3);
    const bg = new Uint8Array(n * 3);
    // A deterministic spread of glyphs (incl. spaces) and varied colours.
    const cps = [..." .,-~:;=!*#$@█▀"].map((c) => c.codePointAt(0));
    for (let i = 0; i < n; i++) {
      glyphs[i] = cps[(i * 7) % cps.length];
      fg[i * 3] = (i * 53) % 256; fg[i * 3 + 1] = (i * 97) % 256; fg[i * 3 + 2] = (i * 29) % 256;
      bg[i * 3] = (i * 13) % 256; bg[i * 3 + 1] = (i * 31) % 256; bg[i * 3 + 2] = (i * 71) % 256;
    }
    const mk = () =>
      new GridRenderer(document.createElement("canvas"), { cellW: 8, cellH: 16, font: "15px monospace" });

    const a = mk(); a.draw(cols, rows, glyphs, fg, bg);          // batched (FR-10.1)
    const b = mk(); b.drawCompat(cols, rows, glyphs, fg, bg);    // per-cell reference

    const W = a.canvas.width, H = a.canvas.height;
    const da = a.ctx.getImageData(0, 0, W, H).data;
    const db = b.ctx.getImageData(0, 0, W, H).data;
    let mismatch = 0, maxdiff = 0;
    for (let k = 0; k < da.length; k++) {
      const d = Math.abs(da[k] - db[k]);
      if (d) { mismatch++; if (d > maxdiff) maxdiff = d; }
    }
    return { mismatch, total: da.length, maxdiff, W, H };
  });

  console.log(
    `pixel-parity: ${res.W}×${res.H} — mismatched bytes ${res.mismatch}/${res.total}, maxdiff ${res.maxdiff}`,
  );
  if (res.mismatch !== 0) {
    console.error("✗ batched draw() diverged from the per-cell reference");
    code = 1;
  } else {
    console.log("✓ batched presenter is pixel-identical to the reference");
  }
} catch (err) {
  console.error("✗ pixel-parity test error:", err.message);
  code = 1;
} finally {
  await browser.close();
}
process.exit(code);
