import { describe, expect, it } from "vitest";
import {
  activeMomentIndex,
  annotationMomentIndex,
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

describe("annotationMomentIndex", () => {
  const tickrate = 64;
  // Far enough apart (10,000 ticks) that their individual windows
  // (-320/+128 ticks) never overlap — isolates the containment check from
  // the nearest-wins tie-break, which gets its own dedicated moments below.
  const moments = [
    { tick: 10_000, kind: "tracked_death" },
    { tick: 20_000, kind: "tracked_death" },
  ];

  it("hits the pre-roll: 3s before the death tick, victim still alive", () => {
    // The whole point of the fix — this was unreachable via activeMomentIndex.
    expect(annotationMomentIndex(moments, 10_000 - 3 * tickrate, tickrate)).toBe(0);
  });

  it("hits the post window: 1s after the death tick", () => {
    expect(annotationMomentIndex(moments, 10_000 + 1 * tickrate, tickrate)).toBe(0);
  });

  it("is -1 outside both edges", () => {
    // 6s before: past the 5s pre-roll boundary.
    expect(annotationMomentIndex(moments, 10_000 - 6 * tickrate, tickrate)).toBe(-1);
    // 3s after: past the 2s post-roll boundary.
    expect(annotationMomentIndex(moments, 10_000 + 3 * tickrate, tickrate)).toBe(-1);
  });

  it("picks the nearer moment when two windows overlap", () => {
    // 100 ticks (~1.56s) apart at 64 tickrate — well inside each other's
    // 5s pre-roll, so both windows contain ticks between them.
    const overlapping = [
      { tick: 10_000, kind: "tracked_death" },
      { tick: 10_100, kind: "tracked_death" },
    ];
    // Closer to the first moment (30 vs 70 ticks away).
    expect(annotationMomentIndex(overlapping, 10_030, tickrate)).toBe(0);
    // Closer to the second moment (70 vs 30 ticks away).
    expect(annotationMomentIndex(overlapping, 10_070, tickrate)).toBe(1);
  });

  it("ignores non-death kinds", () => {
    const mixed = [{ tick: 10_000, kind: "utility_wasted" }];
    expect(annotationMomentIndex(mixed, 10_000, tickrate)).toBe(-1);
  });

  it("is -1 for an empty moment list", () => {
    expect(annotationMomentIndex([], 10_000, tickrate)).toBe(-1);
  });
});
