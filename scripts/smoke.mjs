// Playwright smoke test for the WASM demo (FR-8.4 / NFR-18).
//
// Serves `web/` (the caller backgrounds `python3 -m http.server`, which serves
// .wasm as application/wasm just like Pages), loads the page in headless
// Chromium, and proves the demo actually boots:
//   1. no console errors and no uncaught page errors, and
//   2. #status reaches a live FPS readout (proves WASM init + render loop ran).
//
// Exits non-zero on any failure. Run: `node scripts/smoke.mjs`.
import { chromium } from "playwright";

const URL = process.env.SMOKE_URL ?? "http://localhost:8000/";
// The render loop writes `${w}×${h} · ${fps} FPS` into #status; match only the
// ASCII FPS tail to dodge Unicode (× / ·) escaping pitfalls.
const READY = /\d+ FPS/;

const errors = [];

const browser = await chromium.launch();
const page = await browser.newPage();

page.on("console", (msg) => {
  if (msg.type() === "error") errors.push(`console.error: ${msg.text()}`);
});
page.on("pageerror", (err) => errors.push(`pageerror: ${err.message}`));

let exitCode = 0;
try {
  // Retry the initial load while the background http.server warms up.
  let loaded = false;
  for (let attempt = 0; attempt < 20 && !loaded; attempt++) {
    try {
      await page.goto(URL, { waitUntil: "domcontentloaded", timeout: 5000 });
      loaded = true;
    } catch {
      await page.waitForTimeout(500);
    }
  }
  if (!loaded) throw new Error(`server never came up at ${URL}`);

  await page.waitForFunction(
    (re) => re.test(document.querySelector("#status")?.textContent ?? ""),
    READY,
    { timeout: 30000 },
  );

  const status = await page.$eval("#status", (el) => el.textContent);
  console.log(`#status reached: ${status}`);

  if (errors.length) {
    throw new Error(`page reported errors:\n  ${errors.join("\n  ")}`);
  }
  console.log("✓ demo smoke test passed");
} catch (err) {
  exitCode = 1;
  console.error(`✗ demo smoke test failed: ${err.message}`);
  if (errors.length) console.error(`  ${errors.join("\n  ")}`);
} finally {
  await browser.close();
}

process.exit(exitCode);
