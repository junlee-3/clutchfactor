import { describe, expect, it } from "vitest";
import type { GrenadeInfo } from "../lib/ipc";
import { utilityWindows } from "./utility";

const TICKRATE = 64;

function nade(
  kind: string,
  tick: number,
  x = 0,
  y = 0,
): GrenadeInfo {
  return { tick, kind, thrower: "a", x, y, z: 0 };
}

describe("utilityWindows", () => {
  it("pairs a smoke with its expiry at the same spot", () => {
    const windows = utilityWindows(
      [nade("smoke", 1000, 100, 100), nade("smoke_expired", 1800, 105, 100)],
      TICKRATE,
    );
    expect(windows).toHaveLength(1);
    expect(windows[0]).toMatchObject({ kind: "smoke", startTick: 1000, endTick: 1800 });
  });

  it("falls back to 19.5s for an unpaired smoke", () => {
    const windows = utilityWindows([nade("smoke", 1000)], TICKRATE);
    expect(windows[0].endTick).toBe(1000 + Math.round(19.5 * TICKRATE));
  });

  it("does not pair an expiry that is too far away", () => {
    const windows = utilityWindows(
      [nade("smoke", 1000, 0, 0), nade("smoke_expired", 1800, 5000, 0)],
      TICKRATE,
    );
    expect(windows[0].endTick).toBe(1000 + Math.round(19.5 * TICKRATE));
  });

  it("consumes each expiry once — nearest smoke wins", () => {
    const windows = utilityWindows(
      [
        nade("smoke", 1000, 0, 0),
        nade("smoke", 1010, 40, 0),
        nade("smoke_expired", 1900, 35, 0),
      ],
      TICKRATE,
    );
    const near = windows.find((w) => w.x === 40)!;
    const far = windows.find((w) => w.x === 0)!;
    expect(near.endTick).toBe(1900);
    expect(far.endTick).toBe(1000 + Math.round(19.5 * TICKRATE));
  });

  it("pairs molotov start with expire, falls back to 7s", () => {
    const windows = utilityWindows(
      [nade("molotov_start", 500, 10, 10), nade("molotov_expire", 800, 12, 10), nade("molotov_start", 2000)],
      TICKRATE,
    );
    const paired = windows.find((w) => w.startTick === 500)!;
    const unpaired = windows.find((w) => w.startTick === 2000)!;
    expect(paired.kind).toBe("molly");
    expect(paired.endTick).toBe(800);
    expect(unpaired.endTick).toBe(2000 + Math.round(7 * TICKRATE));
  });

  it("flash and he are brief pops", () => {
    const windows = utilityWindows(
      [nade("flashbang", 100), nade("he", 200)],
      TICKRATE,
    );
    expect(windows.find((w) => w.kind === "flash")!.endTick).toBe(100 + 32);
    expect(windows.find((w) => w.kind === "he")!.endTick).toBe(200 + 32);
  });

  it("ignores expiry-only kinds as windows", () => {
    const windows = utilityWindows(
      [nade("smoke_expired", 1800), nade("molotov_expire", 900)],
      TICKRATE,
    );
    expect(windows).toHaveLength(0);
  });
});
