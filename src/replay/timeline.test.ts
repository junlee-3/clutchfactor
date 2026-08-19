import { describe, expect, it } from "vitest";
import { fmtClock, fracToTick, tickToFrac } from "./timeline";

const spec = { startTick: 1000, endTick: 5000 };

describe("timeline", () => {
  it("tick↔frac round-trips", () => {
    expect(tickToFrac(spec, 1000)).toBe(0);
    expect(tickToFrac(spec, 5000)).toBe(1);
    expect(tickToFrac(spec, 3000)).toBeCloseTo(0.5);
    expect(fracToTick(spec, 0.5)).toBe(3000);
    expect(fracToTick(spec, tickToFrac(spec, 2345))).toBe(2345);
  });

  it("clamps out-of-range values", () => {
    expect(tickToFrac(spec, 0)).toBe(0);
    expect(tickToFrac(spec, 9999)).toBe(1);
    expect(fracToTick(spec, -0.5)).toBe(1000);
    expect(fracToTick(spec, 1.5)).toBe(5000);
  });

  it("formats elapsed clock time", () => {
    expect(fmtClock(spec, 1000, 64)).toBe("0:00");
    expect(fmtClock(spec, 1000 + 64 * 83, 64)).toBe("1:23");
    expect(fmtClock(spec, 1000 + 64 * 5, 64)).toBe("0:05");
  });

  it("degenerate zero-length spec doesn't divide by zero", () => {
    const z = { startTick: 100, endTick: 100 };
    expect(tickToFrac(z, 100)).toBe(0);
    expect(fracToTick(z, 0.7)).toBe(100);
  });
});
