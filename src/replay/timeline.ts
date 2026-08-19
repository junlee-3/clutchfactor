// Timeline math for the scrubber: tick ↔ fraction ↔ clock text.

export interface TimelineSpec {
  startTick: number;
  endTick: number;
}

export function tickToFrac(spec: TimelineSpec, tick: number): number {
  const span = spec.endTick - spec.startTick;
  if (span <= 0) return 0;
  return Math.min(1, Math.max(0, (tick - spec.startTick) / span));
}

export function fracToTick(spec: TimelineSpec, frac: number): number {
  const f = Math.min(1, Math.max(0, frac));
  return Math.round(spec.startTick + (spec.endTick - spec.startTick) * f);
}

/** Elapsed time since round start, "m:ss". */
export function fmtClock(
  spec: TimelineSpec,
  tick: number,
  tickrate: number,
): string {
  const s = Math.max(0, Math.floor((tick - spec.startTick) / tickrate));
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${m}:${sec.toString().padStart(2, "0")}`;
}
