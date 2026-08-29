import { describe, expect, it } from "vitest";
import { clipFileName, clipProgress, clipWindow, pickMimeType } from "./clip";
import type { TimelineSpec } from "./timeline";

const TR = 64;
const SPEC: TimelineSpec = { startTick: 1000, endTick: 8000 };

describe("clipWindow", () => {
  it("uses the moment's -5s/+2s overlay window when a death moment is active", () => {
    expect(clipWindow(SPEC, 4000, 5000, TR)).toEqual({
      startTick: 5000 - 5 * TR,
      endTick: 5000 + 2 * TR,
    });
  });

  it("uses -6s/+3s around the playhead when no moment is active", () => {
    expect(clipWindow(SPEC, 4000, null, TR)).toEqual({
      startTick: 4000 - 6 * TR,
      endTick: 4000 + 3 * TR,
    });
  });

  it("clamps the start to the round start", () => {
    expect(clipWindow(SPEC, 1100, null, TR).startTick).toBe(SPEC.startTick);
  });

  it("clamps the end to the round end", () => {
    expect(clipWindow(SPEC, 7900, null, TR).endTick).toBe(SPEC.endTick);
  });

  it("extends forward when clamping at the round start leaves under a second", () => {
    // Moment 100 ticks before the round start: only its +2s tail overlaps.
    const win = clipWindow(SPEC, 1000, SPEC.startTick - 100, TR);
    expect(win.startTick).toBe(SPEC.startTick);
    expect(win.endTick - win.startTick).toBe(TR);
  });

  it("extends backward when clamping at the round end leaves under a second", () => {
    // Moment past the round end: only its -5s head overlaps, by 20 ticks.
    const win = clipWindow(SPEC, 8000, SPEC.endTick + 5 * TR - 20, TR);
    expect(win.endTick).toBe(SPEC.endTick);
    expect(win.endTick - win.startTick).toBe(TR);
  });

  it("never leaves the round when the round itself is under a second", () => {
    const tiny: TimelineSpec = { startTick: 1000, endTick: 1040 };
    expect(clipWindow(tiny, 1020, null, TR)).toEqual({
      startTick: 1000,
      endTick: 1040,
    });
  });

  it("returns whole ticks for a fractional playhead", () => {
    const win = clipWindow(SPEC, 4000.7, null, TR);
    expect(Number.isInteger(win.startTick)).toBe(true);
    expect(Number.isInteger(win.endTick)).toBe(true);
  });
});

describe("clipFileName", () => {
  it("names the file map-round-clock.ext", () => {
    const win = { startTick: SPEC.startTick + 34 * TR, endTick: SPEC.endTick };
    expect(clipFileName("de_mirage", 12, SPEC, win, TR, "mp4")).toBe(
      "mirage-r12-0m34s.mp4",
    );
  });

  it("pads the seconds and carries the minute past 0:59", () => {
    const win = { startTick: SPEC.startTick + 63 * TR, endTick: SPEC.endTick };
    expect(clipFileName("de_dust2", 3, SPEC, win, TR, "webm")).toBe(
      "dust2-r3-1m03s.webm",
    );
  });

  it("keeps the name filesystem-safe for an unusual map slug", () => {
    const win = { startTick: SPEC.startTick, endTick: SPEC.endTick };
    const name = clipFileName("de_Weird Map!", 1, SPEC, win, TR, "mp4");
    expect(name).toMatch(/^[a-z0-9-]+\.[a-z0-9]+$/);
    expect(name).toBe("weird-map-r1-0m00s.mp4");
  });
});

describe("pickMimeType", () => {
  it("prefers mp4/avc1 when the WebView supports it", () => {
    expect(pickMimeType(() => true)).toEqual({
      mime: "video/mp4;codecs=avc1",
      ext: "mp4",
    });
  });

  it("falls back to webm/vp9 when mp4 is unsupported", () => {
    expect(pickMimeType((t) => !t.startsWith("video/mp4"))).toEqual({
      mime: "video/webm;codecs=vp9",
      ext: "webm",
    });
  });

  it("falls back to webm/vp8 when neither mp4 nor vp9 is supported", () => {
    expect(pickMimeType((t) => t === "video/webm;codecs=vp8")).toEqual({
      mime: "video/webm;codecs=vp8",
      ext: "webm",
    });
  });

  it("falls back to bare webm as the last candidate", () => {
    expect(pickMimeType((t) => t === "video/webm")).toEqual({
      mime: "video/webm",
      ext: "webm",
    });
  });

  it("returns null when the WebView supports none of them", () => {
    expect(pickMimeType(() => false)).toBeNull();
  });
});

describe("clipProgress", () => {
  const win = { startTick: 1000, endTick: 1000 + 7 * TR };

  it("reports zero done and the window length at the start", () => {
    expect(clipProgress(win, 1000, TR)).toEqual({ done: 0, total: 7 });
  });

  it("reports seconds elapsed mid-clip", () => {
    expect(clipProgress(win, 1000 + 3 * TR, TR)).toEqual({ done: 3, total: 7 });
  });

  it("clamps done to the total past the end of the window", () => {
    expect(clipProgress(win, 1000 + 99 * TR, TR)).toEqual({ done: 7, total: 7 });
  });

  it("never reports negative progress before the window starts", () => {
    expect(clipProgress(win, 900, TR).done).toBe(0);
  });
});
