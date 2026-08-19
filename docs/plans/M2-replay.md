# M2 — Replay Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 2D radar replay of any imported round at 60 fps — interpolated player dots, deaths, utility lifetimes, bomb state, kill feed, roster HP, scrubber with event pips, and `showEvidence()` deep links — plus a Windows CI build job (PROMPT.md §13 M2 DoD: watch a full real round smoothly at 60 fps; jump to any kill from the timeline).

**Architecture:** cf-store gains read models (`MatchDetail`, `RoundTicks` — columnar) served by two new commands. Radar assets vendored from the awpy artifact bundle into `assets/maps/` (Vite `publicDir` pointed at `assets/`). All replay math (world→radar coords, interpolation, utility lifetime windows, timeline mapping) is pure TS in `src/replay/`, unit-tested exhaustively per §10.2. A canvas renderer draws from those pure functions inside one rAF loop.

**Tech Stack:** Canvas 2D (§3 — WebGL only if profiling demands), awpy maps artifact (build 17595823), existing TanStack Query/router.

**Spec:** PROMPT.md §6.3 (coordinate mapping), §7 screen 3 (replay features + accessibility floor), §10.4 (60 fps, scrub < 100 ms), §13 M2. Evidence contract §4: `EvidenceRef { round, tick_start, tick_end, focus_players, camera_hint }`.

## Global Constraints

- Conventional commits; push per task; CI green (§2.1). No demoparser2 types past cf-parser (§4). Steamids are strings over IPC (mirror checklist rule).
- Radar mapping (§6.3, verified against awpy data format): for 1024×1024 images, `img_x = (world_x - pos_x) / scale`, `img_y = (pos_y - world_y) / scale`; use `<map>_lower.png` when `z < lower_level_max_units` (present for nuke/train/vertigo/baggage; `-1000000.0` = no lower level).
- awpy artifact source: `https://awpycs.com/17595823/maps.zip` → 17 PNGs + `map-data.json` (`{pos_x:int, pos_y:int, scale:float, rotate:int|null, zoom:float|null, lower_level_max_units:float}` per map). `rotate` is 0/null in current data — assert-ignore with a comment.
- Utility lifetimes (frontend, from grenade events): smoke = detonate → paired `smoke_expired` (nearest within 25 s and 150 u), fallback 19.5 s; molly = `molotov_start` → paired `molotov_expire` (same pairing), fallback 7 s; flash/he = 0.5 s visual pop.
- UI-milestone skills (§7): invoke `frontend-design:frontend-design` before the replay screen (Task 6) and `dataviz` before the scrubber/timeline (same task, before writing timeline code).
- Keyboard floor: Space play/pause, ←/→ seek ±2 s (Shift = ±10 s), visible focus on scrubber.

---

### Task 1: Parser — smoke expiry events

**Files:** Modify `src-tauri/crates/cf-parser/src/extract.rs` (add `smokegrenade_expired` to `WANTED_EVENTS`, map to kind `"smoke_expired"` in `extract_events`); regenerate both goldens (grenade counts change).

- [ ] Add the event; `cargo test -p cf-parser` (goldens fail) → regenerate via `print_match --golden` for both fixture demos → tests green.
- [ ] Commit `feat(parser): smoke expiry events for utility lifetimes`, push.

### Task 2: cf-store read models — MatchDetail + RoundTicks

**Files:** Modify `src-tauri/crates/cf-store/src/store.rs`; tests extended in-module.

**Interfaces (produces — mirrored in Task 3):**

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundInfo { pub number: u32, pub start_tick: i32, pub freeze_end_tick: Option<i32>,
    pub end_tick: i32, pub officially_ended_tick: Option<i32>, pub winner: String, pub reason: String }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct KillInfo { pub round: u32, pub tick: i32, pub attacker: Option<String>, pub victim: String,
    pub assister: Option<String>, pub weapon: String, pub headshot: bool }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GrenadeInfo { pub tick: i32, pub kind: String, pub thrower: Option<String>,
    pub x: f32, pub y: f32, pub z: f32 }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BombInfo { pub tick: i32, pub kind: String, pub player: Option<String> }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PlayerInfo { pub steamid: String, pub name: String }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchDetail { pub id: i64, pub map: String, pub tickrate: f32, pub sample_every: u32,
    pub score_a: u32, pub score_b: u32, pub players: Vec<PlayerInfo>, pub rounds: Vec<RoundInfo>,
    pub kills: Vec<KillInfo>, pub grenades: Vec<GrenadeInfo>, pub bomb_events: Vec<BombInfo>,
    pub round_sides: Vec<RoundSideInfo> }

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundSideInfo { pub number: u32, pub steamid: String, pub side: String }

