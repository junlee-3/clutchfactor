import { describe, expect, it } from "vitest";
import {
  activeMomentIndex,
  nextFlagged,
  overlayWindow,
  prevFlagged,
} from "./rail";

describe("activeMomentIndex", () => {
  const moments = [{ tick: 1000 }, { tick: 2000 }, { tick: 3000 }];

  it("is -1 before the first moment", () => {
    expect(activeMomentIndex(moments, 500)).toBe(-1);
  });

  it("picks the earlier index when between two moments", () => {
    expect(activeMomentIndex(moments, 1500)).toBe(0);
    expect(activeMomentIndex(moments, 2500)).toBe(1);
  });

  it("picks the exact index when the tick matches a moment exactly", () => {
    expect(activeMomentIndex(moments, 1000)).toBe(0);
    expect(activeMomentIndex(moments, 2000)).toBe(1);
    expect(activeMomentIndex(moments, 3000)).toBe(2);
  });

  it("is the last index after the last moment", () => {
    expect(activeMomentIndex(moments, 9999)).toBe(2);
  });

  it("is -1 for an empty moment list", () => {
    expect(activeMomentIndex([], 100)).toBe(-1);
  });
});

describe("nextFlagged / prevFlagged", () => {
  const reviews = [
    { round: 1, selected: false },
    { round: 2, selected: true },
    { round: 3, selected: false },
    { round: 4, selected: false },
    { round: 5, selected: true },
    { round: 6, selected: false },
  ];

  it("nextFlagged skips unflagged rounds and lands on the next flagged one", () => {
    expect(nextFlagged(reviews, 1)).toBe(2);
    expect(nextFlagged(reviews, 2)).toBe(5);
    expect(nextFlagged(reviews, 3)).toBe(5);
    expect(nextFlagged(reviews, 4)).toBe(5);
  });

  it("nextFlagged returns null at the end (no flagged round ahead)", () => {
    expect(nextFlagged(reviews, 5)).toBeNull();
    expect(nextFlagged(reviews, 6)).toBeNull();
  });

  it("prevFlagged skips unflagged rounds and lands on the previous flagged one", () => {
    expect(prevFlagged(reviews, 6)).toBe(5);
    expect(prevFlagged(reviews, 5)).toBe(2);
    expect(prevFlagged(reviews, 4)).toBe(2);
    expect(prevFlagged(reviews, 3)).toBe(2);
  });

  it("prevFlagged returns null at the end (no flagged round behind)", () => {
    expect(prevFlagged(reviews, 2)).toBeNull();
    expect(prevFlagged(reviews, 1)).toBeNull();
  });

  it("returns null when no rounds are flagged at all", () => {
    const none = [
      { round: 1, selected: false },
      { round: 2, selected: false },
    ];
    expect(nextFlagged(none, 1)).toBeNull();
    expect(prevFlagged(none, 2)).toBeNull();
  });
});

describe("overlayWindow", () => {
  it("spans 5s before to 2s after at 64 tickrate", () => {
    // 5s * 64 = 320, 2s * 64 = 128 (the evidence_around convention)
    expect(overlayWindow(10_000, 64)).toEqual({ start: 9680, end: 10128 });
  });

  it("never produces a negative start below the moment tick minus the window", () => {
    // No clamping to 0 here — that's the caller's job when seeking ticks.
    expect(overlayWindow(100, 64)).toEqual({ start: -220, end: 228 });
  });
});
