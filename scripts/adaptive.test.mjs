// FR-10.3: unit-test the adaptive-resolution policy (pure, no browser/FPS).
import assert from "node:assert/strict";
import { nextScale } from "../web/adaptive.js";

const cases = [
  [1, 10, 0.85, "downscale when slow"],
  [1, 23, 0.85, "just below the low threshold downscales"],
  [1, 24, 1, "at the low threshold: hold"],
  [0.5, 10, 0.5, "hold at the floor (min)"],
  [0.7, 60, 0.85, "upscale when fast"],
  [1, 60, 1, "hold at the ceiling (max)"],
  [1, 35, 1, "dead band: hold (high scale)"],
  [0.7, 35, 0.7, "dead band: hold (low scale)"],
];

let failed = 0;
for (const [scale, fps, expected, label] of cases) {
  const got = nextScale(scale, fps);
  try {
    assert.equal(got, expected, `${label}: nextScale(${scale}, ${fps}) = ${got}, want ${expected}`);
    console.log(`✓ ${label}`);
  } catch (err) {
    console.error(`✗ ${err.message}`);
    failed = 1;
  }
}
process.exit(failed);
