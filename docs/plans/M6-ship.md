# M6 — Trends, Settings, polish, ship v0 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship installable v0: Trends screen (detector metrics across matches), Settings (tracked identity + thresholds view + match deletion), empty/error states, app icon, README with screenshots, tagged release with Windows installer + macOS app.

**Architecture:** Trends is a read-only screen over two new lean store readers (per-match class-13 share + per-match rule counts), assembled by one `get_trends` command and rendered with pure TS spark/streak math (TDD) + inline SVG. Settings writes the existing `settings` table (tracked override already honored by `tracked_steamid()`); deletion enables the honest re-analyze path (delete → re-import). Release via a tag-triggered GitHub Actions workflow building both platforms.

**Tech Stack:** existing (Tauri 2, Rust workspace, React 19 + TS + Vite, rusqlite, vitest). New: `tauri-apps/tauri-action` in CI for release builds; `pnpm tauri icon` for the icon set.

**Spec:** PROMPT.md §13 M6, §7 screens 4/6, §5A (Trends renders class shares + habit trends), §11 (README/docs), plus docs/PROGRESS.md M6 debt list (only the items named in tasks below are in scope — the rest wait for the owner's post-v0 change list).

## Global Constraints

- Owner directive: **ship v0 first** — no scope creep beyond this plan; deferred debt stays deferred.
- Steamids: u64 in Rust core, **strings at every JS/store boundary**.
- Corpus-blindness: every tracked-player query filters `kind='own'`.
- Thresholds via DetectorConfig only; `CLUTCHFACTOR_CONFIG` stays dev-only; Settings displays thresholds read-only in v0 (editing = post-v0).
- Charts: dataviz rules — one sequential hue for magnitude; CT `#4aa3ff` / T `#f5b83d` reserved for side identity; text wears text tokens (`--text`, `--text-dim`), never series color; no dual axes; no rainbow.
- §7 voice: errors say what happened and what to do next, never vague; empty screens invite action.
- Conventional commits; push per task; CI green; milestone tag `m6` + release tag `v0.1.0`.
- Skills: `frontend-design:frontend-design` + `dataviz` MUST be invoked via the Skill tool before Trends/Settings UI code (Task 4/5).
- DoD (PROMPT §13): owner installs the Windows build from CI artifacts on their gaming PC and analyzes a fresh match unassisted — the final handoff message must include install instructions + the outstanding 8-pro-demo corpus ask.

---

### Task 1 (inline): correctness debt that gates shipping

**Files:** `src-tauri/crates/cf-store/src/store.rs`, `src-tauri/src/commands.rs`, `src/screens/Library.tsx`, `src/screens/Corpus.tsx`, `src/lib/basename.ts` (create) + `src/lib/basename.test.ts`.

1. **Windows-safe basename** (the Windows build ships this milestone): create `src/lib/basename.ts`:
   ```ts
   /** Last path segment for display — handles both / and \ (Windows). */
   export function basename(path: string): string {
     const seg = path.split(/[\\/]/).filter(Boolean);
     return seg[seg.length - 1] ?? path;
   }
   ```
   Test: `basename("C:\\demos\\a.dem") === "a.dem"`, `basename("/x/y/b.dem") === "b.dem"`, `basename("plain.dem") === "plain.dem"`. Replace both `path.split("/").pop() ?? path` sites (Library.tsx, Corpus.tsx) with `basename(path)`.
2. **`rule_severity_confidence` kind filter** (defense in depth): add `JOIN matches m ON m.id = rf.match_id AND m.kind = 'own'` to its SQL; extend `corpus_matches_are_invisible_to_tracked_analytics` to assert a corpus-side rule flag would not surface (insert one rule_flag row for a corpus match id inside the test, assert the fn result is unchanged).
3. **`positions_at` round bound** (disconnect ghosts): add parameter `min_tick: i32`; subquery gains `AND t2.tick >= ?3`. Callers pass the round's `start_tick` (both build_corpus and run_positioning have the RoundInfo in hand). Update the nearest-≤ test: a sample at tick 1100 must NOT satisfy `positions_at(id, 2150, 2000)` for that player.
- [x] TDD each; `cargo test -p cf-store && cargo test -p cf-analysis && pnpm vitest run`; fmt/clippy/tsc/eslint; commit `fix(store+ui): kind-filter severity reader, round-bounded positions, portable basename`, push.

### Task 2 (SUBAGENT, worktree): trend readers + get_trends command

**Files:** `src-tauri/crates/cf-store/src/store.rs` (+tests), `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (register), `src/lib/ipc.ts`, `src/lib/queries.ts`.

**Interfaces (frozen):**
```rust
// store.rs — own matches only, chronological (imported_at ASC, id ASC):
pub struct TrendMatchRow { pub match_id: i64, pub imported_at: String, pub map: String,
  pub deaths: u32 /* tracked player deaths (kills.victim = tracked) */,
  pub class13_pct: f32 /* 100 * class 13 / death_class rows for this match; 0.0 when no rows */ }
pub fn trend_matches(&self, tracked: &str) -> Result<Vec<TrendMatchRow>, StoreError>;
pub struct RuleTrendCell { pub match_id: i64, pub rule_id: String, pub count: u32 }
pub fn rule_trend_counts(&self, tracked: &str) -> Result<Vec<RuleTrendCell>, StoreError>;
// (rule_flags for the tracked victim, grouped by match+rule, own matches only)

// commands.rs:
#[derive(serde::Serialize)] pub struct TrendsDto {
  pub matches: Vec<cf_store::store::TrendMatchRow>,
  pub rules: Vec<RuleSeries>, }
