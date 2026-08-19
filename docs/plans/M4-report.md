# M4 — Match Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development for Tasks 2–4 (independent: two rule families + the narrator crate; fresh subagent each in an isolated worktree, coordinator reviews & merges); superpowers:executing-plans inline for Tasks 0–1 and 5–9. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The money screen: an insight feed with TemplateNarrator coaching text and evidence chips into the replay, a round timeline strip, D4+D5 detectors, and §5A cross-demo habit promotion (PROMPT.md §13 M4 DoD: the owner reviews one of their own matches end-to-end and gets ≥1 insight they agree is real and actionable).

**Architecture:** Two new rule families (`h14_entry` = D4 opening-duel structure, `h11_timing` = D5 timing/rotation incl. class-11 `H6_PUSH_WITHOUT_INFO`). `cf-narrator` gets the §8 trait + `TemplateNarrator` (deterministic variants, coach-voice quality bar). Cross-demo: cf-store aggregates rule flags + death positions across matches; pure promotion/clustering fns in cf-analysis (`habits.rs`). One `get_match_report` command returns insights+narration+classes+per-round stats; `get_habits` returns cross-match habits with per-match evidence. React `/report/:matchId` screen (frontend-design + dataviz invoked before UI work).

**Tech Stack:** existing crates; no new deps expected (narrator is plain Rust string templates).

**Spec:** PROMPT.md §5 (D4/D5), §7 screen 2, §8 (narrator), §13 M4; docs/spec/death-taxonomy.md (§1 honesty rules, §2 H2-baited caption rules, §5A integration notes: ranking = severity × confidence × recurrence-across-demos; H2_BAITED/H2_FAILED both recurring ⇒ team-pattern caption; never promote baited alone).

## Global Constraints

- Skills: invoke `superpowers:subagent-driven-development` before dispatching Tasks 2–4; `frontend-design:frontend-design` + `dataviz` before Task 7 UI code (Skill tool invocations, not from-context).
- Conventional commits, push per task, CI green. Detectors pure, no demoparser types, thresholds only via DetectorConfig (§6.4: early aggressive death = 20 s after freeze_end; trade/support distance 700 u). Steamids: u64 core / strings at boundaries; stringify steamids inside `details`.
- Death-anchored flags: tick = kill tick, steamid = victim (classifier convention). Bias to silence; confidence caps on approximations (info-proxy ≤ 0.6).
- Narration quality bar (§8): specific, actionable, no filler; multiple variants selected deterministically (hash of detector+round+count — no RNG); H2_BAITED text must name the non-following teammate and read as "you were third man in a two-man fight", never blame; team_pattern insights say it's a team problem.
- Class 12 stays reserved at M4 (per-death hotspot classing needs re-analysis of old imports — architecture note, not built; hotspots ship as cross-demo habits). Re-analyze command deferred to M6; M4 verification uses a fresh DB re-import.

---

### Task 0 (inline): Config + classifier extensions (shared interfaces frozen for subagents)

**Files:** `cf-analysis/src/config.rs`, `cf-analysis/src/classify.rs`, `cf-analysis/src/types.rs` (no changes expected), `cf-analysis/src/habits.rs` (new — types only used by Task 5 but config lives here).

Config additions (defaults; serde-default pattern identical to existing):
```rust
pub struct EntryCfg { pub support_distance_u: f32 /*700.0*/, pub opening_window_s: f32 /*15.0 — first engagement must start within this after freeze_end to count as an "entry" rather than a mid-round pick*/ }
pub struct TimingCfg { pub early_aggression_s: f32 /*20.0 §6.4*/, pub rotate_radius_u: f32 /*800.0*/, pub rotate_max_s: f32 /*25.0*/, pub min_spawn_distance_u: f32 /*750.0 — dying this close to your own round-start position is not "aggressive depth"*/ }
pub struct HabitCfg { pub min_matches: usize /*3*/, pub window_matches: usize /*10*/, pub hotspot_radius_u: f32 /*250.0 spec H4*/, pub hotspot_min_deaths: usize /*3*/, pub hotspot_min_matches: usize /*2*/ }
// SeverityCfg += h14_unsupported_entry (0.6), h11_slow_rotation (0.5), h11_early_aggressive_death (0.6), h6_push_without_info (0.7), habit defaults are per-rule severities reused.
```
Classifier: insert `(11, &["H6_PUSH_WITHOUT_INFO"])` into `PRIORITY` between the `(9, …)` entry and nothing (append — order in the array is class-priority order: after 7 comes 9, then 11). CAREFUL: spec priority is class-number order 1..12, so the tuple goes after `(9, …)`. Update `print_insights` CLASS_NAMES: 11 loses "[not built]".

