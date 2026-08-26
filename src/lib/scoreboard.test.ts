import { describe, expect, it } from "vitest";
import { aggregate, sortRoundRows } from "./scoreboard";
import type { PlayerRoundStatsDto } from "./ipc";

const r = (o: Partial<PlayerRoundStatsDto>): PlayerRoundStatsDto => ({ round: 1, steamid: "1", name: "me", side: "CT", kills: 0, deaths: 0, assists: 0, damage: 0, headshots: 0, survived: true, traded: false, entry: null, tracked: true, ...o });

describe("aggregate", () => {
  it("sums per player across rounds and derives adr, hs%, kast%, entry", () => {
    const rows = [
      r({ round: 1, kills: 2, headshots: 1, damage: 150, entry: "win" }),
      r({ round: 2, kills: 0, deaths: 1, survived: false, traded: true, damage: 30 }),
      r({ round: 3, kills: 0, deaths: 1, survived: false, damage: 0, entry: "loss" }),
      r({ round: 1, steamid: "9", name: "them", side: "T", tracked: false, deaths: 1, survived: false }),
    ];
    const [me, them] = aggregate(rows);
    expect(me).toMatchObject({ steamid: "1", rounds: 3, kills: 2, deaths: 2, damage: 180, adr: 60, hsPct: 50, kastPct: 67, entryWins: 1, entryAttempts: 2, traded: 1 });
    expect(them).toMatchObject({ steamid: "9", side: "T", rounds: 1, kastPct: 0, hsPct: null });
  });
  it("orders CT before T and by kills within a side", () => {
    const rows = [r({ steamid: "9", side: "T", kills: 5, tracked: false }), r({ steamid: "1", kills: 1 }), r({ steamid: "2", name: "mate", kills: 3, tracked: false })];
    expect(aggregate(rows).map((x) => x.steamid)).toEqual(["2", "1", "9"]);
  });
});

describe("sortRoundRows", () => {
  it("orders CT before T and by kills desc within a side, matching aggregate's ordering", () => {
    const rows = [
      r({ steamid: "9", side: "T", kills: 5, tracked: false }),
      r({ steamid: "1", kills: 1 }),
      r({ steamid: "2", name: "mate", kills: 3, tracked: false }),
    ];
    expect(sortRoundRows(rows).map((x) => x.steamid)).toEqual(["2", "1", "9"]);
  });
});
