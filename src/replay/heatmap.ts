// Pure math for the pro-corpus occupancy heatmap (PROMPT.md M5 §corpus).
// No I/O, no canvas — the component owns drawing; this owns the numbers so
// they're unit-testable without a DOM.

export interface GridDto {
  map: string;
  side: "CT" | "T";
  phase: string;
  size: number;
  counts: number[];
  demos: number;
  samples: number;
}

/** Sqrt ramp so low-density cells stay visible instead of vanishing linearly.
 *  0 when count is 0 or max is 0 (0-safe); capped at 0.85 so the radar under
 *  the fill never fully disappears; monotonic in count. */
export function densityToAlpha(count: number, max: number): number {
  if (count <= 0 || max <= 0) return 0;
  return Math.min(0.85, Math.sqrt(count / max) * 0.85);
}

/** Largest count in the grid; 0 for an empty or all-zero grid. */
export function gridMax(counts: number[]): number {
  let max = 0;
  for (const c of counts) {
    if (c > max) max = c;
  }
  return max;
}

/** Row-major index → canvas-pixel rect for cell (x, y), where
 *  index = y * size + x. counts are 1024×1024-source, size×size buckets;
 *  canvasPx is the on-screen square canvas side length. */
export function cellRect(
  index: number,
  size: number,
  canvasPx: number,
): { x: number; y: number; w: number; h: number } {
  const x = index % size;
  const y = Math.floor(index / size);
  const cell = canvasPx / size;
  return { x: x * cell, y: y * cell, w: cell, h: cell };
}
