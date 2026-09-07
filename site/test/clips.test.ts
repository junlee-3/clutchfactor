import { describe, expect, it } from "vitest";
import { MIN_VIDEO_WIDTH, nextIndex, parseClipsManifest, shouldPlayVideo } from "../src/clips";

describe("parseClipsManifest", () => {
  it("accepts an array of { file: *.mp4 }", () => {
    expect(parseClipsManifest([{ file: "clip-01.mp4" }, { file: "clip-02.mp4" }])).toEqual([
      { file: "clip-01.mp4" },
      { file: "clip-02.mp4" },
    ]);
  });
  it("accepts an empty array (no clips yet)", () => {
    expect(parseClipsManifest([])).toEqual([]);
  });
  it("rejects anything else", () => {
    expect(parseClipsManifest(null)).toBeNull();
    expect(parseClipsManifest({ file: "clip-01.mp4" })).toBeNull();
    expect(parseClipsManifest([{ file: "clip-01.webm" }])).toBeNull();
    expect(parseClipsManifest([{ src: "clip-01.mp4" }])).toBeNull();
    expect(parseClipsManifest([{ file: "../clip-01.mp4" }])).toBeNull();
  });
});

describe("shouldPlayVideo", () => {
  const ok = { reducedMotion: false, saveData: false, viewportWidth: 1440, clips: [{ file: "clip-01.mp4" }] };
  it("plays on a normal desktop with clips", () => expect(shouldPlayVideo(ok)).toBe(true));
  it("never under reduced motion", () => expect(shouldPlayVideo({ ...ok, reducedMotion: true })).toBe(false));
  it("never with Save-Data", () => expect(shouldPlayVideo({ ...ok, saveData: true })).toBe(false));
  it(`never below ${MIN_VIDEO_WIDTH}px`, () => {
    expect(shouldPlayVideo({ ...ok, viewportWidth: MIN_VIDEO_WIDTH - 1 })).toBe(false);
    expect(shouldPlayVideo({ ...ok, viewportWidth: MIN_VIDEO_WIDTH })).toBe(true);
  });
  it("never without clips", () => expect(shouldPlayVideo({ ...ok, clips: [] })).toBe(false));
});

describe("nextIndex", () => {
  it("wraps", () => {
    expect(nextIndex(0, 3)).toBe(1);
    expect(nextIndex(2, 3)).toBe(0);
    expect(nextIndex(0, 1)).toBe(0);
  });
});
