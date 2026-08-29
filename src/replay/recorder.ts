// The DOM half of "Export clip": mirror the live replay canvas into an
// offscreen 1024² canvas, stamp the clip's own caption on it, and let
// MediaRecorder encode that at 1× while the round plays. Deliberately thin —
// every decision worth testing (window, file name, container, progress)
// lives in clip.ts.

import { getToken, rgba } from "../lib/theme";
import { pickMimeType } from "./clip";

/** Export resolution. The on-screen canvas is 1024·dpr (up to 2048²); the
 *  file is a fixed 1024² regardless of the display it was recorded on, so a
 *  clip is the same size and cost on every machine. */
const EXPORT_W = 1024;

/** Frames per second handed to `captureStream` — the replay's own rAF loop
 *  is what actually paces the frames. */
const CAPTURE_FPS = 30;

/** ~4 Mbps at 1024²/30 fps: clean radar lines without a 100 MB file. */
const BITRATE = 4_000_000;

/** Caption band along the bottom edge: solid, translucent, tokens only —
 *  a clip leaving the app still says which map, round and player it is. */
const BAND_H = 30;
const BAND_PAD = 12;
const SANS_STACK =
  'system-ui, -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif';

export interface ClipRecorder {
  /** Stops recording and resolves with the encoded video. */
  stop(): Promise<Blob>;
  /** Stops recording and throws the bytes away. */
  cancel(): void;
}

/** The container this WebView can record, or null when it cannot record at
 *  all — the button is then disabled rather than failing under the click. */
export function supportedMime(): { mime: string; ext: "mp4" | "webm" } | null {
  if (typeof MediaRecorder === "undefined") return null;
  if (typeof MediaRecorder.isTypeSupported !== "function") return null;
  if (typeof HTMLCanvasElement === "undefined") return null;
  if (typeof HTMLCanvasElement.prototype.captureStream !== "function") return null;
  return pickMimeType((type) => MediaRecorder.isTypeSupported(type));
}

/** Starts recording `source` right now. Call this from a click (or the `E`
 *  hotkey) only — never from an effect, which StrictMode would run twice and
 *  leave a second recorder running against a canvas nobody stops. */
export function startClipRecorder(
  source: HTMLCanvasElement,
  mime: string,
  label: string,
): ClipRecorder {
  const target = document.createElement("canvas");
  target.width = EXPORT_W;
  target.height = EXPORT_W;
  const ctx = target.getContext("2d");
  if (!ctx) throw new Error("this WebView wouldn't give the recorder a canvas");

  const stream = target.captureStream(CAPTURE_FPS);
  let media: MediaRecorder;
  try {
    media = new MediaRecorder(stream, {
      mimeType: mime,
      videoBitsPerSecond: BITRATE,
    });
  } catch (e) {
    // The container was supported a moment ago but isn't now: let go of the
    // capture stream before the error leaves here.
    for (const track of stream.getTracks()) track.stop();
    throw e;
  }
  const chunks: Blob[] = [];
  media.ondataavailable = (e) => {
    if (e.data.size > 0) chunks.push(e.data);
  };

  let raf = 0;
  const paint = () => {
    ctx.drawImage(source, 0, 0, EXPORT_W, EXPORT_W);
    ctx.fillStyle = rgba("--bg-tape", 0.78);
    ctx.fillRect(0, EXPORT_W - BAND_H, EXPORT_W, BAND_H);
    ctx.fillStyle = getToken("--line");
    ctx.fillRect(0, EXPORT_W - BAND_H, EXPORT_W, 1);
    ctx.fillStyle = getToken("--ink");
    ctx.font = `13px ${SANS_STACK}`;
    ctx.textBaseline = "middle";
    ctx.fillText(label, BAND_PAD, EXPORT_W - BAND_H / 2);
    raf = requestAnimationFrame(paint);
  };

  const teardown = () => {
    cancelAnimationFrame(raf);
    for (const track of stream.getTracks()) track.stop();
  };
  // Until stop()/cancel() take it over: an encoder that dies on its own must
  // still take the mirror loop down with it, or it paints for the rest of
  // the session.
  media.onerror = teardown;

  let settled = false;
  try {
    media.start();
  } catch (e) {
    teardown();
    throw e;
  }
  // Armed only once the encoder is actually running, so a throwing start()
  // can never leave a loop behind.
  raf = requestAnimationFrame(paint);

  return {
    stop() {
      return new Promise<Blob>((resolve, reject) => {
        if (settled) {
          reject(new Error("that recording already stopped"));
          return;
        }
        settled = true;
        media.onstop = () => {
          teardown();
          resolve(new Blob(chunks, { type: chunks[0]?.type || mime }));
        };
        media.onerror = () => {
          teardown();
          reject(new Error("the recording stopped early — nothing was saved"));
        };
        if (media.state === "inactive") {
          // It already stopped on its own (an encoder error) — there is no
          // onstop coming, so settle here instead of throwing inside the
          // executor and leaving the loop painting.
          teardown();
          reject(new Error("the recording stopped early — nothing was saved"));
          return;
        }
        media.stop();
      });
    },
    cancel() {
      if (settled) return;
      settled = true;
      media.onstop = teardown;
      media.onerror = teardown;
      if (media.state !== "inactive") media.stop();
      else teardown();
    },
  };
}
