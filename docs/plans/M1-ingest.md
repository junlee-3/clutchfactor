> **Historical record** of a completed milestone. The "push origin main" / "push per task" steps below predate ADR-0005 - `main` is PR-only now (branch, `gh pr create`, auto-merge). Do not copy the push flow from this file.

# M1 — Ingest Pipeline & Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import real demos through the UI into SQLite with streaming progress; Library screen lists them with persisted, correct match data (PROMPT.md §13 M1 DoD: import 3 real demos through the UI; relaunch persistence; golden snapshots committed).

**Architecture:** `cf-parser` gains the real `MatchData` model (players, normalized rounds, kills/blinds/grenades/bomb events, downsampled tick table) extracted in one demoparser2 pass. `cf-store` owns SQLite (rusqlite bundled) with embedded versioned migrations and save/list/settings APIs. The Tauri app wires `import_demo` (async, `tauri::ipc::Channel` progress) + query commands, with hand-mirrored TS types. React Library screen (TanStack Query over invoke) lists matches and hosts the import flow.

**Tech Stack:** demoparser2 pinned rev (unchanged), rusqlite (bundled), tauri-plugin-dialog v2, @tanstack/react-query, react-router.

**Spec:** `PROMPT.md` §4 (data flow, boundary rule), §6.2 (round quirks), §13 M1; `docs/spec/death-taxonomy.md` integration notes (schema must anticipate death_class/rule flags at M3 — M1 ships schema v1 *without* those tables; they arrive as migration 2 in M3, but cross-demo keys — steamid, map, demo id — are designed in now).

## Global Constraints

