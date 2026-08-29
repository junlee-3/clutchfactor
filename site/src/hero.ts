import { nextIndex, type ClipEntry } from "./clips";

/** Two <video> elements take turns: the live one plays to `ended`, the other
 *  has the next clip preloaded; then a 1 s opacity crossfade (CSS) and swap.
 *  Any error on any clip → poster (the CSS fallback already underneath). */
export function initHero(root: HTMLElement, clips: ClipEntry[]): void {
  const videos = Array.from(root.querySelectorAll<HTMLVideoElement>(".hero__video"));
  if (videos.length !== 2 || clips.length === 0) return;

  const src = (i: number) => `/clips/${clips[i].file}`;
  const fail = () => {
    for (const v of videos) {
      v.classList.remove("is-live");
      v.removeAttribute("src");
    }
    root.classList.remove("hero--video");
  };

  if (clips.length === 1) {
    const v = videos[0];
    v.loop = true;
    v.src = src(0);
    v.addEventListener("error", fail, { once: true });
    v.addEventListener("playing", () => v.classList.add("is-live"), { once: true });
    root.classList.add("hero--video");
    void v.play().catch(fail);
    return;
  }

  let live = 0; // index into videos
  let clip = 0; // index into clips

  const arm = (v: HTMLVideoElement, i: number) => {
    v.src = src(i);
    v.preload = "auto";
    v.load();
  };

  const swap = () => {
    const next = 1 - live;
    const v = videos[next];
    v.classList.add("is-live");
    videos[live].classList.remove("is-live");
    live = next;
    clip = nextIndex(clip, clips.length);
    void v.play().catch(fail);
    arm(videos[1 - live], nextIndex(clip, clips.length));
  };

  for (const v of videos) {
    v.addEventListener("error", fail);
    v.addEventListener("ended", () => {
      if (v === videos[live]) swap();
    });
  }

  arm(videos[0], 0);
  arm(videos[1], nextIndex(0, clips.length));
  videos[0].addEventListener("playing", () => videos[0].classList.add("is-live"), { once: true });
  root.classList.add("hero--video");
  void videos[0].play().catch(fail);
}