- [ ] Config structs + severity fields + defaults test extension (assert new defaults incl. early_aggression_s 20.0); classifier PRIORITY insert + a test (`push_without_info_flag_classifies_as_11_below_crossfire`: crossfire + info flags on same death → class 9 wins, info in tags; info alone → 11). `cargo test -p cf-analysis` green; fmt/clippy; commit `feat(analysis): config + classifier slots for D4/D5 (class 11)`, push.

### Task 1 (inline): Store — cross-demo aggregation queries

**Files:** `cf-store/src/store.rs` (+tests).

**Produces:**
```rust
pub struct RuleMatchCount { pub match_id: i64, pub map: String, pub imported_at: String, pub count: u32, pub first_evidence_json: String }
impl Store {
    /// Per-match flag counts for one rule for the tracked player, newest first, capped to `window` matches.
    pub fn rule_counts_across_matches(&self, tracked: &str, rule_id: &str, window: usize) -> Result<Vec<RuleMatchCount>, StoreError>;
    /// Tracked player's death positions (from nearest tick_sample <= kill tick) per map across all matches.
    pub fn death_positions(&self, tracked: &str) -> Result<Vec<DeathPos>, StoreError>;
    /// All distinct rule_ids that ever flagged for the tracked player.
    pub fn flagged_rule_ids(&self, tracked: &str) -> Result<Vec<String>, StoreError>;
    /// Per-round tracked K/D + winner for the timeline strip.
    pub fn per_round_stats(&self, match_id: i64, tracked: &str) -> Result<Vec<RoundStat>, StoreError>;
}
pub struct DeathPos { pub match_id: i64, pub map: String, pub round: u32, pub tick: i32, pub x: f32, pub y: f32 }
pub struct RoundStat { pub number: u32, pub winner: String, pub tracked_side: Option<String>, pub kills: u32, pub deaths: u32 }
```
SQL notes: `first_evidence_json` = `(SELECT evidence_json… )` no — flags store evidence inside details? No: rule_flags has no evidence column! Flags' evidence lives only in insights. FIX: habit evidence = for each match, the first flag's (round,tick) rebuilt into an EvidenceRef client-side is wrong (no focus). CORRECT approach: add `evidence_json` column to rule_flags in **migration 3** (`ALTER TABLE rule_flags ADD COLUMN evidence_json TEXT`), and make `save_analysis` write `f.evidence`; old rows NULL → habit uses (round, tick±5 s, focus=[tracked]) fallback built in Rust. death_positions: `SELECT k.match_id, m.map, k.round, k.tick, t.x, t.y FROM kills k JOIN matches m ON m.id=k.match_id JOIN tick_samples t ON t.match_id=k.match_id AND t.steamid=k.victim AND t.tick=(SELECT MAX(tick) FROM tick_samples WHERE match_id=k.match_id AND steamid=k.victim AND tick<=k.tick) WHERE k.victim=?1`.

- [ ] Migration 3 (rule_flags evidence_json) + save_analysis writes it; tests: migration v3 applies; rule_counts window/ordering; death_positions returns nearest-sample coords (extend `sample_match`); per_round_stats winner/K-D correctness; flagged_rule_ids distinct. fmt/clippy/test; commit `feat(store): cross-demo aggregation queries + flag evidence (migration 3)`, push.

