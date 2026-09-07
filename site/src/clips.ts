export interface ClipEntry {
  file: string;
}

export interface VideoEnv {
  reducedMotion: boolean;
  saveData: boolean;
  viewportWidth: number;
  clips: readonly ClipEntry[];
}

/** Below this viewport width the hero shows the poster only (spec §5). */
export const MIN_VIDEO_WIDTH = 720;

const FILE = /^[a-z0-9-]+\.mp4$/;

/** `clips.json` → entries, or null when the manifest is not what we expect
 *  (the caller falls back to the poster; nothing is logged). */
export function parseClipsManifest(json: unknown): ClipEntry[] | null {
  if (!Array.isArray(json)) return null;
  const out: ClipEntry[] = [];
  for (const item of json) {
    if (typeof item !== "object" || item === null) return null;
    const file = (item as { file?: unknown }).file;
    if (typeof file !== "string" || !FILE.test(file)) return null;
    out.push({ file });
  }
  return out;
}

export function shouldPlayVideo(env: VideoEnv): boolean {
  return !env.reducedMotion && !env.saveData && env.viewportWidth >= MIN_VIDEO_WIDTH && env.clips.length > 0;
}

export function nextIndex(i: number, n: number): number {
  return (i + 1) % n;
}