#[derive(serde::Serialize)] pub struct RuleSeries {
  pub rule_id: String, pub title: String /* cf_narrator::narrate_habit(rule_id, matches_hit, window, total, &json!({})).title — same helper get_habits uses (commands.rs:457); pass real matches-hit/window/total from the series */,
  pub counts: Vec<u32> /* aligned to matches order, 0-filled */, pub total: u32 }
#[tauri::command] pub fn get_trends(state: State<'_, AppState>) -> Result<TrendsDto, String>;
// rules sorted by total desc, capped to the 8 largest totals; rules with total < 2 dropped (single events are noise — §7).
```
TS mirrors `TrendMatchRow`/`RuleSeries`/`TrendsDto` in ipc.ts (+ MIRROR CHECKLIST lines), `getTrends()` wrapper, `useTrends()` hook in queries.ts (queryKey `["trends"]`).
- [x] TDD store readers (two matches with known kills/death_class rows → exact deaths/class13_pct/counts; corpus match excluded); command compiles; `cargo test -p cf-store`, tsc/eslint; commit `feat(store+app): trend series readers + get_trends command`, push (coordinator merges).

### Task 3 (SUBAGENT, worktree): pure trends math

**Files:** `src/lib/trends.ts` (create), `src/lib/trends.test.ts`.

```ts
export interface SparkPoint { x: number; y: number }
/** Map a series to SVG points in a w×h box, y inverted (0 at bottom), padding p.
 *  Constant series renders mid-height; empty series → []. */
export function sparkPoints(values: number[], w: number, h: number, p?: number): SparkPoint[];
/** Trailing strictly-monotonic run length (≥2 means a streak; direction -1 down, +1 up, 0 none).
 *  streak([5,4,3,2]) = {len: 4, dir: -1}; streak([1,1,2]) = {len: 2, dir: +1}; streak([2]) = {len: 1, dir: 0}. */
export function streak(values: number[]): { len: number; dir: -1 | 0 | 1 };
/** §7 copy: "Isolated deaths trending down 4 matches straight" — null when len < 3.
 *  Down-streak on a bad-thing metric is good news: prefix "Good news: " when dir < 0. */
