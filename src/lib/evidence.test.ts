import { describe, expect, it } from "vitest";
import { evidenceUrl, parseEvidenceParams } from "./evidence";

describe("evidence deep links", () => {
  it("builds and parses a full evidence url round-trip", () => {
    const url = evidenceUrl(7, {
      round: 12,
      tick_start: 76013,
      tick_end: 76500,
      focus_players: ["76561199228328773", "76561199011427752"],
    });
    expect(url).toBe(
      "/replay/7?round=12&tick=76013&focus=76561199228328773%2C76561199011427752",
    );
    const parsed = parseEvidenceParams(
      new URL(`http://x${url}`).searchParams,
    );
    expect(parsed.round).toBe(12);
    expect(parsed.tick).toBe(76013);
    expect(parsed.focus).toEqual([
      "76561199228328773",
      "76561199011427752",
    ]);
  });

  it("defaults sanely on missing/garbage params", () => {
    const p = parseEvidenceParams(new URLSearchParams("round=zzz&tick=abc"));
    expect(p.round).toBe(1);
    expect(p.tick).toBeNull();
    expect(p.focus).toEqual([]);
  });
});
