# M3 — Core Detectors & Death Taxonomy MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development for Tasks 3–7 (independent rule families, one fresh subagent each, reviewed before merge); superpowers:executing-plans inline for Tasks 0–2 and 8–9. Steps use checkbox (`- [ ]`) syntax.

**Goal:** D1/D2/D3 shipped as the §5A rule engine: taxonomy MVP (classes 1–7, 9, 13–15) over rule families H2/H3/H16/H4-Tier-1 + flash/utility-economy insights, all TDD with scenario builders, persisted with EvidenceRefs, hand-verified per §12 (PROMPT.md §13 M3 DoD: real demo produces correct, evidence-backed insights for deaths/flashes/utility).

**Architecture:** cf-parser gains the event/prop data the rules need (shots, hurts, reloads, is_scoped, death/round-end inventories via a targeted third pass). cf-analysis is pure over `MatchData`: an `AnalysisContext` (indexes + spatial/temporal helpers), a `ScenarioBuilder` for tests, one module per rule family emitting `RuleFlag`s and `Insight`s, and a priority-ordered classifier producing one `DeathClass` per tracked-player death. cf-store migration 2 persists everything. Import pipeline: parse → analyze → save.

**Execution model:** Tasks 0–2 inline (foundation — shared interfaces). Tasks 3–7 are **parallel subagents** (disjoint files: `families/<name>.rs` + tests; all shared types/config pre-defined by Task 2). Tasks 8–9 inline (assembly, persistence wiring, hand-verification needs the app + owner demos + judgment).

**Spec:** `docs/spec/death-taxonomy.md` (governing), PROMPT.md §5 (D1–D3 definitions), §6.4 (defaults), §5A, §12 (verification bar).

## Global Constraints

- Detectors are pure functions over `MatchData` + `DetectorConfig` — **no I/O, no demoparser types** (§4).
- **Rule ids are load-bearing** — exactly as spec'd; new ids added here (documented additions, never renames): `H5_DIED_FLASHED` (class 3 source), `H6_DEAD_TIME_SMOKE`, `H6_UTIL_TEAM_DAMAGE`, `H14_DIED_SELF_OR_WORLD` (class 14 source).
- Every rule emits `confidence`; approximation caps per spec (no-LOS approximations ≤ 0.75, audio-free blind inference ≤ 0.8). **Bias to silence**: when required data is missing (old import, NULL column), the rule emits nothing.
- Thresholds in seconds/world-units only, from `DetectorConfig` (serde-YAML, embedded defaults per §6.4). No magic numbers in rule code.
- Severity 0–1 guidance: H2_ISOLATED 0.8 / H2_FAILED_TRADE 0.6 / **H2_BAITED_TRADE capped 0.35** (spec: cap well below isolated; evidence must name the non-following teammate; never habit-promoted alone). Class 13/14 emit death_class rows but NO insight (not coaching moments).
- Every rule: TDD via ScenarioBuilder — both firing and suppression cases. Class-13 share asserted in goldens (CI regression metric per spec).
- Verified parser facts (probe 2026-08-19): `player_hurt` fields `dmg_health/dmg_armor/weapon(String, unprefixed)/hitgroup/user_*/attacker_*`; `weapon_fire` fields `weapon (weapon_-prefixed)/user_steamid`; `weapon_reload` fields `user_steamid`; per-tick props `is_scoped`, `inventory` (StringVec) exist in maps.rs; demoparser supports `wanted_ticks` for targeted sampling.
- Steamids: u64 inside Rust core; strings at store/IPC boundaries.

---

### Task 0 (inline): Parser — rule-engine data

**Files:** `src-tauri/crates/cf-parser/src/{model.rs,extract.rs}`, goldens regenerated.

