import type { MatchStatsDto } from "./ipc";

export const STAT_KEYS = ["kd", "adr", "hs", "kast", "entry", "trade", "clutch"] as const;
export type StatKey = (typeof STAT_KEYS)[number];
export const STAT_TITLES: Record<StatKey, string> = { kd: "K/D", adr: "ADR", hs: "HS%", kast: "KAST", entry: "Entry", trade: "Trades", clutch: "Clutch" };

/** Why the number matters (spec §3): one line per stat, in the §7 coaching
 *  voice — no thresholds and no numbers (those are on Watches, rendered
 *  from the live config), no exclamation. Rendered under the Trends cell
 *  value; the link on the title goes to the rules behind it. */
export const STAT_WHY: Record<StatKey, string> = {
  kd: "Every death has a class; the fixable ones are the untraded and unforced",
  adr: "Damage is what utility and trades convert into; low ADR with many deaths points at exposure",
  hs: "An aim outcome, not a habit — the engine measures results, not crosshair placement",
  kast: "Rounds you took part in; an untraded death with nothing else is the miss the H2 rules describe",
  entry: "Opening duels shape the round; an entry with no teammate in trade range is the H14 flag",
  trade: "Trades are the team's insurance; both sides of it are counted here",
  clutch: "Last alive against several; these rounds are where the coach's verdicts matter most",
};

const ratio = (n: number, d: number) => (d > 0 ? `${n}/${d}` : "—");

/** The number and the sentence that says how it was counted (spec §1).
 *  The "2 s" in the trade detail is `trade.commit_window_s`'s documented
 *  default (DetectorConfig) — the Watches screen (Task 9) shows the live
 *  configured value; this strip's detail is a hover tooltip, so the
 *  constant is acceptable here. */
export function formatStat(key: StatKey, s: MatchStatsDto): { value: string; detail: string } {
  switch (key) {
    case "kd":
      return { value: s.kd === null ? `${s.kills}-${s.deaths}` : s.kd.toFixed(2), detail: `${s.kills} kills / ${s.deaths} deaths` };
    case "adr":
      return { value: s.adr === null ? "—" : s.adr.toFixed(1), detail: `damage to enemies per round, ${s.rounds_played} rounds` };
    case "hs":
      return { value: s.hs_pct === null ? "—" : `${s.hs_pct}%`, detail: "headshot kills / kills" };
    case "kast":
      return { value: s.kast_pct === null ? "—" : `${s.kast_pct}%`, detail: "rounds with a kill, assist, survival or traded death" };
    case "entry":
      return { value: ratio(s.entry_wins, s.entry_attempts), detail: "opening duels won / taken" };
    case "trade":
      return { value: ratio(s.traded_deaths, s.deaths), detail: `deaths traded within 2 s · ${ratio(s.trade_kills, s.trade_opportunities)} trades you took` };
    case "clutch":
      return { value: ratio(s.clutch_wins, s.clutch_attempts), detail: "1vX rounds won / attempted" };
  }
}
