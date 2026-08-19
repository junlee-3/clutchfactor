// Utility lifetime windows from grenade events (M2 plan constraints):
// smoke detonate → paired smoke_expired (≤25 s, ≤150 u, nearest, consumed
// once) else +19.5 s; molotov_start → molotov_expire (same rule) else +7 s;
// flash/he are 0.5 s visual pops.

import type { GrenadeInfo } from "../lib/ipc";

export interface UtilityWindow {
  kind: "smoke" | "molly" | "flash" | "he";
  x: number;
  y: number;
  z: number;
  startTick: number;
  endTick: number;
}

const PAIR_MAX_SECONDS = 25;
const PAIR_MAX_UNITS = 150;
const SMOKE_FALLBACK_S = 19.5;
const MOLLY_FALLBACK_S = 7;
const POP_S = 0.5;

function pairWindows(
  starts: GrenadeInfo[],
  ends: GrenadeInfo[],
  kind: "smoke" | "molly",
  fallbackS: number,
  tickrate: number,
): UtilityWindow[] {
  const used = new Set<number>();
  // For each expiry, find the nearest eligible start; ties go to the closer
  // one, so process expiries in tick order and pick min distance.
  const matchedEnd = new Map<GrenadeInfo, GrenadeInfo>();
  for (const end of ends) {
    let best: GrenadeInfo | null = null;
    let bestDist = Infinity;
    for (let i = 0; i < starts.length; i++) {
      if (used.has(i)) continue;
      const s = starts[i];
      if (end.tick < s.tick) continue;
      if (end.tick - s.tick > PAIR_MAX_SECONDS * tickrate) continue;
      const dist = Math.hypot(end.x - s.x, end.y - s.y);
      if (dist > PAIR_MAX_UNITS) continue;
      if (dist < bestDist) {
        bestDist = dist;
        best = s;
      }
    }
    if (best) {
      used.add(starts.indexOf(best));
      matchedEnd.set(best, end);
    }
  }
  return starts.map((s) => {
    const end = matchedEnd.get(s);
    return {
      kind,
      x: s.x,
      y: s.y,
      z: s.z,
      startTick: s.tick,
      endTick: end ? end.tick : s.tick + Math.round(fallbackS * tickrate),
    };
  });
}

export function utilityWindows(
  grenades: GrenadeInfo[],
  tickrate: number,
): UtilityWindow[] {
  const of = (kind: string) => grenades.filter((g) => g.kind === kind);
  const windows: UtilityWindow[] = [
    ...pairWindows(of("smoke"), of("smoke_expired"), "smoke", SMOKE_FALLBACK_S, tickrate),
    ...pairWindows(of("molotov_start"), of("molotov_expire"), "molly", MOLLY_FALLBACK_S, tickrate),
  ];
  for (const g of of("flashbang")) {
    windows.push({
      kind: "flash",
      x: g.x,
      y: g.y,
      z: g.z,
      startTick: g.tick,
      endTick: g.tick + Math.round(POP_S * tickrate),
    });
  }
  for (const g of of("he")) {
    windows.push({
      kind: "he",
      x: g.x,
      y: g.y,
      z: g.z,
      startTick: g.tick,
      endTick: g.tick + Math.round(POP_S * tickrate),
    });
  }
  windows.sort((a, b) => a.startTick - b.startTick);
  return windows;
}