**Produces (MatchData additions):**
```rust
pub struct Shot { pub tick: i32, pub player: u64, pub weapon: String }
pub struct Hurt { pub tick: i32, pub victim: u64, pub attacker: Option<u64>, pub dmg_health: i32, pub weapon: String, pub hitgroup: String }
pub struct Reload { pub tick: i32, pub player: u64 }
pub struct InventorySample { pub tick: i32, pub steamid: u64, pub items: Vec<String> }
// MatchData += shots, hurts, reloads, inventories (death ticks + round-end ticks)
// TickTable += is_scoped: Vec<bool>
```
- [x] Add `weapon_fire`/`weapon_reload`/`player_hurt` to WANTED_EVENTS + extraction; add `is_scoped` to WANTED_PROPS + TickTable.
- [x] Third pass: `wanted_ticks` = death ticks ∪ round end_ticks, `wanted_player_props=["inventory"]` → `Vec<InventorySample>` (verify `VarVec::StringVec` shape at runtime; only_header like the ticks pass).
- [x] Run on mirage-tie: sanity (shots ≈ 3.3k, hurts ≈ 644, reloads ≈ 78, inventories ≈ (183 deaths + 24 rounds) × alive players). Regenerate both goldens (add counts to `MatchGolden`). `cargo test` green. Commit/push.

### Task 1 (inline): Store — schema migration 2

**Files:** `cf-store/migrations/0002_analysis.sql`, `store.rs`.

Migration 2: `shots(match_id,tick,player)`, `hurts(match_id,tick,victim,attacker,dmg_health,weapon,hitgroup)`, `reloads(match_id,tick,player)`, `inventories(match_id,tick,steamid,items_json)`, `death_class(match_id,round,tick,victim,class_id,class_source,secondary_tags_json,confidence)` (spec §1 storage), `rule_flags(id,match_id,rule_id,round,tick,steamid,confidence,severity,details_json)`, `insights(id,match_id,detector,category,severity,confidence,round,player,title_data_json,metrics_json,evidence_json)`; `ALTER TABLE tick_samples ADD COLUMN is_scoped INTEGER` (nullable — old imports stay NULL → rules silent).

- [x] Migration runner test (v1→v2 fresh + reopen); `save_match` persists new MatchData fields; `save_analysis(match_id, &AnalysisOutput)` + `insights_for_match`/`death_classes_for_match` readers; tests with synthetic data. Commit/push.

### Task 2 (inline): cf-analysis foundation

**Files:** `cf-analysis/src/{lib.rs,types.rs,config.rs,context.rs,scenario.rs,classify.rs}`, `cf-analysis/default-config.yaml`, `Cargo.toml` (cf-parser path dep, serde, serde_json, YAML crate — check `serde_yaml_ng`/current maintained fork; pin).

