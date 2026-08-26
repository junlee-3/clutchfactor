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

export interface Extrema {
  minIndex: number;
  maxIndex: number;
}

/** Index of the series' first minimum and first maximum (a tie keeps the
 *  earliest occurrence, so a flat run reports its leading edge for both).
 *  {-1, -1} for an empty series — callers must check length before indexing.
 *  Used to place the big trend line's min/max annotations (dataviz:
 *  "label the endpoint, the extreme" — never every point). */
export function extrema(values: number[]): Extrema {
  if (values.length === 0) return { minIndex: -1, maxIndex: -1 };
  let minIndex = 0;
  let maxIndex = 0;
  for (let i = 1; i < values.length; i++) {
    if (values[i] < values[minIndex]) minIndex = i;
    if (values[i] > values[maxIndex]) maxIndex = i;
  }
  return { minIndex, maxIndex };
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

export interface SparkSegmentPoint {
  x: number;
  y: number;
  v: number;
}

export interface SparkSegmentExtrema {
  min: number;
  max: number;
}

export interface SparkSegments {
  paths: string[];
  last: SparkSegmentPoint | null;
  extrema: SparkSegmentExtrema | null;
  /** Coordinates of the min/max value's first occurrence (extrema()'s own
   *  earliest-occurrence tie-break) — only present when there is at least
   *  one real value. Callers place on-chart min/max labels from these
   *  directly instead of re-deriving the series' scaling a second time. */
  minPoint?: SparkSegmentPoint;
  maxPoint?: SparkSegmentPoint;
}

/** Sparkline segments for a series with holes (matches analyzed before a
 *  stat existed). Holes break the line; a lone point becomes a dot-length
 *  segment so it still renders. Y scales over the real values only. */
export function sparkSegments(
  values: (number | null)[],
  w: number,
  h: number,
  p = 2,
): SparkSegments {
  const real = values.filter((v): v is number => v !== null);
  if (real.length === 0) return { paths: [], last: null, extrema: null };
  const min = Math.min(...real);
  const max = Math.max(...real);
  const n = values.length;
  const x = (i: number) => (n === 1 ? w / 2 : p + (i * (w - 2 * p)) / (n - 1));
  const y = (v: number) => (max === min ? h / 2 : p + ((max - v) * (h - 2 * p)) / (max - min));
  const paths: string[] = [];
  let run: string[] = [];
  let last: SparkSegmentPoint | null = null;
  let minPoint: SparkSegmentPoint | undefined;
  let maxPoint: SparkSegmentPoint | undefined;
  values.forEach((v, i) => {
    if (v === null) {
      if (run.length) paths.push(run.join(" "));
      run = [];
      return;
    }
    const px = x(i);
    const py = y(v);
    run.push(`${run.length ? "L" : "M"}${px.toFixed(1)},${py.toFixed(1)}`);
    if (run.length === 1) run.push(`L${px.toFixed(1)},${py.toFixed(1)}`);
    last = { x: px, y: py, v };
    if (minPoint === undefined && v === min) minPoint = { x: px, y: py, v };
    if (maxPoint === undefined && v === max) maxPoint = { x: px, y: py, v };
  });
  if (run.length) paths.push(run.join(" "));
  return { paths, last, extrema: { min, max }, minPoint, maxPoint };
}
