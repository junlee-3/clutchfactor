import { nextIndex, type ClipEntry } from "./clips";

/** Matches the `.hero__video` opacity transition in site.css. */
export const CROSSFADE_MS = 1000;

/** Two <video> elements take turns. The live one plays to `ended`; the other is
 *  armed with the next clip only after the live one is actually `playing`, and
 *  only after the previous crossfade has finished — so one clip is in flight
 *  at a time and the outgoing frame is never blanked mid-fade. On `ended` the
 *  armed element plays and becomes live (CSS crossfades on `.is-live`). Any
 *  `error` or rejected `play()` clears both elements; the poster underneath
 *  stays. Nothing is logged. */
export function initHero(root: HTMLElement, clips: ClipEntry[]): void {
  const videos = Array.from(root.querySelectorAll<HTMLVideoElement>(".hero__video"));
  if (videos.length !== 2 || clips.length === 0) return;

  const src = (i: number) => `/clips/${clips[i].file}`;
  let failed = false;
  const fail = () => {
    failed = true;
    for (const v of videos) {
      v.classList.remove("is-live");
      v.removeAttribute("src");
      v.load();
    }
  };
  const arm = (v: HTMLVideoElement, i: number) => {
    v.setAttribute("preload", "auto");
    v.src = src(i);
    v.load();
  };
  const play = (v: HTMLVideoElement) => {
    void v.play().catch(fail);
  };

  if (clips.length === 1) {
    const v = videos[0];
    v.loop = true;
    v.addEventListener("error", fail);
    v.addEventListener("playing", () => v.classList.add("is-live"), { once: true });
    arm(v, 0);
    play(v);
    return;
  }

  let live = 0; // index into videos
  let clip = 0; // index into clips — the clip the live video is showing

  for (const v of videos) {
    v.addEventListener("error", fail);
    v.addEventListener("playing", () => {
      if (failed || v !== videos[live]) return;
      v.classList.add("is-live");
      // Arm the other element only once this one is on screen and any
      // crossfade has finished (the other element may still be fading out).
      setTimeout(() => {
        if (!failed) arm(videos[1 - live], nextIndex(clip, clips.length));
      }, CROSSFADE_MS);
    });
    v.addEventListener("ended", () => {
      if (failed || v !== videos[live]) return;
      v.classList.remove("is-live");
      live = 1 - live;
      clip = nextIndex(clip, clips.length);
      play(videos[live]);
    });
  }

  arm(videos[0], 0);
  play(videos[0]);
}
