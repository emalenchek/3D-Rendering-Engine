// FR-10.3 adaptive-resolution policy — a pure function so it can be unit-tested
// without a browser or live FPS. Given the current scale and the measured FPS,
// return the next scale: shrink the cell grid when sustained FPS is below `low`,
// grow it back (up to `max`) when comfortably above `high`. The dead band
// (`low`..`high`) plus the per-step bound prevents oscillation.

export function nextScale(
  scale,
  fps,
  { low = 24, high = 50, step = 0.15, min = 0.5, max = 1 } = {},
) {
  if (fps < low && scale > min) return Math.max(min, Number((scale - step).toFixed(4)));
  if (fps > high && scale < max) return Math.min(max, Number((scale + step).toFixed(4)));
  return scale;
}
