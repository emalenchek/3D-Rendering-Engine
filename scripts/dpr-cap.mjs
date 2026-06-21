// FR-10.2: the presenter must cap the backing-store DPR (default ≤ 2) and keep
// the canvas under iOS Safari's area limit, even on a retina phone. Loads the
// demo in a deviceScaleFactor=3 context and checks the live canvas. Run with the
// demo server already up: `node scripts/dpr-cap.mjs`.
import { chromium } from "playwright";

const URL = process.env.SMOKE_URL ?? "http://localhost:8000/";
const MAX_CANVAS_AREA = 16_777_216; // iOS pre-18 cap (4096²)
const COLS = 120, ROWS = 60, CELL_W = 8; // must match app.js / GridRenderer opts

const browser = await chromium.launch();
const context = await browser.newContext({ deviceScaleFactor: 3 });
const page = await context.newPage();
let code = 0;
try {
  await page.goto(URL, { waitUntil: "domcontentloaded", timeout: 30000 });
  await page.waitForFunction(
    () => /\d+ FPS/.test(document.querySelector("#status")?.textContent ?? ""),
    { timeout: 30000 },
  );
  const info = await page.evaluate(() => {
    const c = document.getElementById("screen");
    return { w: c.width, h: c.height, dpr: window.devicePixelRatio };
  });
  const effectiveDpr = info.w / (COLS * CELL_W);
  console.log(
    `dpr-cap: devicePixelRatio=${info.dpr}, canvas=${info.w}×${info.h}, effective dpr=${effectiveDpr}`,
  );

  if (info.dpr < 3) {
    console.warn(`note: context devicePixelRatio is ${info.dpr}, not 3 — cap not exercised`);
  }
  if (effectiveDpr > 2) {
    console.error(`✗ DPR not capped (effective ${effectiveDpr} > 2)`);
    code = 1;
  } else if (info.w * info.h > MAX_CANVAS_AREA) {
    console.error(`✗ backing area ${info.w * info.h} exceeds the iOS cap`);
    code = 1;
  } else {
    console.log("✓ DPR capped at ≤ 2 and backing area within the iOS limit");
  }
} catch (err) {
  console.error("✗ dpr-cap test error:", err.message);
  code = 1;
} finally {
  await browser.close();
}
process.exit(code);
