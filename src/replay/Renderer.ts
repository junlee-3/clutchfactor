// Canvas 2D scene renderer. Pure draw: everything it needs arrives in the
// Scene; no fetching, no React. Radar is the hero; chrome recedes (§7).

import type { KillInfo } from "../lib/ipc";
import { getToken, rgba } from "../lib/theme";
import { radarLayer, worldToRadar } from "./coords";
import type { MapCalibration } from "./coords";
import { stateAt } from "./interp";
import type { PlayerTrack } from "./interp";
import type { UtilityWindow } from "./utility";

// Snapshotted once at module import — fine for the dark-only theme (no
// runtime theme switch exists); a future light/dark toggle would need these
// re-read (or converted to getToken() calls at each use) instead.
export const CT_COLOR = getToken("--ct");
export const T_COLOR = getToken("--t");

// Game-world effect colors — molotov/incendiary fire and flash-white are
// properties of the game itself, not app chrome, so they stay fixed named
// constants rather than UI tokens (docs/design/design-system.md §2 & §9).
const FIRE_RGB = "224, 116, 60";
const FLASH_RGB = "255, 255, 255";

export interface BombState {
  plantTick: number;
  endTick: number; // defused/exploded/round officially over
  x: number;
  y: number;
}

export interface Scene {
  cal: MapCalibration;
  upperImage: HTMLImageElement | null;
  lowerImage: HTMLImageElement | null;
  tracks: PlayerTrack[];
  names: Map<string, string>;
  sides: Map<string, "CT" | "T">; // this round's side per steamid
  kills: KillInfo[]; // this round's kills (sorted by tick)
  killPositions: Map<KillInfo, { x: number; y: number; z: number }>;
  utility: UtilityWindow[];
  bomb: BombState | null;
  tick: number;
  tickrate: number;
  focus: Set<string>; // empty = no dimming
}

function sideColor(side: "CT" | "T" | undefined): string {
  return side === "T" ? T_COLOR : CT_COLOR;
}

/** Which radar layer to show: the one most alive players are on. */
export function activeLayer(scene: Scene): "upper" | "lower" {
  if (!scene.lowerImage) return "upper";
  let lower = 0;
  let upper = 0;
  for (const track of scene.tracks) {
    const s = stateAt(track, scene.tick);
    if (!s || !s.isAlive) continue;
    if (radarLayer(scene.cal, s.z) === "lower") lower++;
    else upper++;
  }
  return lower > upper ? "lower" : "upper";
}

export function draw(ctx: CanvasRenderingContext2D, scene: Scene): void {
  const layer = activeLayer(scene);
  const img = layer === "lower" ? scene.lowerImage : scene.upperImage;
  ctx.clearRect(0, 0, 1024, 1024);
  ctx.fillStyle = getToken("--bg-tape");
  ctx.fillRect(0, 0, 1024, 1024);
  if (img && img.complete && img.naturalWidth > 0) {
    ctx.globalAlpha = 0.9;
    ctx.drawImage(img, 0, 0, 1024, 1024);
    ctx.globalAlpha = 1;
  }

  drawUtility(ctx, scene);
  drawBomb(ctx, scene);
  drawDeaths(ctx, scene);
  drawPlayers(ctx, scene, layer);
}

