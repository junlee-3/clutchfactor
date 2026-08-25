// Pure logic for the round-by-round coach rail: which moment is "active" as
// the scrubber moves, flagged-round navigation, and the replay overlay
// window. No I/O, no DOM — the rail component owns rendering; this owns the
// numbers so they're unit-testable without React.

/** Index of the last moment with tick <= displayTick; -1 before the first
 *  moment (or for an empty list). Assumes moments are ordered by tick
 *  ascending, as the backend produces them. */
export function activeMomentIndex(
  moments: { tick: number }[],
  displayTick: number,
): number {
  for (let i = moments.length - 1; i >= 0; i--) {
    if (moments[i].tick <= displayTick) return i;
  }
  return -1;
}

/** Nearest flagged (selected) round strictly after `current`; null if none —
 *  navigation moves between flagged rounds only, skipping the rest. */
export function nextFlagged(
  reviews: { round: number; selected: boolean }[],
  current: number,
): number | null {
  let best: number | null = null;
  for (const r of reviews) {
    if (r.selected && r.round > current && (best === null || r.round < best)) {
      best = r.round;
    }
  }
  return best;
}

/** Nearest flagged (selected) round strictly before `current`; null if none. */
export function prevFlagged(
  reviews: { round: number; selected: boolean }[],
  current: number,
): number | null {
  let best: number | null = null;
  for (const r of reviews) {
    if (r.selected && r.round < current && (best === null || r.round > best)) {
      best = r.round;
    }
  }
  return best;
}

/** Replay overlay window around a moment: 5s before -> 2s after, at the
 *  given tickrate — the same pre-/post-roll as cf-analysis's
 *  `evidence_around` (src-tauri/crates/cf-analysis/src/lib.rs). Not clamped
 *  to the round's own tick bounds; the caller (replay seek) does that. */
export function overlayWindow(
  momentTick: number,
  tickrate: number,
): { start: number; end: number } {
  return {
    start: momentTick - Math.round(5 * tickrate),
    end: momentTick + Math.round(2 * tickrate),
  };
}

/** Index of the `tracked_death` moment whose `overlayWindow` CONTAINS
 *  `displayTick`; -1 if none does. This is deliberately NOT
 *  `activeMomentIndex` (last moment with tick <= displayTick) — that
 *  definition can never be true before the moment's own tick, which would
 *  make the overlay's -5s pre-roll unreachable (the whole point of the
 *  window is to show the play developing BEFORE the death, while the
 *  victim is still alive — see canvas annotation, issue #9 §5). Windows can
 *  overlap for back-to-back deaths; when more than one contains
 *  `displayTick`, the moment whose own tick is nearest wins, so the
 *  annotation always reflects the death actually unfolding right now, not
 *  a stale earlier one whose +2s post-roll simply hasn't expired yet.
 *  Non-`tracked_death` moments (e.g. utility/positioning notes) are never
 *  candidates — the canvas overlay only ever draws a death's ink lines. */
export function annotationMomentIndex(
  moments: { tick: number; kind: string }[],
  displayTick: number,
  tickrate: number,
): number {
  let bestIdx = -1;
  let bestDist = Infinity;
  for (let i = 0; i < moments.length; i++) {
    const m = moments[i];
    if (m.kind !== "tracked_death") continue;
    const w = overlayWindow(m.tick, tickrate);
    if (displayTick < w.start || displayTick > w.end) continue;
    const dist = Math.abs(m.tick - displayTick);
    if (dist < bestDist) {
      bestDist = dist;
      bestIdx = i;
    }
  }
  return bestIdx;
}

/** The coach's per-play comments keyed by play tick (a tick can carry more
 *  than one line; the rail renders them under the first row at that tick). */
export function commentsByTick(
  plays: { tick: number; comment: string }[],
): Map<number, string[]> {
  const m = new Map<number, string[]>();
  for (const p of plays) {
    const list = m.get(p.tick);
    if (list) list.push(p.comment);
    else m.set(p.tick, [p.comment]);
  }
  return m;
}

export type StripeTone = "loss" | "win" | "neutral";

/** Color of the rail's active-row edge (spec §1 "Rail per the mockup"): a
 *  measured quality tag first, then a fired rule (always a cost), then the
 *  sign of the win-prob delta. Neutral when nothing measured says otherwise
 *  — the stripe never guesses. */
export function stripeTone(m: {
  delta_p: number | null;
  rule_id: string | null;
  quality?: string | null;
}): StripeTone {
  if (m.quality === "bad") return "loss";
  if (m.quality === "good") return "win";
  if (m.quality === "neutral") return "neutral";
  if (m.rule_id) return "loss";
  if (m.delta_p !== null && m.delta_p !== undefined) {
    if (m.delta_p < 0) return "loss";
    if (m.delta_p > 0) return "win";
  }
  return "neutral";
}