- Conventional commits; push after every task; CI stays green (PROMPT.md §2.1, §11.5).
- Boundary rule: no demoparser2 types escape cf-parser (§4). cf-store/cf-analysis/tauri app consume only `cf_parser::MatchData` types.
- No fake data; detectors absent in M1 — no placeholder insights anywhere (§2.4).
- Thresholds/config in seconds & world units (§5A conventions).
- TS types for every command payload hand-mirrored in `src/lib/ipc.ts` under a `// MIRROR CHECKLIST` comment naming the Rust source structs (tauri-specta still RC — re-evaluate at M2).
- Verified parser facts to build on (research spike 2026-08-19, `docs/PROGRESS.md` gotchas):
  - MM demos: `round_end` fields `round=U32`, `winner=String("CT"|"T")`, `reason=String("t_killed"|"ct_killed"|"bomb_defused"|…)`; `round_start.round=I32` 1-based; `round_officially_ended` fires ×2 at same tick.
  - GOTV demos: `round_end` `winner=I32(2=T|3=CT)`, `reason=I32` (7=defused, 9=T elim, 8=CT elim, 1=exploded, 12=time/save) + `message=String("#SFUI_…")`; **no** `round` field; **round 1 may lack `round_start` entirely** (recording starts mid-freeze); single `round_officially_ended`.
  - Match end marker: `cs_win_panel_match`. Halftime: `announce_phase_end` (don't rely on it — derive sides from `team_num` samples).
  - Per-tick friendly props (maps.rs verified): `X Y Z yaw pitch health is_alive team_num active_weapon spotted last_place_name armor_value balance flash_duration is_scoped`. Use `rm_user_friendly_names` to convert, and read output columns via `output.prop_controller.prop_infos` (id → `output.df[id]`, `prop_friendly_name`, rows aligned; special columns tick/steamid/name auto-added). Filter on `is_alive` before consuming per-tick fields.
  - One `Parser` pass may request events + player props together (python binding splits calls but ParserInputs carries both; verify in Task 2 and fall back to two passes if combined output misbehaves).

---

### Task 1: MatchData types + round normalization (pure, TDD)

**Files:**
- Create: `src-tauri/crates/cf-parser/src/model.rs` (public MatchData types)
- Create: `src-tauri/crates/cf-parser/src/rounds.rs` (normalization from a neutral RawRoundEvent stream)
- Modify: `src-tauri/crates/cf-parser/src/lib.rs` (`pub mod model; pub mod rounds;`)

**Interfaces:**
- Produces (consumed by Tasks 2–4):

```rust
// model.rs — the cf-parser boundary types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Side { Ct, T }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RoundEndReason { TKilled, CtKilled, BombDefused, BombExploded, TargetSaved, Other(String) }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Round {
    pub number: u32,                    // 1-based, normalized
    pub start_tick: i32,                // synthesized = prev officially_ended (or 0) when missing
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub officially_ended_tick: Option<i32>, // None on final round sometimes
    pub winner: Side,
    pub reason: RoundEndReason,
    pub ct_steamids: Vec<u64>,          // filled by Task 2 (tick-table side sampling)
    pub t_steamids: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlayerMeta { pub steamid: u64, pub name: String }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Kill {
    pub tick: i32, pub round: u32,
    pub attacker: Option<u64>, pub victim: u64, pub assister: Option<u64>,
    pub weapon: String, pub headshot: bool, pub penetrated: i32,
    pub thru_smoke: bool, pub attacker_blind: bool, pub assistedflash: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Blind { pub tick: i32, pub victim: u64, pub attacker: Option<u64>, pub duration: f32 }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrenadeEvent { pub tick: i32, pub kind: String, pub thrower: Option<u64>, pub x: f32, pub y: f32, pub z: f32 }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BombEvent { pub tick: i32, pub kind: String, pub player: Option<u64> }

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TickTable {
    pub sample_every: u32,              // e.g. 4 => ~16 Hz
    pub tick: Vec<i32>, pub steamid: Vec<u64>,
    pub x: Vec<f32>, pub y: Vec<f32>, pub z: Vec<f32>, pub yaw: Vec<f32>,
    pub health: Vec<i32>, pub is_alive: Vec<bool>, pub team_num: Vec<i32>,
    pub active_weapon: Vec<Option<String>>, pub spotted: Vec<bool>,
    pub last_place: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchData {
    pub map: String,
    pub tickrate: f32,
    pub players: Vec<PlayerMeta>,
    pub rounds: Vec<Round>,
    pub kills: Vec<Kill>,
    pub blinds: Vec<Blind>,
    pub grenades: Vec<GrenadeEvent>,
    pub bomb_events: Vec<BombEvent>,
    pub ticks: TickTable,
}
```

```rust
// rounds.rs — input is a neutral event stream so tests need no demoparser types
#[derive(Debug, Clone, PartialEq)]
pub enum RawRoundEvent {
    Start { tick: i32, round: Option<u32> },
    FreezeEnd { tick: i32 },
    End { tick: i32, winner: RawWinner, reason: RawReason },
    OfficiallyEnded { tick: i32 },
    WinPanelMatch { tick: i32 },
}
#[derive(Debug, Clone, PartialEq)]
pub enum RawWinner { Str(String), Num(i32) }
#[derive(Debug, Clone, PartialEq)]
pub enum RawReason { Str(String), Num(i32) }

pub fn normalize_rounds(events: &[RawRoundEvent]) -> Vec<Round>;
```

**Normalization algorithm (implement exactly):** sort by tick; dedup consecutive identical `(variant, tick)` pairs (MM duplicate `OfficiallyEnded`); iterate building one `Round` per `End` event — `number` = count so far + 1 (validate against `Start.round` when present; if they disagree, trust the sequence and keep going); `start_tick` = most recent `Start` after previous `End` (else previous `OfficiallyEnded`, else 0 for round 1 — the GOTV missing-first-start case); `freeze_end_tick` = latest `FreezeEnd` in (start, end); ignore any `End` after `WinPanelMatch`; drop rounds with zero duration (`end <= start` — restart artifacts). Winner decode: `Str("CT")→Ct, Str("T")→T, Num(3)→Ct, Num(2)→T`. Reason decode: strings `t_killed/ct_killed/bomb_defused/bomb_exploded/target_saved` map to variants (anything else → `Other(s)`); numbers `9→TKilled, 8→CtKilled, 7→BombDefused, 1→BombExploded, 12→TargetSaved`, else `Other(n.to_string())`. `ct_steamids`/`t_steamids` left empty here (Task 2 fills them).

- [x] **Step 1: Write failing unit tests in `rounds.rs`** covering: (a) clean MM stream (string winner/reason, round numbers, dup OfficiallyEnded → 24 rounds, correct winners); (b) GOTV stream (numeric winner/reason, no round numbers, missing first Start → round 1 start_tick 0); (c) End-after-WinPanelMatch dropped; (d) zero-duration round dropped; (e) sequence-vs-round-field disagreement keeps sequence. Build streams with small helper fns. Run `cargo test -p cf-parser` → compile FAIL (types/fn missing).
- [x] **Step 2: Implement `model.rs` + `normalize_rounds` per the algorithm above.** Run tests → PASS.
- [x] **Step 3: `cargo fmt`, `clippy -D warnings`, commit** `feat(parser): MatchData model + round normalization (MM + GOTV encodings)`, push.

### Task 2: Full extraction — `parse_match()` + goldens + downsampling measurement

**Files:**
- Create: `src-tauri/crates/cf-parser/src/extract.rs`
- Modify: `src-tauri/crates/cf-parser/src/lib.rs`, `examples/print_match.rs` (switch to MatchData summary output), `tests/golden_proof.rs` → extend/rename `tests/golden_match.rs`
- Create: `fixtures/goldens/mirage-tie.match.json`, `fixtures/goldens/navi-javelins-mirage.match.json` (compact golden: map, players, rounds incl. sides, score, counts of kills/blinds/grenades/bomb events, first+last kill, tick-table row count)
- Create: `docs/adr/ADR-0002-position-downsampling.md`

**Interfaces:**
- Produces: `cf_parser::extract::parse_match(path: &Path, sample_every: u32, progress: &mut dyn FnMut(ImportStage, f32)) -> Result<MatchData, ParseError>`
  - `pub enum ImportStage { Reading, Parsing, Normalizing }` and `#[derive(thiserror::Error)] pub enum ParseError { Io(String), Demo(String), Empty(String) }` (add `thiserror = "2"` to cf-parser).
  - `pub fn derive_score(rounds: &[Round]) -> (Vec<u64>, Vec<u64>, u32, u32)` — returns (roster_a, roster_b, wins_a, wins_b) where roster_a = CT side of round 1; per-round win attribution follows each roster through side swaps via that round's `ct_steamids`/`t_steamids`.
  - `pub fn detect_tracked_candidates(players_per_match: &[Vec<u64>]) -> Vec<u64>` — steamids ordered by appearance count (used by identity detection in Task 4; pure, unit-tested).

**Implementation notes (verified patterns):** one demoparser2 pass with `wanted_events = [round_start, round_freeze_end, round_end, round_officially_ended, cs_win_panel_match, player_death, player_blind, flashbang_detonate, smokegrenade_detonate, hegrenade_detonate, inferno_startburn, inferno_expire, bomb_planted, bomb_defused, bomb_exploded]` and `wanted_player_props = [X, Y, Z, yaw, health, is_alive, team_num, active_weapon, spotted, last_place_name]` converted via `rm_user_friendly_names` (build `real_name_to_og_name` from the zip, like the python binding); `parse_ents: true`. Read tick columns via `prop_infos` → `output.df[&info.id]` matched on `VarVec` variants; rows where `is_alive == false` keep only tick/steamid/team_num (positions of dead players are spectator junk — write defaults and rely on `is_alive` column). Downsample by `tick % sample_every == 0` at copy time. Round side assignment: for each round, take each player's modal `team_num` over samples in `[freeze_end, end_tick]` (3=CT, 2=T). Tickrate: from header `playback_ticks`/`playback_time` when present else 64.0. Events map to model structs via the existing `field_*` helpers (move them from `proof.rs` into a shared `events.rs` or `pub(crate)` module — keep `proof.rs` compiling or delete it and its golden in the same commit that replaces them with the richer match goldens; **do not keep two parallel proof paths**).

- [x] **Step 1: Write `extract.rs`** per notes; make `print_match` print a MatchData summary (players, per-round winners with sides, derived score, event counts). Run on `mirage-tie` — verify against known reality: 24 rounds, 12–12, misosoupy3 present, sides swap after round 12.
- [x] **Step 2: Downsampling measurement (ADR-0002).** Run extraction at `sample_every` ∈ {2, 4, 8} on mirage-tie; record tick-table row counts and `serde_json` byte size (proxy for storage). Write ADR-0002 choosing the rate (expect 4 ≈ 16 Hz per PROMPT §4 unless data argues otherwise; note replay interpolation budget from §10.4).
- [x] **Step 3: Unit tests** for `derive_score` (synthetic rounds with a halftime swap; OT swap pattern) and `detect_tracked_candidates`. Run → PASS.
- [x] **Step 4: Goldens.** Add `--golden` mode writing the compact summary struct (`MatchGolden` — define in `extract.rs` with exactly: map, tickrate, players sorted, rounds (number/winner/reason/freeze_end_tick/end_tick/ct+t steamid counts), score line, event counts, tick rows). Generate for mirage-tie (MM) and navi-javelins (GOTV). Gated golden test like M0's (skip when demo absent). Hand-validate: mirage 12–12 with misosoupy3 on the 12-win… (check in replaywatch reality: both halves 12 rounds each side); navi 13–10 matching M0 validation. Record in `fixtures/goldens/README.md`.
- [x] **Step 5: fmt/clippy/test, commit** `feat(parser): full MatchData extraction, score/side derivation, goldens (MM+GOTV)`, push.

### Task 3: cf-store — SQLite schema v1, migrations, save/list/settings

**Files:**
- Modify: `src-tauri/crates/cf-store/Cargo.toml` (`rusqlite = { version = "0.37", features = ["bundled"] }` — check latest, pin; `serde`, `serde_json`, `thiserror`; dev-dep `tempfile`)
- Create: `src-tauri/crates/cf-store/src/{migrations.rs, store.rs}`, `src-tauri/crates/cf-store/migrations/0001_schema_v1.sql`
- Create: `docs/adr/ADR-0003-db-schema-v1.md`

**Interfaces:**
- Produces (consumed by Task 4):

```rust
pub struct Store { /* rusqlite::Connection */ }
impl Store {
    pub fn open(db_path: &Path) -> Result<Store, StoreError>;      // runs pending migrations
    pub fn save_match(&mut self, file_name: &str, file_hash: &str, data: &MatchData) -> Result<i64, StoreError>; // tx; upsert-by-hash → returns match id; re-import same hash = error DuplicateImport
    pub fn list_matches(&self) -> Result<Vec<MatchSummary>, StoreError>;
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError>;
    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError>;
    pub fn tracked_steamid(&self) -> Result<Option<u64>, StoreError>; // setting override, else modal steamid across matches_players
}
pub struct MatchSummary {
    pub id: i64, pub file_name: String, pub map: String, pub imported_at: String,
    pub score_a: u32, pub score_b: u32,
    pub tracked_steamid: Option<u64>, pub tracked_result: Option<String>, // "win"|"loss"|"tie" from tracked player's roster
    pub tracked_kills: Option<u32>, pub tracked_deaths: Option<u32>, pub tracked_hs_pct: Option<f32>,
    pub rounds: u32,
}
```

**Schema v1 (0001_schema_v1.sql):** `schema_migrations(version PK, applied_at)`; `settings(key TEXT PK, value TEXT)`; `matches(id INTEGER PK AUTOINCREMENT, file_name, file_hash TEXT UNIQUE, map, tickrate, imported_at, sample_every, score_a, score_b, roster_a_json, roster_b_json)`; `players(match_id, steamid, name, PK(match_id,steamid))`; `rounds(match_id, number, start_tick, freeze_end_tick, end_tick, officially_ended_tick, winner, reason, PK(match_id,number))`; `round_sides(match_id, number, steamid, side, PK(match_id,number,steamid))`; `kills(id PK, match_id, round, tick, attacker, victim, assister, weapon, headshot, penetrated, thru_smoke, attacker_blind, assistedflash)`; `blinds(id PK, match_id, tick, victim, attacker, duration)`; `grenades(id PK, match_id, tick, kind, thrower, x, y, z)`; `bomb_events(id PK, match_id, tick, kind, player)`; `tick_samples(match_id, tick, steamid, x, y, z, yaw, health, is_alive, team_num, active_weapon, spotted, last_place, PK(match_id,steamid,tick)) WITHOUT ROWID`; indexes: `kills(match_id, victim)`, `kills(match_id, attacker)`, `tick_samples` PK covers the replay scan. Foreign keys ON; `PRAGMA journal_mode=WAL`. Migration runner: embedded via `include_str!`, table-driven `[(1, SQL)]`, applies in a transaction, records version — test: fresh open applies v1; reopen applies nothing; version table correct.

- [x] **Step 1: Failing tests first** (`store.rs` `#[cfg(test)]`, tempfile DBs): migration fresh/reopen; `save_match` with a small synthetic MatchData (2 players, 2 rounds incl. sides, 3 kills, 1 blind, tick rows) then `list_matches` returns correct summary incl. tracked K/D/hs%; duplicate hash → `DuplicateImport`; settings roundtrip; `tracked_steamid` modal fallback across two saved matches.
- [x] **Step 2: Implement migrations + store.** Tests PASS.
- [x] **Step 3: Write ADR-0003** (schema v1: tables, WITHOUT ROWID tick_samples choice, hash-dedup import, settings kv; death_class/rule-flag tables deliberately deferred to migration 2 at M3 with cross-demo keys already present via steamid+match_id+map).
- [x] **Step 4: fmt/clippy/test, commit** `feat(store): SQLite schema v1, migrations, save/list/settings`, push.

### Task 4: Tauri commands — import with Channel progress, list, identity

**Files:**
- Modify: `src-tauri/Cargo.toml` (deps: cf-parser, cf-store, tauri-plugin-dialog; `sha2` for file hash), `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json` (dialog permission)
- Create: `src-tauri/src/commands.rs`

**Interfaces:**
- Produces (mirrored by Task 5's `src/lib/ipc.ts`):

```rust
#[derive(Clone, serde::Serialize)]
pub struct ProgressEvent { pub stage: String, pub pct: f32, pub detail: String }
#[derive(serde::Serialize)]
pub struct ImportResult { pub match_id: i64, pub map: String, pub score_a: u32, pub score_b: u32 }

#[tauri::command] async fn import_demo(app: AppHandle, path: String, on_progress: tauri::ipc::Channel<ProgressEvent>) -> Result<ImportResult, String>;
#[tauri::command] fn list_matches(app: AppHandle) -> Result<Vec<cf_store::MatchSummary>, String>;
#[tauri::command] fn tracked_player(app: AppHandle) -> Result<Option<String>, String>; // steamid as string (JS u64 safety)
```

**Notes:** Verify `tauri::ipc::Channel` against docs.rs for the pinned tauri 2.x before coding (expected: command arg, `.send(T)`). DB path: `app.path().app_data_dir()?.join("clutchfactor.db")`; `Store` behind `tauri::State<Mutex<Store>>` initialized in `setup`. `import_demo` runs `parse_match` inside `tauri::async_runtime::spawn_blocking`, forwarding the progress closure into the Channel (stages: hashing 0–5 %, parsing 5–80 %, normalizing 80–90 %, saving 90–100 %). Steamids cross IPC as **strings** everywhere (JS Number cannot hold SteamID64 — this is a mirror-checklist rule). Register dialog plugin; keep the template `greet` removed in this task (delete the demo command + its UI usage).

- [x] **Step 1: Implement commands + wiring; `cargo check` the app crate.**
- [x] **Step 2: Manual smoke via `pnpm tauri dev`** console: `window.__TAURI__` invoke `list_matches` (empty array on fresh DB). (Frontend UI lands next task; this step just proves IPC + DB init.)
- [x] **Step 3: fmt/clippy/test, commit** `feat(app): import_demo with Channel progress, list_matches, tracked_player`, push.

### Task 5: Library screen + import flow (frontend)

**Files:**
- Create: `src/lib/ipc.ts` (mirrored types + typed invoke wrappers, MIRROR CHECKLIST comment), `src/lib/queries.ts` (TanStack hooks), `src/screens/Library.tsx`, `src/components/ImportProgress.tsx`
- Modify: `src/App.tsx` (router: `/` → Library), `src/main.tsx` (QueryClientProvider), `package.json` (`@tanstack/react-query`, `react-router-dom`, `@tauri-apps/plugin-dialog`)
- Create: `src/lib/score.ts` + `src/lib/score.test.ts` (pure: format result line from summary — "13–4 L · de_dust2" from tracked perspective; first vitest logic tests)

**Steps:**
- [x] **Step 1: Invoke `frontend-design:frontend-design` skill** (spec §7 mandate at UI-milestone start) and apply its direction to the Library layout: dark, calm, information-dense; CT `#4aa3ff`-ish / T `#f5b83d`-ish accents only; no charts yet (dataviz skill not needed until real charts appear).
- [x] **Step 2: TDD the pure bits:** `score.test.ts` (win/loss/tie from tracked roster membership; steamid-as-string handling) → implement `score.ts`.
- [x] **Step 3: Build `ipc.ts` + `queries.ts`** (`useMatches`, `useImportDemo` mutation wiring Channel `onmessage` → progress state, invalidate matches on success).
- [x] **Step 4: Library screen:** match list rows (map, date, score with W/L/T color, tracked K/D + HS%, rounds), empty state ("Import your first demo"), Import button → dialog plugin `open({ filters: [{ name: 'CS2 demo', extensions: ['dem'] }] })`, inline progress bar while importing, error surface (corrupt file → readable message).
- [x] **Step 5: `pnpm typecheck && pnpm lint && pnpm test:run`; commit** `feat(ui): Library screen with demo import + streaming progress`, push.

### Task 6: End-to-end verification, docs, tag m1

- [x] **Step 1 (superpowers:verification-before-completion):** full check suite (fmt/clippy/cargo test incl. goldens, typecheck/lint/vitest). Launch app (`run` skill / `pnpm tauri dev`): import `mirage-tie`, `inferno-win`, `dust2-loss` through the UI watching live progress; verify rows show correct map/score/K/D vs known values (24r 12–12 tie, 20r 13–7 W 18/14, 17r 13–4 L 6/17); relaunch app → all three persist; re-import same file → friendly duplicate error.
- [x] **Step 2: Docs:** CLAUDE.md dev commands (unchanged commands verified; add anything new), PROGRESS.md (M1 done, gotchas learned, Now → M2), PROMPT.md §13 M1 checkbox, goldens README rows for the two match goldens.
- [x] **Step 3: Commit, tag `m1`, push with tags. CI green confirmed before tagging.**

---

## Self-review notes

- §13 M1 DoD coverage: MatchData finalized (T1/T2) ✓; §6.2 quirks normalized + golden-tested for both demo origins (T1/T2) ✓; SQLite schema v1 + migrations (T3) ✓; import command with streaming progress (T4) ✓; Library screen with real imports (T5) ✓; player-identity detection (T2 `detect_tracked_candidates` + T3 `tracked_steamid` + T4 command) ✓; DoD imports/persistence/goldens (T6) ✓.
- Type consistency: `MatchData`/`Round`/`MatchSummary`/`ProgressEvent` names used identically across T1–T5; steamids-as-strings rule stated in both T4 and T5.
- Known uncertainties flagged for in-task verification, not guessed: `tauri::ipc::Channel` exact API; combined events+props single pass; rusqlite current version; header tickrate fields. Each has a fallback stated.
- §5A anticipation: schema v1 carries the cross-demo keys (steamid, match_id, map, tick) that death_class/rule-flag tables (migration 2, M3) will join on; tick_samples includes `spotted`/`active_weapon` per spec §5.2–5.3.
