// Typed wrappers around Tauri invoke.
//
// MIRROR CHECKLIST — these types are hand-mirrored from Rust; update BOTH
// sides in the same commit (tauri-specta is still RC, re-evaluate at M2):
//   MatchSummary   <- src-tauri/crates/cf-store/src/store.rs
//   MatchDetail (+ RoundInfo/KillInfo/GrenadeInfo/BombInfo/PlayerInfo/RoundSideInfo)
//                  <- src-tauri/crates/cf-store/src/store.rs
//   RoundTicks     <- src-tauri/crates/cf-store/src/store.rs
//   ProgressEvent  <- src-tauri/src/commands.rs
//   ImportResult   <- src-tauri/src/commands.rs
//   MatchReport (+ NarratedInsight/NarrationDto/DeathClassRow/RoundStat)
//                  <- src-tauri/src/commands.rs + cf-store store.rs
//   HabitReport (+ HabitEvidence) <- src-tauri/src/commands.rs
//   EvidenceRefDto <- EvidenceRef, src-tauri/crates/cf-analysis/src/types.rs
//   RoundReviewDto (+ RailMomentDto) <- src-tauri/src/commands.rs
//   GridDto (src/replay/heatmap.ts) <- GridRow, src-tauri/crates/cf-store/src/store.rs
//   GridStatus/CorpusMapCount <- src-tauri/crates/cf-store/src/store.rs
//   CorpusStatus   <- src-tauri/src/commands.rs
//   TrendMatchRow  <- src-tauri/crates/cf-store/src/store.rs
//   TrendsDto (+ RuleSeries) <- src-tauri/src/commands.rs
//   AppSettings (+ ThresholdRow) <- src-tauri/src/commands.rs
// Conventions: steamids are strings (steamid64 overflows JS number);
// command names are snake_case; Rust arg names arrive camelCased.

import { Channel, invoke } from "@tauri-apps/api/core";
import type { GridDto } from "../replay/heatmap";

export type { GridDto };

export interface MatchSummary {
  id: number;
  file_name: string;
  map: string;
  imported_at: string;
  rounds: number;
  score_a: number;
  score_b: number;
  tracked_steamid: string | null;
  tracked_result: "win" | "loss" | "tie" | null;
  tracked_kills: number | null;
  tracked_deaths: number | null;
  tracked_hs_pct: number | null;
}

export interface ProgressEvent {
  stage: string;
  pct: number;
  detail: string;
}

export interface ImportResult {
  match_id: number;
  map: string;
  score_a: number;
  score_b: number;
}

export interface RoundInfo {
  number: number;
  start_tick: number;
  freeze_end_tick: number | null;
  end_tick: number;
  officially_ended_tick: number | null;
  winner: "CT" | "T";
  reason: string;
}

export interface KillInfo {
  round: number;
  tick: number;
  attacker: string | null;
  victim: string;
  assister: string | null;
  weapon: string;
  headshot: boolean;
}

export interface GrenadeInfo {
  tick: number;
  kind: string;
  thrower: string | null;
  x: number;
  y: number;
  z: number;
}

export interface BombInfo {
  tick: number;
  kind: "planted" | "defused" | "exploded";
  player: string | null;
}

export interface PlayerInfo {
  steamid: string;
  name: string;
}

export interface RoundSideInfo {
  number: number;
  steamid: string;
  side: "CT" | "T";
}

export interface MatchDetail {
  id: number;
  map: string;
  tickrate: number;
  sample_every: number;
  score_a: number;
  score_b: number;
  players: PlayerInfo[];
  rounds: RoundInfo[];
  kills: KillInfo[];
  grenades: GrenadeInfo[];
  bomb_events: BombInfo[];
  round_sides: RoundSideInfo[];
}

export interface RoundTicks {
  tick: number[];
  steamid: string[];
  x: number[];
  y: number[];
  z: number[];
  yaw: number[];
  health: number[];
  is_alive: boolean[];
  team_num: number[];
  active_weapon: (string | null)[];
  last_place: (string | null)[];
}

export function listMatches(): Promise<MatchSummary[]> {
  return invoke<MatchSummary[]>("list_matches");
}

export interface EvidenceRefDto {
  round: number;
  tick_start: number;
  tick_end: number;
  focus_players: string[];
  camera_hint: string | null;
}

export interface NarrationDto {
  title: string;
  body: string;
}

export interface NarratedInsight {
  detector: string;
  category: "deaths" | "utility" | "positioning" | "timing";
  severity: number;
  confidence: number;
  round: number;
  score: number;
  title: string;
  body: string;
  metrics: Record<string, unknown>;
  evidence: EvidenceRefDto[];
}

export interface DeathClassRow {
  round: number;
  tick: number;
  victim: string;
  class_id: number;
  class_source: string;
  secondary_tags_json: string;
  confidence: number;
}