export function streakCallout(title: string, values: number[]): string | null;
```
- [x] TDD (exact cases above + sparkPoints geometry for [0,10] in 100×20 → first point at y=18 with p=2, last at y=2); vitest + tsc + eslint; commit `feat(ui): trends spark/streak math`, push (coordinator merges).

### Task 4 (inline): Trends screen

**Files:** `src/screens/Trends.tsx` (create), `src/App.tsx` (route `/trends`), `src/components/TopNav.tsx` (create — shared nav: Library · Trends · Corpus · Settings; replace ad-hoc topnav in Library/Corpus), `src/styles.css`.
**Before code: invoke `frontend-design:frontend-design` and `dataviz` (Skill tool).**
Layout (§7 screen 4, §5A): header; class-13 share line ("share of deaths that were pure aim duels" — magnitude, one hue, inline SVG polyline over match index, direct-labeled last value); per-rule spark rows (rule title, sparkline from `sparkPoints`, total, streak callout from `streakCallout` in `--text-dim`); per-map split chips (matches count per map, click filters the series client-side); empty state ("Trends need at least 2 matches — import more demos."). No dual axes; hover tooltip per spark row showing per-match counts (title attr is acceptable v0).
- [x] Build; typecheck/lint/vitest; AX-verify vs real DB (5 matches: e.g. H2_FAILED_TRADE series totals match `SELECT` by hand); commit `feat(ui): Trends screen — rule sparklines, class-13 share, streaks`, push.

### Task 5 (inline): Settings screen + match deletion

**Files:** `src-tauri/crates/cf-store/src/store.rs` (+ `delete_match`), `src-tauri/src/commands.rs` (+ `delete_match`, `set_tracked_override`, `get_app_settings`), `src-tauri/src/lib.rs`, `src/lib/ipc.ts`, `src/lib/queries.ts`, `src/screens/Settings.tsx` (create), `src/App.tsx` (route `/settings`), `src/screens/Library.tsx` (per-row Delete), `src/styles.css`.

```rust
// store.rs: pub fn delete_match(&mut self, id: i64) -> Result<(), StoreError>;
// single DELETE FROM matches WHERE id=?1 — children cascade (FK ON DELETE CASCADE, pragma already ON).
// Test: save two matches + analysis, delete one → its kills/death_class/rule_flags/insights gone, other intact, file_hash reusable (has_file_hash false).
// commands.rs:
// get_app_settings() -> { tracked_override: Option<String> /* settings key 'tracked_steamid' */,
//                         tracked_effective: Option<String> /* store.tracked_steamid() */,
//                         db_path: String, thresholds_yaml: String /* serde of DetectorConfig defaults? NO — just include_str the doc §6.4 values is overkill; render key thresholds list built in Rust: (name, value, unit) triples for trade/flash/h3/h16/timing/corpus */ }
// set_tracked_override(steamid: Option<String>) -> () — validates 17-digit numeric when Some; writes/deletes settings key.
// delete_match(match_id: i64) -> () — refuses when match kind is 'corpus'? No: allow both; Corpus screen debt post-v0. Library only shows own.
```
Settings layout (§7 screen 6): identity card (effective tracked id + name, override input with Save/Clear, §7-voice note: "Applies to new imports. To re-analyze an existing match, delete it in the Library and import the demo again."); thresholds card (read-only table name/value/unit, note that v0 ships fixed defaults); data card (db path, total matches, corpus demos count). Library rows get a small Delete button (confirm via one inline "Delete? / Cancel" toggle, §7 voice, no browser confirm()).
- [x] TDD store delete; command compile + mirrors; AX-verify: set override to a roster mate → new import tracks them (or at minimum settings row updates + tracked chip changes after reload); delete a match → row gone, report/replay routes for it 404-safe; commit `feat(app+ui): Settings (identity override, thresholds view) + match deletion`, push.

### Task 6 (inline): error/empty states, app icon, branding

**Files:** `src-tauri/icons/*` (regenerate), `app-icon.svg` (create, repo root), `src/screens/*.tsx` (states pass), `src-tauri/src/commands.rs` (error copy), `src-tauri/tauri.conf.json` (productName/window title check).

1. Error copy pass (§7 voice, errors say what to do next): parse failure → `"Couldn't parse {file}: {err}. If this demo is from a different game or corrupted, re-download it and try again."`; duplicate import already reads well; wrong-extension guard in both pickers already filters `.dem`. Mid-parse crash recovery: verify (test exists?) that a failed parse leaves no partial rows — save_match runs after parse and is transactional; add store test `failed_import_leaves_no_rows` if missing (call save_match with data, roll back scenario n/a — instead assert import_demo error path adds no matches row by checking has_file_hash stays false; do it at store level: duplicate-hash error keeps rowcount).
2. Empty states: Report with zero insights ("Clean match — nothing recurring to coach. Play more and patterns will show."); Replay for missing match id; Trends <2 matches (Task 4); Corpus done (M5).
3. App icon: author `app-icon.svg` — dark rounded square `#151a21`, CT-blue crosshair with a spark polyline through it (the product: aim + trend), no text; run `pnpm tauri icon app-icon.svg` (writes src-tauri/icons full set); verify tauri.conf.json references them and `productName` is `ClutchFactor`.
- [x] vitest/tsc/eslint + cargo tests; dev-run visual check of icon in dock; commit `feat(app): v0 polish — error voice, empty states, app icon`, push.

### Task 7 (inline): release pipeline, README, E2E, ship

**Files:** `.github/workflows/release.yml` (create), `README.md` (rewrite), `docs/PROGRESS.md`, PROMPT.md §13 checkbox.

1. `release.yml`: trigger `on: push: tags: ['v*']`; matrix `macos-latest` (aarch64) + `windows-latest`; steps: checkout, pnpm setup (packageManager pin), Rust stable, protoc (arduino/setup-protoc — csgoproto needs it), `pnpm install`, `tauri-apps/tauri-action@v0` with `tagName: ${{ github.ref_name }}`, `releaseName: "ClutchFactor ${{ github.ref_name }}"`, draft `false`, artifacts attach dmg/app.tar.gz + nsis exe/msi.
2. README: what it is (coaching, not stats), screenshots (Library, Report, Replay, Corpus heatmap, Trends — `screencapture -l$(osascript window id)` from the dev app into `docs/screenshots/`), install (release links + "unsigned build" Gatekeeper/SmartScreen notes), dev setup (CLAUDE.md dev commands), fixtures note, §5A one-paragraph explanation of the taxonomy + honesty rules.
3. Full E2E sweep on the dev app: import fresh → report → chip → replay; trends numbers vs SQL; settings override; delete+reimport; corpus screen intact.
4. Docs: PROGRESS (M6 done, v0 shipped, post-v0 = owner change list), PROMPT §13 M6 checkbox, plan boxes. Tag `m6` + `v0.1.0`, push tags, watch release workflow to green, download + smoke the mac artifact locally (`hdiutil attach` + launch).
5. Final whole-branch review (fable) + ONE fix wave; then the owner handoff message: Windows install steps from CI artifacts (their DoD), the 8-pro-Mirage-demo corpus ask, and the invitation to send the step-by-step change list.
- [x] All checks green; commit `feat(release): v0 release workflow + README`, push, tag, ship.

---

## Self-review notes

- §13 M6 coverage: Trends (T2–4) ✓; Settings thresholds+identity (T5) ✓; empty/error states (T6, T4, T5) ✓; app icon (T6) ✓; tagged release win+mac (T7) ✓; README+screenshots (T7) ✓. DoD is owner-executed (Windows install) — handoff message carries it (T7.5).
- §5A "M6 Trends renders class shares and habit trends": class-13 share line + per-rule series (T2/T4) ✓.
- Debt folded: Windows basename, severity kind filter, positions_at bound (T1); re-analyze answered honestly via delete+reimport (T5). Everything else stays in PROGRESS debt for the owner's post-v0 list.
- Placeholder scan: clean — T2 titles resolved to the existing `cf_narrator::narrate_habit` API (same as get_habits).
- Type consistency: TrendMatchRow/RuleSeries/TrendsDto names match across store/commands/TS in T2 and consumption in T4; basename() consumed in T1's two screens; streak/sparkPoints signatures in T3 consumed by T4.
