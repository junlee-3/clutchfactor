import { describe, expect, it } from "vitest";
import type { RoundTicks } from "../lib/ipc";
import { buildTracks, stateAt } from "./interp";

/** Two players (a, b); a moves 0→100 on x between ticks 100→200, yaw 350→10. */
function fixture(): RoundTicks {
  return {
    tick: [100, 100, 200, 200],
    steamid: ["a", "b", "a", "b"],
    x: [0, 50, 100, 50],
    y: [0, 5, 0, 5],
    z: [0, 0, 0, 0],
    yaw: [350, 90, 10, 90],
    health: [100, 80, 40, 80],
    is_alive: [true, true, true, false],
    team_num: [3, 2, 3, 2],
    active_weapon: ["weapon_ak47", "weapon_glock", "weapon_awp", null],
    last_place: ["TSpawn", null, "Mid", null],
  };
}

describe("buildTracks", () => {
  it("splits columnar data into per-player tracks sorted by tick", () => {
    const tracks = buildTracks(fixture());
    expect(tracks.map((t) => t.steamid).sort()).toEqual(["a", "b"]);
    const a = tracks.find((t) => t.steamid === "a")!;
    expect(a.ticks).toEqual([100, 200]);
    expect(a.x).toEqual([0, 100]);
  });
});

describe("stateAt", () => {
  const tracks = buildTracks(fixture());
  const a = tracks.find((t) => t.steamid === "a")!;
  const b = tracks.find((t) => t.steamid === "b")!;

  it("returns exact sample values on a sample tick", () => {
    const s = stateAt(a, 100)!;
    expect(s.x).toBe(0);
    expect(s.health).toBe(100);
    expect(s.weapon).toBe("weapon_ak47");
  });

  it("lerps position mid-gap", () => {
    const s = stateAt(a, 150)!;
    expect(s.x).toBeCloseTo(50);
    expect(s.y).toBeCloseTo(0);
  });

  it("interpolates yaw along the shortest arc through 0", () => {
    // 350° → 10° should pass through 0°, so at midpoint yaw = 0 (mod 360)
    const s = stateAt(a, 150)!;
    expect(((s.yaw % 360) + 360) % 360).toBeCloseTo(0);
  });

  it("steps (not lerps) discrete fields from the previous sample", () => {
    const s = stateAt(a, 150)!;
    expect(s.health).toBe(100); // not 70
    expect(s.weapon).toBe("weapon_ak47");
    expect(s.place).toBe("TSpawn");
  });

  it("returns null outside the sampled range", () => {
    expect(stateAt(a, 50)).toBeNull();
    expect(stateAt(a, 250)).toBeNull();
  });

  it("keeps a dead player's last state with isAlive false", () => {
    const s = stateAt(b, 200)!;
    expect(s.isAlive).toBe(false);
    expect(s.x).toBe(50);
  });
});
