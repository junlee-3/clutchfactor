import { describe, expect, it } from "vitest";
import { placeLabels, labelFontPx, CALLOUT_MIN_CSS_PX } from "./callouts";

describe("placeLabels", () => {
  it("keeps the first of two overlapping labels and both of two apart", () => {
    const a = { name: "A", x: 100, y: 100, w: 60, h: 12 };
    const b = { name: "B", x: 110, y: 104, w: 60, h: 12 };
    const c = { name: "C", x: 400, y: 400, w: 60, h: 12 };
    expect(placeLabels([a, b, c]).map((l) => l.name)).toEqual(["A", "C"]);
  });
});

describe("labelFontPx", () => {
  it("renders 11 css px regardless of the canvas scale and hides below the floor", () => {
    expect(labelFontPx(1024, 1024)).toBe(11);
    expect(labelFontPx(1024, CALLOUT_MIN_CSS_PX)).toBe(20); // exactly at the floor is visible (11 * 1024 / 560 = 20.1)
    expect(labelFontPx(1024, 660)).toBe(17);
    expect(labelFontPx(1024, 559)).toBe(null);
  });
});
