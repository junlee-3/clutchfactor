# Marketing Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the ClutchFactor marketing page — `site/` in this repo, deployable to Vercel — implementing `docs/spec/marketing-site.md` (direction A "The Tape" + the C ledger section).

**Architecture:** A standalone Vite project under `site/` (own `package.json` + lockfile, no pnpm workspace) with vanilla TypeScript and plain CSS. Pure logic (platform detection, ledger timing, clip manifest/env rules, release constants) lives in small modules with vitest tests; `main.ts` is DOM glue. Screenshots are generated into `site/public/shots/` from `docs/screenshots/` by a sharp script and committed. Vercel builds with Root Directory `site`.

**Tech Stack:** Vite 7 · TypeScript 5.8 · Vitest 4 (+ happy-dom for two DOM tests) · ESLint 10 + typescript-eslint 8 · sharp (dev only) · Google Fonts (Inter, JetBrains Mono) · Vercel · GitHub Actions.

**Spec:** `docs/spec/marketing-site.md` (read it first; ADR-0012 records the decisions).

## Global Constraints

- Node 22, pnpm 10.33.2 (`packageManager` in root `package.json`); `site/` has its own `package.json` and `pnpm-lock.yaml`; **no `pnpm-workspace.yaml`** (ADR-0012 §3).
- Real content only: every headline, ledger row, insight card and stat is copied verbatim from spec §3 (which copied them from real screenshots). No lorem, no invented numbers.
- Single dark theme. Tokens in `site/src/styles/tokens.css` must equal the app's `src/styles/tokens.css` for every token except `--font-*` and `--text-*` (spec §2, §8).
- Accent (`--accent` #4aa3ff) fills exactly one element per screen: the primary download button. The nav button is secondary style.
- Dashed stroke = clickable evidence (chips are `<a>`); solid hairline = furniture.
- Motion budget (spec §4): clip crossfade 1000 ms, reveals 200 ms, ledger sequence, nav fade 200 ms — nothing else; all off under `prefers-reduced-motion: reduce`.
- Video is skipped (poster only) when: reduced motion, `navigator.connection.saveData`, viewport < 720 px, empty/invalid manifest, or a clip errors (spec §5).
- No analytics, cookies or third-party scripts. Fonts from Google Fonts only.
- Owner-only steps (`vercel login`, `vercel link`, recording clips) are **handed off, never automated**.
- Conventional commits; work on branch `feat/site` (rename the current `feat/site-spec`: `git branch -m feat/site`). PR to `main` at the end; `main` is ruleset-protected.
- Run every command from the repo root unless the step says otherwise; site commands are `pnpm -C site <script>`.

---

## File map

| Path | Responsibility |
|---|---|
| `site/package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `vite.config.ts`, `eslint.config.js` | standalone toolchain |
| `site/index.html` | the whole page: semantic sections + final copy (spec §3) |
| `site/src/main.ts` | boot: imports CSS, marks `html.js`, wires hero, ledger, CTA, reveals, nav |
| `site/src/release.ts` | version, asset file names, byte sizes, `assetUrl`, `formatMb` — the ONE place |
| `site/src/platform.ts` | `detectPlatform(ua, maxTouchPoints)` → `"mac" \| "windows" \| "other"` |
| `site/src/cta.ts` | `renderDownloadButtons(doc, platform)`, `applyNavDownload(doc, platform)` |
| `site/src/ledger.ts` | `parseClock`, `ledgerSchedule(rows)` → delays (ms) |
| `site/src/clips.ts` | `parseClipsManifest`, `shouldPlayVideo(env)`, `nextIndex` |
| `site/src/hero.ts` | `initHero(root, clips)`: two `<video>`s, crossfade rotation, poster fallback |
| `site/src/styles/tokens.css` | copy of the app's tokens (drift-tested) |
| `site/src/styles/site.css` | type scale, components, sections, responsive, reduced motion |
| `site/scripts/shots.mjs` | sharp: screenshots → `public/shots/*.webp`, radar fallback, `og.jpg` |
| `site/public/clips/README.md`, `clips.json` | clip spec + manifest (empty until the owner records) |
| `site/public/favicon.svg` | copy of `app-icon.svg` |
| `site/vercel.json` | cleanUrls, cache + security headers |
| `site/test/*.test.ts` | vitest: platform, release, ledger, clips, cta (happy-dom), html structure, tokens drift |
| `.github/workflows/ci.yml` | new `site` job; `secrets` job also ignores `site/pnpm-lock.yaml` |
| `eslint.config.js` (root) | ignore `site/` |
| `CLAUDE.md`, `docs/PROGRESS.md` | map line, dev command, release checklist step, state |
| `docs/design/walkthrough-site/` | sign-off renders + README |

---

### Task 1: Scaffold the standalone `site/` package

**Files:**
- Create: `site/package.json`, `site/tsconfig.json`, `site/vite.config.ts`, `site/eslint.config.js`, `site/index.html` (temporary stub), `site/src/main.ts` (stub), `site/test/smoke.test.ts`
- Modify: `eslint.config.js:6` (root ignores)

**Interfaces:**
- Produces: the scripts every later task runs — `pnpm -C site typecheck | lint | test:run | build | dev | preview | shots`.

- [ ] **Step 1: Rename the branch and create the package files**

```bash
git branch -m feat/site
mkdir -p site/src site/test site/public
```

`site/package.json`:

```json
{
  "name": "clutchfactor-site",
  "private": true,
  "version": "1.0.0",
  "type": "module",
  "packageManager": "pnpm@10.33.2",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview --port 4173 --strictPort",
    "typecheck": "tsc --noEmit",
    "lint": "eslint .",
    "test": "vitest",
    "test:run": "vitest run",
    "shots": "node scripts/shots.mjs"
  },
  "devDependencies": {
    "@eslint/js": "^10.0.1",
    "@types/node": "^22.20.1",
    "eslint": "^10.8.1",
    "typescript": "~5.8.3",
    "typescript-eslint": "^8.67.0",
    "vite": "^7.0.4",
    "vitest": "^4.1.11"
  }
}
```

`site/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noEmit": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "types": ["vite/client", "node"]
  },
  "include": ["src", "test", "vite.config.ts"]
}
```

`site/vite.config.ts`:

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  build: { target: "es2022", assetsInlineLimit: 0 },
  test: { include: ["test/**/*.test.ts"] },
});
```

`site/eslint.config.js`:

```js
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/", "public/"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { process: "readonly", console: "readonly" } },
  },
);
```

`site/index.html` (stub, replaced in Task 7):

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>ClutchFactor — CS2 demo coach</title>
  </head>
  <body>
    <main id="main"><h1>ClutchFactor</h1></main>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`site/src/main.ts` (stub):

```ts
document.documentElement.classList.add("js");
```

`site/test/smoke.test.ts`:

```ts
import { expect, it } from "vitest";

it("vitest runs in site/", () => {
  expect(1 + 1).toBe(2);
});
```

- [ ] **Step 2: Tell the root linter to ignore `site/`**

In root `eslint.config.js` change line 6 from
`{ ignores: ["dist/", "src-tauri/", ".claude/"] },` to
`{ ignores: ["dist/", "src-tauri/", ".claude/", "site/"] },`

- [ ] **Step 3: Install and add the two packages whose versions we don't pin by hand**

```bash
pnpm -C site install
pnpm -C site add -D happy-dom sharp
```

Expected: `site/node_modules/` and `site/pnpm-lock.yaml` created; no `pnpm-workspace.yaml` anywhere (`ls pnpm-workspace.yaml` → "No such file").

- [ ] **Step 4: Run every script once**

```bash
pnpm -C site typecheck && pnpm -C site lint && pnpm -C site test:run && pnpm -C site build
pnpm lint   # root: must still pass and must not descend into site/
```

Expected: all green; `site/dist/index.html` exists.

- [ ] **Step 5: Commit**

```bash
git add site/package.json site/pnpm-lock.yaml site/tsconfig.json site/vite.config.ts site/eslint.config.js site/index.html site/src/main.ts site/test/smoke.test.ts eslint.config.js
git commit -m "chore(site): scaffold standalone Vite + TS package under site/"
```

---

### Task 2: `release.ts` and `platform.ts` (pure, TDD)

**Files:**
- Create: `site/src/release.ts`, `site/src/platform.ts`, `site/test/release.test.ts`, `site/test/platform.test.ts`
- Modify: `docs/spec/marketing-site.md` §6 (signature + MiB rounding, see Step 5)

**Interfaces:**
- Produces:
  - `release` — `{ version: "1.0.0"; tag: "v1.0.0"; mac: { file; bytes; arch }; win: { file; bytes }; msi: { file; bytes } }`
  - `REPO_URL: string`, `assetUrl(file: string): string`, `formatMb(bytes: number): string`
  - `type Platform = "mac" | "windows" | "other"`, `detectPlatform(userAgent: string, maxTouchPoints?: number): Platform`

- [ ] **Step 1: Write the failing tests**

`site/test/release.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { REPO_URL, assetUrl, formatMb, release } from "../src/release";

describe("release constants", () => {
  it("points at the v1.0.0 GitHub release assets", () => {
    expect(assetUrl(release.mac.file)).toBe(
      "https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_aarch64.dmg",
    );
    expect(assetUrl(release.win.file)).toBe(`${REPO_URL}/releases/download/v1.0.0/ClutchFactor_1.0.0_x64-setup.exe`);
    expect(assetUrl(release.msi.file)).toBe(`${REPO_URL}/releases/download/v1.0.0/ClutchFactor_1.0.0_x64_en-US.msi`);
  });

  it("renders whole MiB the way the download page quotes them", () => {
    expect(formatMb(release.mac.bytes)).toBe("10 MB");
    expect(formatMb(release.win.bytes)).toBe("8 MB");
    expect(formatMb(release.msi.bytes)).toBe("10 MB");
  });
});
```

`site/test/platform.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { detectPlatform } from "../src/platform";

const UA = {
  win: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  mac: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  ipadDesktopMode:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Safari/605.1.15",
  iphone:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.5 Mobile/15E148 Safari/604.1",
  linux: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36",
  android: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36",
};

