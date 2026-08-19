// Typed wrappers around Tauri invoke.
//
// MIRROR CHECKLIST — these types are hand-mirrored from Rust; update BOTH
// sides in the same commit (tauri-specta is still RC, re-evaluate at M2):
//   MatchSummary   <- src-tauri/crates/cf-store/src/store.rs
//   ProgressEvent  <- src-tauri/src/commands.rs
//   ImportResult   <- src-tauri/src/commands.rs
// Conventions: steamids are strings (steamid64 overflows JS number);
// command names are snake_case; Rust arg names arrive camelCased.

import { Channel, invoke } from "@tauri-apps/api/core";

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

export function listMatches(): Promise<MatchSummary[]> {
  return invoke<MatchSummary[]>("list_matches");
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
