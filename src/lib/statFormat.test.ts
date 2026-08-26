import { describe, expect, it } from "vitest";
import { formatStat, STAT_KEYS, STAT_WHY } from "./statFormat";
import type { MatchStatsDto } from "./ipc";

const s: MatchStatsDto = { rounds_played: 24, kills: 18, deaths: 15, assists: 4, kd: 1.2, adr: 71.3, hs_pct: 44, kast_pct: 67,
  entry_attempts: 6, entry_wins: 2, traded_deaths: 5, trade_kills: 3, trade_opportunities: 7, clutch_attempts: 3, clutch_wins: 1 };

describe("formatStat", () => {
  it("renders every key with a value and a formula detail", () => {
    expect(formatStat("kd", s)).toEqual({ value: "1.20", detail: "18 kills / 15 deaths" });
    expect(formatStat("adr", s)).toEqual({ value: "71.3", detail: "damage to enemies per round, 24 rounds" });
    expect(formatStat("hs", s)).toEqual({ value: "44%", detail: "headshot kills / kills" });
    expect(formatStat("kast", s)).toEqual({ value: "67%", detail: "rounds with a kill, assist, survival or traded death" });
    expect(formatStat("entry", s)).toEqual({ value: "2/6", detail: "opening duels won / taken" });
    expect(formatStat("trade", s)).toEqual({ value: "5/15", detail: "deaths traded within 2 s · 3/7 trades you took" });
    expect(formatStat("clutch", s)).toEqual({ value: "1/3", detail: "1vX rounds won / attempted" });
    expect(STAT_KEYS).toHaveLength(7);
  });
  it("shows a dash, never a zero, when the ratio is undefined", () => {
    const empty = { ...s, kills: 0, deaths: 0, kd: null, adr: null, hs_pct: null, kast_pct: null, entry_attempts: 0, entry_wins: 0, clutch_attempts: 0, clutch_wins: 0 };
    expect(formatStat("kd", empty).value).toBe("0-0");
    expect(formatStat("adr", empty).value).toBe("—");
    expect(formatStat("hs", empty).value).toBe("—");
    expect(formatStat("kast", empty).value).toBe("—");
    expect(formatStat("entry", empty).value).toBe("—");
    expect(formatStat("trade", empty).value).toBe("—");
    expect(formatStat("clutch", empty).value).toBe("—");
  });
});

describe("STAT_WHY", () => {
  it("gives every stat a why-it-matters line in the §7 voice", () => {
    // Seven lines, one per stat; no exclamation marks and no numbers —
    // thresholds live on Watches, rendered from the live config. Rule-family
    // ids ("H2", "H14") are names, not numbers, so they are stripped before
    // the digit check.
    expect(Object.keys(STAT_WHY).sort()).toEqual([...STAT_KEYS].sort());
    for (const k of STAT_KEYS) {
      expect(STAT_WHY[k].replace(/\bH\d+\b/g, "")).not.toMatch(/[!\d]/);
      expect(STAT_WHY[k].length).toBeGreaterThan(20);
    }
  });
});