/// Columnar, sorted by tick then steamid; range = [round.start_tick, officially_ended ?? end_tick].
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct RoundTicks { pub tick: Vec<i32>, pub steamid: Vec<String>, pub x: Vec<f32>, pub y: Vec<f32>,
    pub z: Vec<f32>, pub yaw: Vec<f32>, pub health: Vec<i32>, pub is_alive: Vec<bool>,
    pub team_num: Vec<i32>, pub active_weapon: Vec<Option<String>>, pub last_place: Vec<Option<String>> }

impl Store {
    pub fn match_detail(&self, id: i64) -> Result<Option<MatchDetail>, StoreError>;
    pub fn round_ticks(&self, id: i64, round: u32) -> Result<RoundTicks, StoreError>;
}
```

- [ ] Tests first (extend existing synthetic `sample_match`): `match_detail` returns players/rounds/kills/sides correctly, `None` for unknown id; `round_ticks` returns only in-range rows sorted by tick. Red → implement → green.
- [ ] Commit `feat(store): MatchDetail + RoundTicks read models`, push.

### Task 3: Commands + TS mirrors

**Files:** Modify `src-tauri/src/commands.rs` (+ register in `lib.rs`), `src/lib/ipc.ts`.

```rust
#[tauri::command] pub fn get_match_detail(state: State<'_, AppState>, matchId: i64) -> Result<Option<MatchDetail>, String>;
#[tauri::command] pub fn get_round_ticks(state: State<'_, AppState>, matchId: i64, round: u32) -> Result<RoundTicks, String>;
```

(Note: Rust arg names snake_case — `match_id` — arrive camelCased from JS.)

- [ ] Implement, mirror all structs in `ipc.ts` under the MIRROR CHECKLIST, add invoke wrappers + `useMatchDetail`/`useRoundTicks` queries. `cargo check` + `pnpm typecheck` green.
- [ ] Commit `feat(app): match detail + round tick commands`, push.

### Task 4: Radar assets vendored (ADR-0004)

**Files:** Create `assets/maps/*.png` + `assets/maps/map-data.json` + `assets/maps/ATTRIBUTION.md`; modify `vite.config.ts` (`publicDir: "assets"`); delete template `public/` svgs (move nothing — the template icons are unused); create `docs/adr/ADR-0004-radar-assets.md`.

- [ ] Copy the downloaded awpy bundle (scratchpad `awpy-maps/`) into `assets/maps/`; drop `workshop_preview`/`de_dust` (non-active, keep bundle lean) — keep all `de_*` active-duty + `_lower` variants + cs_italy/cs_office/ar_* (tiny, future-proof). ATTRIBUTION.md: awpy (MIT, pnxenopoulos/awpy), artifact build 17595823, radar imagery derived from CS2 game files © Valve.
- [ ] `vite.config.ts`: `publicDir: "assets"` so `/maps/de_mirage.png` serves at runtime; verify `pnpm build` includes them.
- [ ] ADR-0004: sourcing (awpy artifact mirror), licensing posture (MIT tooling; radar imagery is Valve-derived community-standard usage in a local, free tool; swappable per §6.3), update path (new patch → re-download artifact, diff map-data.json).
- [ ] Commit `feat(assets): vendor awpy radar images + calibration (ADR-0004)`, push.

### Task 5: Replay math — pure TS, exhaustive TDD

**Files:** Create `src/replay/coords.ts`, `src/replay/interp.ts`, `src/replay/utility.ts`, `src/replay/timeline.ts` + a `.test.ts` for each.

**Interfaces (produces):**

```ts
// coords.ts
export interface MapCalibration { pos_x: number; pos_y: number; scale: number; lower_level_max_units: number; }
export function worldToRadar(cal: MapCalibration, x: number, y: number): { u: number; v: number }; // 0..1024 image px
export function radarLayer(cal: MapCalibration, z: number): "upper" | "lower";
export function radarImageUrl(map: string, layer: "upper" | "lower"): string; // /maps/<map>.png | /maps/<map>_lower.png

// interp.ts — columnar RoundTicks pre-indexed per player
export interface PlayerTrack { steamid: string; ticks: Int32Array | number[]; x: number[]; y: number[]; z: number[];
  yaw: number[]; health: number[]; isAlive: boolean[]; teamNum: number[]; weapon: (string | null)[]; place: (string | null)[]; }
export function buildTracks(rt: RoundTicks): PlayerTrack[];
export interface PlayerState { x: number; y: number; z: number; yaw: number; health: number; isAlive: boolean; teamNum: number; weapon: string | null; place: string | null; }
export function stateAt(track: PlayerTrack, tick: number): PlayerState | null; // null before first/after last sample
// yaw interpolates along the shortest arc (359°→1° passes through 0°, not 180°)

// utility.ts
export interface UtilityWindow { kind: "smoke" | "molly" | "flash" | "he"; x: number; y: number; z: number; startTick: number; endTick: number; }
export function utilityWindows(grenades: GrenadeInfo[], tickrate: number): UtilityWindow[];
// smoke: detonate→nearest smoke_expired (≤25 s, ≤150 u, each expiry consumed once) else +19.5 s
// molly: molotov_start→nearest molotov_expire (same rule) else +7 s; flash/he: +0.5 s

// timeline.ts
export interface TimelineSpec { startTick: number; endTick: number; }
export function tickToFrac(spec: TimelineSpec, tick: number): number;   // clamped 0..1
export function fracToTick(spec: TimelineSpec, frac: number): number;
export function fmtClock(spec: TimelineSpec, tick: number, tickrate: number): string; // "1:23" elapsed
```

- [ ] Write failing tests per module first. Required cases: coords — known mirage point round-trips, y-axis inversion sign, nuke z above/below `lower_level_max_units`, no-lower-level maps always "upper"; interp — exact sample hit, mid-gap lerp, yaw wrap 350°→10° goes through 0°, health/weapon step (not lerped — nearest-previous), null outside range, dead player keeps last position with isAlive false; utility — paired expiry shortens smoke, unpaired falls back 19.5 s, two smokes near one expiry consume it once (nearest wins), molly pairing, flash pop window; timeline — frac round-trips, clamping, clock format at 0 and >60 s.
- [ ] Implement to green. `pnpm test:run` all pass.
- [ ] Commit `feat(replay): coordinate/interpolation/utility/timeline math (TDD)`, push.

### Task 6: Replay screen — canvas renderer, panels, scrubber, deep links

**Files:** Create `src/replay/Renderer.ts`, `src/screens/Replay.tsx`, `src/replay/ReplayCanvas.tsx`, `src/components/{RosterPanel,KillFeed,Scrubber}.tsx`, `src/lib/evidence.ts`; modify `src/App.tsx` (route `/replay/:matchId`), `src/screens/Library.tsx` (row click → replay), `src/styles.css`.

**Before coding:** invoke `frontend-design:frontend-design` (replay layout: radar is the hero, chrome recedes) and `dataviz` (scrubber/timeline + pips + HP bars are data displays).

**Structure:**
- `evidence.ts`: `interface EvidenceRef { round: number; tick_start: number; tick_end: number; focus_players: string[]; camera_hint?: string }` + `evidenceUrl(matchId, ev)` → `/replay/:id?round=&tick=&focus=` + `parseEvidenceParams(searchParams)`. This is the §4 evidence contract's frontend half — M3's insights will emit it.
- `Replay.tsx`: loads detail + round ticks (query keys `["match", id]`, `["ticks", id, round]`); owns playback state `{ tick, playing, speed }` in a ref-driven store (not React state per frame); URL params seed round/tick/focus; round selector strip.
- `ReplayCanvas.tsx`: rAF loop — advance tick by `tickrate * speed * dt` while playing; draws via `Renderer.draw(ctx, scene)`; devicePixelRatio-scaled; radar images preloaded (`Image`) per layer; FPS meter (rolling 60-frame average) rendered as a `<span data-testid="fps">` dev overlay so verification can read it from the accessibility tree.
- `Renderer.ts` draw order: radar → utility (smoke: filled circle r≈145 u/scale with remaining-life ring; molly: orange pulsing area r≈120 u; flash: brief white pop) → bomb (planted: pulsing icon at planter position sample) → dead markers (× at death position, fade over 3 s, persistent small ×) → players (team-colored dot r 7 px, view wedge 30°, name label 10 px, focus dimming: non-focus at 35 % alpha when focus set) → HUD text. CT `#4aa3ff`, T `#f5b83d` (§7 — the two loud hues live here).
- `Scrubber.tsx`: track with round-relative progress, kill pips (click → seek to `tick - 2 s`), bomb pips; `role="slider"` + `aria-valuenow`, keyboard as in constraints; seek must apply within one frame (<100 ms budget §10.4).
- `KillFeed.tsx`: kills with `tick <= current`, newest 5, attacker→victim colored by side that round, headshot glyph; each row clickable → seek (DoD: jump to any kill).
- `RosterPanel.tsx`: per-side player rows — name, HP bar (width = health, side hue at 70 %), dead = struck through; weapon name small.
- Library row click → `navigate(/replay/{id})`.

- [ ] Build it; `pnpm typecheck && pnpm lint && pnpm test:run` green.
- [ ] Commit `feat(replay): canvas replay viewer with scrubber, kill feed, deep links`, push.

### Task 7: Windows CI build job

**Files:** Modify `.github/workflows/ci.yml`.

```yaml
  windows-build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: arduino/setup-protoc@v3
        with: { repo-token: "${{ secrets.GITHUB_TOKEN }}" }
      - uses: Swatinem/rust-cache@v2
        with: { workspaces: src-tauri }
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - run: pnpm build
      - run: cargo build --workspace --locked
        working-directory: src-tauri
```

- [ ] Add job, push, `gh run watch` → green (first Windows cargo build is slow; cache warms it).
- [ ] Commit is the push above (`ci: windows build job`).

### Task 8: E2E verification, docs, tag m2

- [ ] Full suite (fmt/clippy/cargo test incl. goldens, typecheck/lint/vitest all green).
- [ ] Re-import demos into a fresh dev DB (smoke_expired events now needed): delete `~/Library/Application Support/com.clutchfactor.app/clutchfactor.db*`, launch app, import mirage-tie + inferno-win + dust2-loss through the UI (AX scripting), re-set tracked_steamid setting.
- [ ] Watch a full real round: play round 1 of mirage-tie start→end at 1×; read FPS meter from AX tree ≥ ~58; scrub via keyboard; jump to a kill from the kill feed and from a pip; verify deep-link URL with `?round=&tick=&focus=` opens at the right moment with focus dimming; switch to de_nuke match and verify lower-level radar swap on Z.
- [ ] Sanity vs demo reality (§12): the round-1 opening kill in the viewer matches the kill feed (attacker/victim/location plausible on Mirage).
- [ ] Docs: PROGRESS.md (M2 done, gotchas), PROMPT.md §13 checkbox, CLAUDE.md if commands changed, plan checkboxes. Tag `m2`, push --tags, CI green.

---

## Self-review notes

- DoD coverage: radar+calibration wired w/ ADR (T4) ✓; playback dots/deaths/utility/bomb/killfeed/roster (T5–6) ✓; scrubber + pips (T6) ✓; `show_evidence` deep link end-to-end (T6 evidence.ts + URL params + focus dimming; exercised in T8) ✓; Windows CI job (T7) ✓; 60 fps + jump-to-kill verified (T8) ✓.
- Types consistent: `RoundTicks` columnar shape identical Rust↔TS; `EvidenceRef` field names match §4 exactly (snake_case in the contract).
- Flagged uncertainties: awpy `rotate` field (assert 0/null); smoke radius in world units (~144 u standard — verify visually against a known smoke lineup in T8); Tauri IPC payload size for RoundTicks (~1–2 MB — if slow, switch to `tauri::ipc::Response` binary later, noted not built).
- Deviation from §4 layout: assets served via Vite `publicDir: "assets"` — recorded in ADR-0004.
