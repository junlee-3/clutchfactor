/** Callout labels: drawn only when the radar is wide enough to read them,
 *  at a constant 11 css px, with greedy overlap removal so a crowded map
 *  shows its biggest places first (input order = priority). */
// 560, not 600: this floor exists to guard label density at very small
// radar sizes, not to gate the common case — 600 hid labels at this app's
// own default windowed radar size on a standard laptop display (measured
// ~596 css px at the OS's max windowed height), defeating the feature for
// most users. placeLabels() already drops overlapping labels, so a lower
// floor doesn't risk a crowded/illegible radar; it just widens when the
// feature turns on (Task 10 fix round 1).
export const CALLOUT_MIN_CSS_PX = 560;
export const CALLOUT_CSS_FONT_PX = 11;

export function labelFontPx(canvasPx: number, cssPx: number): number | null {
  if (cssPx < CALLOUT_MIN_CSS_PX) return null;
  return Math.round((CALLOUT_CSS_FONT_PX * canvasPx) / cssPx);
}

export interface LabelBox {
  name: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export function placeLabels(items: LabelBox[]): LabelBox[] {
  const kept: LabelBox[] = [];
  for (const it of items) {
    const clash = kept.some(
      (k) =>
        Math.abs(k.x - it.x) * 2 < k.w + it.w &&
        Math.abs(k.y - it.y) * 2 < k.h + it.h,
    );
    if (!clash) kept.push(it);
  }
  return kept;
}
