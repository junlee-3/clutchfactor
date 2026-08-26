import { describe, expect, it } from "vitest";
import { extrema, sparkPoints, sparkSegments, streak, streakCallout } from "./trends";

describe("sparkPoints", () => {
  it("returns [] for an empty series", () => {
    expect(sparkPoints([], 100, 20)).toEqual([]);
  });

  it("centers a single-value series at x=w/2", () => {
    const points = sparkPoints([5], 100, 20);
    expect(points).toEqual([{ x: 50, y: 10 }]);
  });

  it("maps [0,10] in a 100x20 box (p=2) to y=18 at the first point and y=2 at the last", () => {
    const points = sparkPoints([0, 10], 100, 20, 2);
    expect(points[0]).toEqual({ x: 2, y: 18 });
    expect(points[points.length - 1]).toEqual({ x: 98, y: 2 });
  });

  it("spaces x evenly across the full series, first at x=p and last at x=w-p", () => {
    const points = sparkPoints([1, 2, 3, 4, 5], 100, 20, 2);
    expect(points.map((pt) => pt.x)).toEqual([2, 26, 50, 74, 98]);
  });

  it("renders every point at mid-height for a constant series", () => {
    const points = sparkPoints([7, 7, 7], 100, 20, 2);
    expect(points).toEqual([
      { x: 2, y: 10 },
      { x: 50, y: 10 },
      { x: 98, y: 10 },
    ]);
  });

  it("defaults padding to 2 when omitted", () => {
    expect(sparkPoints([0, 10], 100, 20)).toEqual(sparkPoints([0, 10], 100, 20, 2));
  });
});

describe("extrema", () => {
  it("returns -1/-1 for an empty series", () => {
    expect(extrema([])).toEqual({ minIndex: -1, maxIndex: -1 });
  });

  it("finds the min and max index in a mixed series", () => {
    expect(extrema([40, 70, 10, 55])).toEqual({ minIndex: 2, maxIndex: 1 });
  });

  it("keeps the earliest occurrence on a tie", () => {
    expect(extrema([5, 9, 9, 5])).toEqual({ minIndex: 0, maxIndex: 1 });
  });

  it("reports the same index for both on a single-value series", () => {
    expect(extrema([42])).toEqual({ minIndex: 0, maxIndex: 0 });
  });

  it("reports the same index for both on a constant series", () => {
    expect(extrema([7, 7, 7])).toEqual({ minIndex: 0, maxIndex: 0 });
  });
});

describe("streak", () => {
  it("detects a trailing strictly-decreasing run", () => {
    expect(streak([5, 4, 3, 2])).toEqual({ len: 4, dir: -1 });
  });

  it("detects a trailing strictly-increasing run", () => {
    expect(streak([1, 1, 2])).toEqual({ len: 2, dir: 1 });
  });

  it("reports no streak for a single value", () => {
    expect(streak([2])).toEqual({ len: 1, dir: 0 });
  });

  it("reports no streak for an empty series", () => {
    expect(streak([])).toEqual({ len: 0, dir: 0 });
  });

  it("stops the run at the most recent break, counting backward from the end", () => {
    // 5,3,4 -> last two (3,4) strictly increasing; the 5 breaks it.
    expect(streak([5, 3, 4])).toEqual({ len: 2, dir: 1 });
  });

  it("treats a flat trailing pair as no streak", () => {
    expect(streak([3, 3])).toEqual({ len: 1, dir: 0 });
  });
});

describe("streakCallout", () => {
  it("returns the exact §7 copy for a 4-match downward streak", () => {
    expect(streakCallout("Isolated deaths", [10, 8, 6, 4])).toBe(
      "Good news: Isolated deaths trending down 4 matches straight",
    );
  });

  it("does not prefix 'Good news:' for an upward streak", () => {
    expect(streakCallout("Isolated deaths", [2, 4, 6, 8])).toBe(
      "Isolated deaths trending up 4 matches straight",
    );
  });

  it("returns null when the streak is shorter than 3", () => {
    expect(streakCallout("Isolated deaths", [10, 8])).toBeNull();
  });

  it("returns null when there is no streak at all", () => {
    expect(streakCallout("Isolated deaths", [5, 5, 5])).toBeNull();
  });

  it("returns null for an empty series", () => {
    expect(streakCallout("Isolated deaths", [])).toBeNull();
  });
});

describe("sparkSegments", () => {
  it("breaks the line at holes and reports the last real point", () => {
    const s = sparkSegments([1, 2, null, 4, 5], 100, 20, 0);
    expect(s.paths).toHaveLength(2);
    expect(s.last?.v).toBe(5);
    expect(s.extrema).toEqual({ min: 1, max: 5 });
  });
  it("is empty for an all-null series", () => {
    expect(sparkSegments([null, null], 100, 20)).toEqual({ paths: [], last: null, extrema: null });
  });
  it("draws a lone point as a zero-length segment", () => {
    const s = sparkSegments([null, 3, null], 100, 20, 0);
    expect(s.paths).toHaveLength(1);
    expect(s.last).toMatchObject({ v: 3 });
  });

  it("reports the min and max value's coordinates at their first occurrence", () => {
    const s = sparkSegments([1, 2, null, 4, 5], 100, 20, 0);
    expect(s.minPoint?.v).toBe(1);
    expect(s.maxPoint?.v).toBe(5);
  });
});