export interface RoundStat {
  number: number;
  freeze_end_tick: number | null;
  winner: "CT" | "T";
  tracked_side: "CT" | "T" | null;
  kills: number;
  deaths: number;
}

export interface MatchReport {
  match_id: number;
  map: string;
  score_a: number;
  score_b: number;
  tracked: string | null;
  tracked_result: "win" | "loss" | "tie" | null;
  summary: NarrationDto | null;
  insights: NarratedInsight[];
  death_classes: DeathClassRow[];
  class_13_share_pct: number;
  per_round: RoundStat[];
  classes_not_built: number[];
}

export interface HabitEvidence {
  match_id: number;
  map: string;
  evidence: EvidenceRefDto;
}

export interface HabitReport {
  rule_id: string;
  title: string;
  body: string;
  matches_hit: number;
  window: number;
  total: number;
  score: number;
  evidence: HabitEvidence[];
}

export function getMatchReport(matchId: number): Promise<MatchReport | null> {
  return invoke<MatchReport | null>("get_match_report", { matchId });
}

export function getHabits(): Promise<HabitReport[]> {
  return invoke<HabitReport[]>("get_habits");
}

// ---- V1.2: round-by-round coach rail ----

export interface RailMomentDto {
  tick: number;
  headline: string;
  facts: string[];
  rule_id: string | null;
  delta_p: number | null;
  kind: string;
  focus: string[];
}

export interface RoundReviewDto {
  round: number;
  impact: number;
  verdict: string;
  verdict_label: string;
  attention: "none" | "dim" | "bright";
  selected: boolean;
  pivotal_tick: number;
  side: "CT" | "T";
  won: boolean;
  kills: number;
  deaths: number;
  man_context: string | null;
  moments: RailMomentDto[];
  why_it_mattered: string | null;
  what_to_practise: string | null;
}

export function getRoundReview(matchId: number): Promise<RoundReviewDto[]> {
  return invoke<RoundReviewDto[]>("get_round_review", { matchId });
}

export function getMatchDetail(matchId: number): Promise<MatchDetail | null> {
  return invoke<MatchDetail | null>("get_match_detail", { matchId });
}

export function getRoundTicks(
  matchId: number,
  round: number,
): Promise<RoundTicks> {
  return invoke<RoundTicks>("get_round_ticks", { matchId, round });
}

export function trackedPlayer(): Promise<string | null> {
  return invoke<string | null>("tracked_player");
}

export function importDemo(
  path: string,
  onProgress: (e: ProgressEvent) => void,
): Promise<ImportResult> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<ImportResult>("import_demo", { path, onProgress: channel });
}

// ---- M5: reference corpus + D6 positioning ----

export interface CorpusMapCount {
  map: string;
  demos: number;
}

export interface GridStatus {
  map: string;
  side: "CT" | "T";
  phase: string;
  demos: number;
  samples: number;
  built_at: string;
}

export interface CorpusStatus {
  maps: CorpusMapCount[];
  grids: GridStatus[];
  min_demos_per_map: number;
}

export function importCorpusDemo(
  path: string,
  onProgress: (e: ProgressEvent) => void,
): Promise<ImportResult> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<ImportResult>("import_corpus_demo", {
    path,
    onProgress: channel,
  });
}

export function buildCorpus(
  map: string | null,
  onProgress: (e: ProgressEvent) => void,
): Promise<number> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<number>("build_corpus", { map, onProgress: channel });
}

export function corpusStatus(): Promise<CorpusStatus> {
  return invoke<CorpusStatus>("corpus_status");
}

export function getGrid(
  map: string,
  side: "CT" | "T",
  phase: string,
): Promise<GridDto | null> {
  return invoke<GridDto | null>("get_grid", { map, side, phase });
}

export function analyzePositioning(matchId: number): Promise<number> {
  return invoke<number>("analyze_positioning", { matchId });
}

// ---- M6: trends ----

export interface TrendMatchRow {
  match_id: number;
  imported_at: string;
  map: string;
  deaths: number;
  class13_pct: number;
}

export interface RuleSeries {
  rule_id: string;
  title: string;
  counts: number[];
  total: number;
}

export interface TrendsDto {
  matches: TrendMatchRow[];
  rules: RuleSeries[];
}

export function getTrends(): Promise<TrendsDto> {
  return invoke<TrendsDto>("get_trends");
}

// ---- M6: settings + housekeeping ----

export interface ThresholdRow {
  name: string;
  value: string;
  unit: string;
}

export interface AppSettings {
  tracked_override: string | null;
  tracked_effective: string | null;
  tracked_name: string | null;
  db_path: string;
  own_matches: number;
  corpus_demos: number;
  thresholds: ThresholdRow[];
}

export function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_app_settings");
}

export function setTrackedOverride(steamid: string | null): Promise<void> {
  return invoke<void>("set_tracked_override", { steamid });
}

export function deleteMatch(matchId: number): Promise<void> {
  return invoke<void>("delete_match", { matchId });
}
