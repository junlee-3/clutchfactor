/** Callout labels: drawn only when the radar is wide enough to read them,
 *  at a constant 11 css px, with greedy overlap removal so a crowded map
 *  shows its biggest places first (input order = priority). */
export const CALLOUT_MIN_CSS_PX = 600;
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