### Task 2 (SUBAGENT, worktree): D4 family — `families/h14_entry.rs`

Rules (register `pub struct H14EntryStructure` in families/mod.rs in own worktree; coordinator re-merges):
- Opening duel per round = the first `Kill` of the round with both sides known and tick ≤ freeze_end + opening_window_s… actually: first kill of the round whose tick ≥ round.start; "entry attempt on T side" = its T-side participant. For EVERY round compute: entry player (T participant), victim/winner, supported = ∃ teammate of the entry player within support_distance_u OR same `last_place` at the kill tick (state_at).
- `H14_UNSUPPORTED_ENTRY` (flag, NOT death-anchored — anchor tick = opening kill tick, steamid = tracked): fires when the tracked player was the T-side opening-duel participant AND unsupported (no teammate within support params). Fires whether they won or lost the duel (the mistake is structural); details {won: bool, opponent, nearest_teammate, distance}; confidence 0.7; severity cfg.
- insights(): `D4_ENTRY_PROFILE` (match-level, category Positioning, ALWAYS when tracked took ≥3 T-side entries): metrics {entries, entry_wins, supported, unsupported, team_entries, team_entry_wins}; plus count of rounds where tracked was the closest teammate to a dying entry and did not commit (reuse pattern: check H2_FAILED_TRADE flags at opening-kill ticks — flags of other families aren't visible to this one, so compute directly: teammate entry died, tracked within support_distance_u, tracked fired no shot/dealt no damage to the killer within trade window) → metrics.non_trading_on_entries; evidence = up to 8 unsupported-entry refs.
- TDD: entry detection (first kill only; second kill of round ignored); T-side selection (kill with T attacker vs T victim); supported suppression (teammate 400 u); unsupported fires with details naming nearest teammate; won-but-unsupported still fires; tracked-not-involved rounds silent; CT-side tracked rounds silent for the flag but counted in team metrics; insight gating ≥3 entries; non_trading_on_entries counting.

- [ ] Dispatch, review report + code, merge, run suite, commit `feat(analysis): D4 entry structure family (subagent-built, reviewed)`, push.

### Task 3 (SUBAGENT, worktree): D5 family — `families/h11_timing.rs`

Rules (`pub struct H11Timing`):
- `H11_EARLY_AGGRESSIVE_DEATH` (flag, death-anchored): tracked died within early_aggression_s of freeze_end; travelled ≥ min_spawn_distance_u from their freeze-end position (state_at at freeze_end vs at death, XY distance); no teammate within support_distance_u (reuse trade.distance_u) at death. Confidence 0.7. Not a class source (class 8 reserved for H1).
- `H6_PUSH_WITHOUT_INFO` (→ class 11, death-anchored): all H11_EARLY_AGGRESSIVE_DEATH conditions AND the info-proxy says the team had nothing: no enemy's `spotted` flag was true in any sample from freeze_end→death (spotted on an enemy row = someone on our side sees them), AND no damage in either direction between the teams before the death, AND no enemy shots fired before the death (silence = no info). Confidence 0.6 (proxy), severity cfg. details {seconds_in: f32, distance_from_spawn}.
- `H11_SLOW_ROTATION` (flag, tick = arrival-or-round-end): tracked on CT side, bomb planted (bomb_events), tracked alive at plant, plant position known (planter state_at), tracked's distance to plant position > rotate_radius_u at plant AND tracked never came within rotate_radius_u before min(round end, death, plant+rotate_max_s)… flag when they were still outside the radius at plant + rotate_max_s while ALIVE (dead players can't rotate — silent). Round must have been LOST (precision-first: a won round's rotation choice was evidently fine). Confidence 0.65. details {seconds_late_or_never: value|null, distance_at_plant}.
- insights(): `D5_TIMING` match-level when ≥2 flags across the two H11 rules: metrics {early_aggressive_deaths, slow_rotations, push_without_info}; evidence cap 8. Category Timing.
- TDD: early-aggro fires (die at 15 s, 900 u from spawn, no support); suppressed at 25 s; suppressed near spawn; suppressed with teammate close; push-without-info fires only when info-proxy empty; suppressed when an enemy was spotted before death; suppressed when team exchanged damage first; slow-rotation fires (CT alive far from plant, never arrives, round lost); suppressed when round won / when arrives in time / when tracked dead at plant; insight gating.

- [ ] Dispatch, review, merge, suite, commit `feat(analysis): D5 timing family (subagent-built, reviewed)`, push.

### Task 4 (SUBAGENT, worktree): cf-narrator — trait + TemplateNarrator

**Files:** `cf-narrator/Cargo.toml` (deps: cf-analysis path, serde_json), `cf-narrator/src/lib.rs`, `cf-narrator/src/templates.rs`.

**Interface (§8, frozen):**
```rust
pub struct MatchContext { pub map: String, pub tracked: u64, pub names: std::collections::HashMap<u64, String>, pub score: (u32, u32), pub tracked_result: Option<String>, pub total_deaths: usize, pub class_13_share_pct: f32 }
pub struct Narration { pub title: String, pub body: String }
pub trait CoachingNarrator {
    fn narrate(&self, insight: &cf_analysis::Insight, ctx: &MatchContext) -> Narration;
    fn summarize(&self, insights: &[cf_analysis::Insight], ctx: &MatchContext) -> Option<Narration>;
}
pub struct TemplateNarrator;
```
Behavior: template per detector id (H2_ISOLATED_DEATH, H2_FAILED_TRADE, H2_BAITED_TRADE, H3_VULNERABLE_DEATHS, H3_WASTED_UTILITY, H4_KILLED_WITHOUT_CONTACT, H4_CAUGHT_IN_CROSSFIRE, H16_UTILITY_EXPOSURE, D2_FLASH_EFFECTIVENESS, H6_UTIL_TEAM_DAMAGE, H6_UNUSED_UTIL_AT_ROUND_END, H6_DEAD_TIME_SMOKE, D4_ENTRY_PROFILE, D5_TIMING, HABIT_* handled generically via a habit template taking label data) + a neutral fallback naming the detector and count (never empty output). ≥2 phrasing variants per high-frequency template (isolated, failed-trade, vulnerable, without-contact), variant = hash(detector, round, count) % n. Facts come from insight.title_data/metrics (counts, pct) — numbers formatted plainly ("7 of 19 deaths"), steamid strings resolved via ctx.names (fallback to the raw id). Quality bar examples to match (§8): coach voice, specific, actionable second sentence ("Either arrive with the Connector player or hold one step deeper."). H2_BAITED: MUST name the teammate + "you did the right thing — the follow-up never came; this is a team spacing problem" tone; when title_data.team_pattern == true say the failed/baited combination is a team problem. summarize(): 2–3 sentence match summary from class-13 share + top category + score result.
TDD: exact-string tests for fixed inputs per template (≥12 tests), variant determinism (same input twice = same output; different round = may differ), baited names teammate + contains no blame words ("your fault" absent, teammate name present), fallback for unknown detector, summarize composition.

- [ ] Dispatch, review (READ the template text against the §8 quality bar — this one is taste-checked, not just test-checked), merge, suite, commit `feat(narrator): CoachingNarrator trait + TemplateNarrator v1 (subagent-built, reviewed)`, push.

### Task 5 (inline): Habits — promotion + hotspots (pure) and wiring

**Files:** `cf-analysis/src/habits.rs` (+tests), `cf-store` glue in Task 6 command.

```rust
pub struct HabitInput { pub rule_id: String, pub severity: f32, pub confidence: f32, pub per_match: Vec<(i64 /*match_id*/, u32 /*count*/)> } // newest first, already windowed
pub struct Habit { pub rule_id: String, pub matches_hit: usize, pub window: usize, pub total: u32, pub score: f32 }
pub fn promote_habits(inputs: &[HabitInput], cfg: &HabitCfg) -> Vec<Habit>;
// promoted when matches_hit >= cfg.min_matches within the window; H2_BAITED_TRADE never promoted alone (spec) — only if H2_FAILED_TRADE also promotes this window (pass both, enforce inside);
// score = severity × confidence × (matches_hit as f32 / window as f32) × ln(1 + total as f32) — deterministic ordering desc.
pub struct Hotspot { pub map: String, pub center: (f32, f32), pub deaths: usize, pub matches: usize, pub members: Vec<(i64, u32, i32)> /*match_id, round, tick*/ }
pub fn death_hotspots(points: &[DeathPoint], cfg: &HabitCfg) -> Vec<Hotspot>; // greedy radius clustering per map: seed = earliest unclustered point, member iff within hotspot_radius_u of seed; cluster kept iff deaths >= hotspot_min_deaths AND distinct matches >= hotspot_min_matches; deterministic
pub struct DeathPoint { pub match_id: i64, pub map: String, pub round: u32, pub tick: i32, pub x: f32, pub y: f32 }
```
- [ ] TDD: promotion at 3/10, not at 2/10; window truncation; baited-alone suppressed / baited+failed both promoted; score ordering; hotspot cluster of 3-across-2-matches found, 3-in-1-match rejected, radius boundary, two separate clusters, deterministic order. fmt/clippy/test; commit `feat(analysis): cross-demo habit promotion + death hotspots`, push.

### Task 6 (inline): Report + habits commands, TS mirrors

**Files:** `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src/lib/ipc.ts`, `src/lib/queries.ts`.

```rust
#[derive(serde::Serialize)] pub struct NarratedInsight { pub detector: String, pub category: String, pub severity: f32, pub confidence: f32, pub round: u32, pub score: f32, pub title: String, pub body: String, pub metrics: serde_json::Value, pub evidence: Vec<cf_analysis::EvidenceRef> }
#[derive(serde::Serialize)] pub struct MatchReport { pub match_id: i64, pub map: String, pub score_a: u32, pub score_b: u32, pub tracked: Option<String>, pub tracked_result: Option<String>, pub summary: Option<Narration-as-{title,body}>, pub insights: Vec<NarratedInsight> /*sorted by score desc, grouped client-side*/, pub death_classes: Vec<DeathClassDbRow>, pub class_13_share_pct: f32, pub per_round: Vec<RoundStat>, pub classes_not_built: Vec<u8> /*[8,10,12] after class 11 ships*/ }
#[tauri::command] pub fn get_match_report(state, match_id: i64) -> Result<Option<MatchReport>, String>; // reads insights_for_match + death_classes + per_round_stats; builds MatchContext (names from players table), narrates each insight + summarize; score = severity×confidence×ln(1+count from metrics.count||1)
#[derive(serde::Serialize)] pub struct HabitReport { pub rule_id: String, pub title: String, pub body: String, pub matches_hit: usize, pub window: usize, pub total: u32, pub score: f32, pub evidence: Vec<HabitEvidence> } // HabitEvidence { match_id: i64, map: String, evidence: EvidenceRef }
#[tauri::command] pub fn get_habits(state) -> Result<Vec<HabitReport>, String>; // flagged_rule_ids → rule_counts_across_matches → promote_habits; hotspots via death_positions → death_hotspots → HabitReport with rule_id "H4_REPEAT_HOTSPOT" narrated ("You've died N times within the same spot on {map} across M matches"); evidence from stored flag evidence_json (fallback tick±5 s)
```
- [ ] Implement + mirror types in ipc.ts (MIRROR CHECKLIST) + `useMatchReport`/`useHabits` hooks; cargo check + typecheck; commit `feat(app): match report + habits commands with narration`, push.

### Task 7 (inline): Match Report screen

**Files:** `src/screens/Report.tsx`, `src/components/{RoundStripReport,InsightCard,ClassBreakdown,HabitCard}.tsx`, `src/App.tsx` (route `/report/:matchId`), `src/screens/Library.tsx` (row → report; "watch replay" link inside report header), `src/styles.css`.

**Before any code: invoke `frontend-design:frontend-design` (Skill tool) for the screen direction and `dataviz` (Skill tool) for the class breakdown + round strip.** Structure (§7 screen 2): header (map, score colored by result, tracked stats, "Open replay" link); round timeline strip (one cell per round: side-colored winner, tracked K/D dots, click → `/replay/:id?round=N`); death-class breakdown (horizontal bars via dataviz method — counts per class, class-13 called out as "fair duels — good news", honesty footnote listing not-yet-built classes); habits section (cross-match, top 3, evidence chips per match); insight feed grouped Deaths/Utility/Positioning/Timing ranked by score, card = narration title + body, metric chips, evidence chips ("R3 · 0:31" via fmtClock on round spec… chips label = `R{round}` + tick offset) → `evidenceUrl(matchId, ev)` navigation.
- [ ] Build; pure helpers unit-tested (`groupInsights`, chip label fn); typecheck/lint/vitest; commit `feat(ui): Match Report screen — narrated insight feed, class breakdown, habits, round strip`, push.

### Task 8 (inline): Analysis goldens refresh + docs

- [ ] Regenerate both analysis goldens (new rules change counts); update goldens README (note D4/D5 additions); `print_insights` CLASS_NAMES already updated (Task 0). Full workspace suite + frontend suite green. Commit, push.

### Task 9 (inline): E2E verification, docs, tag m4 — then OWNER review (the DoD)

- [ ] Fresh DB; set tracked setting; UI-import all 5 own demos (AX); open Report for mirage-tie: read narration texts via AX (title/body present, no template holes like "{}"), click one evidence chip → replay opens at right round (AX: header shows round); habits section shows ≥1 promoted habit (isolated/failed-trade will promote on this corpus); class breakdown renders.
- [ ] §12 sanity: 3 narrated insights spot-checked against DB facts; D4/D5 flags SQL cross-checked (≥3 instances: unsupported entry distances, early-aggro death timing/distance, slow-rotation distance at plant).
- [ ] Docs: PROGRESS (M4 done → M5 next), PROMPT §13 checkbox, spec addenda (new rule ids: H14_UNSUPPORTED_ENTRY, H11_EARLY_AGGRESSIVE_DEATH, H11_SLOW_ROTATION + class 11 shipped), CLAUDE.md if commands changed, plan checkboxes. Tag `m4`, push, CI green.
- [ ] **Hand the DoD to the owner**: final message asks them to open their Mirage report and confirm ≥1 insight is real and actionable — the milestone DoD is theirs to sign off.

---

## Self-review notes

- §13 M4 coverage: insight feed UI (T7) ✓; grouping/ranking severity×recurrence (§5 cross-cutting: score uses count; §5A adds confidence + cross-demo in habits) ✓; evidence chips → replay deep links (T7 via existing evidenceUrl) ✓; round timeline strip (T7 + T1 per_round_stats) ✓; TemplateNarrator v1 with §8 bar (T4, taste-reviewed) ✓; D4 (T2) + D5 (T3) ✓; cross-demo habits incl. H4_REPEAT_HOTSPOT (T5–6) ✓; owner DoD gate (T9) ✓.
- Placeholder scan: none — every rule has concrete conditions/confidences; narrator templates are enumerated with tone requirements and exact-string tests.
- Type consistency: RoundStat/DeathPos (T1) consumed by T6; HabitCfg (T0) by T5; Narration/MatchContext (T4) by T6; EvidenceRef unchanged.
- Known risks flagged: rule_flags previously lacked evidence (fixed via migration 3 + fallback); opening-duel definition is first-kill-based (first-damage refinement noted for later); slow-rotation gated on round-lost for precision; class 8/10/12 stay reserved and are listed in the report's honesty note.
