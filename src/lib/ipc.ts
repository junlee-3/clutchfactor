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
//   RoundReviewDto (+ RailMomentDto/PlayDto/TimelineDto) <- src-tauri/src/commands.rs
//   GridDto (src/replay/heatmap.ts) <- GridRow, src-tauri/crates/cf-store/src/store.rs
//   GridStatus/CorpusMapCount <- src-tauri/crates/cf-store/src/store.rs
//   CorpusStatus   <- src-tauri/src/commands.rs
//   TrendMatchRow  <- src-tauri/crates/cf-store/src/store.rs
//   TrendsDto (+ RuleSeries) <- src-tauri/src/commands.rs
//   AppSettings (+ ThresholdRow) <- src-tauri/src/commands.rs
//   ReAnalyzeResult <- src-tauri/src/commands.rs
//   CoachStatusDto/CoachRoundsDto (+ RoundCommentaryDto/PlayCommentDto)/CoachSynthesisDto (+ MatchSynthesisDto) <- src-tauri/src/commands.rs
//   MatchStatsDto <- src-tauri/src/commands.rs
//   PlayerRoundStatsDto <- src-tauri/src/commands.rs
//   CatalogEntryDto/ClassEntryDto/CatalogDto <- src-tauri/src/commands.rs
//   CalloutDto <- src-tauri/src/commands.rs
//   StatSeries (TrendsDto.stats) <- src-tauri/src/commands.rs
//   TrackedPlayer <- src-tauri/src/commands.rs
// Conventions: steamids are strings (steamid64 overflows JS number);
// command names are snake_case; Rust arg names arrive camelCased.

import { Channel, invoke } from "@tauri-apps/api/core";
import type { GridDto } from "../replay/heatmap";

export type { GridDto };

/** The single seam between the UI and the Rust side. In dev builds each
 *  call leaves a `performance.measure("ipc:<cmd>")` so the DevTools
 *  Performance tab shows IPC time per command; production pays nothing.
 *  `VITE_FAIL_IPC=<cmd>` (dev only) forces that one command to reject, so
 *  every screen's error branch can be provoked without touching the Rust
 *  side (polish-and-release.md §2). */
export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.DEV && import.meta.env.VITE_FAIL_IPC === cmd) {
    throw new Error(`forced failure: ${cmd}`);
  }
  if (!import.meta.env.DEV) return invoke<T>(cmd, args);
  const start = `ipc:${cmd}:start:${performance.now()}`;
  try {
    performance.mark(start);
  } catch {
    // Measurement must never break a call.
  }
  try {
    return await invoke<T>(cmd, args);
  } finally {
    try {
      performance.measure(`ipc:${cmd}`, start);
    } catch {
      // Measurement must never break a call.
    }
  }
}

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
  return call<MatchSummary[]>("list_matches");
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
  return call<MatchReport | null>("get_match_report", { matchId });
}

export function getHabits(): Promise<HabitReport[]> {
  return call<HabitReport[]>("get_habits");
}

// ---- V1.2: round-by-round coach rail ----

export interface RailMomentDto {
  tick: number;
  headline: string;
  facts: string[];
  rule_id: string | null;
  delta_p: number | null;
  kind: string;
  // Presence-ordered (victim, then killer, then nearest teammate — each
  // only when known); never assume a fixed slot. Read `killer` below for
  // the killer specifically, not focus[1].
  focus: string[];
  killer: string | null;
}

export interface PlayDto {
  tick: number;
  kind: string;
  phase: string;
  headline: string;
  facts: string[];
  quality: "good" | "bad" | "neutral" | null;
  rule_id: string | null;
  delta_p: number | null;
  focus: string[];
  killer: string | null;
}

export interface TimelineDto {
  tick: number;
  kind: string;
  actor: string | null;
  subject: string | null;
  side: "CT" | "T" | null;
  weapon: string | null;
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
  plays: PlayDto[];
  timeline: TimelineDto[];
  why_it_mattered: string | null;
  what_to_practise: string | null;
}

export function getRoundReview(matchId: number): Promise<RoundReviewDto[]> {
  return call<RoundReviewDto[]>("get_round_review", { matchId });
}

export function getMatchDetail(matchId: number): Promise<MatchDetail | null> {
  return call<MatchDetail | null>("get_match_detail", { matchId });
}

export function getRoundTicks(
  matchId: number,
  round: number,
): Promise<RoundTicks> {
  return call<RoundTicks>("get_round_ticks", { matchId, round });
}

// The sidebar's profile chip. `name` is the Steam persona when the profile
// could be reached, else the in-game name from the most recent own demo;
// `avatar` is an inlined data: URI, so rendering it never hits the network.
export interface TrackedPlayer {
  steamid: string;
  name: string | null;
  avatar: string | null;
}

export function trackedPlayer(): Promise<TrackedPlayer | null> {
  return call<TrackedPlayer | null>("tracked_player");
}

// Talks to Steam when the cached profile has aged out, then returns the
// tracked player again. Kept separate from `trackedPlayer` so a slow or
// unreachable Steam delays only the avatar, never the footer.
export function refreshTrackedProfile(): Promise<TrackedPlayer | null> {
  return call<TrackedPlayer | null>("refresh_tracked_profile");
}

