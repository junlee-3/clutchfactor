// World → radar image coordinate mapping (PROMPT.md §6.3, ADR-0004).
// Calibration values come from /maps/map-data.json (awpy artifact).

export interface MapCalibration {
  pos_x: number;
  pos_y: number;
  scale: number;
  lower_level_max_units: number;
}

/** Maps world (x, y) to 1024×1024 radar image pixels. +y (north) is up → smaller v. */
export function worldToRadar(
  cal: MapCalibration,
  x: number,
  y: number,
): { u: number; v: number } {
  return {
    u: (x - cal.pos_x) / cal.scale,
    v: (cal.pos_y - y) / cal.scale,
  };
}

export function radarLayer(cal: MapCalibration, z: number): "upper" | "lower" {
  return z < cal.lower_level_max_units ? "lower" : "upper";
}

export function radarImageUrl(map: string, layer: "upper" | "lower"): string {
  return layer === "lower" ? `/maps/${map}_lower.png` : `/maps/${map}.png`;
}

/** CS2 competitive-queue scenic map screenshot (not the radar). */
export function mapPreviewImageUrl(map: string): string {
  return `/maps/previews/${map}.png`;
}
