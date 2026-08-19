import { describe, expect, it } from "vitest";
import type { NarratedInsight, RoundStat } from "./ipc";
import { chipLabel, classLabel, groupInsights, roundResult } from "./report";

function insight(category: NarratedInsight["category"], score: number): NarratedInsight {
  return {
    detector: "X",
    category,
    severity: 0.5,
    confidence: 0.8,
    round: 0,
    score,
    title: "t",
    body: "b",
    metrics: {},
    evidence: [],
  };
}

describe("groupInsights", () => {
  it("groups by category in fixed order, keeping score order within", () => {
    const groups = groupInsights([
      insight("utility", 0.3),
      insight("deaths", 0.9),
      insight("deaths", 0.5),
      insight("timing", 0.4),
    ]);
    expect(groups.map((g) => g.category)).toEqual([
      "deaths",
      "utility",
      "timing",
    ]);
    expect(groups[0].insights.map((i) => i.score)).toEqual([0.9, 0.5]);
  });

  it("omits empty categories", () => {
    const groups = groupInsights([insight("positioning", 1)]);
    expect(groups).toHaveLength(1);
    expect(groups[0].category).toBe("positioning");
  });
});

describe("chipLabel", () => {
  const rounds: RoundStat[] = [
    { number: 3, freeze_end_tick: 9931, winner: "CT", tracked_side: "CT", kills: 1, deaths: 1 },
  ];
  it("renders round + elapsed clock from freeze end", () => {
    expect(
      chipLabel({ round: 3, tick_start: 11900, tick_end: 12000, focus_players: [], camera_hint: null }, rounds, 64),
    ).toBe("R3 · 0:30");
  });
  it("clamps negative offsets and missing rounds", () => {
    expect(
      chipLabel({ round: 3, tick_start: 9000, tick_end: 9500, focus_players: [], camera_hint: null }, rounds, 64),
    ).toBe("R3 · 0:00");
    expect(
      chipLabel({ round: 9, tick_start: 100, tick_end: 200, focus_players: [], camera_hint: null }, rounds, 64),
    ).toBe("R9");
  });
});

describe("classLabel and roundResult", () => {
  it("labels taxonomy classes", () => {
    expect(classLabel(6)).toBe("Isolated & untradeable");
    expect(classLabel(13)).toBe("Outaimed in a fair duel");
    expect(classLabel(99)).toBe("Class 99");
  });
  it("derives per-round result for the tracked side", () => {
    expect(
      roundResult({ number: 1, freeze_end_tick: null, winner: "CT", tracked_side: "CT", kills: 2, deaths: 0 }),
    ).toBe("won");
    expect(
      roundResult({ number: 2, freeze_end_tick: null, winner: "T", tracked_side: "CT", kills: 0, deaths: 1 }),
    ).toBe("lost");
    expect(
      roundResult({ number: 3, freeze_end_tick: null, winner: "T", tracked_side: null, kills: 0, deaths: 0 }),
    ).toBe("unknown");
  });
});
