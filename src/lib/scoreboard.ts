import type { PlayerRoundStatsDto } from "./ipc";

export interface ScoreRow { steamid: string; name: string; side: string; tracked: boolean; rounds: number; kills: number; deaths: number; assists: number; damage: number; adr: number; hsPct: number | null; kastPct: number | null; entryWins: number; entryAttempts: number; traded: number }

const pct = (n: number, d: number) => (d > 0 ? Math.round((n / d) * 100) : null);

/** Match totals from the per-round rows — the same counting the engine
 *  did, so the "Match" tab agrees with match_stats for the tracked player. */
export function aggregate(rows: PlayerRoundStatsDto[]): ScoreRow[] {
  const by = new Map<string, ScoreRow & { hs: number; kast: number }>();
  for (const x of rows) {
    const a = by.get(x.steamid) ?? { steamid: x.steamid, name: x.name, side: x.side, tracked: x.tracked, rounds: 0, kills: 0, deaths: 0, assists: 0, damage: 0, adr: 0, hsPct: null, kastPct: null, entryWins: 0, entryAttempts: 0, traded: 0, hs: 0, kast: 0 };
    a.rounds += 1; a.kills += x.kills; a.deaths += x.deaths; a.assists += x.assists; a.damage += x.damage; a.hs += x.headshots;
    if (x.kills > 0 || x.assists > 0 || x.survived || x.traded) a.kast += 1;
    if (x.traded) a.traded += 1;
    if (x.entry) { a.entryAttempts += 1; if (x.entry === "win") a.entryWins += 1; }
    a.side = x.side; // last known side (halves swap); the table groups by it
    by.set(x.steamid, a);
  }
  return [...by.values()]
    .map(({ hs, kast, ...a }) => ({ ...a, adr: a.rounds ? Math.round((a.damage / a.rounds) * 10) / 10 : 0, hsPct: pct(hs, a.kills), kastPct: pct(kast, a.rounds) }))
    .sort((p, q) => (p.side === q.side ? q.kills - p.kills : p.side === "CT" ? -1 : 1));
}

/** Round-tab ordering: the store already orders rows by round, side,
 *  steamid — this re-sorts kills desc within each side (same predicate as
 *  aggregate's, kept separate/inline so aggregate stays exactly as
 *  specified) for display on the Round tab. */
export function sortRoundRows(rows: PlayerRoundStatsDto[]): PlayerRoundStatsDto[] {
  return [...rows].sort((p, q) => (p.side === q.side ? q.kills - p.kills : p.side === "CT" ? -1 : 1));
}
