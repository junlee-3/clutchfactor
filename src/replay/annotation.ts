// Pure helpers for the replay canvas's tracked-death annotation (issue #9
// §5, Task 9): the dashed chalk line from the victim to their nearest living
// teammate, distance labelled, plus a solid line to the killer. No I/O, no
// canvas — Renderer.ts (drawAnnotation/drawAnnotationTag) draws with these.

/** A player's world-space position and status at one tick — the shape
 *  Renderer builds from `stateAt` + `scene.sides` at the death tick. */
export interface AnnotationPoint {
  id: string;
  x: number;
  y: number;
  side: "CT" | "T" | undefined;
  alive: boolean;
}

/** Nearest living, same-side teammate to `victimId`, by straight-line world
 *  distance (computed pre-transform — radar-pixel scaling never enters this
 *  math). Excludes the victim's own record, dead players, and the enemy
 *  side. Returns null when `victimId` isn't in `states` or has no living
 *  teammate (the round's last man standing, or a fully-wiped side). */
export function nearestLivingTeammate(
  states: AnnotationPoint[],
  victimId: string,
): { id: string; dist: number } | null {
  const victim = states.find((s) => s.id === victimId);
  if (!victim) return null;

  let best: { id: string; dist: number } | null = null;
  for (const s of states) {
    if (s.id === victimId || !s.alive || s.side !== victim.side) continue;
    const dist = Math.hypot(s.x - victim.x, s.y - victim.y);
    if (best === null || dist < best.dist) best = { id: s.id, dist };
  }
  return best;
}

/** Thousands-separated world-unit distance for the canvas tag: "1,223 u".
 *  Mirrors cf-narrator::rail's `fmt_units` (Rust) exactly — the rail's own
 *  "away at <callout>" fact and this tag must read the same number. */
export function fmtUnits(n: number): string {
  return `${Math.round(n).toLocaleString("en-US")} u`;
}
