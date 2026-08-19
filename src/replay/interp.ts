// Per-player interpolation over the 16 Hz tick table (ADR-0002).
// Positions/yaw lerp between samples; discrete fields (health, weapon,
// place, isAlive, team) step from the previous sample.

import type { RoundTicks } from "../lib/ipc";

export interface PlayerTrack {
  steamid: string;
  ticks: number[];
  x: number[];
  y: number[];
  z: number[];
  yaw: number[];
  health: number[];
  isAlive: boolean[];
  teamNum: number[];
  weapon: (string | null)[];
  place: (string | null)[];
}

export interface PlayerState {
  x: number;
  y: number;
  z: number;
  yaw: number;
  health: number;
  isAlive: boolean;
  teamNum: number;
  weapon: string | null;
  place: string | null;
}

export function buildTracks(rt: RoundTicks): PlayerTrack[] {
  const byPlayer = new Map<string, PlayerTrack>();
  for (let i = 0; i < rt.tick.length; i++) {
    const sid = rt.steamid[i];
    let t = byPlayer.get(sid);
    if (!t) {
      t = {
        steamid: sid,
        ticks: [],
        x: [],
        y: [],
        z: [],
        yaw: [],
        health: [],
        isAlive: [],
        teamNum: [],
        weapon: [],
        place: [],
      };
      byPlayer.set(sid, t);
    }
    t.ticks.push(rt.tick[i]);
    t.x.push(rt.x[i]);
    t.y.push(rt.y[i]);
    t.z.push(rt.z[i]);
    t.yaw.push(rt.yaw[i]);
    t.health.push(rt.health[i]);
    t.isAlive.push(rt.is_alive[i]);
    t.teamNum.push(rt.team_num[i]);
    t.weapon.push(rt.active_weapon[i]);
    t.place.push(rt.last_place[i]);
  }
  // Input is (tick, steamid)-sorted, so per-player ticks are already ascending.
  return [...byPlayer.values()];
}

/** Index of the last sample with tick <= target, or -1. */
function prevIndex(ticks: number[], tick: number): number {
  let lo = 0;
  let hi = ticks.length - 1;
  let ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (ticks[mid] <= tick) {
      ans = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return ans;
}

function lerpYaw(a: number, b: number, f: number): number {
  // Shortest arc: 350° → 10° passes through 0°, not 180°.
  const d = ((((b - a) % 360) + 540) % 360) - 180; // -180..180
  return a + d * f;
}

export function stateAt(track: PlayerTrack, tick: number): PlayerState | null {
  const n = track.ticks.length;
  if (n === 0 || tick < track.ticks[0] || tick > track.ticks[n - 1]) {
    return null;
  }
  const i = prevIndex(track.ticks, tick);
  const j = Math.min(i + 1, n - 1);
  const t0 = track.ticks[i];
  const t1 = track.ticks[j];
  const f = t1 > t0 ? (tick - t0) / (t1 - t0) : 0;
  return {
    x: track.x[i] + (track.x[j] - track.x[i]) * f,
    y: track.y[i] + (track.y[j] - track.y[i]) * f,
    z: track.z[i] + (track.z[j] - track.z[i]) * f,
    yaw: lerpYaw(track.yaw[i], track.yaw[j], f),
    health: track.health[i],
    isAlive: track.isAlive[i],
    teamNum: track.teamNum[i],
    weapon: track.weapon[i],
    place: track.place[i],
  };
}