**Produces (the interfaces every subagent consumes — exact):**
```rust
// types.rs
pub struct EvidenceRef { pub round: u32, pub tick_start: i32, pub tick_end: i32, pub focus_players: Vec<u64>, pub camera_hint: Option<String> }
pub struct Insight { pub detector: String /*rule id*/, pub category: Category, pub severity: f32, pub confidence: f32, pub round: u32, pub player: u64, pub title_data: serde_json::Value, pub metrics: serde_json::Value, pub evidence: Vec<EvidenceRef> }
pub enum Category { Deaths, Utility, Positioning, Timing }
pub struct RuleFlag { pub rule_id: &'static str, pub round: u32, pub tick: i32, pub steamid: u64, pub confidence: f32, pub severity: f32, pub details: serde_json::Value }
pub struct DeathClassRow { pub round: u32, pub tick: i32, pub victim: u64, pub class_id: u8, pub class_source: String, pub secondary_tags: Vec<String>, pub confidence: f32 }
pub struct AnalysisOutput { pub flags: Vec<RuleFlag>, pub insights: Vec<Insight>, pub death_classes: Vec<DeathClassRow> }

// context.rs — built once per match, passed to every family
pub struct AnalysisContext<'a> { /* MatchData + prebuilt indexes */ }
impl AnalysisContext<'_> {
    pub fn data(&self) -> &MatchData;
    pub fn tracked(&self) -> u64;
    pub fn state_at(&self, steamid: u64, tick: i32) -> Option<PlayerState>;   // nearest sample ≤ tick (x,y,z,yaw,health,is_alive,team_num,weapon,place,is_scoped)
    pub fn side_of(&self, steamid: u64, round: u32) -> Option<Side>;
    pub fn teammates_alive_at(&self, steamid: u64, round: u32, tick: i32) -> Vec<(u64, PlayerState)>;
    pub fn enemies_alive_at(&self, steamid: u64, round: u32, tick: i32) -> Vec<(u64, PlayerState)>;
    pub fn nearest_teammate(&self, steamid: u64, round: u32, tick: i32) -> Option<(u64, f32 /*dist*/)>;
    pub fn nearest_enemy(&self, steamid: u64, round: u32, tick: i32) -> Option<(u64, f32)>;
    pub fn shots_by_in(&self, steamid: u64, t0: i32, t1: i32) -> &[Shot];      // binary-searched slices
    pub fn hurts_dealt_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&Hurt>;
    pub fn hurts_taken_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&Hurt>;
    pub fn reloads_by_in(&self, steamid: u64, t0: i32, t1: i32) -> &[Reload];
    pub fn blind_window_at(&self, steamid: u64, tick: i32) -> Option<&Blind>;  // active enemy-thrown blind covering tick
    pub fn inventory_at(&self, steamid: u64, tick: i32) -> Option<&InventorySample>; // exact-tick sample (death/round-end)
    pub fn kill_of(&self, victim: u64, round: u32) -> Option<&Kill>;
    pub fn seconds(&self, s: f32) -> i32;                                      // seconds → ticks
    pub fn dist(a: &PlayerState, b: &PlayerState) -> f32;                      // 3D, z weighted ×2 per spec H2 refinement
}

// Detector trait + config
pub trait Detector { fn rule_ids(&self) -> &'static [&'static str]; fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag>; fn insights(&self, ctx: &AnalysisContext, cfg: &DetectorConfig, flags: &[RuleFlag]) -> Vec<Insight>; }
```
`config.rs`: `DetectorConfig { trade: TradeCfg { window_s: 3.0, distance_u: 700.0, isolation_u: 900.0, commit_window_s: 2.0 }, flash: FlashCfg { effective_s: 1.1, conversion_window_s: 2.0 }, h3: H3Cfg { switch_window_s: 0.3, reload_window_s: 2.0, scoped_close_u: 600.0 }, h16: H16Cfg { no_shot_window_s: 3.0, no_contact_window_s: 2.0, fire_linger_dmg: 20, fire_linger_s: 1.0 }, h4: H4Cfg { crossfire_engage_window_s: 2.0, crossfire_min_angle_deg: 45.0, contactless_window_s: 2.0 }, util: UtilCfg { dead_time_after_end: true, min_unused_nades: 2 }, severity: SeverityCfg { per-rule map with caps incl. baited 0.35 }, ... }` — serde-Deserialize, `DetectorConfig::default_yaml()` embedded via include_str, `load(path) -> merge over defaults`.
`scenario.rs`: `Scenario::new(map).players_ct(&[1,2,..]).players_t(&[...]).round(1, freeze_end, end).sample(sid, tick, x, y, z /*+builder setters for hp, weapon, scoped, alive*/).kill(att, vic, tick, weapon).with_kill_flags(...).blind(att, vic, tick, dur).shot(sid, tick).hurt(att, vic, tick, dmg, weapon).reload(sid, tick).inventory(sid, tick, &["Flashbang","Smoke Grenade"]).build() -> MatchData` — samples auto-densified (every 4 ticks between explicit waypoints, lerped).
`classify.rs`: skeleton — priority table + fallthrough (fleshed out Task 8), `class_14(kill) -> bool` (attacker None | == victim | weapon ∈ {world, planted_c4, inferno? no — inferno w/ enemy attacker is class 2}).

