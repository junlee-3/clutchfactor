// Pure math and naming for "Export clip" — what slice of the round gets
// recorded, what the file is called, which container this WebView can write,
// and how far along the recording is. No DOM, no MediaRecorder: the recorder
// module owns those (recorder.ts), this owns the numbers so they are
// unit-testable without a browser.

import { mapName } from "../lib/mapName";
import { overlayWindow } from "./rail";
import { fmtClock, type TimelineSpec } from "./timeline";

export interface ClipWindow {
  startTick: number;
  endTick: number;
}

/** Pre-/post-roll around the playhead when no death moment is active —
 *  slightly wider than a moment's own overlay window because there is no
 *  known beat to centre on. */
const PLAYHEAD_PRE_S = 6;
const PLAYHEAD_POST_S = 3;

/** A clip shorter than this is not worth writing to disk. */
const MIN_CLIP_S = 1;

/** The tick span to record. Centred on the active tracked-death moment
 *  (`overlayWindow`'s -5s/+2s, so the exported clip frames the play exactly
 *  the way the canvas annotation does) or, with no moment, on the playhead.
 *  Always inside `spec`; never shorter than 1 s unless the round itself is,
 *  in which case the whole round is the clip. */
export function clipWindow(
  spec: TimelineSpec,
  playheadTick: number,
  momentTick: number | null,
  tickrate: number,
): ClipWindow {
  const base =
    momentTick !== null
      ? overlayWindow(momentTick, tickrate)
      : {
          start: playheadTick - Math.round(PLAYHEAD_PRE_S * tickrate),
          end: playheadTick + Math.round(PLAYHEAD_POST_S * tickrate),
        };

  let startTick = Math.round(Math.max(base.start, spec.startTick));
  let endTick = Math.round(Math.min(base.end, spec.endTick));

  // Clamping can leave a sliver (a moment at the very edge of the round).
  // Grow away from the edge that clipped it, then back the other way.
  const floor = Math.round(MIN_CLIP_S * tickrate);
  if (endTick - startTick < floor) {
    endTick = Math.min(startTick + floor, spec.endTick);
    if (endTick - startTick < floor) {
      startTick = Math.max(endTick - floor, spec.startTick);
    }
  }
  return { startTick, endTick };
}

/** Lowercase, hyphen-joined, alphanumerics only — the filename alphabet the
 *  Rust side accepts (`save_clip`). */
function slug(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/** "mirage-r12-0m34s.mp4" — map, round, and the clip's start as the round
 *  clock the scrubber shows, so a file on disk can be found again in the
 *  replay. Filesystem-safe by construction: `[a-z0-9-]+` and one dot. */
export function clipFileName(
  map: string,
  round: number,
  spec: TimelineSpec,
  win: ClipWindow,
  tickrate: number,
  ext: string,
): string {
  const name = slug(mapName(map)) || "map";
  const [minutes, seconds] = fmtClock(spec, win.startTick, tickrate).split(":");
  return `${name}-r${Math.trunc(round)}-${minutes}m${seconds}s.${slug(ext) || "webm"}`;
}

/** Container preference: mp4 first (WKWebView writes it, and every editor
 *  reads it), then the webm ladder WebView2 offers. */
const MIME_CANDIDATES: { mime: string; ext: "mp4" | "webm" }[] = [
  { mime: "video/mp4;codecs=avc1", ext: "mp4" },
  { mime: "video/webm;codecs=vp9", ext: "webm" },
  { mime: "video/webm;codecs=vp8", ext: "webm" },
  { mime: "video/webm", ext: "webm" },
];

/** The first candidate this WebView can record, or null when it can record
 *  none — the caller disables the button rather than failing mid-click. */
export function pickMimeType(
  isTypeSupported: (type: string) => boolean,
): { mime: string; ext: "mp4" | "webm" } | null {
  for (const candidate of MIME_CANDIDATES) {
    if (isTypeSupported(candidate.mime)) return { ...candidate };
  }
  return null;
}

/** Seconds recorded / seconds to record, for the button's live label. */
export function clipProgress(
  win: ClipWindow,
  tick: number,
  tickrate: number,
): { done: number; total: number } {
  const span = Math.max(0, win.endTick - win.startTick);
  const elapsed = Math.min(Math.max(tick - win.startTick, 0), span);
  return { done: elapsed / tickrate, total: span / tickrate };
}

/** The recording button's own label, counting in tenths. `.rpl-clip-btn`
 *  holds a fixed width so the transport never reflows as it ticks. */
export function recordingLabel(
  win: ClipWindow,
  tick: number,
  tickrate: number,
): string {
  const { done, total } = clipProgress(win, tick, tickrate);
  return `Recording ${done.toFixed(1)} s / ${total.toFixed(1)} s`;
}

/** The same sentence at whole-second granularity, for the polite live
 *  region: the string only changes once a second, so a 7 s clip is
 *  announced about seven times instead of seventy. The total rounds up so
 *  the announcement never promises a shorter clip than is being recorded. */
export function recordingAnnouncement(
  win: ClipWindow,
  tick: number,
  tickrate: number,
): string {
  const { done, total } = clipProgress(win, tick, tickrate);
  return `Recording ${Math.floor(done)} s / ${Math.ceil(total)} s`;
}
