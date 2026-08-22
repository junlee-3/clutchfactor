import { describe, expect, it } from "vitest";
import { fmtUnits, nearestLivingTeammate } from "./annotation";

describe("nearestLivingTeammate", () => {
  const victim = { id: "1", x: 0, y: 0, side: "T" as const, alive: false };

  it("picks the nearest same-side alive teammate among several", () => {
    const states = [
      victim,
      { id: "2", x: 100, y: 0, side: "T" as const, alive: true }, // 100 away
      { id: "3", x: 0, y: 40, side: "T" as const, alive: true }, // 40 away
      { id: "5", x: 5, y: 0, side: "T" as const, alive: true }, // 5 away — nearest
    ];
    expect(nearestLivingTeammate(states, "1")).toEqual({ id: "5", dist: 5 });
  });

  it("excludes the victim itself", () => {
    const states = [victim, { id: "2", x: 0, y: 0, side: "T" as const, alive: true }];
    // "2" sits exactly on the victim's spot but is a distinct id — still valid.
    expect(nearestLivingTeammate(states, "1")).toEqual({ id: "2", dist: 0 });
    // A record sharing the victim's own id must never be treated as a candidate.
    expect(nearestLivingTeammate([victim], "1")).toBeNull();
  });

  it("excludes dead teammates", () => {
    const states = [
      victim,
      { id: "2", x: 10, y: 0, side: "T" as const, alive: false },
      { id: "3", x: 200, y: 0, side: "T" as const, alive: true },
    ];
    expect(nearestLivingTeammate(states, "1")).toEqual({ id: "3", dist: 200 });
  });

  it("excludes enemies regardless of distance", () => {
    const states = [
      victim,
      { id: "6", x: 1, y: 0, side: "CT" as const, alive: true },
      { id: "3", x: 300, y: 0, side: "T" as const, alive: true },
    ];
    expect(nearestLivingTeammate(states, "1")).toEqual({ id: "3", dist: 300 });
  });

  it("is null when the victim is alone (no living teammate)", () => {
    const states = [
      victim,
      { id: "2", x: 10, y: 0, side: "T" as const, alive: false },
      { id: "6", x: 5, y: 0, side: "CT" as const, alive: true },
    ];
    expect(nearestLivingTeammate(states, "1")).toBeNull();
  });

  it("is null when the victim isn't present in states", () => {
    const states = [{ id: "2", x: 10, y: 0, side: "T" as const, alive: true }];
    expect(nearestLivingTeammate(states, "1")).toBeNull();
  });
});

describe("fmtUnits", () => {
  it("rounds and thousands-separates", () => {
    expect(fmtUnits(1223.4)).toBe("1,223 u");
  });

  it("passes through small numbers with no separator", () => {
    expect(fmtUnits(818)).toBe("818 u");
  });

  it("rounds .5 up", () => {
    expect(fmtUnits(999.6)).toBe("1,000 u");
  });
});
