import { describe, expect, it } from "vitest";
import { radarImageUrl, radarLayer, worldToRadar } from "./coords";
import type { MapCalibration } from "./coords";

// Real values from assets/maps/map-data.json
const mirage: MapCalibration = {
  pos_x: -3230,
  pos_y: 1713,
  scale: 5.0,
  lower_level_max_units: -1000000.0,
};
const nuke: MapCalibration = {
  pos_x: -3453,
  pos_y: 2887,
  scale: 7.0,
  lower_level_max_units: -495.0,
};

describe("worldToRadar", () => {
  it("maps the calibration origin to the top-left corner", () => {
    const { u, v } = worldToRadar(mirage, -3230, 1713);
    expect(u).toBe(0);
    expect(v).toBe(0);
  });

  it("maps a known mirage point with y-axis inversion", () => {
    // World (−230, −787): u = (−230 − (−3230))/5 = 600; v = (1713 − (−787))/5 = 500
    const { u, v } = worldToRadar(mirage, -230, -787);
    expect(u).toBeCloseTo(600);
    expect(v).toBeCloseTo(500);
  });

  it("moving north (+y) decreases v", () => {
    const a = worldToRadar(mirage, 0, 0);
    const b = worldToRadar(mirage, 0, 100);
    expect(b.v).toBeLessThan(a.v);
    expect(b.u).toBe(a.u);
  });
});

describe("radarLayer", () => {
  it("uses lower layer below the threshold on nuke", () => {
    expect(radarLayer(nuke, -600)).toBe("lower");
    expect(radarLayer(nuke, -495.0)).toBe("upper"); // boundary: strictly below
    expect(radarLayer(nuke, 0)).toBe("upper");
  });

  it("single-level maps are always upper", () => {
    expect(radarLayer(mirage, -5000)).toBe("upper");
  });
});

describe("radarImageUrl", () => {
  it("builds layer-specific urls", () => {
    expect(radarImageUrl("de_nuke", "upper")).toBe("/maps/de_nuke.png");
    expect(radarImageUrl("de_nuke", "lower")).toBe("/maps/de_nuke_lower.png");
  });
});
