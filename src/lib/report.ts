// Pure presentation logic for the Match Report (unit-tested, IPC-free).

import type { EvidenceRefDto, NarratedInsight, RoundStat } from "./ipc";

const CATEGORY_ORDER = ["deaths", "utility", "positioning", "timing"] as const;

export const CATEGORY_TITLES: Record<string, string> = {
  deaths: "Deaths",
  utility: "Utility",
  positioning: "Positioning",
  timing: "Timing",
};

export interface InsightGroup {
  category: (typeof CATEGORY_ORDER)[number];
  insights: NarratedInsight[];
}

/** Fixed category order (§7 screen 2); score order preserved within. */
export function groupInsights(insights: NarratedInsight[]): InsightGroup[] {
  return CATEGORY_ORDER.map((category) => ({
    category,
    insights: insights.filter((i) => i.category === category),
  })).filter((g) => g.insights.length > 0);
}

/** "R3 · 0:31" — round + elapsed time since that round's freeze end. */
export function chipLabel(
  ev: EvidenceRefDto,
  rounds: RoundStat[],
  tickrate: number,
): string {
  const round = rounds.find((r) => r.number === ev.round);
  if (!round || round.freeze_end_tick === null) return `R${ev.round}`;
  const s = Math.max(
    0,
    Math.floor((ev.tick_start - round.freeze_end_tick) / tickrate),
  );
  const m = Math.floor(s / 60);
  const sec = (s % 60).toString().padStart(2, "0");
  return `R${ev.round} · ${m}:${sec}`;
}

const CLASS_LABELS: Record<number, string> = {
  1: "Caught in utility animation",
  2: "Caught in grenade damage",
  3: "Blinded / flashed out",
  4: "Caught reloading or scoped",
  5: "No-engagement death",
  6: "Isolated & untradeable",
  7: "Baited trade attempt",
  8: "Over-peek at man disadvantage",
  9: "Crossfire death",
  10: "Lost angle-advantage duel",
  11: "Pushed without info",
  12: "Repeat-hotspot death",
  13: "Outaimed in a fair duel",
  14: "Self / world / teammate",
  15: "Unclassified",
};

export function classLabel(id: number): string {
  return CLASS_LABELS[id] ?? `Class ${id}`;
}

/** Classes that are not the player's mistake (rendered differently). */
export const GOOD_NEWS_CLASS = 13;
export const HYGIENE_CLASSES = [14, 15];

export function roundResult(r: RoundStat): "won" | "lost" | "unknown" {
  if (!r.tracked_side) return "unknown";
  return r.winner === r.tracked_side ? "won" : "lost";
}