export function importDemo(
  path: string,
  onProgress: (e: ProgressEvent) => void,
): Promise<ImportResult> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return call<ImportResult>("import_demo", { path, onProgress: channel });
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
  return call<ImportResult>("import_corpus_demo", {
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
  return call<number>("build_corpus", { map, onProgress: channel });
}

export function corpusStatus(): Promise<CorpusStatus> {
  return call<CorpusStatus>("corpus_status");
}

export function getGrid(
  map: string,
  side: "CT" | "T",
  phase: string,
): Promise<GridDto | null> {
  return call<GridDto | null>("get_grid", { map, side, phase });
}

export function analyzePositioning(matchId: number): Promise<number> {
  return call<number>("analyze_positioning", { matchId });
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

export interface StatSeries {
  key: string; // "kd" | "adr" | "hs" | "kast" | "entry" | "trade" | "clutch"
  title: string;
  unit: string;
  values: (number | null)[];
}

export interface TrendsDto {
  matches: TrendMatchRow[];
  rules: RuleSeries[];
  stats: StatSeries[];
}

export function getTrends(): Promise<TrendsDto> {
  return call<TrendsDto>("get_trends");
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
  return call<AppSettings>("get_app_settings");
}

export function setTrackedOverride(steamid: string | null): Promise<void> {
  return call<void>("set_tracked_override", { steamid });
}

export interface ReAnalyzeResult {
  needs_file: boolean;
  file_name: string;
  map: string;
}

export function reAnalyzeMatch(
  matchId: number,
  path: string | null,
  onProgress: (e: ProgressEvent) => void,
): Promise<ReAnalyzeResult> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  return call<ReAnalyzeResult>("re_analyze_match", { matchId, path, onProgress: channel });
}

export function deleteMatch(matchId: number): Promise<void> {
  return call<void>("delete_match", { matchId });
}

// ---- V1.3: the coach ----

export interface CoachStatusDto {
  enabled: boolean;
  key_source: "env" | "settings" | null;
  key_hint: string | null;
  round_model: string;
  synthesis_model: string;
}

export interface PlayCommentDto {
  tick: number;
  comment: string;
}

export interface RoundCommentaryDto {
  round: number;
  read: string;
  plays: PlayCommentDto[];
  why_it_mattered: string | null;
  what_to_practise: string | null;
  focus: string | null;
  model: string;
}

export interface CoachRoundsDto {
  rounds: RoundCommentaryDto[];
  error: string | null;
}

export interface MatchSynthesisDto {
  opening: string;
  work_on: string[];
  model: string;
}

export interface CoachSynthesisDto {
  synthesis: MatchSynthesisDto | null;
  error: string | null;
}

export function coachStatus(): Promise<CoachStatusDto> {
  return call<CoachStatusDto>("coach_status");
}

export function setGeminiKey(key: string | null): Promise<void> {
  return call<void>("set_gemini_key", { key });
}

export function setCoachModels(
  roundModel: string,
  synthesisModel: string,
): Promise<void> {
  return call<void>("set_coach_models", { roundModel, synthesisModel });
}

export function setCoachEnabled(enabled: boolean): Promise<void> {
  return call<void>("set_coach_enabled", { enabled });
}

export function testGeminiKey(): Promise<string> {
  return call<string>("test_gemini_key");
}

export function getCoachRounds(matchId: number): Promise<CoachRoundsDto> {
  return call<CoachRoundsDto>("get_coach_rounds", { matchId });
}

export function regenerateCoachRound(
  matchId: number,
  round: number,
): Promise<CoachRoundsDto> {
  return call<CoachRoundsDto>("regenerate_coach_round", { matchId, round });
}

export function getCoachSynthesis(matchId: number): Promise<CoachSynthesisDto> {
  return call<CoachSynthesisDto>("get_coach_synthesis", { matchId });
}

export function regenerateCoachSynthesis(
  matchId: number,
): Promise<CoachSynthesisDto> {
  return call<CoachSynthesisDto>("regenerate_coach_synthesis", { matchId });
}

// ---- V1.4: stats & understanding ----

export interface MatchStatsDto {
  rounds_played: number;
  kills: number;
  deaths: number;
  assists: number;
  kd: number | null;
  adr: number | null;
  hs_pct: number | null;
  kast_pct: number | null;
  entry_attempts: number;
  entry_wins: number;
  traded_deaths: number;
  trade_kills: number;
  trade_opportunities: number;
  clutch_attempts: number;
  clutch_wins: number;
}

export interface PlayerRoundStatsDto {
  round: number;
  steamid: string;
  name: string;
  side: "CT" | "T";
  kills: number;
  deaths: number;
  assists: number;
  damage: number;
  headshots: number;
  survived: boolean;
  traded: boolean;
  entry: string | null;
  tracked: boolean;
}

export interface CatalogEntryDto {
  id: string;
  family: string;
  title: string;
  watches_for: string;
  thresholds: string;
  class_id: number | null;
  example: string;
  stat_links: string[];
}

export interface ClassEntryDto {
  id: number;
  name: string;
  source: string;
  built: boolean;
  why_not: string | null;
}

export interface CatalogDto {
  entries: CatalogEntryDto[];
  classes: ClassEntryDto[];
  cannot_see: [string, string][];
}

export interface CalloutDto {
  place: string;
  name: string;
  x: number;
  y: number;
  /** Median height of the place's samples — picks the label's radar layer. */
  z: number;
  samples: number;
}

export function getMatchStats(matchId: number): Promise<MatchStatsDto | null> {
  return call<MatchStatsDto | null>("get_match_stats", { matchId });
}

export function getRoundScoreboard(
  matchId: number,
  round: number | null,
): Promise<PlayerRoundStatsDto[]> {
  return call<PlayerRoundStatsDto[]>("get_round_scoreboard", {
    matchId,
    round,
  });
}

export function getDetectorCatalog(): Promise<CatalogDto> {
  return call<CatalogDto>("get_detector_catalog");
}

export function getMapCallouts(map: string): Promise<CalloutDto[]> {
  return call<CalloutDto[]>("get_map_callouts", { map });
}