function drawUtility(ctx: CanvasRenderingContext2D, scene: Scene): void {
  for (const u of scene.utility) {
    if (scene.tick < u.startTick || scene.tick > u.endTick) continue;
    const { u: px, v: py } = worldToRadar(scene.cal, u.x, u.y);
    const life =
      u.endTick > u.startTick
        ? 1 - (scene.tick - u.startTick) / (u.endTick - u.startTick)
        : 0;
    if (u.kind === "smoke") {
      const r = 144 / scene.cal.scale;
      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      ctx.fillStyle = rgba("--chalk-dim", 0.35); // smoke stays neutral grey, chalk-derived
      ctx.fill();
      // remaining-life ring
      ctx.beginPath();
      ctx.arc(px, py, r, -Math.PI / 2, -Math.PI / 2 + life * Math.PI * 2);
      ctx.strokeStyle = rgba("--chalk", 0.7);
      ctx.lineWidth = 2;
      ctx.stroke();
    } else if (u.kind === "molly") {
      const r = 120 / scene.cal.scale;
      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(${FIRE_RGB}, 0.35)`;
      ctx.fill();
      ctx.strokeStyle = `rgba(${FIRE_RGB}, 0.8)`;
      ctx.lineWidth = 1.5;
      ctx.stroke();
    } else {
      // flash / he: brief expanding pop
      const t = 1 - life;
      const r = 6 + t * 14;
      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      ctx.strokeStyle =
        u.kind === "flash"
          ? `rgba(${FLASH_RGB}, ${0.9 * life})`
          : `rgba(${FIRE_RGB}, ${0.9 * life})`;
      ctx.lineWidth = 2.5;
      ctx.stroke();
    }
  }
}

function drawBomb(ctx: CanvasRenderingContext2D, scene: Scene): void {
  const b = scene.bomb;
  if (!b || scene.tick < b.plantTick || scene.tick > b.endTick) return;
  const { u: px, v: py } = worldToRadar(scene.cal, b.x, b.y);
  const pulse = 0.5 + 0.5 * Math.sin((scene.tick / scene.tickrate) * Math.PI * 2);
  ctx.beginPath();
  ctx.arc(px, py, 6 + pulse * 3, 0, Math.PI * 2);
  ctx.fillStyle = rgba("--loss", 0.9); // bomb IS threat semantics, unlike fire
  ctx.fill();
  ctx.fillStyle = getToken("--bg-tape");
  ctx.font = "bold 8px ui-monospace, monospace";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("C4", px, py);
}

function drawDeaths(ctx: CanvasRenderingContext2D, scene: Scene): void {
  for (const k of scene.kills) {
    if (k.tick > scene.tick) continue;
    const pos = scene.killPositions.get(k);
    if (!pos) continue;
    const { u: px, v: py } = worldToRadar(scene.cal, pos.x, pos.y);
    const age = (scene.tick - k.tick) / scene.tickrate; // seconds
    const alpha = age < 3 ? 0.9 - (age / 3) * 0.55 : 0.35;
    const side = scene.sides.get(k.victim);
    ctx.strokeStyle = sideColor(side);
    ctx.globalAlpha = alpha;
    ctx.lineWidth = 2;
    const r = 5;
    ctx.beginPath();
    ctx.moveTo(px - r, py - r);
    ctx.lineTo(px + r, py + r);
    ctx.moveTo(px + r, py - r);
    ctx.lineTo(px - r, py + r);
    ctx.stroke();
    ctx.globalAlpha = 1;
  }
}

function drawPlayers(
  ctx: CanvasRenderingContext2D,
  scene: Scene,
  shownLayer: "upper" | "lower",
): void {
  for (const track of scene.tracks) {
    const s = stateAt(track, scene.tick);
    if (!s || !s.isAlive) continue;
    const { u: px, v: py } = worldToRadar(scene.cal, s.x, s.y);
    const side = scene.sides.get(track.steamid);
    const color = sideColor(side);
    const onShownLayer =
      !scene.lowerImage || radarLayer(scene.cal, s.z) === shownLayer;
    const focused =
      scene.focus.size === 0 || scene.focus.has(track.steamid);
    const alpha = (focused ? 1 : 0.35) * (onShownLayer ? 1 : 0.4);

    ctx.globalAlpha = alpha;
    // View direction wedge (world yaw 0° = +x/east; canvas v axis is flipped).
    const a = (-s.yaw * Math.PI) / 180;
    const spread = (30 * Math.PI) / 180 / 2;
    ctx.beginPath();
    ctx.moveTo(px, py);
    ctx.arc(px, py, 16, a - spread, a + spread);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.globalAlpha = alpha * 0.35;
    ctx.fill();

    ctx.globalAlpha = alpha;
    ctx.beginPath();
    ctx.arc(px, py, onShownLayer ? 7 : 5, 0, Math.PI * 2);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = getToken("--bg-tape"); // dark rim recedes — separation, not emphasis (§1/§7)
    ctx.lineWidth = 1.5;
    ctx.stroke();

    const name = scene.names.get(track.steamid) ?? "";
    if (name) {
      ctx.font = "10px -apple-system, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      ctx.fillStyle = rgba("--chalk", 0.9);
      ctx.fillText(name, px, py + 9);
    }
    ctx.globalAlpha = 1;
  }
}
