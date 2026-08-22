// Pure presentation logic for match rows (unit-tested; keep IPC-free).

import type { MatchSummary } from "./ipc";

export interface MatchRow {
  /** Own score first when the tracked player played, else A–B as stored. */
  scoreline: string;
  resultLetter: "W" | "L" | "T" | null;
  kd: string | null;
  hs: string | null;
}

export function formatMatchRow(m: MatchSummary): MatchRow {
  const hi = Math.max(m.score_a, m.score_b);
  const lo = Math.min(m.score_a, m.score_b);
  let scoreline: string;
  let resultLetter: MatchRow["resultLetter"] = null;
  switch (m.tracked_result) {
    case "win":
      scoreline = `${hi}–${lo}`;
      resultLetter = "W";
      break;
    case "loss":
      scoreline = `${lo}–${hi}`;
      resultLetter = "L";
      break;
    case "tie":
      scoreline = `${m.score_a}–${m.score_b}`;
      resultLetter = "T";
      break;
    default:
      scoreline = `${m.score_a}–${m.score_b}`;
  }

  const kd =
    m.tracked_kills != null && m.tracked_deaths != null
      ? `${m.tracked_kills} / ${m.tracked_deaths}`
      : null;
  const hs =
    m.tracked_hs_pct != null ? `${Math.round(m.tracked_hs_pct)}% HS` : null;

  return { scoreline, resultLetter, kd, hs };
}
