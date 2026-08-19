import { describe, expect, it } from "vitest";
import { cellRect, densityToAlpha, gridMax } from "./heatmap";

describe("densityToAlpha", () => {
  it("is 0 when count is 0", () => {
    expect(densityToAlpha(0, 100)).toBe(0);
  });

  it("is 0 when max is 0 (0-safe, regardless of count)", () => {
    expect(densityToAlpha(0, 0)).toBe(0);
    expect(densityToAlpha(5, 0)).toBe(0);
  });

  it("caps at 0.85 when count equals max", () => {
    expect(densityToAlpha(100, 100)).toBeCloseTo(0.85);
  });

  it("follows a sqrt ramp, not linear", () => {
    // sqrt(25/100) * 0.85 = 0.5 * 0.85 = 0.425
    expect(densityToAlpha(25, 100)).toBeCloseTo(0.425);
    // sqrt(50/100) * 0.85 ≈ 0.7071 * 0.85 ≈ 0.6010
    expect(densityToAlpha(50, 100)).toBeCloseTo(0.601, 3);
  });

  it("is monotonically non-decreasing as count rises toward max", () => {
    const max = 200;
    let prev = -1;
    for (let count = 0; count <= max; count += 10) {
      const alpha = densityToAlpha(count, max);
      expect(alpha).toBeGreaterThanOrEqual(prev);
      prev = alpha;
    }
  });

  it("never exceeds 0.85 even if count exceeds max", () => {
    expect(densityToAlpha(1000, 100)).toBeLessThanOrEqual(0.85);
  });
});

describe("gridMax", () => {
  it("returns 0 for an empty array", () => {
    expect(gridMax([])).toBe(0);
  });

  it("returns 0 for an all-zero array", () => {
    expect(gridMax([0, 0, 0])).toBe(0);
  });

  it("returns the largest count", () => {
    expect(gridMax([3, 17, 5, 9])).toBe(17);
  });
});

describe("cellRect", () => {
  const size = 128;
  const canvasPx = 512;
  const cellPx = canvasPx / size; // 4

  it("maps the top-left cell (index 0) to the canvas origin", () => {
    expect(cellRect(0, size, canvasPx)).toEqual({ x: 0, y: 0, w: cellPx, h: cellPx });
  });

  it("maps the top-right cell of row 0 to the right edge", () => {
    const index = size - 1; // x = 127, y = 0
    expect(cellRect(index, size, canvasPx)).toEqual({
      x: (size - 1) * cellPx,
      y: 0,
      w: cellPx,
      h: cellPx,
    });
  });

  it("maps the bottom-left cell of the last row", () => {
    const index = (size - 1) * size; // x = 0, y = 127
    expect(cellRect(index, size, canvasPx)).toEqual({
      x: 0,
      y: (size - 1) * cellPx,
      w: cellPx,
      h: cellPx,
    });
  });

  it("maps the bottom-right cell (last index) to the bottom-right corner", () => {
    const index = size * size - 1; // x = 127, y = 127
    expect(cellRect(index, size, canvasPx)).toEqual({
      x: (size - 1) * cellPx,
      y: (size - 1) * cellPx,
      w: cellPx,
      h: cellPx,
    });
  });
});
