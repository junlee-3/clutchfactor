// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ClipEntry } from "../src/clips";
import { CROSSFADE_MS, initHero } from "../src/hero";

/** A hero root with two stubbed <video>s. jsdom/happy-dom have no media stack,
 *  so `play`/`load` are spies; events are dispatched by hand. */
function setup(clipCount: number, videoCount = 2) {
  document.body.innerHTML = `<section data-hero><div class="hero__media">${'<video class="hero__video"></video>'.repeat(videoCount)}</div></section>`;
  const root = document.querySelector<HTMLElement>("[data-hero]")!;
  const videos = Array.from(root.querySelectorAll<HTMLVideoElement>(".hero__video"));
  for (const v of videos) {
    v.play = vi.fn(() => Promise.resolve());
    v.load = vi.fn();
  }
  const clips: ClipEntry[] = Array.from({ length: clipCount }, (_, i) => ({ file: `clip-0${i + 1}.mp4` }));
  return { root, videos, clips, a: videos[0], b: videos[1] };
}

const src = (v: HTMLVideoElement) => v.getAttribute("src");
const live = (v: HTMLVideoElement) => v.classList.contains("is-live");
/** Let the `play()` promise's `.catch` reaction run. */
const flush = () => Promise.resolve();

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("initHero", () => {
  it("boots the first clip and arms the second only after the crossfade", () => {
    const { root, clips, a, b } = setup(3);
    initHero(root, clips);

    expect(src(a)).toBe("/clips/clip-01.mp4");
    expect(a.play).toHaveBeenCalledTimes(1);
    expect(src(b)).toBe(null);
    expect(b.load).not.toHaveBeenCalled();

    a.dispatchEvent(new Event("playing"));
    expect(live(a)).toBe(true);
    // still nothing armed — the crossfade has not finished
    expect(src(b)).toBe(null);

    vi.advanceTimersByTime(CROSSFADE_MS);
    expect(src(b)).toBe("/clips/clip-02.mp4");
    expect(b.getAttribute("preload")).toBe("auto");
    expect(b.load).toHaveBeenCalled();
  });

  it("rotates A → B → A and wraps back to the first clip", () => {
    const { root, clips, a, b } = setup(3);
    initHero(root, clips);
    a.dispatchEvent(new Event("playing"));
    vi.advanceTimersByTime(CROSSFADE_MS);

    a.dispatchEvent(new Event("ended"));
    expect(live(a)).toBe(false);
    expect(b.play).toHaveBeenCalledTimes(1);

    b.dispatchEvent(new Event("playing"));
    expect(live(b)).toBe(true);
    expect(src(a)).toBe("/clips/clip-01.mp4"); // not re-armed mid-fade
    vi.advanceTimersByTime(CROSSFADE_MS);
    expect(src(a)).toBe("/clips/clip-03.mp4");

    b.dispatchEvent(new Event("ended"));
    expect(live(b)).toBe(false);
    expect(a.play).toHaveBeenCalledTimes(2);

    a.dispatchEvent(new Event("playing"));
    vi.advanceTimersByTime(CROSSFADE_MS);
    expect(src(b)).toBe("/clips/clip-01.mp4");
  });

  it("ignores `ended` on the element that is not live", () => {
    const { root, clips, a, b } = setup(3);
    initHero(root, clips);
    a.dispatchEvent(new Event("playing"));
    vi.advanceTimersByTime(CROSSFADE_MS);

    b.dispatchEvent(new Event("ended"));
    expect(b.play).not.toHaveBeenCalled();
    expect(live(a)).toBe(true);
    expect(live(b)).toBe(false);
    expect(a.play).toHaveBeenCalledTimes(1);
  });

  it("loops the only clip on one element and never touches the other", () => {
    const { root, clips, a, b } = setup(1);
    initHero(root, clips);

    expect(a.loop).toBe(true);
    expect(src(a)).toBe("/clips/clip-01.mp4");
    expect(a.play).toHaveBeenCalledTimes(1);
    a.dispatchEvent(new Event("playing"));
    expect(live(a)).toBe(true);

    expect(src(b)).toBe(null);
    expect(b.load).not.toHaveBeenCalled();
    expect(b.play).not.toHaveBeenCalled();
  });

  it("clears both elements on an `error` and stops rotating", () => {
    const { root, clips, a, b } = setup(3);
    initHero(root, clips);
    a.dispatchEvent(new Event("playing"));
    vi.advanceTimersByTime(CROSSFADE_MS);

    b.dispatchEvent(new Event("error"));
    expect(src(a)).toBe(null);
    expect(src(b)).toBe(null);
    expect(live(a)).toBe(false);
    expect(live(b)).toBe(false);

    a.dispatchEvent(new Event("ended"));
    expect(b.play).not.toHaveBeenCalled();
    expect(a.play).toHaveBeenCalledTimes(1);
  });

  it("clears both elements when play() is rejected", async () => {
    const { root, clips, a, b } = setup(3);
    a.play = vi.fn(() => Promise.reject(new Error("blocked")));
    initHero(root, clips);
    await flush();

    expect(src(a)).toBe(null);
    expect(src(b)).toBe(null);
    expect(live(a)).toBe(false);
    expect(live(b)).toBe(false);

    a.dispatchEvent(new Event("ended"));
    expect(b.play).not.toHaveBeenCalled();
  });

  it("is a no-op with no clips or without exactly two videos", () => {
    const empty = setup(0);
    initHero(empty.root, empty.clips);
    expect(empty.a.play).not.toHaveBeenCalled();
    expect(src(empty.a)).toBe(null);

    const one = setup(3, 1);
    initHero(one.root, one.clips);
    expect(one.a.play).not.toHaveBeenCalled();
    expect(src(one.a)).toBe(null);

    const three = setup(3, 3);
    initHero(three.root, three.clips);
    expect(three.a.play).not.toHaveBeenCalled();
    expect(src(three.a)).toBe(null);
  });
});
