import { describe, expect, it } from "vitest";
import type { MatchSummary } from "./ipc";
import { formatMatchRow } from "./score";

function summary(overrides: Partial<MatchSummary>): MatchSummary {
  return {
    id: 1,
    file_name: "m.dem",
    map: "de_mirage",
    imported_at: "2026-08-19 10:00:00",
    rounds: 24,
    score_a: 12,
    score_b: 12,
    tracked_steamid: "76561199228328773",
    tracked_result: "tie",
    tracked_kills: 8,
    tracked_deaths: 19,
    tracked_hs_pct: 37.5,
    ...overrides,
  };
}

describe("formatMatchRow", () => {
  it("orders the scoreline own-score-first for a win", () => {
    const row = formatMatchRow(
      summary({ tracked_result: "win", score_a: 4, score_b: 13 }),
    );
    // Tracked player won, so their 13 leads regardless of roster A/B order.
    expect(row.scoreline).toBe("13–4");
    expect(row.resultLetter).toBe("W");
  });

  it("orders the scoreline own-score-first for a loss", () => {
    const row = formatMatchRow(
      summary({ tracked_result: "loss", score_a: 13, score_b: 4 }),
    );
    expect(row.scoreline).toBe("4–13");
    expect(row.resultLetter).toBe("L");
  });

  it("handles a tie", () => {
    const row = formatMatchRow(summary({ tracked_result: "tie" }));
    expect(row.scoreline).toBe("12–12");
    expect(row.resultLetter).toBe("T");
  });

  it("formats map name without the de_ prefix", () => {
    expect(formatMatchRow(summary({})).mapLabel).toBe("Mirage");
    expect(formatMatchRow(summary({ map: "de_dust2" })).mapLabel).toBe("Dust2");
    expect(formatMatchRow(summary({ map: "cs_office" })).mapLabel).toBe(
      "Office",
    );
  });

  it("formats K/D and HS%", () => {
    const row = formatMatchRow(summary({}));
    expect(row.kd).toBe("8 / 19");
    expect(row.hs).toBe("38% HS");
  });

  it("degrades cleanly when the tracked player was not in the match", () => {
    const row = formatMatchRow(
      summary({
        tracked_result: null,
        tracked_kills: null,
        tracked_deaths: null,
        tracked_hs_pct: null,
      }),
    );
    expect(row.resultLetter).toBeNull();
    expect(row.scoreline).toBe("12–12");
    expect(row.kd).toBeNull();
    expect(row.hs).toBeNull();
  });

  it("omits HS% when there are kills but no headshot data", () => {
    const row = formatMatchRow(summary({ tracked_hs_pct: null }));
    expect(row.kd).toBe("8 / 19");
    expect(row.hs).toBeNull();
  });
});