describe("detectPlatform", () => {
  it("Windows", () => expect(detectPlatform(UA.win)).toBe("windows"));
  it("macOS", () => expect(detectPlatform(UA.mac, 0)).toBe("mac"));
  it("iPadOS in desktop mode reports Macintosh but has touch points → other", () =>
    expect(detectPlatform(UA.ipadDesktopMode, 5)).toBe("other"));
  it("iPhone → other", () => expect(detectPlatform(UA.iphone, 5)).toBe("other"));
  it("Linux and Android → other", () => {
    expect(detectPlatform(UA.linux)).toBe("other");
    expect(detectPlatform(UA.android)).toBe("other");
  });
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `pnpm -C site test:run`
Expected: FAIL — "Failed to resolve import "../src/release"" and the same for platform.

- [ ] **Step 3: Implement**

`site/src/release.ts`:

```ts
/** The ONE place the site knows about a release. Update on every tag
 *  (CLAUDE.md release checklist): version, tag, file names, byte sizes
 *  (`gh release view vX.Y.Z --json assets`). */
export const release = {
  version: "1.0.0",
  tag: "v1.0.0",
  mac: { file: "ClutchFactor_1.0.0_aarch64.dmg", bytes: 10549192, arch: "Apple silicon" },
  win: { file: "ClutchFactor_1.0.0_x64-setup.exe", bytes: 7936273 },
  msi: { file: "ClutchFactor_1.0.0_x64_en-US.msi", bytes: 10166272 },
} as const;

export const REPO_URL = "https://github.com/junlee-3/clutchfactor";

export const assetUrl = (file: string): string =>
  `${REPO_URL}/releases/download/${release.tag}/${file}`;

/** Whole MiB, the number Finder/Explorer show for these files. */
export const formatMb = (bytes: number): string => `${Math.round(bytes / 1048576)} MB`;
```

`site/src/platform.ts`:

```ts
export type Platform = "mac" | "windows" | "other";

/** iPadOS Safari in desktop mode reports "Macintosh"; touch points tell it
 *  apart from a Mac (a Mac reports 0). Pure so it can be tested. */
export function detectPlatform(userAgent: string, maxTouchPoints = 0): Platform {
  if (/Windows|Win64|Win32/.test(userAgent)) return "windows";
  const looksMac = /Macintosh|Mac OS X/.test(userAgent) && !/iPhone|iPad|iPod/.test(userAgent);
  if (looksMac && maxTouchPoints <= 1) return "mac";
  return "other";
}
```

- [ ] **Step 4: Run the tests**

Run: `pnpm -C site test:run`
Expected: PASS (7 tests + smoke).

- [ ] **Step 5: Keep the spec truthful**

In `docs/spec/marketing-site.md` §6 replace the sentence starting "`detectPlatform(userAgent)`:" with:

```
`detectPlatform(userAgent, maxTouchPoints = 0)`: `/Windows|Win64|Win32/` →
`windows`; `/Macintosh|Mac OS X/` without `/iPhone|iPad|iPod/` **and**
`maxTouchPoints ≤ 1` → `mac` (iPadOS in desktop mode reports `Macintosh`
but has touch points); everything else → `other`, which shows both
buttons as secondary. Pure, tested.
```

and replace "Sizes are rendered as whole MB (`Math.round(bytes / 1e6)`)" with "Sizes are rendered as whole MiB (`Math.round(bytes / 1048576)`) — the number Finder/Explorer show."

- [ ] **Step 6: Commit**

```bash
git add site/src/release.ts site/src/platform.ts site/test/release.test.ts site/test/platform.test.ts docs/spec/marketing-site.md
git commit -m "feat(site): release constants and platform detection"
```

---

### Task 3: Tokens copy + drift test

**Files:**
- Create: `site/src/styles/tokens.css`, `site/test/tokens.test.ts`

**Interfaces:**
- Produces: every CSS custom property the app defines (`--bg0` … `--ease`), available to `site.css`.

- [ ] **Step 1: Write the failing test**

`site/test/tokens.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf8");

/** `--name: value;` pairs, comments stripped, ignoring the web-only type tokens. */
function tokens(css: string): Map<string, string> {
  const noComments = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const out = new Map<string, string>();
  for (const m of noComments.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/g)) {
    const name = m[1];
    if (name.startsWith("--font-") || name.startsWith("--text-")) continue;
    out.set(name, m[2].replace(/\s+/g, " ").trim());
  }
  return out;
}

describe("site tokens mirror the app's", () => {
  const app = tokens(read("../../src/styles/tokens.css"));
  const site = tokens(read("../src/styles/tokens.css"));

  it("has the same token names", () => {
    expect([...site.keys()].sort()).toEqual([...app.keys()].sort());
  });

  it("has the same values", () => {
    for (const [name, value] of app) expect(site.get(name), name).toBe(value);
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `pnpm -C site test:run test/tokens.test.ts`
Expected: FAIL — ENOENT `site/src/styles/tokens.css`.

- [ ] **Step 3: Create the copy**

```bash
mkdir -p site/src/styles
{
  printf '/* Copy of src/styles/tokens.css (the app). test/tokens.test.ts fails CI if any\n   token other than --font-* / --text-* drifts. Web-only type tokens live in\n   site.css, never here. */\n';
  sed -n '/^:root {/,$p' src/styles/tokens.css;
} > site/src/styles/tokens.css
```

Then open `site/src/styles/tokens.css` and delete the `--font-sans`, `--font-mono`, `--font-display` and the eight `--text-*` lines (the web page defines its own in `site.css`). Keep `color-scheme: dark;`.

- [ ] **Step 4: Run the test**

Run: `pnpm -C site test:run test/tokens.test.ts`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add site/src/styles/tokens.css site/test/tokens.test.ts
git commit -m "feat(site): design tokens copied from the app with a drift test"
```

---

### Task 4: `ledger.ts` — timestamp → delay schedule (pure, TDD)

**Files:**
- Create: `site/src/ledger.ts`, `site/test/ledger.test.ts`

**Interfaces:**
- Produces: `parseClock(t: string): number` (seconds), `ledgerSchedule(rows: readonly { t: string }[]): number[]` (ms, one per row, non-decreasing), constants `LEDGER_FIRST_MS = 400`, `LEDGER_LAST_MS = 3600`, `LEDGER_STAGGER_MS = 250`.

- [ ] **Step 1: Write the failing tests**

`site/test/ledger.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { ledgerSchedule, parseClock } from "../src/ledger";

describe("parseClock", () => {
  it("m:ss → seconds", () => {
    expect(parseClock("0:05")).toBe(5);
    expect(parseClock("1:12")).toBe(72);
  });
});

describe("ledgerSchedule", () => {
  it("a single row appears at 400 ms", () => {
    expect(ledgerSchedule([{ t: "0:05" }])).toEqual([400]);
  });

  it("is linear in the timestamp between 400 ms and 3600 ms", () => {
    expect(ledgerSchedule([{ t: "0:00" }, { t: "0:10" }, { t: "0:20" }])).toEqual([400, 2000, 3600]);
  });

  it("staggers equal timestamps by 250 ms", () => {
    expect(ledgerSchedule([{ t: "0:05" }, { t: "1:12" }, { t: "1:12" }])).toEqual([400, 3600, 3850]);
    expect(ledgerSchedule([{ t: "0:05" }, { t: "0:05" }, { t: "0:05" }])).toEqual([400, 650, 900]);
  });

  it("schedules the spec's round-2 ledger", () => {
    const rows = [{ t: "0:05" }, { t: "0:31" }, { t: "0:55" }, { t: "1:01" }, { t: "1:12" }, { t: "1:12" }];
    expect(ledgerSchedule(rows)).toEqual([400, 1642, 2788, 3075, 3600, 3850]);
  });

  it("empty input → empty schedule", () => {
    expect(ledgerSchedule([])).toEqual([]);
  });
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `pnpm -C site test:run test/ledger.test.ts`
Expected: FAIL — cannot resolve `../src/ledger`.

- [ ] **Step 3: Implement**

`site/src/ledger.ts`:

```ts
export const LEDGER_FIRST_MS = 400;
export const LEDGER_LAST_MS = 3600;
export const LEDGER_STAGGER_MS = 250;

/** "m:ss" → seconds. */
export function parseClock(t: string): number {
  const [m, s] = t.split(":").map(Number);
  return m * 60 + s;
}

/** Reveal delay per row: first at 400 ms, last at 3600 ms, linear in the
 *  timestamp; equal timestamps stagger by 250 ms so no two rows land at
 *  once. Rows are assumed to be in chronological order. */
export function ledgerSchedule(rows: readonly { t: string }[]): number[] {
  if (rows.length === 0) return [];
  const secs = rows.map((r) => parseClock(r.t));
  const first = secs[0];
  const span = secs[secs.length - 1] - first;
  const out: number[] = [];
  let prev = -Infinity;
  for (const s of secs) {
    let ms = span === 0 ? LEDGER_FIRST_MS : LEDGER_FIRST_MS + ((s - first) / span) * (LEDGER_LAST_MS - LEDGER_FIRST_MS);
    ms = Math.round(ms);
    if (ms <= prev) ms = prev + LEDGER_STAGGER_MS;
    out.push(ms);
    prev = ms;
  }
  return out;
}
```

- [ ] **Step 4: Run the tests**

Run: `pnpm -C site test:run test/ledger.test.ts`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add site/src/ledger.ts site/test/ledger.test.ts
git commit -m "feat(site): ledger reveal schedule from timestamps"
```

---

### Task 5: `clips.ts` — manifest, environment rules, rotation (pure, TDD) + clip docs

**Files:**
- Create: `site/src/clips.ts`, `site/test/clips.test.ts`, `site/public/clips/README.md`, `site/public/clips/clips.json`

**Interfaces:**
- Produces: `interface ClipEntry { file: string }`, `parseClipsManifest(json: unknown): ClipEntry[] | null` (null = invalid → poster), `interface VideoEnv { reducedMotion: boolean; saveData: boolean; viewportWidth: number; clips: readonly ClipEntry[] }`, `shouldPlayVideo(env: VideoEnv): boolean`, `nextIndex(i: number, n: number): number`, `MIN_VIDEO_WIDTH = 720`.

- [ ] **Step 1: Write the failing tests**

`site/test/clips.test.ts`:

```ts
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
```

- [ ] **Step 2: Run them to verify they fail**

Run: `pnpm -C site test:run test/clips.test.ts`
Expected: FAIL — cannot resolve `../src/clips`.

- [ ] **Step 3: Implement**

`site/src/clips.ts`:

```ts
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
```

- [ ] **Step 4: Run the tests**

Run: `pnpm -C site test:run test/clips.test.ts`
Expected: PASS (11 tests).

- [ ] **Step 5: Add the manifest and the owner's clip instructions**

`site/public/clips/clips.json`:

```json
[]
```

`site/public/clips/README.md`:

```markdown
# Hero clips

The hero loops the owner's own CS2 gameplay. Nothing third-party is ever
committed here.

## Spec (per clip)

- H.264 MP4, no audio track, 1280×720 (or 1920×1080), 24–30 fps
- 8–15 s, ≤ 5 MB each; 3–5 clips total
- Name `clip-01.mp4`, `clip-02.mp4`, … (lowercase, digits, hyphens only)
- `poster.jpg`: the first frame of `clip-01.mp4`, 1920×1080, ≤ 300 KB

## Make one from a recording

```sh
ffmpeg -ss 00:01:23 -t 12 -i recording.mp4 -an -vf "scale=1280:-2" \
  -c:v libx264 -crf 26 -preset slow -movflags +faststart clip-01.mp4
ffmpeg -i clip-01.mp4 -frames:v 1 -q:v 3 poster.jpg
```

## Register it

Add the file to `clips.json`, in play order:

```json
[{ "file": "clip-01.mp4" }, { "file": "clip-02.mp4" }]
```

An empty manifest shows the poster; no poster shows a dimmed Inferno radar.
Files here are cached for a year (`vercel.json`) — rename to change.
```

- [ ] **Step 6: Commit**

```bash
git add site/src/clips.ts site/test/clips.test.ts site/public/clips/README.md site/public/clips/clips.json
git commit -m "feat(site): clip manifest parsing, video environment rules, clip instructions"
```

---

### Task 6: Screenshot pipeline (`pnpm -C site shots`) + favicon

**Files:**
- Create: `site/scripts/shots.mjs`, `site/public/shots/*.webp` (generated, committed), `site/public/og.jpg` (generated), `site/public/favicon.svg`

**Interfaces:**
- Produces the files `index.html` references: `/shots/{report,replay,trends,corpus,library,coach,watches}-{1440,960}.webp`, `/shots/radar-inferno.webp`, `/og.jpg`, `/favicon.svg`.

- [ ] **Step 1: Write the script**

`site/scripts/shots.mjs`:

```js
// Screenshots → WebP at two widths, the radar fallback, and og.jpg.
// Run from anywhere: `pnpm -C site shots`. Outputs are committed.
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..", "..");            // repo root
const out = path.resolve(here, "..", "public", "shots");
const pub = path.resolve(here, "..", "public");

const shots = [
  ["docs/screenshots/report.png", "report"],
  ["docs/screenshots/replay.png", "replay"],
  ["docs/screenshots/trends.png", "trends"],
  ["docs/screenshots/corpus.png", "corpus"],
  ["docs/screenshots/library.png", "library"],
  ["docs/design/walkthrough-v1.3/report-coach.png", "coach"],
  ["docs/design/walkthrough-v1.4/04-watches.png", "watches"],
];

await mkdir(out, { recursive: true });

for (const [src, name] of shots) {
  for (const width of [1440, 960]) {
    const file = path.join(out, `${name}-${width}.webp`);
    await sharp(path.join(root, src)).resize({ width }).webp({ quality: 82 }).toFile(file);
    console.log("wrote", path.relative(root, file));
  }
}

await sharp(path.join(root, "assets/maps/de_inferno.png"))
  .resize({ width: 1600 })
  .webp({ quality: 70 })
  .toFile(path.join(out, "radar-inferno.webp"));
console.log("wrote site/public/shots/radar-inferno.webp");

await sharp(path.join(root, "docs/screenshots/report.png"))
  .resize(1200, 630, { fit: "cover", position: "top" })
  .jpeg({ quality: 82 })
  .toFile(path.join(pub, "og.jpg"));
console.log("wrote site/public/og.jpg");
```

- [ ] **Step 2: Run it and copy the favicon**

```bash
pnpm -C site shots
cp app-icon.svg site/public/favicon.svg
ls -la site/public/shots site/public/og.jpg site/public/favicon.svg
```

Expected: 15 `.webp` files (7 × 2 widths + radar), each ≤ 250 KB; `og.jpg` 1200×630 (`sips -g pixelWidth -g pixelHeight site/public/og.jpg`).

- [ ] **Step 3: Lint (the script is plain JS under the site config)**

Run: `pnpm -C site lint`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add site/scripts/shots.mjs site/public/shots site/public/og.jpg site/public/favicon.svg
git commit -m "feat(site): screenshot pipeline (sharp → webp), radar fallback, og image, favicon"
```

---

### Task 7: `index.html` — the whole page, final copy

**Files:**
- Replace: `site/index.html`
- Create: `site/test/html.test.ts`

**Interfaces:**
- Produces the DOM hooks later tasks bind to: `[data-nav]`, `[data-nav-download]`, `[data-hero]`, `.hero__video[data-video="a"|"b"]`, `.hero__poster`, `[data-download-buttons] a[data-os]`, `.chip--evidence[data-target]`, `[data-ledger] .ledger__row[data-t]`, `[data-reveal]`, `#catches .card[id]`.
- Consumes: `release` file names (hard-coded in the HTML as the no-JS fallback; the test ties them to `release.ts`).

- [ ] **Step 1: Write the failing structure test**

`site/test/html.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { release } from "../src/release";

const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");

describe("index.html structure", () => {
  it("has exactly one h1 and the section anchors the nav links to", () => {
    expect(html.match(/<h1\b/g)?.length).toBe(1);
    for (const id of ["top", "ledger", "catches", "coach", "limits", "habits", "download"]) {
      expect(html, id).toContain(`id="${id}"`);
    }
  });

  it("links every release asset by its real file name (no-JS fallback)", () => {
    for (const file of [release.mac.file, release.win.file, release.msi.file]) expect(html).toContain(file);
  });

  it("has the hooks main.ts binds to", () => {
    for (const hook of [
      "data-nav-download",
      'data-video="a"',
      'data-video="b"',
      "data-download-buttons",
      "data-ledger",
      "data-target=",
      "data-reveal",
    ]) {
      expect(html, hook).toContain(hook);
    }
  });

  it("gives every image descriptive alt text", () => {
    const imgs = html.match(/<img\b[^>]*>/g) ?? [];
    expect(imgs.length).toBeGreaterThanOrEqual(5); // replay, coach, watches, trends, corpus
    for (const img of imgs) expect(img).toMatch(/\balt="[^"]{30,}"/);
  });

  it("contains no placeholder copy", () => {
    expect(html).not.toMatch(/lorem|TODO|TBD|placeholder/i);
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `pnpm -C site test:run test/html.test.ts`
Expected: FAIL on the anchors / hooks / image assertions (the stub has none).

- [ ] **Step 3: Write the page**

`site/index.html` (complete file):

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>ClutchFactor — CS2 demo coach</title>
    <meta
      name="description"
      content="Import your CS2 demo. ClutchFactor coaches you round by round and links every insight to the exact second in a 2D replay. Free, local, honest about what it can't see."
    />
    <meta name="theme-color" content="#0e1116" />
    <meta property="og:type" content="website" />
    <meta property="og:title" content="ClutchFactor — CS2 demo coach" />
    <meta
      property="og:description"
      content="Import your CS2 demo. ClutchFactor coaches you round by round and links every insight to the exact second in a 2D replay. Free, local, honest about what it can't see."
    />
    <meta property="og:image" content="https://clutchfactor.vercel.app/og.jpg" />
    <meta name="twitter:card" content="summary_large_image" />
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      rel="stylesheet"
      href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
    />
  </head>
  <body>
    <a class="skip" href="#main">Skip to content</a>

    <header class="nav" data-nav>
      <div class="nav__in container">
        <a class="brand" href="#top" aria-label="ClutchFactor — back to top">
          <svg class="brand__icon" viewBox="0 0 1024 1024" aria-hidden="true">
            <rect width="1024" height="1024" rx="224" fill="#151a21" />
            <circle cx="512" cy="512" r="268" fill="none" stroke="#2c3a4d" stroke-width="34" />
            <g stroke="#4aa3ff" stroke-width="46" stroke-linecap="round">
              <line x1="512" y1="150" x2="512" y2="268" />
              <line x1="512" y1="756" x2="512" y2="874" />
              <line x1="150" y1="512" x2="268" y2="512" />
              <line x1="756" y1="512" x2="874" y2="512" />
            </g>
            <polyline points="330,612 438,548 512,588 606,470 694,414" fill="none" stroke="#4aa3ff" stroke-width="44" stroke-linecap="round" stroke-linejoin="round" />
            <circle cx="694" cy="414" r="42" fill="#f5b83d" />
          </svg>
          <span>ClutchFactor</span>
        </a>
        <nav class="nav__links" aria-label="Sections">
          <a href="#ledger">How it works</a>
          <a href="#coach">The coach</a>
          <a href="#limits">Honest limits</a>
          <a href="https://github.com/junlee-3/clutchfactor" rel="noopener">GitHub</a>
        </nav>
        <a class="btn btn--secondary btn--sm" href="#download" data-nav-download>Download v1.0.0</a>
      </div>
    </header>

    <main id="main">
      <!-- ======================= HERO ======================= -->
      <section class="hero" id="top" data-hero>
        <div class="hero__media" aria-hidden="true">
          <div class="hero__poster"></div>
          <video class="hero__video" data-video="a" muted playsinline preload="metadata"></video>
          <video class="hero__video" data-video="b" muted playsinline preload="none"></video>
        </div>
        <div class="hero__body container">
          <p class="eyebrow">CS2 demo coach · free · runs on your machine</p>
          <h1 class="hero__title">Watch why you died.</h1>
          <p class="lede">
            ClutchFactor reads your CS2 demo and coaches you round by round. Every insight links to the exact second on a
            2D replay — so you watch the play instead of taking a stat's word for it.
          </p>
          <div class="cta" data-download-buttons>
            <a class="btn btn--primary" data-os="windows" href="https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_x64-setup.exe">
              Download for Windows <small>.exe · 8 MB</small>
            </a>
            <a class="btn btn--secondary" data-os="mac" href="https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_aarch64.dmg">
              Download for macOS <small>.dmg · Apple silicon · 10 MB</small>
            </a>
          </div>
          <p class="evidence">
            <span class="evidence__lead">Missed 4 trades in range →</span>
            <a class="chip chip--evidence" href="#catches" data-target="card-missed-trades">R3 · 0:16</a>
            <a class="chip chip--evidence" href="#catches" data-target="card-missed-trades">R12 · 0:28</a>
            <a class="chip chip--evidence" href="#catches" data-target="card-missed-trades">R13 · 0:14</a>
            <a class="chip chip--evidence" href="#catches" data-target="card-missed-trades">R16 · 0:16</a>
          </p>
        </div>
      </section>

      <!-- ======================= HOW IT WORKS / LEDGER ======================= -->
      <section class="section" id="ledger">
        <div class="container split" data-reveal>
          <div class="split__text">
            <p class="eyebrow">How it works</p>
            <h2>Every round, narrated.</h2>
            <p class="body">
              Import a .dem. The engine rebuilds every round as a play ledger — setup, utility, trades, deaths,
              rotations, outcome — and gives each round a verdict. When one play decided it, the report names that
              play.
            </p>
            <div class="ledger" data-ledger>
              <div class="ledger__hd"><span>Round 2 · Inferno · CT</span><b>lost · you 2–1 · 3v0</b></div>
              <div class="ledger__row" data-t="0:05">
                <span class="ledger__t">0:05</span>
                <div><div class="ledger__h">Setup at CT spawn</div><div class="ledger__s">Nearest teammate Roland Pryzbylewski, 432 u · 3 of 4 teammates within 900 u</div></div>
              </div>
              <div class="ledger__row" data-t="0:31">
                <span class="ledger__t">0:31</span>
                <div><div class="ledger__h">Rotated to the plant in 11 s</div><div class="ledger__s">2,320 u from the plant when it went down</div></div>
              </div>
              <div class="ledger__row ledger__row--win" data-t="0:55">
                <span class="ledger__t">0:55</span>
                <div><div class="ledger__h">Killed Konky</div><div class="ledger__s">1,436 u · MP9 · +2% win probability · 3v5 before</div></div>
              </div>
              <div class="ledger__row ledger__row--win" data-t="1:01">
                <span class="ledger__t">1:01</span>
                <div><div class="ledger__h">Killed MyUnit</div><div class="ledger__s">775 u · MP9 · +35% win probability · 3v2 before</div></div>
              </div>
              <div class="ledger__row" data-t="1:12">
                <span class="ledger__t">1:12</span>
                <div><div class="ledger__h">Round lost — bomb exploded</div><div class="ledger__s">1v0 at the end · 2 kills, 121 damage</div></div>
              </div>
              <div class="ledger__row ledger__row--loss" data-t="1:12">
                <span class="ledger__t">1:12</span>
                <div><div class="ledger__h">Death</div><div class="ledger__s">Nearest: Roland Pryzbylewski · At Apartments · Not traded — after the round was decided</div></div>
              </div>
            </div>
            <a class="link" href="#catches">See what it looks for →</a>
          </div>
          <figure class="frame">
            <img
              src="/shots/replay-1440.webp"
              srcset="/shots/replay-960.webp 960w, /shots/replay-1440.webp 1440w"
              sizes="(min-width: 1024px) 58vw, 100vw"
              width="1440" height="900" loading="lazy" decoding="async"
              alt="ClutchFactor replay screen: the Inferno radar with player positions, CT and T rosters with health and weapons, the kill feed, and the round-2 play ledger"
            />
            <figcaption>The 2D replay — positions, health, weapons, kill feed, callouts on the map. 60 fps, deep-linked from every claim.</figcaption>
          </figure>
        </div>
      </section>

      <!-- ======================= WHAT IT CATCHES ======================= -->
      <section class="section section--alt" id="catches">
        <div class="container" data-reveal>
          <p class="eyebrow">What it catches</p>
          <h2>Stop dying for free.</h2>
          <p class="body">
            Deaths are sorted into 15 classes by a rule engine over parsed events — positions, trades, utility, timing.
            Every rule carries a confidence, and rules bias toward silence: a missed detection is fine, a wrong
            accusation is not.
          </p>
          <div class="catches">
            <div class="card">
              <p class="eyebrow">How you died</p>
              <ul class="bars">
                <li><span>Outaimed in a fair duel</span><b>7</b><i class="bars__bar bars__bar--win" style="--w: 100%"></i></li>
                <li><span>No-engagement death</span><b>5</b><i class="bars__bar" style="--w: 71%"></i></li>
                <li><span>Self / world / teammate</span><b>2</b><i class="bars__bar" style="--w: 29%"></i></li>
                <li><span>Isolated &amp; untradeable</span><b>1</b><i class="bars__bar" style="--w: 14%"></i></li>
              </ul>
              <p class="card__note">46.7% were fair duels you lost on mechanics — good news: the rest had a fixable cause.</p>
            </div>
            <div class="cards">
              <article class="card card--loss" id="card-missed-trades" tabindex="-1">
                <header class="card__hd"><h3>Missed 4 trades in range</h3><span class="chip chip--conf">70%</span></header>
                <p>You were close enough to trade 4 teammate deaths and stayed on your angle — rounds 3, 12, 13 and 16. Keep your crosshair where he is fighting so the trade is one step, not a repositioning job.</p>
                <p class="card__chips">
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R3 · 0:16</a>
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R12 · 0:28</a>
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R13 · 0:14</a>
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R16 · 0:16</a>
                </p>
              </article>
              <article class="card card--loss" id="card-isolated" tabindex="-1">
                <header class="card__hd"><h3>Died isolated twice</h3><span class="chip chip--conf">75%</span></header>
                <p>You died isolated twice with no teammate close enough to punish the kill — rounds 6 and 9. Take those duels one angle closer to a teammate: arrive together, or hold until someone can trade you.</p>
                <p class="card__chips">
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R6 · 0:23</a>
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R9 · 0:11</a>
                </p>
              </article>
              <article class="card card--loss" id="card-baited" tabindex="-1">
                <header class="card__hd"><h3>You traded in, nobody followed</h3><span class="chip chip--conf">70%</span></header>
                <p>You committed to the trade and the follow-up never came — twice, rounds 6 and 9. You were the only one who re-peeked — Mashed Potato and Crunchy Potato were nearest and stayed put; that is a team spacing problem, not a reason to stop trading.</p>
                <p class="card__chips">
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R6 · 0:23</a>
                  <a class="chip chip--evidence" href="#download" title="Open the tape — in the app">R9 · 0:11</a>
                </p>
              </article>
              <p class="footnote">Dashed means you can click through to the tape — in the app.</p>
            </div>
          </div>
        </div>
      </section>

      <!-- ======================= THE COACH ======================= -->
      <section class="section" id="coach">
        <div class="container" data-reveal>
          <p class="eyebrow">The AI coach · optional</p>
          <h2>A coach that can't make up numbers.</h2>
          <p class="body">
            With a Gemini key set, the coach reads the measured facts for a round and writes its own read — a judgment
            call, not a template fill-in. Every number, name and callout it cites is checked against those facts before
            you see it; anything that doesn't check out is rejected in favor of the template. No key means no network
            call and no coach — just the plain-language templates.
          </p>
          <figure class="frame frame--wide">
            <img
              src="/shots/coach-1440.webp"
              srcset="/shots/coach-960.webp 960w, /shots/coach-1440.webp 1440w"
              sizes="(min-width: 1240px) 1200px, 100vw"
              width="1440" height="900" loading="lazy" decoding="async"
              alt="ClutchFactor match report for a 12–12 Mirage with the coach's read at the top: a paragraph on spacing and trade discipline, then three bullet points, above the death breakdown"
            />
          </figure>
          <dl class="facts">
            <div><dt>Checked, not trusted</dt><dd>Every figure the coach cites is verified against the parsed demo.</dd></div>
            <div><dt>Local by default</dt><dd>The only network call this app ever makes is to Google's Gemini API, and only with a key you set.</dd></div>
            <div><dt>Bring your own key</dt><dd>Settings → Coach; stored in the app's local database.</dd></div>
          </dl>
        </div>
      </section>

      <!-- ======================= HONEST LIMITS ======================= -->
      <section class="section section--alt" id="limits">
        <div class="container" data-reveal>
          <p class="eyebrow">What your coach watches</p>
          <h2>What it can't see, it says so.</h2>
          <p class="body">
            A dedicated screen lists every detection rule in plain language with its live thresholds, which of the 15
            death classes aren't built yet and why, and what the engine flatly cannot see. Deaths it can't attribute
            stay "Unclassified", and the report says so.
          </p>
          <div class="limits">
            <div class="card">
              <h3>Not built (and why)</h3>
              <ul class="plain">
                <li>Over-peeks (8) and wide peeks (10) need peek geometry the parser doesn't provide.</li>
                <li>Per-death hotspot classification (12) needs a "standard angle" model — hotspots are tracked across matches instead.</li>
              </ul>
            </div>
            <div class="card">
              <h3>Cannot see</h3>
              <ul class="plain plain--inline">
                <li>economy</li><li>utility lineups</li><li>comms</li><li>aim mechanics</li><li>line of sight</li>
              </ul>
            </div>
          </div>
          <figure class="frame frame--wide">
            <img
              src="/shots/watches-1440.webp"
              srcset="/shots/watches-960.webp 960w, /shots/watches-1440.webp 1440w"
              sizes="(min-width: 1240px) 1200px, 100vw"
              width="1440" height="888" loading="lazy" decoding="async"
              alt="The 'What your coach watches' screen: the H2 trade-spacing rules — isolated death, failed trade, baited trade — each with when it counts, how it reads, and which stats it feeds"
            />
          </figure>
        </div>
      </section>

      <!-- ======================= ACROSS MATCHES ======================= -->
      <section class="section" id="habits">
        <div class="container" data-reveal>
          <p class="eyebrow">Across your matches</p>
          <h2>Habits, not hot takes.</h2>
          <p class="body">
            Patterns are promoted only when they repeat — "Left trades on the table in 5 of your last 10 matches" —
            including repeat death hotspots per map, with evidence into each contributing demo. Trends chart every habit
            across your imports and call out streaks, good and bad.
          </p>
          <div class="pair">
            <figure class="frame">
              <img
                src="/shots/trends-1440.webp"
                srcset="/shots/trends-960.webp 960w, /shots/trends-1440.webp 1440w"
                sizes="(min-width: 1024px) 50vw, 100vw"
                width="1440" height="900" loading="lazy" decoding="async"
                alt="Trends screen: the pure-aim-duel share across five matches as a line, then per-habit sparklines with counts and streak callouts like 'missed trades trending up 3 matches straight'"
              />
              <figcaption>Trends: per-habit sparklines, the share of deaths that were pure aim duels, streak callouts.</figcaption>
            </figure>
            <figure class="frame">
              <img
                src="/shots/corpus-1440.webp"
                srcset="/shots/corpus-960.webp 960w, /shots/corpus-1440.webp 1440w"
                sizes="(min-width: 1024px) 50vw, 100vw"
                width="1440" height="900" loading="lazy" decoding="async"
                alt="Reference corpus screen: imported pro demos per map and the positional occupancy heatmap they build for a map, side and round phase"
              />
              <figcaption>Reference corpus: drop pro demos from HLTV; with 8+ on a map it flags spots you hold that pros rarely do — "unusual, not wrong".</figcaption>
            </figure>
          </div>
        </div>
      </section>

      <!-- ======================= DOWNLOAD ======================= -->
      <section class="section section--alt" id="download">
        <div class="container" data-reveal>
          <h2>Download v1.0.0</h2>
          <p class="lede">Free. No account. Your demos never leave your machine.</p>
          <div class="dl">
            <div class="card">
              <h3>macOS</h3>
              <a class="btn btn--primary" href="https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_aarch64.dmg">Download .dmg <small>10 MB · Apple silicon</small></a>
              <p class="card__note">Unsigned build: right-click the app → Open on first launch. Intel Mac? Not built yet — <a href="https://github.com/junlee-3/clutchfactor#development" rel="noopener">build from source</a>.</p>
            </div>
            <div class="card">
              <h3>Windows</h3>
              <a class="btn btn--primary" href="https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_x64-setup.exe">Download .exe <small>8 MB</small></a>
              <p class="card__note">Prefer an installer package? <a href="https://github.com/junlee-3/clutchfactor/releases/download/v1.0.0/ClutchFactor_1.0.0_x64_en-US.msi">.msi</a>. Unsigned build: SmartScreen will warn — "More info → Run anyway".</p>
            </div>
          </div>
          <div class="firstrun">
            <p class="eyebrow">First run</p>
            <ol>
              <li><b>Import demo.</b></li>
              <li><b>Pick a <code>.dem</code> from your own matches</b> — CS2 → Watch → Your Matches → Download, or a FACEIT match room.</li>
              <li><b>The app auto-detects which player you are</b> — the most-seen account across your imports. Override it in Settings if it guesses wrong.</li>
            </ol>
          </div>
        </div>
      </section>
    </main>

    <footer class="footer">
      <div class="container footer__in">
        <span class="brand brand--small">ClutchFactor</span>
        <nav class="footer__links" aria-label="Footer">
          <a href="https://github.com/junlee-3/clutchfactor" rel="noopener">GitHub</a>
          <a href="https://github.com/junlee-3/clutchfactor/releases" rel="noopener">Releases</a>
          <a href="https://github.com/pnxenopoulos/awpy" rel="noopener">Radar images from awpy</a>
        </nav>
        <p class="footer__legal">Not affiliated with Valve Corporation. Counter-Strike is a trademark of Valve Corporation.</p>
      </div>
    </footer>

    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 4: Run the tests and a build**

Run: `pnpm -C site test:run test/html.test.ts && pnpm -C site build`
Expected: PASS (5 tests); build succeeds (images referenced from `/shots/` are in `public/`, so Vite copies them).

- [ ] **Step 5: Commit**

```bash
git add site/index.html site/test/html.test.ts
git commit -m "feat(site): the page — sections and final copy from the spec"
```

---

### Task 8: `site.css` — type scale, components, sections, responsive, reduced motion

**Files:**
- Create: `site/src/styles/site.css`
- Modify: `site/src/main.ts` (import both stylesheets)

**Interfaces:**
- Consumes tokens from Task 3 and the class names from Task 7.
- Produces the state classes Task 9 toggles: `html.js`, `.nav--solid`, `.hero--video`, `.hero__video.is-live`, `.ledger--armed`, `.ledger__row.is-in`, `[data-reveal].is-in`, `.card.is-target`.

- [ ] **Step 1: Import the stylesheets**

`site/src/main.ts`:

```ts
import "./styles/tokens.css";
import "./styles/site.css";

document.documentElement.classList.add("js");
```

- [ ] **Step 2: Write the stylesheet**

`site/src/styles/site.css`:

```css
/* ClutchFactor marketing site — direction A "The Tape" (docs/spec/marketing-site.md §4).
   Colors, space, radius and motion come from tokens.css (the app's). Only the web
   type scale and layout live here. */

:root {
  --font-sans: "Inter", -apple-system, "Segoe UI", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  --t-hero: clamp(40px, 6vw, 68px) / 1.02;
  --t-h2: clamp(30px, 3.6vw, 44px) / 1.08;
  --t-h3: 20px / 1.3;
  --t-lede: 19px / 1.5;
  --t-body: 17px / 1.6;
  --t-ui: 14px / 1.4;
  --t-data: 13px / 1.4;
  --t-micro: 11px / 1.3;
  --gutter: clamp(20px, 4vw, 40px);
  --container: 1200px;
  --section: clamp(64px, 9vw, 112px);
  --nav-h: 64px;
}

/* ---------- base ---------- */
*, *::before, *::after { box-sizing: border-box; }
html { scroll-behavior: smooth; -webkit-text-size-adjust: 100%; }
body {
  margin: 0;
  background: var(--bg0);
  color: var(--ink);
  font: 400 var(--t-body) var(--font-sans);
  -webkit-font-smoothing: antialiased;
}
img { max-width: 100%; height: auto; display: block; }
h1, h2, h3 { margin: 0; color: var(--ink-bright); font-weight: 600; text-wrap: balance; }
h2 { font: 600 var(--t-h2) var(--font-sans); letter-spacing: -0.025em; }
h3 { font: 600 var(--t-h3) var(--font-sans); letter-spacing: -0.01em; }
p { margin: 0; }
a { color: inherit; }
code { font: 500 var(--t-data) var(--font-mono); background: var(--bg2); padding: 1px 5px; border-radius: var(--r-sm); }
b { font-weight: 600; }
ul, ol { margin: 0; padding: 0; }

:focus-visible { outline: 2px solid var(--accent-bright); outline-offset: 2px; }

.skip {
  position: absolute; left: var(--s4); top: -100px; z-index: 100;
  background: var(--bg1); color: var(--ink-bright); padding: var(--s2) var(--s3);
  border-radius: var(--r-sm); border: 1px solid var(--line-strong);
}
.skip:focus { top: var(--s4); }

.container { max-width: var(--container); margin: 0 auto; padding-left: var(--gutter); padding-right: var(--gutter); }

/* ---------- type roles ---------- */
.eyebrow {
  font: 500 var(--t-micro) var(--font-mono);
  letter-spacing: 0.16em; text-transform: uppercase; color: var(--ink-dim);
}
.eyebrow + h1, .eyebrow + h2 { margin-top: var(--s3); }
.lede { font: 400 var(--t-lede) var(--font-sans); color: var(--ink); max-width: 54ch; }
h2 + .lede, h2 + .body { margin-top: var(--s4); }
.body { color: var(--ink-dim); max-width: 68ch; }
.link { color: var(--ink-bright); text-decoration: none; border-bottom: 1px solid var(--line-strong); padding-bottom: 2px; font: 500 var(--t-ui) var(--font-sans); }
.link:hover { border-color: var(--ink-bright); }
.footnote { font: 400 var(--t-data) var(--font-mono); color: var(--ink-dim); }

/* ---------- components ---------- */
.btn {
  display: inline-flex; align-items: baseline; gap: var(--s2);
  padding: 12px 18px; border-radius: var(--r-md); border: 1px solid transparent;
  font: 600 15px/1.2 var(--font-sans); text-decoration: none; cursor: pointer;
  transition: background-color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  min-height: 44px;
}
.btn small { font: 400 var(--t-data) var(--font-mono); opacity: 0.75; }
.btn--primary { background: var(--accent); color: #0b1118; }
.btn--primary:hover { background: var(--accent-bright); }
.btn--secondary { background: color-mix(in srgb, var(--bg1) 70%, transparent); border-color: var(--line-strong); color: var(--ink-bright); }
.btn--secondary:hover { border-color: var(--ink-dim); }
.btn--sm { padding: 8px 12px; font-size: 13.5px; min-height: 36px; }

.chip {
  display: inline-block; font: 400 var(--t-data) var(--font-mono); font-variant-numeric: tabular-nums;
  border-radius: var(--r-sm); text-decoration: none; white-space: nowrap;
}
.chip--evidence {
  color: var(--ink); border-bottom: 1px dashed var(--ink-dim); padding-bottom: 2px; border-radius: 0;
  transition: color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
}
.chip--evidence:hover { color: var(--ink-bright); border-color: var(--ink-bright); }
.chip--conf { background: var(--bg2); color: var(--ink-dim); padding: 3px 8px; }

.card {
  background: var(--bg1); border: 1px solid var(--line); border-radius: var(--r-md); padding: var(--s5);
  transition: box-shadow 1200ms var(--ease);
}
.card--loss { border-left: 2px solid var(--loss); }
.card.is-target { box-shadow: 0 0 0 2px var(--accent-bright); transition: none; }
.card__hd { display: flex; justify-content: space-between; align-items: flex-start; gap: var(--s3); margin-bottom: var(--s3); }
.card p { color: var(--ink); }
.card__chips { display: flex; flex-wrap: wrap; gap: var(--s3) var(--s4); margin-top: var(--s4); }
.card__note { color: var(--ink-dim); font-size: 15px; margin-top: var(--s4); }
.card__note a { color: var(--ink); }

.frame {
  margin: 0; background: var(--bg-tape); border: 1px solid var(--line-strong); border-radius: var(--r-lg);
  overflow: hidden; box-shadow: 0 24px 60px rgb(0 0 0 / 0.55);
}
.frame img { width: 100%; }
.frame figcaption { font: 400 var(--t-data) var(--font-mono); color: var(--ink-dim); padding: var(--s3) var(--s4); border-top: 1px solid var(--line); }
.frame--wide { margin-top: var(--s7); }

/* ---------- nav ---------- */
.nav {
  position: fixed; inset: 0 0 auto 0; z-index: 50; height: var(--nav-h);
  background: transparent; border-bottom: 1px solid transparent;
  transition: background-color var(--dur) var(--ease), border-color var(--dur) var(--ease);
}
.nav--solid { background: color-mix(in srgb, var(--bg0) 92%, transparent); border-color: var(--line); backdrop-filter: blur(10px); }
.nav__in { height: 100%; display: flex; align-items: center; gap: var(--s6); }
.brand { display: inline-flex; align-items: center; gap: 10px; text-decoration: none; color: var(--ink-bright); font-weight: 600; font-size: 15px; }
.brand__icon { width: 24px; height: 24px; border-radius: 6px; }
.nav__links { display: none; gap: var(--s6); margin-left: auto; }
.nav__links a { color: var(--ink-dim); text-decoration: none; font: 500 var(--t-ui) var(--font-sans); }
.nav__links a:hover { color: var(--ink-bright); }
.nav [data-nav-download] { margin-left: auto; }
@media (min-width: 768px) {
  .nav__links { display: flex; }
  .nav [data-nav-download] { margin-left: 0; }
}

/* ---------- hero ---------- */
.hero { position: relative; display: flex; flex-direction: column; min-height: min(92svh, 860px); padding-top: calc(var(--nav-h) + var(--s8)); overflow: hidden; }
.hero__media { position: absolute; inset: 0; background: #05070a; }
.hero__poster, .hero__video {
  position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover;
  filter: saturate(0.55) contrast(1.15) brightness(0.55);
}
.hero__poster {
  background-image: url("/clips/poster.jpg"), url("/shots/radar-inferno.webp");
  background-size: cover, cover; background-position: center, center;
}
.hero__video { opacity: 0; transition: opacity 1000ms var(--ease); }
.hero__video.is-live { opacity: 1; }
.hero__media::after {
  content: ""; position: absolute; inset: 0;
  background:
    linear-gradient(180deg, rgb(14 17 22 / 0.35) 0%, rgb(14 17 22 / 0.15) 35%, rgb(14 17 22 / 0.85) 78%, #0e1116 100%),
    linear-gradient(90deg, rgb(14 17 22 / 0.55) 0%, rgb(14 17 22 / 0) 60%);
}
.hero__body { position: relative; z-index: 1; margin-top: auto; padding-bottom: var(--s8); max-width: min(var(--container), 100%); width: 100%; }
.hero__body > * + * { margin-top: var(--s4); }
.hero__title { font: 600 var(--t-hero) var(--font-sans); letter-spacing: -0.03em; max-width: 14ch; }
.hero .lede { margin-top: var(--s5); }
.cta { display: flex; flex-wrap: wrap; gap: var(--s3); margin-top: var(--s6); }
.cta .btn { flex: 1 1 100%; justify-content: center; }
.cta .btn--primary { order: -1; }
.evidence { display: flex; flex-wrap: wrap; align-items: baseline; gap: var(--s3) var(--s4); margin-top: var(--s6); font: 400 var(--t-data) var(--font-mono); color: var(--ink-dim); }
@media (min-width: 640px) { .cta .btn { flex: 0 1 auto; } }

/* ---------- sections ---------- */
.section { padding: var(--section) 0; }
.section--alt { background: var(--bg1); border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }
.section--alt .card { background: var(--bg0); }
.split { display: grid; grid-template-columns: 1fr; gap: var(--s7); }
@media (min-width: 1024px) { .split { grid-template-columns: 42fr 58fr; gap: var(--s8); align-items: start; } }
.split__text .link { display: inline-block; margin-top: var(--s5); }

/* ledger */
.ledger { margin-top: var(--s6); border-top: 1px solid var(--line); font-family: var(--font-mono); }
.ledger__hd { display: flex; justify-content: space-between; gap: var(--s3); padding: var(--s3) 0; font: 500 var(--t-micro) var(--font-mono); letter-spacing: 0.14em; text-transform: uppercase; color: var(--ink-dim); }
.ledger__row {
  display: grid; grid-template-columns: 44px 1fr; gap: var(--s3); padding: var(--s3) 0 var(--s3) var(--s3);
  border-top: 1px solid var(--line); border-left: 2px solid transparent;
  transition: opacity var(--dur) var(--ease), transform var(--dur) var(--ease);
}
.ledger__t { font: 400 var(--t-data) var(--font-mono); color: var(--ink-dim); font-variant-numeric: tabular-nums; }
.ledger__h { font: 500 15px/1.35 var(--font-sans); color: var(--ink-bright); }
.ledger__s { font: 400 var(--t-data) var(--font-mono); color: var(--ink-dim); margin-top: 3px; }
.ledger__row--win { border-left-color: var(--win); background: linear-gradient(90deg, var(--surface-win), transparent 60%); }
.ledger__row--loss { border-left-color: var(--loss); }
.ledger--armed .ledger__row:not(.is-in) { opacity: 0; transform: translateY(6px); }

/* catches */
.catches { display: grid; grid-template-columns: 1fr; gap: var(--s5); margin-top: var(--s7); }
@media (min-width: 1024px) { .catches { grid-template-columns: 1fr 1.4fr; align-items: start; } }
.cards { display: grid; gap: var(--s4); }
.bars { list-style: none; margin-top: var(--s4); display: grid; gap: var(--s3); }
.bars li { display: grid; grid-template-columns: 1fr auto; row-gap: 6px; font: 400 var(--t-data) var(--font-mono); color: var(--ink); }
.bars li b { font-weight: 500; font-variant-numeric: tabular-nums; }
.bars__bar { grid-column: 1 / -1; height: 4px; background: var(--bg2); border-radius: var(--r-full); position: relative; overflow: hidden; }
.bars__bar::after { content: ""; position: absolute; inset: 0; width: var(--w); background: var(--ink-dim); border-radius: inherit; }
.bars__bar--win::after { background: var(--win); }

/* coach facts, limits, pairs, download */
.facts { display: grid; grid-template-columns: 1fr; margin: var(--s7) 0 0; border-top: 1px solid var(--line); }
.facts > div { padding: var(--s4) 0; border-bottom: 1px solid var(--line); }
.facts dt { font-weight: 600; color: var(--ink-bright); }
.facts dd { margin: var(--s1) 0 0; color: var(--ink-dim); font-size: 15px; }
@media (min-width: 768px) {
  .facts { grid-template-columns: repeat(3, 1fr); border-top: 0; }
  .facts > div { padding: var(--s5); border-bottom: 0; border-left: 1px solid var(--line); }
  .facts > div:first-child { border-left: 0; padding-left: 0; }
}
.limits { display: grid; grid-template-columns: 1fr; gap: var(--s4); margin-top: var(--s7); }
@media (min-width: 768px) { .limits { grid-template-columns: 1fr 1fr; } }
.plain { list-style: none; margin-top: var(--s3); display: grid; gap: var(--s2); color: var(--ink); font-size: 15px; }
.plain--inline { display: flex; flex-wrap: wrap; gap: var(--s2); }
.plain--inline li { font: 400 var(--t-data) var(--font-mono); border: 1px solid var(--line-strong); border-radius: var(--r-sm); padding: 4px 8px; }
.pair { display: grid; grid-template-columns: 1fr; gap: var(--s5); margin-top: var(--s7); }
@media (min-width: 1024px) { .pair { grid-template-columns: 1fr 1fr; } }
.dl { display: grid; grid-template-columns: 1fr; gap: var(--s4); margin-top: var(--s7); }
@media (min-width: 768px) { .dl { grid-template-columns: 1fr 1fr; } }
.dl .card h3 { margin-bottom: var(--s4); }
.dl .btn { width: 100%; justify-content: center; }
.firstrun { margin-top: var(--s7); max-width: 68ch; }
.firstrun ol { margin-top: var(--s3); padding-left: 1.4em; display: grid; gap: var(--s2); color: var(--ink-dim); font-size: 15px; }
.firstrun ol b { color: var(--ink); }

/* footer */
.footer { padding: var(--s7) 0; border-top: 1px solid var(--line); }
.footer__in { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s4) var(--s6); }
.brand--small { font-size: 14px; }
.footer__links { display: flex; flex-wrap: wrap; gap: var(--s5); }
.footer__links a { color: var(--ink-dim); text-decoration: none; font: 500 var(--t-ui) var(--font-sans); }
.footer__links a:hover { color: var(--ink-bright); }
.footer__legal { flex-basis: 100%; font: 400 var(--t-micro) var(--font-mono); letter-spacing: 0.04em; color: var(--ink-faint); }

/* ---------- reveals (only when JS is present so nothing is ever hidden without it) ---------- */
.js [data-reveal] { opacity: 0; transform: translateY(8px); transition: opacity var(--dur) var(--ease), transform var(--dur) var(--ease); }
.js [data-reveal].is-in { opacity: 1; transform: none; }

/* ---------- reduced motion: static page, poster only, everything visible ---------- */
@media (prefers-reduced-motion: reduce) {
  html { scroll-behavior: auto; }
  *, *::before, *::after { transition: none !important; animation: none !important; }
  .js [data-reveal] { opacity: 1; transform: none; }
  .ledger--armed .ledger__row:not(.is-in) { opacity: 1; transform: none; }
  .hero__video { display: none; }
}
```

- [ ] **Step 3: Build and render at three widths**

```bash
pnpm -C site build && (pnpm -C site preview >/dev/null 2>&1 &) && sleep 2
B="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
for w in 375x812 768x1024 1440x900; do
  "$B" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=6000 --window-size=${w/x/,} \
       --screenshot="/tmp/site-$w.png" http://localhost:4173/ >/dev/null 2>&1
done
```

Open the three PNGs (Read tool) and check: no horizontal overflow; `h1` at 375 renders at 40 px on ≤ 3 lines; the primary button is visible without scrolling at 375×812 and 1440×900; the nav is legible over the radar fallback. Then stop the preview server (`pkill -f "vite preview"`).

Expected: no CSS fixes needed; if the hero body overflows at 375 wide, reduce `.hero` `padding-top` to `calc(var(--nav-h) + var(--s6))`.

- [ ] **Step 4: Lint, typecheck, tests**

Run: `pnpm -C site lint && pnpm -C site typecheck && pnpm -C site test:run`
Expected: green.

- [ ] **Step 5: Commit**

```bash
git add site/src/styles/site.css site/src/main.ts
git commit -m "feat(site): stylesheet — the Tape direction on the app's tokens"
```

---

### Task 9: DOM glue — CTA (TDD, happy-dom), hero clips, ledger sequence, reveals, nav

**Files:**
- Create: `site/src/cta.ts`, `site/test/cta.test.ts`, `site/src/hero.ts`
- Modify: `site/src/main.ts`

**Interfaces:**
- Consumes: `detectPlatform`, `release`/`assetUrl`/`formatMb`, `ledgerSchedule`, `parseClipsManifest`/`shouldPlayVideo`/`nextIndex`.
- Produces: `renderDownloadButtons(doc: Document, platform: Platform): void`, `applyNavDownload(doc: Document, platform: Platform): void`, `initHero(root: HTMLElement, clips: ClipEntry[]): void`.

- [ ] **Step 1: Write the failing CTA tests**

`site/test/cta.test.ts`:

```ts
// @vitest-environment happy-dom
import { beforeEach, describe, expect, it } from "vitest";
import { applyNavDownload, renderDownloadButtons } from "../src/cta";
import { assetUrl, release } from "../src/release";

beforeEach(() => {
  document.body.innerHTML = `
    <a data-nav-download href="#download">Download v1.0.0</a>
    <div data-download-buttons>
      <a class="btn btn--primary" data-os="windows" href="#">Download for Windows <small>x</small></a>
      <a class="btn btn--secondary" data-os="mac" href="#">Download for macOS <small>x</small></a>
    </div>`;
});

const btn = (os: string) => document.querySelector<HTMLAnchorElement>(`[data-os="${os}"]`)!;

describe("renderDownloadButtons", () => {
  it("makes the visitor's OS the primary button with the real asset and size", () => {
    renderDownloadButtons(document, "mac");
    expect(btn("mac").classList.contains("btn--primary")).toBe(true);
    expect(btn("windows").classList.contains("btn--secondary")).toBe(true);
    expect(btn("mac").href).toBe(assetUrl(release.mac.file));
    expect(btn("mac").querySelector("small")!.textContent).toBe(".dmg · Apple silicon · 10 MB");
    expect(btn("windows").querySelector("small")!.textContent).toBe(".exe · 8 MB");
  });

  it("shows both as secondary on an unknown OS", () => {
    renderDownloadButtons(document, "other");
    expect(btn("mac").classList.contains("btn--secondary")).toBe(true);
    expect(btn("windows").classList.contains("btn--secondary")).toBe(true);
    expect(btn("mac").classList.contains("btn--primary")).toBe(false);
  });
});

describe("applyNavDownload", () => {
  it("points the nav button at the visitor's installer", () => {
    applyNavDownload(document, "windows");
    expect(document.querySelector<HTMLAnchorElement>("[data-nav-download]")!.href).toBe(assetUrl(release.win.file));
  });
  it("keeps #download for unknown OS", () => {
    applyNavDownload(document, "other");
    expect(document.querySelector<HTMLAnchorElement>("[data-nav-download]")!.getAttribute("href")).toBe("#download");
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm -C site test:run test/cta.test.ts`
Expected: FAIL — cannot resolve `../src/cta`.

- [ ] **Step 3: Implement `cta.ts`**

`site/src/cta.ts`:

```ts
import type { Platform } from "./platform";
import { assetUrl, formatMb, release } from "./release";

const label = {
  mac: () => `.dmg · ${release.mac.arch} · ${formatMb(release.mac.bytes)}`,
  windows: () => `.exe · ${formatMb(release.win.bytes)}`,
};
const file = { mac: release.mac.file, windows: release.win.file };

/** Primary = the visitor's OS; the other OS is secondary; unknown OS → both secondary. */
export function renderDownloadButtons(doc: Document, platform: Platform): void {
  for (const os of ["mac", "windows"] as const) {
    const a = doc.querySelector<HTMLAnchorElement>(`[data-download-buttons] a[data-os="${os}"]`);
    if (!a) continue;
    a.href = assetUrl(file[os]);
    const small = a.querySelector("small");
    if (small) small.textContent = label[os]();
    const primary = platform === os;
    a.classList.toggle("btn--primary", primary);
    a.classList.toggle("btn--secondary", !primary);
  }
}

export function applyNavDownload(doc: Document, platform: Platform): void {
  const a = doc.querySelector<HTMLAnchorElement>("[data-nav-download]");
  if (!a) return;
  if (platform === "mac" || platform === "windows") a.href = assetUrl(file[platform]);
}
```

- [ ] **Step 4: Run the CTA tests**

Run: `pnpm -C site test:run test/cta.test.ts`
Expected: PASS (4 tests).

- [ ] **Step 5: Implement `hero.ts` (DOM glue over the tested rules)**

`site/src/hero.ts`:

```ts
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
```

- [ ] **Step 6: Wire everything in `main.ts`**

`site/src/main.ts` (complete file):

```ts
import "./styles/tokens.css";
import "./styles/site.css";

import { parseClipsManifest, shouldPlayVideo } from "./clips";
import { applyNavDownload, renderDownloadButtons } from "./cta";
import { initHero } from "./hero";
import { ledgerSchedule } from "./ledger";
import { detectPlatform } from "./platform";

document.documentElement.classList.add("js");

const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

// Download buttons follow the visitor's OS.
const platform = detectPlatform(navigator.userAgent, navigator.maxTouchPoints);
renderDownloadButtons(document, platform);
applyNavDownload(document, platform);

// Nav turns solid once the hero starts scrolling away.
const nav = document.querySelector<HTMLElement>("[data-nav]");
const onScroll = () => nav?.classList.toggle("nav--solid", scrollY > 40);
addEventListener("scroll", onScroll, { passive: true });
onScroll();

// Hero clips — only when the environment rules allow it (spec §5).
const hero = document.querySelector<HTMLElement>("[data-hero]");
if (hero) {
  fetch("/clips/clips.json")
    .then((r) => (r.ok ? r.json() : null))
    .then((json) => {
      const clips = parseClipsManifest(json) ?? [];
      const connection = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection;
      if (shouldPlayVideo({ reducedMotion, saveData: connection?.saveData === true, viewportWidth: innerWidth, clips })) {
        initHero(hero, clips);
      }
    })
    .catch(() => undefined); // poster stays
}

// Scroll reveals + the ledger sequence, each once.
const io = new IntersectionObserver(
  (entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      io.unobserve(e.target);
      e.target.classList.add("is-in");
      const ledger = e.target.querySelector<HTMLElement>("[data-ledger]");
      if (ledger) playLedger(ledger);
    }
  },
  { rootMargin: "0px 0px -10% 0px" },
);
for (const el of document.querySelectorAll("[data-reveal]")) io.observe(el);

function playLedger(ledger: HTMLElement) {
  const rows = Array.from(ledger.querySelectorAll<HTMLElement>(".ledger__row"));
  if (reducedMotion) {
    for (const r of rows) r.classList.add("is-in");
    return;
  }
  ledger.classList.add("ledger--armed");
  const delays = ledgerSchedule(rows.map((r) => ({ t: r.dataset.t ?? "0:00" })));
  rows.forEach((r, i) => setTimeout(() => r.classList.add("is-in"), delays[i]));
}

// Hero evidence chips: jump to their insight card and ring it like a focus.
for (const chip of document.querySelectorAll<HTMLAnchorElement>(".chip--evidence[data-target]")) {
  chip.addEventListener("click", () => {
    const card = document.getElementById(chip.dataset.target ?? "");
    if (!card) return;
    card.classList.add("is-target");
    setTimeout(() => card.classList.remove("is-target"), 1200);
  });
}
```

Note: `ledger--armed` is added *before* the section's own `is-in` transition finishes, so rows start hidden and appear on schedule; without JS (`html` lacks `.js`) nothing is ever hidden.

- [ ] **Step 7: Typecheck, lint, all tests, build**

Run: `pnpm -C site typecheck && pnpm -C site lint && pnpm -C site test:run && pnpm -C site build`
Expected: green (≈ 35 tests).

- [ ] **Step 8: Verify the behaviour in a browser render**

```bash
(pnpm -C site preview >/dev/null 2>&1 &) && sleep 2
B="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
"$B" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=6000 --window-size=1440,900 --screenshot=/tmp/site-js.png http://localhost:4173/ >/dev/null 2>&1
"$B" --headless=new --disable-gpu --hide-scrollbars --force-prefers-reduced-motion --virtual-time-budget=6000 --window-size=1440,3200 --screenshot=/tmp/site-rm.png http://localhost:4173/ >/dev/null 2>&1
pkill -f "vite preview"
```

Open both PNGs. Expected: `/tmp/site-js.png` — hero over the radar fallback (the manifest is empty), Windows button primary (headless Chromium on macOS reports "Macintosh" → mac primary — either is fine, but it must be exactly one primary in the hero). `/tmp/site-rm.png` — all six ledger rows visible, all sections visible.

- [ ] **Step 9: Keep the spec truthful**

In `docs/spec/marketing-site.md` §3.2 replace "on arrival the matching card gets `.is-target` (2 px `--loss` edge → fades over 1.2 s)" with "on arrival the matching card gets `.is-target` for 1.2 s — a 2 px `--accent-bright` ring, the same treatment as keyboard focus, fading out".

- [ ] **Step 10: Commit**

```bash
git add site/src/cta.ts site/test/cta.test.ts site/src/hero.ts site/src/main.ts docs/spec/marketing-site.md
git commit -m "feat(site): OS-aware download buttons, hero clip rotation, ledger sequence, reveals"
```

---

### Task 10: Vercel config, CI job, CLAUDE.md + PROGRESS.md

**Files:**
- Create: `site/vercel.json`
- Modify: `.github/workflows/ci.yml` (add `site` job after `web`; extend the `secrets` grep exclusion), `CLAUDE.md:11,18-20,34-43,56`, `docs/PROGRESS.md` ("## Now")

- [ ] **Step 1: `site/vercel.json`**

```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "cleanUrls": true,
  "headers": [
    {
      "source": "/(clips|shots)/(.*)",
      "headers": [{ "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }]
    },
    {
      "source": "/(.*)",
      "headers": [
        { "key": "X-Content-Type-Options", "value": "nosniff" },
        { "key": "Referrer-Policy", "value": "strict-origin-when-cross-origin" }
      ]
    }
  ]
}
```

- [ ] **Step 2: CI — add the `site` job and widen the secrets exclusion**

In `.github/workflows/ci.yml`, after the `web` job (before the `# No key material…` comment) insert:

```yaml
  # The marketing site is a standalone package (ADR-0012); cheap, ubuntu only.
  site:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    defaults:
      run:
        working-directory: site
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: site/pnpm-lock.yaml
      - run: pnpm install --frozen-lockfile
      - run: pnpm typecheck
      - run: pnpm lint
      - run: pnpm test:run
      - run: pnpm build
```

In the `secrets` job change `':!pnpm-lock.yaml'` to `':!pnpm-lock.yaml' ':!site/pnpm-lock.yaml'` (lockfile integrity hashes are base64 and can, rarely, match the key shapes).

- [ ] **Step 3: CLAUDE.md (stay ≤ 120 lines; currently 60)**

- Line 11 (Stack): append ` · marketing site in `site/` (vanilla TS + CSS on Vite, Vercel — `docs/spec/marketing-site.md`, ADR-0012).`
- After line 20 (`pnpm typecheck && …`) add:
  `pnpm -C site dev                                     # marketing site; checks: pnpm -C site typecheck && pnpm -C site lint && pnpm -C site test:run && pnpm -C site build`
- In the architecture map (after the `fixtures/` line) add:
  `site/                          marketing site, standalone package (own lockfile): public/shots via \`pnpm -C site shots\`, release links in src/release.ts`
- Line 56 (Releases): after `then \`gh release edit vX.Y.Z --notes-file …\`` insert `; then update \`site/src/release.ts\` (version, asset names, byte sizes from \`gh release view vX.Y.Z --json assets\`) and re-run \`pnpm -C site shots\` if screenshots changed`.

Run `wc -l CLAUDE.md` → expected ≤ 64.

- [ ] **Step 4: PROGRESS.md**

Read `docs/PROGRESS.md` "## Now" and add one line under it:
`- Marketing site (`site/`, spec `docs/spec/marketing-site.md`, plan `docs/plans/site-marketing.md`): built; awaiting the owner's `vercel link` and hero clips.`

- [ ] **Step 5: Verify locally what CI will run, from a clean install**

```bash
rm -rf site/node_modules && pnpm -C site install --frozen-lockfile
pnpm -C site typecheck && pnpm -C site lint && pnpm -C site test:run && pnpm -C site build
pnpm lint && pnpm typecheck && pnpm test:run     # the app is untouched
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add site/vercel.json .github/workflows/ci.yml CLAUDE.md docs/PROGRESS.md
git commit -m "chore(site): vercel config, CI site job, CLAUDE.md map + release step"
```

---

### Task 11: Sign-off evidence, Lighthouse, link check, PR, owner handoff

**Files:**
- Create: `docs/design/walkthrough-site/README.md`, `docs/design/walkthrough-site/{375,768,1440,reduced-motion}.png`, `docs/design/walkthrough-site/lighthouse.md`

- [ ] **Step 1: Render the DoD screenshots into the repo (spec §8 items 2–3)**

```bash
mkdir -p docs/design/walkthrough-site
pnpm -C site build && (pnpm -C site preview >/dev/null 2>&1 &) && sleep 2
B="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"
"$B" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=6000 --window-size=375,812  --screenshot=docs/design/walkthrough-site/375.png  http://localhost:4173/ >/dev/null 2>&1
"$B" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=6000 --window-size=768,1024 --screenshot=docs/design/walkthrough-site/768.png  http://localhost:4173/ >/dev/null 2>&1
"$B" --headless=new --disable-gpu --hide-scrollbars --virtual-time-budget=6000 --window-size=1440,900 --screenshot=docs/design/walkthrough-site/1440.png http://localhost:4173/ >/dev/null 2>&1
"$B" --headless=new --disable-gpu --hide-scrollbars --force-prefers-reduced-motion --virtual-time-budget=6000 --window-size=1440,3400 --screenshot=docs/design/walkthrough-site/reduced-motion.png http://localhost:4173/ >/dev/null 2>&1
```

Open each PNG and confirm: no horizontal scroll (the page edge is flush at every width), `h1` ≥ 40 px and the primary button above the fold at 375 and 1440, nav legible, reduced-motion shows every ledger row.

- [ ] **Step 2: Lighthouse (spec §8 item 4) against the local preview**

```bash
CHROME_PATH="/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" \
  npx --yes lighthouse@12 http://localhost:4173/ --preset=desktop --quiet --chrome-flags="--headless=new" \
  --output=json --output-path=/tmp/lh.json
node -e 'const r=require("/tmp/lh.json").categories; for (const k in r) console.log(k, Math.round(r[k].score*100))'
pkill -f "vite preview"
```

Expected: performance ≥ 95, accessibility ≥ 95, best-practices ≥ 95, seo ≥ 95. Write the four numbers, the date and the command into `docs/design/walkthrough-site/lighthouse.md`. If accessibility < 95, the report's failing audit names the element — fix it in `index.html`/`site.css`, rebuild, re-run.

- [ ] **Step 3: Link check (spec §8 item 5)**

```bash
for u in $(grep -o 'https://github.com/junlee-3/clutchfactor/releases/download/[^"]*' site/index.html | sort -u); do
  printf '%s ' "$u"; curl -sIL -o /dev/null -w '%{http_code}\n' "$u"
done
```

Expected: `200` for the `.dmg`, `.exe` and `.msi` URLs.

- [ ] **Step 4: Write the walkthrough README**

`docs/design/walkthrough-site/README.md`:

```markdown
# Marketing site — sign-off (v1.0.0 site, <date>)

Spec: `docs/spec/marketing-site.md` §8. Renders from `pnpm -C site preview`
with Brave headless; the hero shows the radar fallback because no clips were
committed yet.

| check | evidence |
|---|---|
| 375×812: no horizontal scroll, h1 ≥ 40 px, primary button above the fold | `375.png` |
| 768×1024 | `768.png` |
| 1440×900: primary button above the fold, nav legible | `1440.png` |
| reduced motion: poster only, all ledger rows visible | `reduced-motion.png` |
| Lighthouse desktop | `lighthouse.md` |
| download URLs answer 200 | `.dmg`, `.exe`, `.msi` — curl -sIL, <date> |
| `pnpm -C site typecheck && lint && test:run && build` | green, <date> |
```

Fill in the dates.

- [ ] **Step 5: Commit, push, open the PR, arm auto-merge, verify**

```bash
git add docs/design/walkthrough-site
git commit -m "docs(site): sign-off renders, lighthouse, link check"
git push -u origin feat/site
gh pr create --title "Marketing site (site/): the Tape direction, Vercel-ready" --body-file - <<'PRBODY'
Implements docs/spec/marketing-site.md (ADR-0012): standalone Vite + TS page under site/,
the app's tokens (drift-tested), OS-aware download buttons, hero clip rotation with poster/radar
fallbacks, the round-2 play ledger sequence, screenshot pipeline, Vercel config, CI `site` job.

Sign-off: docs/design/walkthrough-site/ (renders at 375/768/1440, reduced motion, Lighthouse, link check).

Owner follow-ups: `vercel login && vercel link` (root dir `site`), record hero clips (site/public/clips/README.md).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01QZVCymMMLtRoPcBUyYoXN7
PRBODY
gh pr merge --auto --squash
sleep 60; gh pr view --json state,mergeStateStatus,statusCheckRollup -q '{state: .state, merge: .mergeStateStatus, checks: [.statusCheckRollup[] | {name: .name, status: .conclusion}]}'
```

Expected: checks `rust`, `windows-build`, `web`, `site`, `secrets` all `SUCCESS`; state `MERGED` (or `OPEN` with `mergeStateStatus: CLEAN` until checks finish — re-run the view; if `BEHIND`, `gh pr update-branch` and check again).

- [ ] **Step 6: Hand off the owner-only steps (do not automate)**

Tell the owner, verbatim:

> Merged. Two things only you can do:
> 1. `! cd site && vercel login && vercel link` — when asked, pick the existing team/scope, project name `clutchfactor`, and confirm the Root Directory is `site` in the Vercel dashboard (Settings → General). Then `vercel --prod` once, or just push to main — the Git integration deploys from then on. If the production URL isn't `clutchfactor.vercel.app`, tell me and I'll update `og:image` in `site/index.html`.
> 2. Record 3–5 clips per `site/public/clips/README.md`, drop them + `poster.jpg` in that folder, list them in `clips.json`, and open a PR (or tell me and I'll do the PR).

---

## Self-review (done while writing)

- **Spec coverage:** §1 voice/real-content → Task 7 copy + html test; §2 layout/standalone/tokens → Tasks 1, 3; §3.1–3.9 → Task 7 (+ Task 9 for the chip `.is-target` and nav OS logic); §4 tokens/type/components/motion/breakpoints → Task 8; §5 clips/env/fallbacks/manifest → Tasks 5, 9, cache headers Task 10; §6 release/platform → Task 2, buttons Task 9; §7 Vercel/CI/no analytics → Task 10 (+ owner handoff Task 11); §8 tests → Tasks 2–5, 7, 9; DoD 1–7 → Tasks 10, 11 (CLAUDE.md in Task 10); §9 out of scope respected; §10 owner items → Task 11 step 6.
- **Placeholders:** none; the only `<date>` tokens are in the sign-off README, filled at execution.
- **Type consistency:** `detectPlatform(userAgent, maxTouchPoints?)` (Task 2) is what `main.ts` calls (Task 9); `ledgerSchedule(rows: { t }[])` (Task 4) receives `{ t: r.dataset.t }` (Task 9); `parseClipsManifest`/`shouldPlayVideo`/`nextIndex`/`ClipEntry` (Task 5) are the names `hero.ts`/`main.ts` import; `renderDownloadButtons(doc, platform)`/`applyNavDownload(doc, platform)` (Task 9) match the tests; state classes in `site.css` (Task 8) match the ones `main.ts`/`hero.ts` toggle (`js`, `nav--solid`, `hero--video`, `is-live`, `ledger--armed`, `is-in`, `is-target`).