- [x] Build all of it; unit tests for context helpers (state_at nearest-≤, window slicing, nearest teammate/enemy, dist z-weighting) + ScenarioBuilder self-test + config YAML roundtrip. fmt/clippy/test green. Commit/push. **Interfaces frozen for subagents.**

### Tasks 3–7 (subagents, parallel — disjoint files `cf-analysis/src/families/<x>.rs`)

Every subagent task: read `docs/spec/death-taxonomy.md` + this plan + Task 2's code; TDD with ScenarioBuilder (write firing AND suppression tests first); thresholds only via cfg; return flags + insights with EvidenceRef (window: 5 s before event → 2 s after, focus = involved players); fmt/clippy/test green; do NOT touch shared files beyond adding `pub mod <x>;` to `families/mod.rs` (single line, coordinator resolves).

- [x] **Task 3 — H2 trade spacing (D1, classes 6/7):** `H2_ISOLATED_DEATH` (tracked death: nearest teammate > isolation_u AND not same/adjacent-place AND killer not killed within commit window → class 6; conf 0.75 — LOS approximated by place equality); `H2_FAILED_TRADE` (teammate died within distance_u of tracked player, tracked fired 0 shots at killer & didn't damage killer within commit_window → flag); `H2_BAITED_TRADE` (tracked committed after teammate death — damaged/shot at killer within window — died doing it, no third teammate within distance_u → class 7, severity cap, details name the non-following teammate + dist). Insight per pattern ≥2 occurrences (match-level) + per-death flags.
- [x] **Task 4 — H3 utility vulnerability (D3-half, classes 1/4):** `H3_DIED_WITH_NADE_OUT` (death-sample weapon is a grenade → class 1); `H3_DIED_MID_SWITCH` (active_weapon differs between the two samples ≤ switch_window before death → class 1, conf 0.7 — 16 Hz granularity); `H3_DIED_RELOADING` (reload event ≤ reload_window before death, no shot between → class 4, conf 0.7); `H3_DIED_SCOPED_CLOSE` (is_scoped at death sample AND nearest enemy < scoped_close_u → class 4; silent if is_scoped missing); `H3_WASTED_UTILITY` (death inventory contains ≥1 grenade → flag; match insight "died holding utility in N/M deaths" with per-death evidence). Vulnerable-death-% metric in insight.
- [x] **Task 5 — H16 utility damage (class 2):** `H16_DIED_TO_UTILITY_NO_DUEL` (kill weapon ∈ {hegrenade, inferno, molotov} AND victim fired 0 shots in no_shot_window AND dealt 0 damage in no_contact_window → class 2, conf 0.8); `H16_FIRE_LINGER` (cumulative inferno hurts > fire_linger_dmg with first-burn ≥ fire_linger_s before the damage crossing — flag, works without death). Expect near-zero class-2 volume (spec 5.4) — silent-biased, no match insight unless ≥2.
- [x] **Task 6 — H4 Tier-1 exposure (classes 5/9):** `H4_KILLED_WITHOUT_CONTACT` (kill.thru_smoke OR penetrated>0 → class 5 conf 0.95; else victim fired 0 shots AND took no prior damage from killer within contactless_window → class 5 conf 0.6); `H4_CAUGHT_IN_CROSSFIRE` (victim exchanged damage with enemy A within crossfire_engage_window, killer B ≠ A, angle(A→victim→B) > min_angle at death tick → class 9; positions from state_at).
- [x] **Task 7 — D2 flash + D3 utility economy (class 3):** `H5_DIED_FLASHED` (death inside enemy blind window with duration ≥ effective_s → class 3); flash effectiveness: group blinds by (attacker, tick) = one flash → per-flash {enemies-effective, teammates-blinded, self, converted (blinded enemy died ≤ conversion_window to thrower's team, cross-check kill.assistedflash)}; `H6_FLASH_SELF_OR_TEAM` flags + match insight (team-flash count, worst flash evidence, effective-flash rate metric); `H6_DEAD_TIME_SMOKE` (smoke detonate after round end_tick); `H6_UNUSED_UTIL_AT_ROUND_END` (round-end inventory: ≥ min_unused_nades grenades while round lost... any result — flag per §5 D3 "died with $ of nades" is H3's; this one is round-end holding); `H6_UTIL_TEAM_DAMAGE` (hurt weapon ∈ nade set, attacker teammate of victim, dmg ≥ 1 → flag; aggregate insight).

### Task 8 (inline): Classifier assembly, pipeline, persistence

- [x] `classify.rs`: run families → collect flags per tracked-player death → priority order (spec table): 1 (H3 nade/switch) → 2 (H16) → 3 (H5_DIED_FLASHED) → 4 (H3 reload/scoped) → 5 (H4 contactless) → 6 (H2 isolated) → 7 (H2 baited) → 9 (H4 crossfire) → then 14 (`H14_DIED_SELF_OR_WORLD` event-derived check FIRST in code order before 1 — self/world/c4 deaths never reach other classes) → 13 (fallthrough: victim shot at or damaged killer within 3 s) → 15 (else). One primary class + secondary_tags = every other fired rule id. Unit tests: priority collisions (molly kill while isolated → 2 with H2 tag; nade-out while flashed → 1 with H5 tag), 14-before-all, 13-vs-15 split.
- [x] `analyze(data, cfg) -> AnalysisOutput` registry; wire into `import_demo` (after parse, before save; progress stage "analyzing" 80–90 %); `save_analysis` in the same transaction as save_match.
- [x] `print_insights` example (cf-analysis dev tool): per-demo insight list + death-class table + **class-13 share**. Commit/push each chunk.

### Task 9 (inline): Goldens, hand-verification, docs, tag m3

- [x] Analysis goldens for mirage-tie + navi: per-rule flag counts + full death_class distribution (incl. class-13 share — the regression metric) + insight count. Gated tests like match goldens.
- [x] Wipe dev DB, re-import owner demos through the UI, run analysis; **hand-verify ≥3 flagged instances per family in the replay viewer via each insight's evidence deep link** (§12: does the "isolated" death actually look isolated?). Tune thresholds only on clear false positives (precision over recall; log every judgment in the goldens README).
- [x] Sanity checks from spec: class 15 near-empty; class 14 non-zero across the 5 own demos; class-2 rare; class-13 plausible (~25–40 %).
- [x] Docs (PROGRESS, PROMPT §13 checkbox, CLAUDE if commands changed, spec addendum note for added rule ids), plan checkboxes, tag `m3`, CI green.

---

## Self-review notes

- §13 M3 DoD: D1 ⇒ Task 3 (+classifier) ✓; D2 ⇒ Task 7 flash ✓; D3 ⇒ Tasks 4+7 utility ✓; DetectorConfig §6.4 defaults ✓ (trade 3.0 s/700 u, isolation 900 u, flash 1.1 s/2.0 s in config.rs literals); insights persisted with evidence refs ⇒ Tasks 1+8 ✓; scenario-builder TDD ⇒ every family task ✓; hand-verification ⇒ Task 9 ✓.
- §5A conformance: one primary class by priority + secondary tags ✓; confidence everywhere ✓; silence bias (NULL data → no flag) ✓; class-13 CI metric ✓; class 14 explicit ✓; baited-trade severity/caption rules ✓; rules-as-data partially satisfied (YAML thresholds/severities; predicate DSL deliberately deferred — note in ADR-0005 if written, else PROGRESS decision line).
- Type consistency: RuleFlag/Insight/EvidenceRef defined once in Task 2, consumed by 3–8 unchanged; steamid u64 core / string boundary.
- Flagged uncertainties: `inventory` StringVec item naming (display vs weapon_ names — probe in Task 0 and write the grenade-name set into config); adjacency of `last_place_name` values (M3 uses equality only — adjacency map is future work, noted); classifier order for 8/10/11/12 reserved (families absent M3 — UI "classes not yet detected" note lands with M4 report screen per spec).
