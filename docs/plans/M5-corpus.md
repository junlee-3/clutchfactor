> **Historical record** of a completed milestone. The "push origin main" / "push per task" steps below predate ADR-0005 - `main` is PR-only now (branch, `gh pr create`, auto-merge). Do not copy the push flow from this file.

# M5 — Reference Corpus & D6 Positioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development for Tasks 2–3 (corpus/D6 pure math; heatmap renderer — independent, worktree implementers); superpowers:executing-plans inline for Tasks 0–1 and 4–7. Steps use checkbox (`- [ ]`) syntax.

**Goal:** User-supplied pro demos become a reference corpus; per map/side/phase occupancy grids power heatmaps and the D6 positioning detector (honesty rules + ≥8-demos-per-map gate); the Corpus screen manages it all (PROMPT.md §13 M5 DoD: with ~8 pro demos on one map, positioning comparison produces sane, replay-backed output on the owner's demo).

**Architecture:** Migration 4 adds `matches.kind` ('own'|'corpus') and every tracked-player analytics query filters kind='own' (M4 carry-in: corpus must not dilute habits). Corpus import reuses the existing parse+save pipeline with kind='corpus' and no analysis. Grid building is store-fed: positions at phase-sampled moments → pure `cf-analysis::corpus` functions produce 128×128 radar-space grids (world→radar via calibration embedded from assets/maps/map-data.json), cached in a `corpus_grids` table. D6 is a pure fn over the owner's stored round/tick data + grids, run on demand and at import when grids exist. Corpus screen: demo list per map, build button with progress, heatmap viewer (canvas over radar, sequential single-hue overlay per dataviz).

**Tech Stack:** existing crates; no new deps (grids are Vec<u32>, serialized as little-endian bytes into SQLite blobs).

**Spec:** PROMPT.md §5 D6 (honesty wording: "reference players rarely hold this position here" — unusualness, never wrongness; corpus gate default 8, configurable; silent below), §6.3 (radar mapping), §6.4 (128×128 grid), §7 screen 5, §13 M5. docs/spec/death-taxonomy.md §5A (evidence contract holds). M4 final-review carry-ins: (a) corpus demos must be excluded from habit windows/death_positions/list_matches; (b) H2 insight gains the non-following teammate so baited captions can name them.

## Global Constraints

- Skills: `superpowers:subagent-driven-development` invoked before dispatching Tasks 2–3; `frontend-design:frontend-design` + `dataviz` invoked before Task 5 UI/heatmap code.
- Detectors/grid math pure (no I/O); thresholds via DetectorConfig only; steamids strings at boundaries; conventional commits; push per task; CI green.
- D6 honesty (spec §5, verbatim intent): text says "reference players rarely hold this position here"; measures *unusualness not wrongness*; confidence ≤ 0.6; **silent when corpus < min_demos_per_map (default 8)**.
- Phases (config): freeze_end sample at freeze+1 s; early = freeze+10 s; mid = freeze+35 s (skipped if plant earlier); post_plant = plant+5 s. Sample only ALIVE players.
- Grid: 128×128 over radar space (world→radar via §6.3 calibration ÷ 8); calibration embedded at compile time from `assets/maps/map-data.json` (include_str! — packaged binaries keep it).
- Dev verification uses a lowered gate via config (documented as dev-only); the true DoD needs ~8 owner-supplied pro demos on one map — batched ask to the owner at milestone end (spec §10.1 anticipated this at M0).

---

### Task 0 (inline): Migration 4 — match kind + query filters (M4 carry-in a)

**Files:** `cf-store/migrations/0004_match_kind.sql`, `cf-store/src/{migrations.rs,store.rs}` (+tests).

Migration 4: `ALTER TABLE matches ADD COLUMN kind TEXT NOT NULL DEFAULT 'own';`
Store changes: `save_match(file_name, file_hash, kind: MatchKind, data)` (enum Own|Corpus, breaking signature — update all callers incl. tests); `list_matches` gains `WHERE kind='own'`; `rule_counts_across_matches`, `death_positions`, `flagged_rule_ids`, `tracked_steamid` modal fallback, `per_round_stats` unaffected (match-scoped) — add kind filter to the first three + modal; new `corpus_summary() -> Vec<CorpusMapCount { map, demos }>` (kind='corpus' grouped by map) and `corpus_positions(map) -> ...` lands in Task 4 (grid feed).

- [x] Migration + enum + filters + `corpus_summary`; tests: corpus match invisible in list_matches/habit queries/modal identity; corpus_summary counts; version=4. fmt/clippy/test; commit `feat(store): match kind (own|corpus), corpus-blind analytics filters (migration 4)`, push.

### Task 1 (inline): H2 insight names the non-follower (M4 carry-in b) + narrator uses it

**Files:** `cf-analysis/src/families/h2.rs`, `cf-narrator/src/templates.rs` (+tests in both).

- h2 `insights()`: for the BAITED aggregate, add to title_data `"non_followers": [names… no — steamids as strings]` — collect each baited flag's `details.non_following_teammate` (string steamid), dedupe preserving order, cap 3; key: `title_data.non_following_teammates: Vec<String>`.
- Narrator baited match-insight template: when `non_following_teammates` present, name up to 2 via ctx (resolve steamid→name, fallback raw id): "…the follow-up never came — N times, usually with {name} nearest." Update exact-string tests; blame-free rules unchanged.
- [x] TDD both sides; fmt/clippy/test; commit `feat(analysis+narrator): baited insights name the non-following teammate`, push.

### Task 2 (SUBAGENT, worktree): `cf-analysis/src/corpus.rs` — grids + D6 pure math

**Interfaces (frozen):**
```rust
// config.rs additions (Task 0 of THIS plan? No — inline pre-work in Task 2 dispatch prep by coordinator; see note below):
pub struct CorpusCfg { pub min_demos_per_map: usize /*8*/, pub grid_size: usize /*128*/,
  pub freeze_sample_s: f32 /*1.0*/, pub early_s: f32 /*10.0*/, pub mid_s: f32 /*35.0*/, pub post_plant_s: f32 /*5.0*/,
  pub low_density_pct: f32 /*5.0 — cell density percentile (of non-zero cells) below which a position is "rarely held"*/,
  pub min_recurrences: usize /*3 — rounds with low-density positioning before an insight*/,
  pub neighborhood: usize /*1 — Chebyshev radius of cells pooled around the player's cell*/ }
// corpus.rs:
pub enum Phase { FreezeEnd, Early, Mid, PostPlant }           // as_str: "freeze_end"|"early"|"mid"|"post_plant"
pub struct MapCalibration { pub pos_x: f32, pub pos_y: f32, pub scale: f32 }
pub fn calibration_for(map: &str) -> Option<MapCalibration>;   // parsed once from embedded assets/maps/map-data.json
pub fn grid_cell(cal: &MapCalibration, grid: usize, x: f32, y: f32) -> Option<(usize, usize)>; // None outside 0..1024 radar px
pub struct OccupancyGrid { pub map: String, pub side: Side, pub phase: Phase, pub size: usize, pub counts: Vec<u32> /*size*size, row-major [y][x]*/, pub demos: usize, pub samples: u64 }
pub struct PhaseSample { pub map: String, pub side: Side, pub phase: Phase, pub x: f32, pub y: f32 } // one alive-player position at a phase moment
pub fn build_grids(samples: &[PhaseSample], demos_per_map: &std::collections::HashMap<String, usize>, cfg: &CorpusCfg) -> Vec<OccupancyGrid>;
pub fn pooled_density(grid: &OccupancyGrid, cell: (usize, usize), neighborhood: usize) -> u32;   // sum over (2n+1)^2 cells, clamped at edges
pub fn low_density_threshold(grid: &OccupancyGrid, pct: f32, neighborhood: usize) -> u32;        // pct-percentile of POOLED densities of cells that are non-zero-pooled; 0 grid → 0
pub struct TrackedMoment { pub round: u32, pub tick: i32, pub side: Side, pub phase: Phase, pub x: f32, pub y: f32 }
pub struct PositioningFinding { pub phase: Phase, pub side: Side, pub rounds: Vec<u32>, pub ticks: Vec<i32>, pub cells: Vec<(usize,usize)>, pub pooled_densities: Vec<u32>, pub threshold: u32 }
pub fn unusual_positions(moments: &[TrackedMoment], grids: &[OccupancyGrid], map: &str, cfg: &CorpusCfg) -> Vec<PositioningFinding>;
// groups qualifying moments by (side, phase); a moment qualifies when its pooled density <= threshold for that grid;
// a finding emits only when qualifying moments count >= cfg.min_recurrences; grids with demos < cfg.min_demos_per_map are SKIPPED ENTIRELY (silence gate);
pub fn d6_insights(findings: &[PositioningFinding], map: &str, total_rounds: u32, cfg: &CorpusCfg) -> Vec<Insight>;
// detector id "D6_UNUSUAL_POSITIONING", Category::Positioning, severity 0.5, confidence 0.6 (unusualness cap),
// round 0, title_data {phase, side, count, map}, metrics {rounds, threshold, densities}, evidence = one EvidenceRef per
// qualifying moment (round, tick-5s..tick+2s, focus=[tracked]) capped 8, camera_hint Some("heatmap:{map}:{side}:{phase}")
```
Note: coordinator adds `CorpusCfg` to config.rs + embeds map-data.json BEFORE dispatch (shared files stay coordinator-owned); the subagent implements corpus.rs against them.
- [x] TDD (subagent): grid_cell mapping (mirage known point, out-of-bounds None); build_grids per-map demo counts + sample tallies; pooled_density edge clamping; threshold percentile math (uniform grid, spiked grid, empty grid → 0); silence gate (grid with 7 demos produces no findings); qualifying + recurrence gating (2 moments < min_recurrences → silent; 3 → finding); d6_insights honesty text fields + evidence shape + camera_hint format; determinism.
- [x] Coordinator merges, reviews, commits `feat(analysis): corpus occupancy grids + D6 positioning math (subagent-built, reviewed)`, push.

### Task 3 (SUBAGENT, worktree): heatmap canvas renderer (pure TS + component)

**Files:** `src/replay/heatmap.ts` (+ `heatmap.test.ts`), `src/components/HeatmapCanvas.tsx`.

```ts
// heatmap.ts (pure, tested):
export interface GridDto { map: string; side: "CT" | "T"; phase: string; size: number; counts: number[]; demos: number; samples: number }
export function densityToAlpha(count: number, max: number): number;      // 0 when count 0; sqrt ramp; cap 0.85
export function gridMax(counts: number[]): number;                        // 0-safe
export function cellRect(index: number, size: number, canvasPx: number): { x: number; y: number; w: number; h: number };
// HeatmapCanvas.tsx: props { grid: GridDto | null; map: string } — draws the radar image (radarImageUrl(map, "upper"))
// then per-cell fills using ONE sequential hue (CT-blue family #4aa3ff at computed alpha — magnitude job, single hue
// per dataviz; no rainbow), on a 512px canvas; renders "n demos · m samples" caption in text ink; empty grid state text.
```
- [x] TDD the pure fns (alpha ramp incl. zero/max, sqrt monotonicity; cellRect corners for size=128 canvas=512; gridMax empty); component typechecks/lints; commit `feat(ui): heatmap grid renderer (subagent-built, reviewed)`, push (coordinator merges).

### Task 4 (inline): store grid cache + corpus commands

**Files:** `cf-store/migrations/0005_corpus_grids.sql`, `cf-store/src/store.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/{ipc.ts,queries.ts}`.

Migration 5: `corpus_grids(map TEXT, side TEXT, phase TEXT, size INTEGER, counts BLOB, demos INTEGER, samples INTEGER, built_at TEXT, PRIMARY KEY (map, side, phase))`.
Store: `phase_positions_for_corpus(map) -> Vec<(match_id, side, phase, x, y)>`-shaped rows? NO — phase sampling needs rounds+bomb_events+tick lookups; do it in the command with existing readers per corpus match: rounds (`match_detail` is heavy — add lean `rounds_for_match(id)` + reuse `round_ticks`? round_ticks is per round; fine at corpus scale). Simpler store additions: `corpus_match_ids(map) -> Vec<i64>`, `rounds_for_match(id) -> Vec<RoundInfo>`, `bomb_plant_tick(id, round) -> Option<i32>`, `positions_at(id, tick) -> Vec<(steamid String, x, y, alive bool)>` (nearest ≤ tick per player), `save_grids(&[OccupancyGrid])`, `load_grids(map) -> Vec<OccupancyGrid>`, `grid_status() -> Vec<GridStatus {map, side, phase, demos, samples, built_at}>`.
Commands: `import_corpus_demo(path, on_progress)` (parse → save_match kind Corpus → no analysis); `build_corpus(map: Option<String>, on_progress)` (collect PhaseSamples via store readers + `cf_analysis::corpus` sampling helpers, build_grids, save_grids); `corpus_status() -> {maps: Vec<CorpusMapCount>, grids: Vec<GridStatus>}`; `get_grid(map, side, phase) -> Option<GridDto>` (blob → counts vec); `analyze_positioning(match_id) -> usize` (load own match rounds/sides/tick moments for tracked via store, grids for its map, run unusual_positions+d6_insights, DELETE old D6 insights for match + insert new; returns insight count). import_demo additionally runs analyze_positioning when grids exist for the map.
- [x] Store tests (blob roundtrip, grid_status, positions_at nearest-≤); command compile + TS mirrors (GridDto, CorpusMapCount, GridStatus + wrappers/hooks); fmt/clippy/typecheck; commit `feat(app+store): corpus ingestion, grid cache, build + positioning commands`, push.

### Task 5 (inline): Corpus screen

**Files:** `src/screens/Corpus.tsx`, `src/App.tsx` (route `/corpus`), Library topbar link, `src/styles.css`.
**Before code: invoke `frontend-design:frontend-design` and `dataviz` (Skill tool).**
Layout (§7 screen 5): header; "Add pro demos" button (multi-select dialog → sequential import_corpus_demo with inline progress); per-map demo counts with the ≥8 gate shown ("5/8 — detector silent until 8"); Build/Rebuild button (build_corpus progress); heatmap viewer: map/side/phase selectors (from grid_status) + `HeatmapCanvas` + caption; empty states per §7 voice.
- [x] Build; typecheck/lint/vitest; commit `feat(ui): Corpus screen — ingestion, build status, heatmap viewer`, push.

### Task 6 (inline): D6 narration + goldens

**Files:** `cf-narrator/src/templates.rs` (+test), `cf-analysis` golden refresh only if analyze output changed (it didn't — D6 runs outside `analyze()`; no golden change expected — verify).
- Narrator arm for `D6_UNUSUAL_POSITIONING` (honesty wording per spec §5 verbatim intent): "Reference players rarely hold the spot you took at {phase} on {side} — {count} rounds this match. This measures unusual, not wrong: check the heatmap for where they set up instead." Exact-string test; no blame words; must contain "rarely" and "not wrong"-equivalent.
- [x] TDD; verify analysis goldens unchanged (`cargo test --release` golden suite); commit `feat(narrator): D6 positioning template`, push.

### Task 7 (inline): E2E verification, docs, tag m5

- [x] Dev-gate config: verify end-to-end with the demos on hand (1 pro mirage + any others) using a lowered `min_demos_per_map` via a test-only DetectorConfig (documented dev-only — the shipped default stays 8): import the navi pro demo as corpus via UI, build grids, view mirage CT/T heatmaps in the Corpus screen (AX-verify captions/status), run analyze_positioning on the owner's mirage match, verify D6 insights appear in the Match Report with heatmap camera_hint + evidence chips that open the replay at the right rounds.
- [x] §12 sanity: SQL cross-check ≥3 qualifying moments (tracked position → grid cell → pooled density ≤ threshold recomputed by hand for one grid).
- [x] Corpus-blindness check: after corpus import, Library shows only own matches; habits unchanged (counts identical pre/post corpus import).
- [x] Docs: PROGRESS (M5 done → M6), PROMPT §13 checkbox, plan checkboxes, goldens README note. Tag `m5`, push, CI green.
- [x] **Batched owner ask** (final message): drop ~8 pro Mirage demos into the corpus (HLTV match pages → demo download, unzip, Add pro demos) to clear the honest gate and exercise the real DoD; flag that dev verification used a lowered gate.

---

## Self-review notes

- §13 M5 DoD coverage: Corpus screen + ingestion (T4–5) ✓; occupancy grids per map/side/phase (T2, T4 cache) ✓; D6 honesty + min-corpus gate (T2 silence gate + T6 wording) ✓; heatmap rendering with dataviz invoked (T3, T5) ✓; replay-backed output on owner's demo (T4 analyze_positioning + T7) ✓. M4 carry-ins: kind filter (T0) ✓, baited non-follower naming (T1) ✓.
- Placeholder scan: clean — every fn has a concrete contract; phases/thresholds enumerated in CorpusCfg.
- Type consistency: OccupancyGrid/PhaseSample/TrackedMoment/PositioningFinding defined in T2, consumed T4/T6; GridDto defined T3, mirrored T4; Side reused from model.
- Honest limitation: true DoD (8 pro demos, one map) requires owner-supplied demos — dev-gate verification + batched ask documented in T7; the shipped default gate stays 8 (never silently lowered).
