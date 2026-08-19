// Pure trends math (sparkline geometry + streak detection); no I/O, no DOM.

export interface SparkPoint {
  x: number;
  y: number;
}

/** Map a series to SVG points in a w×h box, y inverted (0 at bottom), padding p.
 *  Constant series renders mid-height; empty series → []. */
export function sparkPoints(
  values: number[],
  w: number,
  h: number,
  p = 2,
): SparkPoint[] {
  const n = values.length;
  if (n === 0) return [];

  const min = Math.min(...values);
  const max = Math.max(...values);
  const constant = max === min;

  return values.map((v, i) => {
    const x = n === 1 ? w / 2 : p + (i * (w - 2 * p)) / (n - 1);
    const y = constant ? h / 2 : p + ((max - v) / (max - min)) * (h - 2 * p);
    return { x, y };
  });
}

/** Trailing strictly-monotonic run length (≥2 means a streak; direction -1 down, +1 up, 0 none).
 *  streak([5,4,3,2]) = {len: 4, dir: -1}; streak([1,1,2]) = {len: 2, dir: +1}; streak([2]) = {len: 1, dir: 0}. */
export function streak(values: number[]): { len: number; dir: -1 | 0 | 1 } {
  const n = values.length;
  if (n === 0) return { len: 0, dir: 0 };
  if (n === 1) return { len: 1, dir: 0 };

  const last = values[n - 1];
  const prev = values[n - 2];
  const dir: -1 | 0 | 1 = last > prev ? 1 : last < prev ? -1 : 0;
  if (dir === 0) return { len: 1, dir: 0 };

  let len = 2;
  for (let i = n - 2; i > 0; i--) {
    const a = values[i - 1];
    const b = values[i];
    const stepDir = b > a ? 1 : b < a ? -1 : 0;
    if (stepDir !== dir) break;
    len++;
  }
  return { len, dir };
}

/** §7 copy: "Isolated deaths trending down 4 matches straight" — null when len < 3.
 *  Down-streak on a bad-thing metric is good news: prefix "Good news: " when dir < 0. */
export function streakCallout(title: string, values: number[]): string | null {
  const { len, dir } = streak(values);
  if (len < 3 || dir === 0) return null;

  const direction = dir < 0 ? "down" : "up";
  const base = `${title} trending ${direction} ${len} matches straight`;
  return dir < 0 ? `Good news: ${base}` : base;
}
