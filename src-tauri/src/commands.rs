//! Tauri commands. Snake_case names, typed payloads mirrored by hand in
//! `src/lib/ipc.ts` (keep the MIRROR CHECKLIST there in sync). Steamids are
//! strings on the wire.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cf_analysis::corpus::{self, Phase, PhaseSample, TrackedMoment};
use cf_parser::extract::{parse_match, ImportStage};
use cf_parser::model::Side;
use cf_store::store::{GridRow, MatchDetail, RoundTicks};
use cf_store::{MatchSummary, Store, StoreError};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::State;

use crate::perf::timed;

/// Per ADR-0002.
const SAMPLE_EVERY: u32 = 4;

/// Detector thresholds: shipped defaults (§6.4), or — when the
/// `CLUTCHFACTOR_CONFIG` env var names a YAML file — that file merged over
/// them field-by-field. Dev/testing escape hatch only (e.g. lowering the D6
/// corpus gate against a small fixture corpus); nothing sets it in a
/// packaged app, so users always run the documented defaults.
fn detector_config() -> cf_analysis::DetectorConfig {
    if let Ok(path) = std::env::var("CLUTCHFACTOR_CONFIG") {
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|y| cf_analysis::DetectorConfig::from_yaml(&y))
        {
            Ok(cfg) => {
                eprintln!("detector config override loaded from {path}");
                return cfg;
            }
            Err(e) => eprintln!("ignoring CLUTCHFACTOR_CONFIG ({path}): {e}"),
        }
    }
    cf_analysis::DetectorConfig::default()
}

/// One async mutex per match id, created on first use (`coach::match_lock`).
/// The outer std lock guards only the map lookup and is never held across
/// an await.
pub type CoachLocks = Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>;

pub struct AppState {
    pub store: Mutex<Store>,
    /// The coach generates a match's commentary under this lock so the two
    /// queries a first open fires (rounds, synthesis) can't both pay for
    /// the same rounds (V1.3 final-review fix #5).
    pub coach_locks: CoachLocks,
    /// Per-match memo of `Store::distinct_places` (V1.5 perf: it scanned
    /// tick_samples on every coach call). Invalidated on re-analyze/delete.
    pub places: crate::coach::places::PlacesCache,
}

#[derive(Clone, serde::Serialize)]
pub struct ProgressEvent {
    pub stage: String,
    pub pct: f32,
    pub detail: String,
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub match_id: i64,
    pub map: String,
    pub score_a: u32,
    pub score_b: u32,
}

fn send(ch: &Channel<ProgressEvent>, stage: &str, pct: f32, detail: &str) {
    let _ = ch.send(ProgressEvent {
        stage: stage.to_string(),
        pct,
        detail: detail.to_string(),
    });
}

/// Reads the whole file and SHA-256-hashes it off the async runtime. Shared
/// by `parse_and_save` (import) and `re_analyze_match` (backfill), so both
/// paths hash identically.
async fn hash_demo(path: &std::path::Path) -> Result<String, String> {
    let hash_path = path.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let bytes = std::fs::read(&hash_path).map_err(|e| format!("cannot read demo: {e}"))?;
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Parses a demo off the async runtime, mapping `ImportStage` to progress
/// events. Shared by `parse_and_save` (import) and `re_analyze_match`
/// (backfill) so both paths report progress and word failures identically
/// (§7 voice).
async fn parse_demo(
    path: &std::path::Path,
    file_name: &str,
    on_progress: &Channel<ProgressEvent>,
) -> Result<cf_parser::model::MatchData, String> {
    let parse_path = path.to_path_buf();
    let progress_channel = on_progress.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut progress = |stage: ImportStage, pct: f32| {
            let (name, detail) = match stage {
                ImportStage::Reading => ("reading", "Reading demo"),
                ImportStage::Parsing => ("parsing", "Parsing events and positions"),
                ImportStage::Normalizing => ("normalizing", "Normalizing rounds"),
            };
            send(&progress_channel, name, pct.clamp(0.0, 0.9), detail);
        };
        parse_match(&parse_path, SAMPLE_EVERY, &mut progress)
    })
    .await
    // §7 voice, belt-and-braces: cf-parser's own catch_unwind (extract.rs)
    // should always turn a demoparser2 panic into a graceful `ParseError`
    // before this JoinError case is even reachable — but if the parse task
    // is ever aborted/panics some other way, never leak the raw JoinError
    // text (e.g. `task N panicked with message "..."`) to the UI.
    .map_err(|_| {
        format!(
            "Couldn't parse {file_name}: the parser crashed on this file. If this \
             demo is from a different game or the download was cut short, \
             re-download it and try again."
        )
    })?
    // §7 voice: say what happened and what to do next. A failed parse has
    // written nothing — the save only happens after this point.
    .map_err(|e| {
        format!(
            "Couldn't parse {file_name}: {e}. If this demo is from a different \
             game or the download was cut short, re-download it and try again."
        )
    })
}

/// Hash → duplicate check → parse → save, shared by own and corpus imports.
/// Returns the saved id, the parsed data (own imports analyze it next) and
/// the derived score.
async fn parse_and_save(
    state: &State<'_, AppState>,
    path: String,
    on_progress: &Channel<ProgressEvent>,
    kind: cf_store::store::MatchKind,
) -> Result<(i64, cf_parser::model::MatchData, u32, u32), String> {
    let file = PathBuf::from(&path);
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| "not a file path".to_string())?;

    send(on_progress, "hashing", 0.0, "Hashing demo file");
    let file_hash = hash_demo(&file).await?;

    // Reject duplicates before the expensive parse (save_match re-checks).
    {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        if store.has_file_hash(&file_hash).map_err(|e| e.to_string())? {
            return Err(StoreError::DuplicateImport.to_string());
        }
    }

    send(on_progress, "parsing", 0.05, "Parsing demo");
    let data = parse_demo(&file, &file_name, on_progress).await?;

    send(on_progress, "saving", 0.88, "Saving to library");
    let match_id = {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let match_id = store
            .save_match(&file_name, &file_hash, kind, &data)
            .map_err(|e| match e {
                StoreError::DuplicateImport => e.to_string(),
                other => format!("failed to save match: {other}"),
            })?;
        store
            .set_source_path(match_id, &path)
            .map_err(|e| format!("failed to record demo path: {e}"))?;
        match_id
    };
    let (_, _, score_a, score_b) = cf_parser::extract::derive_score(&data.rounds);
    Ok((match_id, data, score_a, score_b))
}

/// Detectors → round review → (D6 when grids exist), persisted. Shared by
/// `import_demo` and `re_analyze_match`. A no-op without a tracked player.
async fn analyze_and_persist(
    state: &State<'_, AppState>,
    match_id: i64,
    data: cf_parser::model::MatchData,
    on_progress: &Channel<ProgressEvent>,
) -> Result<(), String> {
    let map = data.map.clone();
    let tracked = {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store.tracked_steamid().map_err(|e| e.to_string())?
    };
    // Analysis needs a tracked player; after save_match the modal fallback
    // always yields one for a non-empty library.
    if let Some(tracked) = tracked.and_then(|t| t.parse::<u64>().ok()) {
        send(on_progress, "analyzing", 0.92, "Running detectors");
        let cfg = detector_config();
        let analysis = tauri::async_runtime::spawn_blocking(move || {
            cf_analysis::analyze(&data, tracked, &cfg)
        })
        .await
        .map_err(|e| e.to_string())?;
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .save_analysis(match_id, &analysis)
            .map_err(|e| e.to_string())?;
        send(on_progress, "analyzing", 0.95, "Scoring rounds");
        run_round_review(&mut store, match_id)?;
        // D6 runs outside analyze(): it needs the corpus grids, which only
        // exist once pro demos were imported and built for this map.
        let has_grids = !store
            .load_grids(&map)
            .map_err(|e| e.to_string())?
            .is_empty();
        if has_grids {
            send(
                on_progress,
                "analyzing",
                0.97,
                "Comparing positioning to corpus",
            );
            run_positioning(&mut store, match_id)?;
        }
    }
    // Runs whether or not analysis ran above: tick_samples exist as soon as
    // the demo is parsed and saved, independent of a resolvable tracked
    // player, so a match with no tracked player must not skip its
    // contribution to the map's callout labels (V1.4 review round 1,
    // finding #1). Still synchronous under this lock, never across an
    // `.await`; non-fatal (§7) — a stale callout label is far less bad than
    // failing the whole import over it.
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    if let Err(e) = refresh_map_callouts(&mut store, &map) {
        eprintln!("map callouts for {map}: {e}");
    }
    Ok(())
}

#[tauri::command]
pub async fn import_demo(
    state: State<'_, AppState>,
    path: String,
    on_progress: Channel<ProgressEvent>,
) -> Result<ImportResult, String> {
    let (match_id, data, score_a, score_b) =
        parse_and_save(&state, path, &on_progress, cf_store::store::MatchKind::Own).await?;
    let map = data.map.clone();
    analyze_and_persist(&state, match_id, data, &on_progress).await?;

    send(&on_progress, "done", 1.0, "Import complete");
    Ok(ImportResult {
        match_id,
        map,
        score_a,
        score_b,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct ReAnalyzeResult {
    /// True when no usable file was found — nothing at the recorded
    /// `source_path`, or a file there whose contents no longer match the
    /// imported demo: the UI must ask the user to pick the demo and call
    /// again with `path`.
    pub needs_file: bool,
    pub file_name: String,
    pub map: String,
}

/// Which file `re_analyze_match` should read, and whether the user picked
/// it: an explicit `path` wins over the stored `source_path`; a candidate
/// that isn't a file on disk is dropped (the caller then asks for the file).
/// `is_file` is injected so the resolution is testable off the disk.
fn resolve_candidate(
    path: Option<String>,
    source_path: Option<String>,
    is_file: impl Fn(&str) -> bool,
) -> Option<(String, bool)> {
    path.map(|p| (p, true))
        .or_else(|| source_path.map(|p| (p, false)))
        .filter(|(p, _)| is_file(p))
}

/// What a hash mismatch means depends on who chose the file (V1.2b
/// final-review fix wave, #4): a stale file sitting at the stored
/// `source_path` (`from_user == false`) is "we couldn't find the demo" —
/// `needs_file`, so the Library opens the picker instead of dead-ending on
/// an error the user can't act on — while a file the user just picked
/// (`from_user == true`) is refused outright rather than silently replacing
/// the match.
fn hash_mismatch_result(from_user: bool, file_name: &str) -> Result<ReAnalyzeResult, String> {
    if from_user {
        Err(format!(
            "That file isn't {file_name} — its contents don't match the imported demo. Pick the original file."
        ))
    } else {
        Ok(ReAnalyzeResult {
            needs_file: true,
            file_name: file_name.to_string(),
            map: String::new(),
        })
    }
}

/// Re-parses a match's demo and re-runs the whole pipeline in place (V1.2b
/// spec §2 "Backfill"). Uses the recorded `source_path`, or `path` when
/// given; the file's hash must equal the stored `file_hash` — a different
/// file is refused rather than silently replacing the match (a mismatch at
/// the stored path just asks for the file — see `hash_mismatch_result`).
///
/// Ordering guarantee: the parse completes (and is hash-verified) before any
/// row changes; `replace_match_data` then swaps the parsed rows AND clears
/// the old analysis rows in one transaction, so a partial failure past that
/// point leaves the match analysis-less (like a fresh import), never
/// contradictory (new parsed rows next to old analysis rows).
#[tauri::command]
pub async fn re_analyze_match(
    state: State<'_, AppState>,
    match_id: i64,
    path: Option<String>,
    on_progress: Channel<ProgressEvent>,
) -> Result<ReAnalyzeResult, String> {
    let file = {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .match_file(match_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "That match is no longer in the library.".to_string())?
    };
    let candidate = resolve_candidate(path, file.source_path.clone(), |p| {
        std::path::Path::new(p).is_file()
    });
    let Some((demo_path, from_user)) = candidate else {
        return Ok(ReAnalyzeResult {
            needs_file: true,
            file_name: file.file_name,
            map: String::new(),
        });
    };

    send(&on_progress, "hashing", 0.0, "Checking the demo file");
    let hash_path = PathBuf::from(&demo_path);
    let file_hash = hash_demo(&hash_path).await?;
    if file_hash != file.file_hash {
        return hash_mismatch_result(from_user, &file.file_name);
    }

    send(&on_progress, "parsing", 0.05, "Parsing demo");
    let parse_path = PathBuf::from(&demo_path);
    let data = parse_demo(&parse_path, &file.file_name, &on_progress).await?;

    send(&on_progress, "saving", 0.88, "Replacing match data");
    let map = data.map.clone();
    {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        // Also clears rule_flags/insights/death_class/round_review/
        // round_plays in the same transaction (cf-store
        // `MATCH_ANALYSIS_TABLES`), so a failure in `analyze_and_persist`
        // below can't leave the fresh rows beside a stale analysis.
        store
            .replace_match_data(match_id, &data)
            .map_err(|e| format!("failed to replace match data: {e}"))?;
        store
            .set_source_path(match_id, &demo_path)
            .map_err(|e| e.to_string())?;
    }
    // The re-parse just replaced tick_samples, so any cached places for
    // this match are stale the moment `replace_match_data` commits —
    // independent of whether the analysis below succeeds.
    state.places.invalidate(match_id);
    if let Err(e) = analyze_and_persist(&state, match_id, data, &on_progress).await {
        return Err(format!(
            "Re-parsed {}, but the analysis didn't finish: {e}. Run Re-analyze again.",
            file.file_name
        ));
    }
    send(&on_progress, "done", 1.0, "Re-analyze complete");
    Ok(ReAnalyzeResult {
        needs_file: false,
        file_name: file.file_name,
        map,
    })
}

// ---- V1.3: the coach ----

#[derive(serde::Serialize)]
pub struct CoachStatusDto {
    pub enabled: bool,
    /// "env" | "settings" | null
    pub key_source: Option<String>,
    /// "…ab12" — never the key itself.
    pub key_hint: Option<String>,
    pub round_model: String,
    pub synthesis_model: String,
}

#[derive(Clone, serde::Serialize)]
pub struct PlayCommentDto {
    pub tick: i32,
    pub comment: String,
}

#[derive(Clone, serde::Serialize)]
pub struct RoundCommentaryDto {
    pub round: u32,
    pub read: String,
    pub plays: Vec<PlayCommentDto>,
    pub why_it_mattered: Option<String>,
    pub what_to_practise: Option<String>,
    pub focus: Option<String>,
    pub model: String,
}

#[derive(serde::Serialize)]
pub struct CoachRoundsDto {
    pub rounds: Vec<RoundCommentaryDto>,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
pub struct MatchSynthesisDto {
    pub opening: String,
    pub work_on: Vec<String>,
    pub model: String,
}

#[derive(serde::Serialize)]
pub struct CoachSynthesisDto {
    pub synthesis: Option<MatchSynthesisDto>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn coach_status(state: State<'_, AppState>) -> Result<CoachStatusDto, String> {
    use crate::coach::key::*;
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let key = resolve_key(&store)?;
    let enabled = coach_enabled(&store)? && key.is_some();
    let model = |k: &str, d: &str| {
        store
            .get_setting(k)
            .map(|v| {
                v.filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| d.to_string())
            })
            .map_err(|e| e.to_string())
    };
    Ok(CoachStatusDto {
        enabled,
        key_source: key.as_ref().map(|(_, s)| s.as_str().to_string()),
        key_hint: key.as_ref().map(|(k, _)| k.hint()),
        round_model: model(
            SETTING_ROUND_MODEL,
            cf_narrator::coach::prompt::DEFAULT_ROUND_MODEL,
        )?,
        synthesis_model: model(
            SETTING_SYNTHESIS_MODEL,
            cf_narrator::coach::prompt::DEFAULT_SYNTHESIS_MODEL,
        )?,
    })
}

/// `None`/empty clears the stored key; anything else must look like a key
/// (no whitespace, at least 20 characters). The value is never logged.
#[tauri::command]
pub fn set_gemini_key(state: State<'_, AppState>, key: Option<String>) -> Result<(), String> {
    use crate::coach::key::SETTING_KEY;
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    match key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty()) {
        None => {
            store
                .delete_setting(SETTING_KEY)
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        Some(k) => {
            if k.len() < 20 || k.chars().any(char::is_whitespace) {
                return Err("That doesn't look like a Gemini API key — paste the whole key from Google AI Studio.".to_string());
            }
            store
                .set_setting(SETTING_KEY, &k)
                .map_err(|e| e.to_string())
        }
    }
}

/// A Gemini model id as we accept it: letters, digits, dots and dashes.
pub fn is_model_id(v: &str) -> bool {
    !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Empty → default model (the setting is deleted). Both ids are validated
/// before either is written, so a bad synthesis id never leaves a half-saved
/// pair behind.
#[tauri::command]
pub fn set_coach_models(
    state: State<'_, AppState>,
    round_model: String,
    synthesis_model: String,
) -> Result<(), String> {
    use crate::coach::key::{SETTING_ROUND_MODEL, SETTING_SYNTHESIS_MODEL};
    let pairs = [
        (SETTING_ROUND_MODEL, round_model.trim()),
        (SETTING_SYNTHESIS_MODEL, synthesis_model.trim()),
    ];
    for (_, v) in &pairs {
        if !v.is_empty() && !is_model_id(v) {
            return Err(format!(
                "\"{v}\" is not a model id (letters, digits, dots and dashes only)."
            ));
        }
    }
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    for (k, v) in pairs {
        if v.is_empty() {
            store.delete_setting(k).map_err(|e| e.to_string())?;
        } else {
            store.set_setting(k, v).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_coach_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    use crate::coach::key::SETTING_ENABLED;
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    if enabled {
        store
            .delete_setting(SETTING_ENABLED)
            .map_err(|e| e.to_string())
    } else {
        store
            .set_setting(SETTING_ENABLED, "0")
            .map_err(|e| e.to_string())
    }
}

/// One tiny round-trip with the configured round model:
/// "Connected — gemini-3.7-flash answered in 812 ms."
#[tauri::command]
pub async fn test_gemini_key(state: State<'_, AppState>) -> Result<String, String> {
    use crate::coach::{gemini::GeminiClient, key::*, CoachError};
    let (key, model) = {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let Some((key, _)) = resolve_key(&store)? else {
            return Err(CoachError::NoKey.to_string());
        };
        let model = store
            .get_setting(SETTING_ROUND_MODEL)
            .map_err(|e| e.to_string())?
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| cf_narrator::coach::prompt::DEFAULT_ROUND_MODEL.to_string());
        (key, model)
    };
    let client = GeminiClient::new(key).map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let schema = serde_json::json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"]});
    client
        .generate_json(
            &model,
            "Answer with JSON.",
            "Reply with {\"ok\": true}.",
            &schema,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "Connected — {model} answered in {} ms.",
        started.elapsed().as_millis()
    ))
}

#[tauri::command]
pub async fn get_coach_rounds(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<CoachRoundsDto, String> {
    crate::coach::round_commentary(&state, match_id, &[]).await
}

#[tauri::command]
pub async fn regenerate_coach_round(
    state: State<'_, AppState>,
    match_id: i64,
    round: u32,
) -> Result<CoachRoundsDto, String> {
    crate::coach::round_commentary(&state, match_id, &[round]).await
}

#[tauri::command]
pub async fn get_coach_synthesis(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<CoachSynthesisDto, String> {
    crate::coach::synthesis(&state, match_id, false).await
}

#[tauri::command]
pub async fn regenerate_coach_synthesis(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<CoachSynthesisDto, String> {
    crate::coach::synthesis(&state, match_id, true).await
}

#[tauri::command]
pub fn list_matches(state: State<'_, AppState>) -> Result<Vec<MatchSummary>, String> {
    timed("list_matches", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store.list_matches().map_err(|e| e.to_string())
    })
}

/// The sidebar's profile chip (issue #39). `name` prefers the Steam persona
/// and falls back to the in-game name from the most recent own demo, so the
/// chip is still readable with no network; `avatar` is an inlined `data:` URI
/// or `None`, which the sidebar renders as an initials placeholder.
#[derive(serde::Serialize)]
pub struct TrackedPlayer {
    pub steamid: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

/// Reads whatever is already on disk — the demo name and the last cached
/// Steam profile, stale or not. Deliberately synchronous and network-free so
/// the sidebar paints immediately; `refresh_tracked_profile` does the
/// talking to Steam.
fn tracked_from_store(store: &Store) -> Result<Option<TrackedPlayer>, String> {
    let Some(steamid) = store.tracked_steamid().map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let demo_name = store.player_name(&steamid).map_err(|e| e.to_string())?;
    let cached = crate::steam::read_cache(store, &steamid).unwrap_or_default();
    Ok(Some(TrackedPlayer {
        steamid,
        name: cached.persona.or(demo_name),
        avatar: cached.avatar,
    }))
}

#[tauri::command]
pub fn tracked_player(state: State<'_, AppState>) -> Result<Option<TrackedPlayer>, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    tracked_from_store(&store)
}

/// Refreshes the cached Steam profile if it has aged out, then returns the
/// tracked player again. The sidebar runs this behind the synchronous read
/// above, so a slow or unreachable Steam delays the avatar but never the
/// footer itself.
#[tauri::command]
pub async fn refresh_tracked_profile(
    state: State<'_, AppState>,
) -> Result<Option<TrackedPlayer>, String> {
    // Scoped: the guard must not survive into the await below.
    let (steamid, fresh) = {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let Some(steamid) = store.tracked_steamid().map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let fresh = crate::steam::read_cache(&store, &steamid).is_some_and(|p| p.is_fresh());
        (steamid, fresh)
    };

    if !fresh {
        // Offline, rate-limited, private: leave the cache alone and let the
        // caller keep showing the last known profile.
        if let Ok(fetched) = crate::steam::fetch(&steamid).await {
            let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
            crate::steam::write_cache(&mut store, &steamid, &fetched);
        }
    }

    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    tracked_from_store(&store)
}

#[tauri::command]
pub fn get_match_detail(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Option<MatchDetail>, String> {
    timed("get_match_detail", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store.match_detail(match_id).map_err(|e| e.to_string())
    })
}

// ---- M4: match report + cross-demo habits ----

#[derive(serde::Serialize)]
pub struct NarrationDto {
    pub title: String,
    pub body: String,
}

#[derive(serde::Serialize)]
pub struct NarratedInsight {
    pub detector: String,
    pub category: String,
    pub severity: f32,
    pub confidence: f32,
    pub round: u32,
    pub score: f32,
    pub title: String,
    pub body: String,
    pub metrics: serde_json::Value,
    pub evidence: Vec<cf_analysis::EvidenceRef>,
}

#[derive(serde::Serialize)]
pub struct MatchReport {
    pub match_id: i64,
    pub map: String,
    pub score_a: u32,
    pub score_b: u32,
    pub tracked: Option<String>,
    pub tracked_result: Option<String>,
    pub summary: Option<NarrationDto>,
    pub insights: Vec<NarratedInsight>,
    pub death_classes: Vec<cf_store::store::DeathClassDbRow>,
    pub class_13_share_pct: f32,
    pub per_round: Vec<cf_store::store::RoundStat>,
    pub classes_not_built: Vec<u8>,
}

#[derive(Clone, serde::Serialize)]
pub struct HabitEvidence {
    pub match_id: i64,
    pub map: String,
    pub evidence: cf_analysis::EvidenceRef,
}

#[derive(Clone, serde::Serialize)]
pub struct HabitReport {
    pub rule_id: String,
    pub title: String,
    pub body: String,
    pub matches_hit: usize,
    pub window: usize,
    pub total: u32,
    pub score: f32,
    pub evidence: Vec<HabitEvidence>,
}

/// Rebuild a cf_analysis::Insight from its stored row (JSON strings → values).
pub(crate) fn insight_from_row(row: &cf_store::store::InsightRow) -> Option<cf_analysis::Insight> {
    use cf_analysis::Category;
    let category = match row.category.as_str() {
        "deaths" => Category::Deaths,
        "utility" => Category::Utility,
        "positioning" => Category::Positioning,
        "timing" => Category::Timing,
        _ => return None,
    };
    Some(cf_analysis::Insight {
        detector: row.detector.clone(),
        category,
        severity: row.severity,
        confidence: row.confidence,
        round: row.round,
        player: row.player.parse().ok()?,
        title_data: serde_json::from_str(&row.title_data_json).ok()?,
        metrics: serde_json::from_str(&row.metrics_json).ok()?,
        evidence: serde_json::from_str(&row.evidence_json).unwrap_or_default(),
    })
}

/// Bundle of everything `get_match_report` and `get_round_review` both need
/// out of a single pass over the store: the narrator's per-match context
/// (display names, score, tracked result, class-13 share) plus the raw
/// fields `get_match_report` also serializes directly onto `MatchReport`.
/// Computing these once here — instead of once inline in `get_match_report`
/// and again inside this helper — is the point: two call sites, one set of
/// store reads.
pub(crate) struct MatchCtxBundle {
    pub(crate) ctx: cf_narrator::MatchContext,
    /// The same `MatchDetail` this function already read to build `ctx` —
    /// handed back so `get_match_report` doesn't issue its own second
    /// `match_detail` read for the raw fields (`map`/`score_a`/`score_b`)
    /// it serializes directly onto `MatchReport` (V1.2 final-review fix
    /// wave, minor #6).
    pub(crate) detail: MatchDetail,
    death_classes: Vec<cf_store::store::DeathClassDbRow>,
    tracked: Option<String>,
    pub(crate) tracked_result: Option<String>,
    class_13_share_pct: f32,
}

/// Builds the shared match-context bundle. `None` when the match doesn't
/// exist.
pub(crate) fn match_context(
    store: &Store,
    match_id: i64,
) -> Result<Option<MatchCtxBundle>, String> {
    let Some(detail) = store.match_detail(match_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let tracked = store.tracked_steamid().map_err(|e| e.to_string())?;
    let tracked_u64 = tracked.as_ref().and_then(|t| t.parse::<u64>().ok());

    let death_classes = store
        .death_classes_for_match(match_id)
        .map_err(|e| e.to_string())?;
    let class_13 = death_classes.iter().filter(|d| d.class_id == 13).count();
    let class_13_share_pct = if death_classes.is_empty() {
        0.0
    } else {
        (class_13 as f32 / death_classes.len() as f32 * 1000.0).round() / 10.0
    };

    let tracked_result = store
        .list_matches()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| m.id == match_id)
        .and_then(|m| m.tracked_result);

    let ctx = cf_narrator::MatchContext {
        map: detail.map.clone(),
        tracked: tracked_u64.unwrap_or(0),
        names: detail
            .players
            .iter()
            .filter_map(|p| Some((p.steamid.parse::<u64>().ok()?, p.name.clone())))
            .collect(),
        score: (detail.score_a, detail.score_b),
        tracked_result: tracked_result.clone(),
        total_deaths: death_classes.len(),
        class_13_share_pct,
    };

    Ok(Some(MatchCtxBundle {
        ctx,
        detail,
        death_classes,
        tracked,
        tracked_result,
        class_13_share_pct,
    }))
}

#[tauri::command]
pub fn get_match_report(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Option<MatchReport>, String> {
    timed("get_match_report", || {
        use cf_narrator::{CoachingNarrator, TemplateNarrator};
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let Some(MatchCtxBundle {
            ctx,
            detail,
            death_classes,
            tracked,
            tracked_result,
            class_13_share_pct,
        }) = match_context(&store, match_id)?
        else {
            return Ok(None);
        };

        let narrator = TemplateNarrator;
        let rows = store
            .insights_for_match(match_id)
            .map_err(|e| e.to_string())?;
        let parsed: Vec<cf_analysis::Insight> = rows.iter().filter_map(insight_from_row).collect();
        let mut insights: Vec<NarratedInsight> = parsed
            .iter()
            .map(|i| {
                let n = narrator.narrate(i, &ctx);
                let count = i.metrics.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as f32;
                NarratedInsight {
                    detector: i.detector.clone(),
                    category: match i.category {
                        cf_analysis::Category::Deaths => "deaths",
                        cf_analysis::Category::Utility => "utility",
                        cf_analysis::Category::Positioning => "positioning",
                        cf_analysis::Category::Timing => "timing",
                    }
                    .to_string(),
                    severity: i.severity,
                    confidence: i.confidence,
                    round: i.round,
                    score: i.severity * i.confidence * (1.0 + count).ln(),
                    title: n.title,
                    body: n.body,
                    metrics: i.metrics.clone(),
                    evidence: i.evidence.clone(),
                }
            })
            .collect();
        insights.sort_by(|a, b| b.score.total_cmp(&a.score));
        let summary = narrator.summarize(&parsed, &ctx).map(|n| NarrationDto {
            title: n.title,
            body: n.body,
        });

        let per_round = tracked
            .as_ref()
            .map(|t| store.per_round_stats(match_id, t))
            .transpose()
            .map_err(|e| e.to_string())?
            .unwrap_or_default();

        Ok(Some(MatchReport {
            match_id,
            map: detail.map,
            score_a: detail.score_a,
            score_b: detail.score_b,
            tracked,
            tracked_result,
            summary,
            insights,
            death_classes,
            class_13_share_pct,
            per_round,
            classes_not_built: unbuilt_class_ids(),
        }))
    })
}

/// Taxonomy classes with no rule behind them yet, straight from the
/// catalog (spec §1's honesty rule: if a class isn't built, the Report says
/// so). Derived, never a literal — shipping a class removes it from the
/// Report by itself.
fn unbuilt_class_ids() -> Vec<u8> {
    cf_analysis::catalog::classes()
        .iter()
        .filter(|c| !c.built)
        .map(|c| c.id)
        .collect()
}

#[tauri::command]
pub fn get_habits(state: State<'_, AppState>) -> Result<Vec<HabitReport>, String> {
    timed("get_habits", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        habit_reports(&store)
    })
}

/// The body of `get_habits`, callable under a lock the caller already holds
/// — the V1.3 coach synthesis feeds these to the model.
pub(crate) fn habit_reports(store: &Store) -> Result<Vec<HabitReport>, String> {
    use cf_analysis::habits::{death_hotspots, promote_habits, DeathPoint, HabitInput};
    let Some(tracked) = store.tracked_steamid().map_err(|e| e.to_string())? else {
        return Ok(vec![]);
    };
    let cfg = detector_config();
    let tracked_u64 = tracked.parse::<u64>().ok();

    // Rule-recurrence habits.
    let mut inputs = vec![];
    let mut evidence_by_rule: std::collections::HashMap<String, Vec<HabitEvidence>> =
        std::collections::HashMap::new();
    for rule_id in store
        .flagged_rule_ids(&tracked)
        .map_err(|e| e.to_string())?
    {
        let counts = store
            .rule_counts_across_matches(&tracked, &rule_id, cfg.habit.window_matches)
            .map_err(|e| e.to_string())?;
        let Some((severity, confidence)) = store
            .rule_severity_confidence(&tracked, &rule_id)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let mut ev = vec![];
        for c in counts.iter().filter(|c| c.count > 0) {
            // Flags stored before migration 3 have no evidence_json. Rebuild a
            // window around the flag's own tick (±5 s / 2 s at 64 tick, same
            // as the hotspot chips below) rather than dropping the chip.
            let evidence = c
                .first_evidence_json
                .as_ref()
                .and_then(|j| serde_json::from_str::<cf_analysis::EvidenceRef>(j).ok())
                .or_else(|| {
                    let (round, tick) = c.first_round.zip(c.first_tick)?;
                    Some(cf_analysis::EvidenceRef {
                        round,
                        tick_start: tick - 320,
                        tick_end: tick + 128,
                        focus_players: tracked_u64.into_iter().collect(),
                        camera_hint: None,
                    })
                });
            if let Some(e) = evidence {
                ev.push(HabitEvidence {
                    match_id: c.match_id,
                    map: c.map.clone(),
                    evidence: e,
                });
            }
        }
        evidence_by_rule.insert(rule_id.clone(), ev);
        inputs.push(HabitInput {
            rule_id,
            severity,
            confidence,
            per_match: counts.iter().map(|c| (c.match_id, c.count)).collect(),
        });
    }
    let mut out: Vec<HabitReport> = promote_habits(&inputs, &cfg.habit)
        .into_iter()
        .map(|h| {
            let places = store
                .rule_place_counts(&tracked, &h.rule_id, cfg.habit.window_matches)
                .unwrap_or_default();
            let extra = serde_json::json!({ "places": places });
            let n =
                cf_narrator::narrate_habit(&h.rule_id, h.matches_hit, h.window, h.total, &extra);
            HabitReport {
                evidence: evidence_by_rule
                    .get(&h.rule_id)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .take(6)
                    .collect(),
                rule_id: h.rule_id,
                title: n.title,
                body: n.body,
                matches_hit: h.matches_hit,
                window: h.window,
                total: h.total,
                score: h.score,
            }
        })
        .collect();

    // Cross-demo death hotspots (spec H4_REPEAT_HOTSPOT).
    let points: Vec<DeathPoint> = store
        .death_positions(&tracked, cfg.habit.death_pos_lookback_s)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| DeathPoint {
            match_id: p.match_id,
            map: p.map,
            round: p.round,
            tick: p.tick,
            x: p.x,
            y: p.y,
            place: p.place,
        })
        .collect();
    // One card per (map, place): callout-titled cards make A site and B site
    // two genuine findings (issue #6 §2 follow-on); keep the deadliest
    // cluster per place only.
    let mut seen_places = std::collections::HashSet::new();
    for hs in death_hotspots(&points, &cfg.habit) {
        if !seen_places.insert((hs.map.clone(), hs.place.clone())) {
            continue;
        }
        let n = cf_narrator::narrate_habit(
            "H4_REPEAT_HOTSPOT",
            hs.matches,
            cfg.habit.window_matches,
            hs.deaths as u32,
            &serde_json::json!({
                "map": hs.map, "place": hs.place,
                "deaths": hs.deaths, "matches": hs.matches
            }),
        );
        out.push(HabitReport {
            rule_id: "H4_REPEAT_HOTSPOT".to_string(),
            title: n.title,
            body: n.body,
            matches_hit: hs.matches,
            window: cfg.habit.window_matches,
            total: hs.deaths as u32,
            score: 0.8 * (hs.deaths as f32).ln().max(0.5),
            evidence: hs
                .members
                .iter()
                .take(6)
                .map(|(match_id, round, tick)| HabitEvidence {
                    match_id: *match_id,
                    map: hs.map.clone(),
                    evidence: cf_analysis::EvidenceRef {
                        round: *round,
                        tick_start: tick - 320,
                        tick_end: tick + 128,
                        focus_players: tracked_u64.into_iter().collect(),
                        camera_hint: None,
                    },
                })
                .collect(),
        });
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(out)
}

// ---- M6: trends ----

#[derive(serde::Serialize)]
pub struct RuleSeries {
    pub rule_id: String,
    pub title: String,
    pub counts: Vec<u32>,
    pub total: u32,
}

#[derive(serde::Serialize)]
pub struct TrendsDto {
    pub matches: Vec<cf_store::store::TrendMatchRow>,
    pub rules: Vec<RuleSeries>,
    pub stats: Vec<StatSeries>,
}

/// Chronological deaths/class-13 series for the tracked player's own matches,
/// plus per-rule flag-count series aligned to the same match order. Single
/// events (total < 2) are dropped as noise (§7); the rest are capped to the
/// 8 largest totals.
#[tauri::command]
pub fn get_trends(state: State<'_, AppState>) -> Result<TrendsDto, String> {
    timed("get_trends", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let Some(tracked) = store.tracked_steamid().map_err(|e| e.to_string())? else {
            return Ok(TrendsDto {
                matches: vec![],
                rules: vec![],
                stats: vec![],
            });
        };

        let matches = store.trend_matches(&tracked).map_err(|e| e.to_string())?;
        let window = matches.len();
        let match_index: HashMap<i64, usize> = matches
            .iter()
            .enumerate()
            .map(|(i, m)| (m.match_id, i))
            .collect();

        let mut by_rule: HashMap<String, Vec<u32>> = HashMap::new();
        for cell in store
            .rule_trend_counts(&tracked)
            .map_err(|e| e.to_string())?
        {
            let Some(&idx) = match_index.get(&cell.match_id) else {
                continue;
            };
            by_rule
                .entry(cell.rule_id)
                .or_insert_with(|| vec![0u32; window])[idx] = cell.count;
        }

        let mut rules: Vec<RuleSeries> = by_rule
            .into_iter()
            .map(|(rule_id, counts)| {
                let matches_hit = counts.iter().filter(|&&c| c > 0).count();
                let total: u32 = counts.iter().sum();
                let n = cf_narrator::narrate_habit(
                    &rule_id,
                    matches_hit,
                    window,
                    total,
                    &serde_json::json!({}),
                );
                RuleSeries {
                    rule_id,
                    title: n.title,
                    counts,
                    total,
                }
            })
            .filter(|r| r.total >= 2)
            .collect();
        rules.sort_by(|a, b| {
            b.total
                .cmp(&a.total)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        rules.truncate(8);

        let ids: Vec<i64> = matches.iter().map(|m| m.match_id).collect();
        let by_id: HashMap<i64, cf_store::store::MatchStatsRow> = store
            .match_stats_for_matches(&ids)
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let rows: Vec<Option<cf_store::store::MatchStatsRow>> =
            ids.iter().map(|id| by_id.get(id).copied()).collect();
        let stats = stat_series(&rows);

        Ok(TrendsDto {
            matches,
            rules,
            stats,
        })
    })
}

#[tauri::command]
pub fn get_round_ticks(
    state: State<'_, AppState>,
    match_id: i64,
    round: u32,
) -> Result<RoundTicks, String> {
    timed("get_round_ticks", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .round_ticks(match_id, round)
            .map_err(|e| e.to_string())
    })
}

// ---- M5: reference corpus + D6 positioning ----

const D6_DETECTOR: &str = "D6_UNUSUAL_POSITIONING";

fn side_from_str(s: &str) -> Option<Side> {
    match s {
        "CT" => Some(Side::Ct),
        "T" => Some(Side::T),
        _ => None,
    }
}

fn phase_from_str(p: &str) -> Option<Phase> {
    match p {
        "freeze_end" => Some(Phase::FreezeEnd),
        "early" => Some(Phase::Early),
        "mid" => Some(Phase::Mid),
        "post_plant" => Some(Phase::PostPlant),
        _ => None,
    }
}

fn occupancy_to_row(g: &corpus::OccupancyGrid) -> GridRow {
    GridRow {
        map: g.map.clone(),
        side: match g.side {
            Side::Ct => "CT".to_string(),
            Side::T => "T".to_string(),
        },
        phase: g.phase.as_str().to_string(),
        size: g.size,
        counts: g.counts.clone(),
        demos: g.demos as u32,
        samples: g.samples,
    }
}

fn row_to_occupancy(r: GridRow) -> Option<corpus::OccupancyGrid> {
    Some(corpus::OccupancyGrid {
        side: side_from_str(&r.side)?,
        phase: phase_from_str(&r.phase)?,
        map: r.map,
        size: r.size,
        counts: r.counts,
        demos: r.demos as usize,
        samples: r.samples,
    })
}

#[derive(serde::Serialize)]
pub struct CorpusStatus {
    pub maps: Vec<cf_store::store::CorpusMapCount>,
    pub grids: Vec<cf_store::store::GridStatus>,
    /// The honest D6 gate (§5): grids from fewer corpus demos stay silent.
    pub min_demos_per_map: usize,
}

/// Import a pro demo as reference-corpus data: parsed and saved with
/// kind='corpus' (invisible to the library, reports and habits), never
/// analyzed — corpus players aren't coached.
#[tauri::command]
pub async fn import_corpus_demo(
    state: State<'_, AppState>,
    path: String,
    on_progress: Channel<ProgressEvent>,
) -> Result<ImportResult, String> {
    let (match_id, data, score_a, score_b) = parse_and_save(
        &state,
        path,
        &on_progress,
        cf_store::store::MatchKind::Corpus,
    )
    .await?;
    {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        if let Err(e) = refresh_map_callouts(&mut store, &data.map) {
            eprintln!("map callouts for {}: {e}", data.map);
        }
    }
    send(&on_progress, "done", 1.0, "Corpus import complete");
    Ok(ImportResult {
        match_id,
        map: data.map,
        score_a,
        score_b,
    })
}

/// (Re)build occupancy grids from every corpus demo — all maps, or just one.
/// Returns how many grids were built. Synchronous store scan: corpus sizes
/// are tens of demos, not thousands.
#[tauri::command]
pub async fn build_corpus(
    state: State<'_, AppState>,
    map: Option<String>,
    on_progress: Channel<ProgressEvent>,
) -> Result<usize, String> {
    let cfg = detector_config().corpus;
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let maps: Vec<String> = match map {
        Some(m) => vec![m],
        None => store
            .corpus_summary()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|c| c.map)
            .collect(),
    };
    let mut total_grids = 0usize;
    for (mi, map) in maps.iter().enumerate() {
        let ids = store.corpus_match_ids(map).map_err(|e| e.to_string())?;
        let demos_per_map: HashMap<String, usize> = HashMap::from([(map.clone(), ids.len())]);
        let mut samples: Vec<PhaseSample> = Vec::new();
        for (di, id) in ids.iter().enumerate() {
            send(
                &on_progress,
                "building",
                (mi as f32 + di as f32 / ids.len().max(1) as f32) / maps.len().max(1) as f32,
                &format!("{map}: demo {}/{}", di + 1, ids.len()),
            );
            let Some((_, tickrate)) = store.match_map_tickrate(*id).map_err(|e| e.to_string())?
            else {
                continue;
            };
            for r in store.rounds_for_match(*id).map_err(|e| e.to_string())? {
                let Some(freeze_end) = r.freeze_end_tick else {
                    continue;
                };
                let plant = store
                    .bomb_plant_tick(*id, r.number)
                    .map_err(|e| e.to_string())?;
                let sides: HashMap<String, String> = store
                    .sides_for_round(*id, r.number)
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .collect();
                for (phase, tick) in
                    corpus::phase_moments(freeze_end, r.end_tick, plant, tickrate as f32, &cfg)
                {
                    for p in store
                        .positions_at(*id, tick, r.start_tick)
                        .map_err(|e| e.to_string())?
                    {
                        if !p.alive {
                            continue;
                        }
                        let Some(side) = sides.get(&p.steamid).and_then(|s| side_from_str(s))
                        else {
                            continue;
                        };
                        samples.push(PhaseSample {
                            map: map.clone(),
                            side,
                            phase,
                            x: p.x,
                            y: p.y,
                        });
                    }
                }
            }
        }
        let grids = corpus::build_grids(&samples, &demos_per_map, &cfg);
        let rows: Vec<GridRow> = grids.iter().map(occupancy_to_row).collect();
        store.save_grids(&rows).map_err(|e| e.to_string())?;
        total_grids += rows.len();
        // Fresh grids make existing D6 results stale: re-run positioning
        // for the player's own matches on this map.
        for own_id in store.own_match_ids(map).map_err(|e| e.to_string())? {
            run_positioning(&mut store, own_id)?;
        }
    }
    send(&on_progress, "done", 1.0, "Corpus build complete");
    Ok(total_grids)
}

#[tauri::command]
pub fn corpus_status(state: State<'_, AppState>) -> Result<CorpusStatus, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    Ok(CorpusStatus {
        maps: store.corpus_summary().map_err(|e| e.to_string())?,
        grids: store.grid_status().map_err(|e| e.to_string())?,
        min_demos_per_map: detector_config().corpus.min_demos_per_map,
    })
}

#[tauri::command]
pub fn get_grid(
    state: State<'_, AppState>,
    map: String,
    side: String,
    phase: String,
) -> Result<Option<GridRow>, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    Ok(store
        .load_grids(&map)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|g| g.side == side && g.phase == phase))
}

/// D6 for one own match: sample the tracked player's phase moments, compare
/// against this map's grids, and replace the match's D6 insights. Returns
/// the number of insights written (0 = corpus silent or nothing unusual).
fn run_positioning(store: &mut Store, match_id: i64) -> Result<usize, String> {
    let cfg = detector_config().corpus;
    let Some((map, tickrate)) = store
        .match_map_tickrate(match_id)
        .map_err(|e| e.to_string())?
    else {
        return Err("unknown match".to_string());
    };
    let tracked = store
        .tracked_steamid()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no tracked player".to_string())?;
    let grids: Vec<corpus::OccupancyGrid> = store
        .load_grids(&map)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(row_to_occupancy)
        .collect();

    let rounds = store
        .rounds_for_match(match_id)
        .map_err(|e| e.to_string())?;
    let total_rounds = rounds.len() as u32;
    let mut moments: Vec<TrackedMoment> = Vec::new();
    for r in &rounds {
        let Some(freeze_end) = r.freeze_end_tick else {
            continue;
        };
        let plant = store
            .bomb_plant_tick(match_id, r.number)
            .map_err(|e| e.to_string())?;
        let side = store
            .sides_for_round(match_id, r.number)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|(sid, _)| *sid == tracked)
            .and_then(|(_, s)| side_from_str(&s));
        let Some(side) = side else {
            continue;
        };
        for (phase, tick) in
            corpus::phase_moments(freeze_end, r.end_tick, plant, tickrate as f32, &cfg)
        {
            let pos = store
                .positions_at(match_id, tick, r.start_tick)
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.steamid == tracked);
            if let Some(p) = pos.filter(|p| p.alive) {
                moments.push(TrackedMoment {
                    round: r.number,
                    tick,
                    side,
                    phase,
                    x: p.x,
                    y: p.y,
                });
            }
        }
    }

    let findings = corpus::unusual_positions(&moments, &grids, &map, &cfg);
    let tracked_u64 = tracked
        .parse::<u64>()
        .map_err(|_| "tracked steamid is not a number".to_string())?;
    let insights = corpus::d6_insights(
        &findings,
        &map,
        tracked_u64,
        total_rounds,
        tickrate as f32,
        &cfg,
    );
    store
        .replace_detector_insights(match_id, D6_DETECTOR, &insights)
        .map_err(|e| e.to_string())?;
    Ok(insights.len())
}

#[tauri::command]
pub fn analyze_positioning(state: State<'_, AppState>, match_id: i64) -> Result<usize, String> {
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    run_positioning(&mut store, match_id)
}

// ---- V1.2: round-by-round review (issue #9; ADR-0008) ----

/// Assembles `cf_analysis::round_review`'s narrow input from stored rows.
/// `None` when there is no tracked player — nothing to score from their
/// perspective, so the caller stays silent rather than scoring round 0.
fn build_review_input(
    store: &Store,
    match_id: i64,
) -> Result<Option<cf_analysis::round_review::RoundReviewInput>, String> {
    use cf_analysis::round_review::{ReviewBomb, ReviewFlag, ReviewKill, ReviewRound};

    let Some(tracked) = store
        .tracked_steamid()
        .map_err(|e| e.to_string())?
        .and_then(|t| t.parse::<u64>().ok())
    else {
        return Ok(None);
    };
    let Some(detail) = store.match_detail(match_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };

    let mut rounds = Vec::with_capacity(detail.rounds.len());
    for r in &detail.rounds {
        // Defensive: a malformed winner should never happen (`side_from_str`
        // only ever writes "CT"/"T") — skip rather than guess.
        let Some(winner) = side_from_str(&r.winner) else {
            continue;
        };
        let (mut ct, mut t) = (vec![], vec![]);
        for (steamid, side) in store
            .sides_for_round(match_id, r.number)
            .map_err(|e| e.to_string())?
        {
            let Some(sid) = steamid.parse::<u64>().ok() else {
                continue; // unparseable steamid: silence, drop the roster slot
            };
            match side_from_str(&side) {
                Some(Side::Ct) => ct.push(sid),
                Some(Side::T) => t.push(sid),
                None => {}
            }
        }
        rounds.push(ReviewRound {
            number: r.number,
            start_tick: r.start_tick,
            freeze_end_tick: r.freeze_end_tick,
            end_tick: r.end_tick,
            officially_ended_tick: r.officially_ended_tick,
            winner,
            ct,
            t,
        });
    }

    let kills = detail
        .kills
        .iter()
        .filter_map(|k| {
            Some(ReviewKill {
                round: k.round,
                tick: k.tick,
                attacker: k.attacker.as_ref().and_then(|a| a.parse::<u64>().ok()),
                victim: k.victim.parse::<u64>().ok()?,
                weapon: k.weapon.clone(),
            })
        })
        .collect();

    let bomb_events = detail
        .bomb_events
        .iter()
        .map(|b| ReviewBomb {
            tick: b.tick,
            kind: b.kind.clone(),
            player: b.player.as_ref().and_then(|p| p.parse::<u64>().ok()),
        })
        .collect();

    let flags = store
        .flags_for_match(match_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|f| {
            Some(ReviewFlag {
                rule_id: f.rule_id,
                round: f.round,
                tick: f.tick,
                steamid: f.steamid.parse::<u64>().ok()?,
                severity: f.severity,
                confidence: f.confidence,
                details: serde_json::from_str(&f.details_json).unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();

    Ok(Some(cf_analysis::round_review::RoundReviewInput {
        tracked,
        tickrate: detail.tickrate,
        rounds,
        kills,
        bomb_events,
        flags,
    }))
}

/// Scores every round of a match and (re)persists the result — the same
/// DELETE+INSERT-in-one-tx replace `save_round_reviews` already does. A
/// no-op (not an error) when there's no tracked player yet.
fn run_round_review(store: &mut Store, match_id: i64) -> Result<(), String> {
    let Some(input) = build_review_input(store, match_id)? else {
        return Ok(());
    };
    let cfg = detector_config();
    let reviews = cf_analysis::round_review::review_rounds(&input, &cfg);
    let fingerprint = cf_analysis::round_review::cfg_fingerprint(&cfg.rbr);
    let rows: Vec<cf_store::store::RoundReviewRow> = reviews
        .iter()
        .map(|r| -> Result<cf_store::store::RoundReviewRow, String> {
            Ok(cf_store::store::RoundReviewRow {
                round: r.round,
                impact: r.impact,
                verdict: r.verdict.as_str().to_string(),
                attention: r.attention.as_str().to_string(),
                selected: r.selected,
                pivotal_tick: r.pivotal_tick,
                header_json: serde_json::to_string(&r.header).map_err(|e| e.to_string())?,
                moments_json: serde_json::to_string(&r.moments).map_err(|e| e.to_string())?,
                cfg_fingerprint: fingerprint.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    store
        .save_round_reviews(match_id, &rows)
        .map_err(|e| e.to_string())
}

/// Focus steamids for the replay overlay (Task 9's canvas rail): victim,
/// killer, nearest teammate — in that priority order, but PRESENCE-ordered,
/// not fixed-slot: a key the moment's facts don't carry is skipped rather
/// than left as a hole, so the list compacts (e.g. `[victim]` alone when the
/// killer is unknown, or `[victim, nearest_teammate]` when the killer is
/// unknown but the nearest teammate is known). Never index into this array
/// assuming a fixed position — `RailMomentDto.killer` exists for exactly
/// that reason (V1.2 final-review fix wave, finding #3): read it directly
/// instead of assuming `focus[1]` is the killer. Raw steamid strings, built
/// straight from the moment's own facts; name resolution is the frontend's
/// job.
fn moment_focus(m: &cf_analysis::round_review::Moment) -> Vec<String> {
    ["victim", "killer", "nearest_teammate"]
        .iter()
        .filter_map(|k| m.facts.get(*k).and_then(|v| v.as_str()).map(str::to_string))
        .collect()
}

/// The moment's killer steamid, when its facts carry one — explicit,
/// non-positional accessor for `RailMomentDto.killer` (V1.2 final-review fix
/// wave, finding #3): `moment_focus`'s list compacts when a key is absent,
/// so a positional read (`focus[1]`) can silently pick up a DIFFERENT
/// person (e.g. `nearest_teammate`) when there's no killer. This is the only
/// correct way to get the killer id off a moment.
fn moment_killer(m: &cf_analysis::round_review::Moment) -> Option<String> {
    m.facts
        .get("killer")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// A ledger play and a review moment describe the same event when their
/// kinds correspond (the ledger says "kill", the review "tracked_kill").
fn moment_kind_matches(moment_kind: &str, play_kind: &str) -> bool {
    matches!(
        (moment_kind, play_kind),
        ("tracked_kill", "kill")
            | ("tracked_death", "death")
            | ("plant", "plant")
            | ("defuse", "defuse")
    )
}

/// One ledger play, narrated into its DTO — `None` for a fact-less `flag`
/// play (`should_suppress_flag_moment`: the same silence the moments path
/// applies, so the ledger never renders a bare rule label either; V1.2b
/// final-review fix wave, #6). `delta_p` comes from the ADR-0008 engine
/// only: joined here by tick to this round's review moments
/// (kill/death/plant/defuse) via `moment_kind_matches` — never computed by
/// this function.
fn play_dto(
    mut p: cf_analysis::play_ledger::Play,
    moments: &[cf_analysis::round_review::Moment],
    ctx: &cf_narrator::MatchContext,
) -> Option<PlayDto> {
    p.delta_p = moments
        .iter()
        .find(|m| m.tick == p.tick && moment_kind_matches(&m.kind, &p.kind))
        .and_then(|m| m.delta_p);
    let t = cf_narrator::plays::narrate_play(&p, ctx);
    if should_suppress_flag_moment(&p.kind, &t.facts) {
        return None;
    }
    let killer = p
        .facts
        .get("killer")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let focus = ["victim", "killer", "nearest_teammate", "teammate"]
        .iter()
        .filter_map(|k| p.facts.get(*k).and_then(|v| v.as_str()).map(str::to_string))
        .collect();
    Some(PlayDto {
        tick: p.tick,
        kind: p.kind.clone(),
        phase: p.phase.clone(),
        headline: t.headline,
        facts: t.facts,
        quality: p.quality.map(|q| match q {
            cf_analysis::play_ledger::Quality::Good => "good".to_string(),
            cf_analysis::play_ledger::Quality::Bad => "bad".to_string(),
            cf_analysis::play_ledger::Quality::Neutral => "neutral".to_string(),
        }),
        rule_id: p.rule_id.clone(),
        delta_p: p.delta_p,
        focus,
        killer,
    })
}

/// One ledger timeline event, with `actor`/`subject` steamids resolved to
/// display names (falling back to the raw string if it isn't numeric).
fn timeline_dto(
    e: cf_analysis::play_ledger::TimelineEvent,
    ctx: &cf_narrator::MatchContext,
) -> TimelineDto {
    let name = |id: &Option<String>| -> Option<String> {
        id.as_ref().map(|s| {
            s.parse::<u64>()
                .map(|i| ctx.name(i))
                .unwrap_or_else(|_| s.clone())
        })
    };
    TimelineDto {
        tick: e.tick,
        kind: e.kind,
        actor: name(&e.actor),
        subject: name(&e.subject),
        side: e.side,
        weapon: e.weapon,
    }
}

/// Silence over a bare label (V1.2 final-review fix wave, finding #4): a
/// standalone `flag` moment whose narrated facts came out empty carries no
/// evidence for the rail to show — the CLAUDE.md evidence contract's
/// "no evidence -> doesn't ship" rule, applied to a moment instead of a
/// whole `Insight`. Non-`flag` kinds always ship even with empty facts
/// (e.g. a plant/defuse with an unobserved `delta_p` still marks the event).
fn should_suppress_flag_moment(kind: &str, facts: &[String]) -> bool {
    kind == "flag" && facts.is_empty()
}

#[derive(Clone, serde::Serialize)]
pub struct RailMomentDto {
    pub tick: i32,
    pub headline: String,
    pub facts: Vec<String>,
    pub rule_id: Option<String>,
    pub delta_p: Option<f32>,
    pub kind: String,
    pub focus: Vec<String>,
    /// The moment's killer steamid, when known — explicit (V1.2
    /// final-review fix wave, finding #3) so the frontend never has to
    /// infer it from `focus`'s position, which shifts when other keys are
    /// absent (see `moment_focus`'s doc comment).
    pub killer: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct PlayDto {
    pub tick: i32,
    pub kind: String,
    pub phase: String,
    pub headline: String,
    pub facts: Vec<String>,
    /// "good" | "bad" | "neutral" | null (spec §2: only when measured)
    pub quality: Option<String>,
    pub rule_id: Option<String>,
    pub delta_p: Option<f32>,
    pub focus: Vec<String>,
    pub killer: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct TimelineDto {
    pub tick: i32,
    pub kind: String,
    pub actor: Option<String>,   // display name
    pub subject: Option<String>, // display name
    pub side: Option<String>,
    pub weapon: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct RoundReviewDto {
    pub round: u32,
    pub impact: f32,
    pub verdict: String,       // snake_case
    pub verdict_label: String, // "Cost you"
    pub attention: String,     // "none"|"dim"|"bright"
    pub selected: bool,
    pub pivotal_tick: i32,
    pub side: String,
    pub won: bool,
    pub kills: u32,
    pub deaths: u32,
    pub man_context: Option<String>,
    pub moments: Vec<RailMomentDto>,
    pub plays: Vec<PlayDto>,
    pub timeline: Vec<TimelineDto>,
    pub why_it_mattered: Option<String>,
    pub what_to_practise: Option<String>,
}

/// Every round's review, narrated for the coach rail. Computes and persists
/// on first call for a match imported before V1.2 (lazy backfill) — imports
/// from here on already have rows via the `import_demo` hook.
#[tauri::command]
pub fn get_round_review(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Vec<RoundReviewDto>, String> {
    timed("get_round_review", || {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        assemble_round_reviews(&mut store, match_id)
    })
}

/// The body of `get_round_review`, callable under a lock the caller already
/// holds — the V1.3 coach orchestrator reads the same reviews.
pub(crate) fn assemble_round_reviews(
    store: &mut Store,
    match_id: i64,
) -> Result<Vec<RoundReviewDto>, String> {
    use cf_analysis::round_review::{Attention, Moment, RoundHeader, RoundReview, Verdict};
    use cf_narrator::rail;

    let mut rows = store
        .load_round_reviews(match_id)
        .map_err(|e| e.to_string())?;
    // Empty (never computed) or stale (computed under an old engine
    // version / a since-changed RbrCfg threshold) both force a recompute —
    // a stored review must never be served once its fingerprint no longer
    // matches what the current config would produce (V1.2 final-review fix
    // wave, finding #5).
    let current_fingerprint = cf_analysis::round_review::cfg_fingerprint(&detector_config().rbr);
    let stale = rows
        .first()
        .is_some_and(|r| r.cfg_fingerprint != current_fingerprint);
    if rows.is_empty() || stale {
        run_round_review(store, match_id)?;
        rows = store
            .load_round_reviews(match_id)
            .map_err(|e| e.to_string())?;
    }

    let Some(MatchCtxBundle { ctx, .. }) = match_context(store, match_id)? else {
        return Ok(vec![]);
    };

    let ledger_by_round: std::collections::HashMap<u32, cf_store::store::RoundPlaysRow> = store
        .load_round_plays(match_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| (r.round, r))
        .collect();

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        // A single corrupt row (bad JSON, or an unrecognized verdict/
        // attention string) must not blank the whole rail — skip just that
        // round rather than erroring the entire response.
        let Ok(header) = serde_json::from_str::<RoundHeader>(&row.header_json) else {
            continue;
        };
        let Ok(moments) = serde_json::from_str::<Vec<Moment>>(&row.moments_json) else {
            continue;
        };
        let Ok(verdict) = row.verdict.parse::<Verdict>() else {
            continue;
        };
        let Ok(attention) = row.attention.parse::<Attention>() else {
            continue;
        };

        let review = RoundReview {
            round: row.round,
            impact: row.impact,
            verdict,
            attention,
            selected: row.selected,
            pivotal_tick: row.pivotal_tick,
            header: header.clone(),
            moments: moments.clone(),
        };

        let rail_moments: Vec<RailMomentDto> = moments
            .iter()
            .filter_map(|m| {
                let text = rail::narrate_moment(m, &ctx);
                if should_suppress_flag_moment(&m.kind, &text.facts) {
                    return None;
                }
                Some(RailMomentDto {
                    tick: m.tick,
                    headline: text.headline,
                    facts: text.facts,
                    rule_id: text.rule_id,
                    delta_p: m.delta_p,
                    kind: m.kind.clone(),
                    focus: moment_focus(m),
                    killer: moment_killer(m),
                })
            })
            .collect();

        let plays: Vec<PlayDto> = ledger_by_round
            .get(&row.round)
            .and_then(|r| {
                serde_json::from_str::<Vec<cf_analysis::play_ledger::Play>>(&r.plays_json).ok()
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| play_dto(p, &moments, &ctx))
            .collect();
        let timeline: Vec<TimelineDto> = ledger_by_round
            .get(&row.round)
            .and_then(|r| {
                serde_json::from_str::<Vec<cf_analysis::play_ledger::TimelineEvent>>(
                    &r.timeline_json,
                )
                .ok()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|e| timeline_dto(e, &ctx))
            .collect();

        // Narration prose only for selected rounds: moments now exist for
        // every round (rbr-v2), so this gate is the only thing keeping an
        // unselected round's rail free of why/practise claims.
        let (why_it_mattered, what_to_practise) = if row.selected {
            (
                rail::why_it_mattered(&review, &ctx),
                rail::what_to_practise(&review, &ctx),
            )
        } else {
            (None, None)
        };

        out.push(RoundReviewDto {
            round: row.round,
            impact: row.impact,
            verdict: row.verdict,
            verdict_label: rail::verdict_label(verdict).to_string(),
            attention: row.attention,
            selected: row.selected,
            pivotal_tick: row.pivotal_tick,
            side: header.side,
            won: header.won,
            kills: header.kills,
            deaths: header.deaths,
            man_context: header.man_context,
            moments: rail_moments,
            plays,
            timeline,
            why_it_mattered,
            what_to_practise,
        });
    }
    Ok(out)
}

// ---- M6: settings + housekeeping ----

#[derive(serde::Serialize)]
pub struct ThresholdRow {
    pub name: String,
    pub value: String,
    pub unit: String,
}

#[derive(serde::Serialize)]
pub struct AppSettings {
    /// The settings-table override, if the player set one.
    pub tracked_override: Option<String>,
    /// What analysis actually uses right now (override, else the player
    /// seen in the most own matches).
    pub tracked_effective: Option<String>,
    pub tracked_name: Option<String>,
    pub db_path: String,
    pub own_matches: u32,
    pub corpus_demos: u32,
    pub thresholds: Vec<ThresholdRow>,
}

/// Every tunable threshold, straight from `cf_analysis::config`'s single
/// source of truth (the same rows `catalog::render_thresholds` resolves
/// `{trade.isolation_u}`-style placeholders against) — row names are now
/// dotted config paths, not prose, and count-type rows carry an empty unit;
/// the plain-language explanations live on the Watches screen (Task 9).
fn threshold_rows(cfg: &cf_analysis::DetectorConfig) -> Vec<ThresholdRow> {
    cf_analysis::config::threshold_values(cfg)
        .into_iter()
        .map(|(name, value, unit)| ThresholdRow { name, value, unit })
        .collect()
}

#[tauri::command]
pub fn get_app_settings(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<AppSettings, String> {
    use tauri::Manager;
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let tracked_override = store
        .get_setting("tracked_steamid")
        .map_err(|e| e.to_string())?;
    let tracked_effective = store.tracked_steamid().map_err(|e| e.to_string())?;
    let tracked_name = match &tracked_effective {
        Some(sid) => store.player_name(sid).map_err(|e| e.to_string())?,
        None => None,
    };
    let db_path = app
        .path()
        .app_data_dir()
        .map(|d| d.join("clutchfactor.db").to_string_lossy().into_owned())
        .unwrap_or_default();
    let own_matches = store.list_matches().map_err(|e| e.to_string())?.len() as u32;
    let corpus_demos: u32 = store
        .corpus_summary()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|c| c.demos)
        .sum();
    Ok(AppSettings {
        tracked_override,
        tracked_effective,
        tracked_name,
        db_path,
        own_matches,
        corpus_demos,
        thresholds: threshold_rows(&detector_config()),
    })
}

/// Sets or clears the tracked-player override. Applies to new imports —
/// existing matches keep their analysis until deleted and re-imported.
#[tauri::command]
pub fn set_tracked_override(
    state: State<'_, AppState>,
    steamid: Option<String>,
) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    match steamid {
        Some(s) => {
            let s = s.trim().to_string();
            if s.len() != 17 || !s.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!(
                    "\"{s}\" is not a SteamID64 — expected 17 digits, e.g. 76561199228328773."
                ));
            }
            store
                .set_setting("tracked_steamid", &s)
                .map_err(|e| e.to_string())
        }
        None => store
            .delete_setting("tracked_steamid")
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn delete_match(state: State<'_, AppState>, match_id: i64) -> Result<(), String> {
    {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store.delete_match(match_id).map_err(|e| e.to_string())?;
    }
    // Even if the match was already gone, a stale places entry for its id
    // must not survive (the id could be reused only by SQLite rowid reuse,
    // which we don't rely on — this is just always-safe hygiene).
    state.places.invalidate(match_id);
    Ok(())
}

// ---- V1.4: stats & understanding ----

#[derive(Clone, serde::Serialize)]
pub struct MatchStatsDto {
    pub rounds_played: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub kd: Option<f32>,
    pub adr: Option<f32>,
    pub hs_pct: Option<u32>,
    pub kast_pct: Option<u32>,
    pub entry_attempts: u32,
    pub entry_wins: u32,
    pub traded_deaths: u32,
    pub trade_kills: u32,
    pub trade_opportunities: u32,
    pub clutch_attempts: u32,
    pub clutch_wins: u32,
}

#[derive(Clone, serde::Serialize)]
pub struct PlayerRoundStatsDto {
    pub round: u32,
    pub steamid: String,
    pub name: String,
    pub side: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub damage: u32,
    pub headshots: u32,
    pub survived: bool,
    pub traded: bool,
    pub entry: Option<String>,
    pub tracked: bool,
}

#[derive(serde::Serialize)]
pub struct CatalogEntryDto {
    pub id: String,
    pub family: String,
    pub title: String,
    pub watches_for: String,
    /// Rendered against live `DetectorConfig` values — never a raw
    /// `{placeholder}` (catalog.rs's coverage test enforces this).
    pub thresholds: String,
    pub class_id: Option<u8>,
    pub example: String,
    pub stat_links: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct ClassEntryDto {
    pub id: u8,
    pub name: String,
    pub source: String,
    pub built: bool,
    pub why_not: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CatalogDto {
    pub entries: Vec<CatalogEntryDto>,
    pub classes: Vec<ClassEntryDto>,
    pub cannot_see: Vec<(String, String)>,
}

#[derive(Clone, serde::Serialize)]
pub struct CalloutDto {
    pub place: String,
    /// `cf_narrator::callouts::callout_name` — the raw `last_place` value
    /// turned into a human label ("BombsiteA" -> "Bombsite A").
    pub name: String,
    pub x: f32,
    pub y: f32,
    /// Median height — the replay picks the label's radar layer from it.
    pub z: f32,
    pub samples: u32,
}

#[derive(serde::Serialize)]
pub struct StatSeries {
    /// "kd" | "adr" | "hs" | "kast" | "entry" | "trade" | "clutch"
    pub key: String,
    pub title: String,
    pub unit: String,
    pub values: Vec<Option<f32>>,
}

/// The 14 stored counters -> `cf_analysis::stats::MatchStats`, the type
/// whose methods (`kd`/`adr`/`hs_pct`/`kast_pct`) actually compute the
/// ratios. Shared by `stats_dto` and `stat_series` so the field mapping
/// exists exactly once.
fn row_to_stats(r: &cf_store::store::MatchStatsRow) -> cf_analysis::stats::MatchStats {
    cf_analysis::stats::MatchStats {
        rounds_played: r.rounds_played,
        kills: r.kills,
        deaths: r.deaths,
        assists: r.assists,
        damage: r.damage,
        headshots: r.headshots,
        kast_rounds: r.kast_rounds,
        entry_attempts: r.entry_attempts,
        entry_wins: r.entry_wins,
        traded_deaths: r.traded_deaths,
        trade_kills: r.trade_kills,
        trade_opportunities: r.trade_opportunities,
        clutch_attempts: r.clutch_attempts,
        clutch_wins: r.clutch_wins,
    }
}

fn stats_dto(s: &cf_store::store::MatchStatsRow) -> MatchStatsDto {
    let ms = row_to_stats(s);
    MatchStatsDto {
        rounds_played: s.rounds_played,
        kills: s.kills,
        deaths: s.deaths,
        assists: s.assists,
        kd: ms.kd(),
        adr: ms.adr(),
        hs_pct: ms.hs_pct(),
        kast_pct: ms.kast_pct(),
        entry_attempts: s.entry_attempts,
        entry_wins: s.entry_wins,
        traded_deaths: s.traded_deaths,
        trade_kills: s.trade_kills,
        trade_opportunities: s.trade_opportunities,
        clutch_attempts: s.clutch_attempts,
        clutch_wins: s.clutch_wins,
    }
}

#[tauri::command]
pub fn get_match_stats(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Option<MatchStatsDto>, String> {
    timed("get_match_stats", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        Ok(store
            .load_match_stats(match_id)
            .map_err(|e| e.to_string())?
            .map(|s| stats_dto(&s)))
    })
}

#[tauri::command]
pub fn get_round_scoreboard(
    state: State<'_, AppState>,
    match_id: i64,
    round: Option<u32>,
) -> Result<Vec<PlayerRoundStatsDto>, String> {
    timed("get_round_scoreboard", || {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let tracked = store.tracked_steamid().map_err(|e| e.to_string())?;
        let names = store.player_names(match_id).map_err(|e| e.to_string())?;
        Ok(store
            .load_round_player_stats(match_id, round)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|r| PlayerRoundStatsDto {
                name: names
                    .get(&r.steamid)
                    .cloned()
                    .unwrap_or_else(|| r.steamid.clone()),
                tracked: tracked.as_deref() == Some(r.steamid.as_str()),
                round: r.round,
                steamid: r.steamid,
                side: r.side,
                kills: r.kills,
                deaths: r.deaths,
                assists: r.assists,
                damage: r.damage,
                headshots: r.headshots,
                survived: r.survived,
                traded: r.traded,
                entry: r.entry,
            })
            .collect())
    })
}

#[tauri::command]
pub fn get_detector_catalog() -> CatalogDto {
    let values = cf_analysis::config::threshold_values(&detector_config());
    CatalogDto {
        entries: cf_analysis::catalog::entries()
            .iter()
            .map(|e| CatalogEntryDto {
                id: e.id.into(),
                family: e.family.into(),
                title: e.title.into(),
                watches_for: e.watches_for.into(),
                thresholds: cf_analysis::catalog::render_thresholds(e.thresholds, &values),
                class_id: e.class_id,
                example: e.example.into(),
                stat_links: e.stat_links.iter().map(|s| s.to_string()).collect(),
            })
            .collect(),
        classes: cf_analysis::catalog::classes()
            .iter()
            .map(|c| ClassEntryDto {
                id: c.id,
                name: c.name.into(),
                source: c.source.into(),
                built: c.built,
                why_not: c.why_not.map(str::to_string),
            })
            .collect(),
        cannot_see: cf_analysis::catalog::CANNOT_SEE
            .iter()
            .map(|(t, s)| (t.to_string(), s.to_string()))
            .collect(),
    }
}

/// Per-place median of the raw positions; places with fewer than
/// `min_samples` rows are dropped (a label for a spot nobody stood in is
/// noise). Even-length medians take the lower middle (a real sample, so the
/// label sits on the map). z is taken the same way — the label's layer.
pub fn callout_medians(
    positions: &[(String, f32, f32, f32)],
    min_samples: u32,
) -> Vec<cf_store::store::MapCalloutRow> {
    /// One place's x, y and z samples.
    type PlaceSamples = (Vec<f32>, Vec<f32>, Vec<f32>);
    let mut by_place: HashMap<&str, PlaceSamples> = HashMap::new();
    for (p, x, y, z) in positions {
        let e = by_place.entry(p.as_str()).or_default();
        e.0.push(*x);
        e.1.push(*y);
        e.2.push(*z);
    }
    let mut out: Vec<cf_store::store::MapCalloutRow> = by_place
        .into_iter()
        .filter(|(_, (xs, _, _))| xs.len() as u32 >= min_samples)
        .map(|(place, (mut xs, mut ys, mut zs))| {
            let sort = |v: &mut Vec<f32>| {
                v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            };
            sort(&mut xs);
            sort(&mut ys);
            sort(&mut zs);
            let mid = (xs.len() - 1) / 2;
            cf_store::store::MapCalloutRow {
                place: place.to_string(),
                x: xs[mid],
                y: ys[mid],
                z: zs[mid],
                samples: xs.len() as u32,
            }
        })
        .collect();
    out.sort_by(|a, b| a.place.cmp(&b.place));
    out
}

pub const CALLOUT_MIN_SAMPLES: u32 = 30;

/// Recompute a map's callout labels from every match on it. Called after an
/// own import, a corpus import and a re-analyze (cheap: 1 Hz samples).
pub fn refresh_map_callouts(store: &mut Store, map: &str) -> Result<usize, String> {
    let positions = store.place_positions(map).map_err(|e| e.to_string())?;
    let rows = callout_medians(&positions, CALLOUT_MIN_SAMPLES);
    store
        .save_map_callouts(map, &rows)
        .map_err(|e| e.to_string())?;
    Ok(rows.len())
}

#[tauri::command]
pub fn get_map_callouts(
    state: State<'_, AppState>,
    map: String,
) -> Result<Vec<CalloutDto>, String> {
    timed("get_map_callouts", || {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        // Callouts are written after an import or a re-analyze, so a map whose
        // matches were all analyzed before V1.4 has none. Fill it in on first
        // ask (measured 0.05-0.09 s over a map's 1 Hz samples) rather than
        // showing a Callouts toggle that does nothing.
        let missing = store
            .load_map_callouts(&map)
            .map_err(|e| e.to_string())?
            .is_empty();
        if missing && store.match_count_for_map(&map).map_err(|e| e.to_string())? > 0 {
            if let Err(e) = refresh_map_callouts(&mut store, &map) {
                eprintln!("callout refresh for {map} failed: {e}");
                return Ok(vec![]);
            }
        }
        let mut out: Vec<CalloutDto> = store
            .load_map_callouts(&map)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|r| CalloutDto {
                name: cf_narrator::callouts::callout_name(&r.place),
                place: r.place,
                x: r.x,
                y: r.y,
                z: r.z,
                samples: r.samples,
            })
            .collect();
        // Task 10 needs the densest labels first for its priority layout.
        out.sort_by_key(|c| std::cmp::Reverse(c.samples));
        Ok(out)
    })
}

fn pct(n: u32, d: u32) -> Option<f32> {
    (d > 0).then(|| (n as f32 / d as f32 * 1000.0).round() / 10.0)
}

/// One series per spec §1 stat, aligned with the trend matches; `None`
/// where a match has no stats row (pre-V1.4 import) or the ratio is
/// undefined.
pub fn stat_series(rows: &[Option<cf_store::store::MatchStatsRow>]) -> Vec<StatSeries> {
    let series =
        |key: &str,
         title: &str,
         unit: &str,
         f: &dyn Fn(&cf_store::store::MatchStatsRow) -> Option<f32>| StatSeries {
            key: key.into(),
            title: title.into(),
            unit: unit.into(),
            values: rows.iter().map(|r| r.as_ref().and_then(f)).collect(),
        };
    vec![
        series("kd", "K/D", "", &|r| {
            row_to_stats(r).kd().map(|v| (v * 100.0).round() / 100.0)
        }),
        series("adr", "ADR", "dmg/round", &|r| row_to_stats(r).adr()),
        series("hs", "Headshot %", "%", &|r| {
            row_to_stats(r).hs_pct().map(|v| v as f32)
        }),
        series("kast", "KAST", "%", &|r| {
            row_to_stats(r).kast_pct().map(|v| v as f32)
        }),
        series("entry", "Entry wins", "%", &|r| {
            pct(r.entry_wins, r.entry_attempts)
        }),
        series("trade", "Deaths traded", "%", &|r| {
            pct(r.traded_deaths, r.deaths)
        }),
        series("clutch", "Clutches won", "%", &|r| {
            pct(r.clutch_wins, r.clutch_attempts)
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn moment(facts: serde_json::Value) -> cf_analysis::round_review::Moment {
        cf_analysis::round_review::Moment {
            tick: 100,
            kind: "tracked_death".to_string(),
            rule_id: None,
            delta_p: Some(-0.2),
            facts,
        }
    }

    /// V1.2 final-review fix wave, finding #3: `moment_focus` is
    /// presence-ordered, so a missing `killer` compacts the list rather
    /// than leaving a hole — `focus[1]` is NOT reliably the killer.
    #[test]
    fn moment_focus_compacts_when_a_key_is_absent() {
        let victim_and_teammate_only = moment(json!({
            "victim": "1",
            "nearest_teammate": "3",
        }));
        assert_eq!(
            moment_focus(&victim_and_teammate_only),
            vec!["1".to_string(), "3".to_string()],
            "with no killer, focus[1] is the nearest teammate, not a killer"
        );

        let all_three = moment(json!({
            "victim": "1",
            "killer": "2",
            "nearest_teammate": "3",
        }));
        assert_eq!(
            moment_focus(&all_three),
            vec!["1".to_string(), "2".to_string(), "3".to_string()]
        );
    }

    #[test]
    fn moment_killer_reads_the_named_field_not_a_position() {
        let with_killer = moment(json!({ "victim": "1", "killer": "2" }));
        assert_eq!(moment_killer(&with_killer), Some("2".to_string()));

        let without_killer = moment(json!({
            "victim": "1",
            "nearest_teammate": "3",
        }));
        assert_eq!(
            moment_killer(&without_killer),
            None,
            "must not fall back to focus[1] (which would be \"3\" here)"
        );
    }

    /// V1.2 final-review fix wave, finding #4: only a fact-less STANDALONE
    /// flag moment is suppressed — other kinds ship even with empty facts
    /// (e.g. an unobserved delta on a plant/defuse).
    #[test]
    fn suppresses_fact_less_flag_moments_only() {
        assert!(should_suppress_flag_moment("flag", &[]));
        assert!(!should_suppress_flag_moment(
            "flag",
            &["1 blinded: Alice".to_string()]
        ));
        assert!(!should_suppress_flag_moment("tracked_kill", &[]));
        assert!(!should_suppress_flag_moment("plant", &[]));
    }

    // ---- play_dto / timeline_dto (task-11 review finding #2) --------------

    fn narrator_ctx() -> cf_narrator::MatchContext {
        cf_narrator::MatchContext {
            map: "de_mirage".to_string(),
            tracked: 1,
            names: std::collections::HashMap::from([
                (1, "me".to_string()),
                (2, "Sam".to_string()),
                (9, "Kit".to_string()),
            ]),
            score: (13, 9),
            tracked_result: Some("win".to_string()),
            total_deaths: 10,
            class_13_share_pct: 20.0,
        }
    }

    fn play(
        tick: i32,
        kind: &str,
        facts: serde_json::Value,
        quality: Option<cf_analysis::play_ledger::Quality>,
    ) -> cf_analysis::play_ledger::Play {
        cf_analysis::play_ledger::Play {
            tick,
            phase: "mid".to_string(),
            kind: kind.to_string(),
            facts,
            quality,
            rule_id: None,
            delta_p: None,
        }
    }

    fn rr_moment(tick: i32, kind: &str, delta_p: Option<f32>) -> cf_analysis::round_review::Moment {
        cf_analysis::round_review::Moment {
            tick,
            kind: kind.to_string(),
            rule_id: None,
            delta_p,
            facts: json!({}),
        }
    }

    #[test]
    fn play_dto_joins_delta_p_from_the_matching_moment_only() {
        let death = play(500, "death", json!({"victim": "1", "killer": "9"}), None);

        let matching = [rr_moment(500, "tracked_death", Some(-0.23))];
        assert_eq!(
            play_dto(death.clone(), &matching, &narrator_ctx())
                .expect("play")
                .delta_p,
            Some(-0.23)
        );

        // Same tick, but the only moment there is a tracked_kill, not a
        // tracked_death — kind must match too, not just the tick.
        let wrong_kind = [rr_moment(500, "tracked_kill", Some(0.5))];
        assert_eq!(
            play_dto(death.clone(), &wrong_kind, &narrator_ctx())
                .expect("play")
                .delta_p,
            None
        );

        // No moment shares the tick at all.
        let no_match = [rr_moment(600, "tracked_death", Some(-0.9))];
        assert_eq!(
            play_dto(death, &no_match, &narrator_ctx())
                .expect("play")
                .delta_p,
            None
        );
    }

    #[test]
    fn play_dto_focus_is_presence_ordered_and_killer_is_explicit() {
        let death = play(
            100,
            "death",
            json!({"victim": "1", "killer": "9", "nearest_teammate": "2"}),
            None,
        );
        let dto = play_dto(death, &[], &narrator_ctx()).expect("play");
        assert_eq!(
            dto.focus,
            vec!["1".to_string(), "9".to_string(), "2".to_string()]
        );
        assert_eq!(dto.killer, Some("9".to_string()));

        let trade = play(100, "trade", json!({"teammate": "2", "killer": "9"}), None);
        let dto2 = play_dto(trade, &[], &narrator_ctx()).expect("play");
        assert_eq!(dto2.focus, vec!["9".to_string(), "2".to_string()]);
        assert_eq!(dto2.killer, Some("9".to_string()));
    }

    #[test]
    fn play_dto_quality_maps_to_the_wire_strings() {
        use cf_analysis::play_ledger::Quality;
        let good = play(100, "flash", json!({}), Some(Quality::Good));
        assert_eq!(
            play_dto(good, &[], &narrator_ctx()).expect("play").quality,
            Some("good".to_string())
        );
        let bad = play(100, "flash", json!({}), Some(Quality::Bad));
        assert_eq!(
            play_dto(bad, &[], &narrator_ctx()).expect("play").quality,
            Some("bad".to_string())
        );
        let neutral = play(100, "flash", json!({}), Some(Quality::Neutral));
        assert_eq!(
            play_dto(neutral, &[], &narrator_ctx())
                .expect("play")
                .quality,
            Some("neutral".to_string())
        );
        let none = play(100, "flash", json!({}), None);
        assert_eq!(
            play_dto(none, &[], &narrator_ctx()).expect("play").quality,
            None
        );
    }

    /// V1.2b final-review fix wave, #6: the ledger path applies the same
    /// silence as the moments path — a `flag` play whose narrated facts
    /// came out empty (`H6_DEAD_TIME_SMOKE`'s schema carries nothing to
    /// show) is dropped rather than rendered as a bare rule label.
    #[test]
    fn play_dto_suppresses_a_fact_less_flag_play() {
        use cf_analysis::play_ledger::Quality;
        let mut dead_time = play(100, "flag", json!({"round": 7}), Some(Quality::Bad));
        dead_time.rule_id = Some("H6_DEAD_TIME_SMOKE".to_string());
        assert!(play_dto(dead_time, &[], &narrator_ctx()).is_none());

        let mut unused = play(
            100,
            "flag",
            json!({"round": 7, "held": ["Flashbang"]}),
            Some(Quality::Bad),
        );
        unused.rule_id = Some("H6_UNUSED_UTIL_AT_ROUND_END".to_string());
        let dto = play_dto(unused, &[], &narrator_ctx()).expect("a flag with facts ships");
        assert_eq!(dto.facts, vec!["1 held: Flashbang".to_string()]);

        // Non-flag kinds ship even with nothing to say.
        let plant = play(100, "plant", json!({}), None);
        assert!(play_dto(plant, &[], &narrator_ctx()).is_some());
    }

    // ---- re_analyze_match resolution (V1.2b final-review fix wave, #4) ----

    #[test]
    fn resolve_candidate_prefers_the_users_pick_and_drops_missing_files() {
        let exists = |_: &str| true;
        assert_eq!(
            resolve_candidate(None, Some("/demos/a.dem".into()), exists),
            Some(("/demos/a.dem".to_string(), false)),
            "the stored source_path is not the user's pick"
        );
        assert_eq!(
            resolve_candidate(
                Some("/picked/a.dem".into()),
                Some("/demos/a.dem".into()),
                exists
            ),
            Some(("/picked/a.dem".to_string(), true)),
            "an explicit path wins and is the user's pick"
        );
        assert_eq!(resolve_candidate(None, None, exists), None);
        let missing = |_: &str| false;
        assert_eq!(
            resolve_candidate(None, Some("/demos/gone.dem".into()), missing),
            None,
            "a stored path that no longer exists asks for the file"
        );
        assert_eq!(
            resolve_candidate(Some("/picked/gone.dem".into()), None, missing),
            None
        );
    }

    #[test]
    fn hash_mismatch_at_the_stored_path_asks_for_the_file_but_a_users_pick_is_refused() {
        let stale = hash_mismatch_result(false, "a.dem").expect("needs_file, not an error");
        assert!(stale.needs_file);
        assert_eq!(stale.file_name, "a.dem");
        assert_eq!(stale.map, "");

        let err = hash_mismatch_result(true, "a.dem").expect_err("a wrong pick is refused");
        assert!(err.contains("That file isn't a.dem"), "{err}");
        assert!(err.contains("Pick the original file."), "{err}");
    }

    #[test]
    fn timeline_dto_resolves_known_names_and_keeps_unknown_ids_raw() {
        let known = cf_analysis::play_ledger::TimelineEvent {
            tick: 100,
            kind: "kill".to_string(),
            actor: Some("2".to_string()),
            subject: Some("9".to_string()),
            side: Some("CT".to_string()),
            weapon: Some("ak47".to_string()),
        };
        let dto = timeline_dto(known, &narrator_ctx());
        assert_eq!(dto.actor, Some("Sam".to_string()));
        assert_eq!(dto.subject, Some("Kit".to_string()));
        assert_eq!(dto.side, Some("CT".to_string()));

        let unknown = cf_analysis::play_ledger::TimelineEvent {
            tick: 100,
            kind: "kill".to_string(),
            actor: Some("999999999".to_string()),
            subject: None,
            side: None,
            weapon: None,
        };
        let dto2 = timeline_dto(unknown, &narrator_ctx());
        assert_eq!(dto2.actor, Some("999999999".to_string()));
        assert_eq!(dto2.subject, None);
    }

    // ---- V1.4: stats & understanding ----------------------------------

    #[test]
    fn callout_medians_take_the_per_place_median_and_drop_thin_places() {
        let pos: Vec<(String, f32, f32, f32)> = vec![
            ("BombsiteA".into(), -300.0, -1900.0, -400.0),
            ("BombsiteA".into(), -400.0, -1800.0, -420.0),
            ("BombsiteA".into(), -380.0, -1890.0, -410.0),
            ("Ladder".into(), 100.0, 100.0, 0.0),
        ];
        let rows = callout_medians(&pos, 3);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (
                rows[0].place.as_str(),
                rows[0].x,
                rows[0].y,
                rows[0].samples
            ),
            ("BombsiteA", -380.0, -1890.0, 3)
        );
    }

    /// Nuke's lower level: the label has to carry a z of its own or the
    /// renderer draws "B site" on top of "A site" on the upper radar.
    #[test]
    fn callout_medians_take_the_z_median_so_labels_know_their_layer() {
        let pos: Vec<(String, f32, f32, f32)> = vec![
            ("BombsiteB".into(), 0.0, 0.0, -700.0),
            ("BombsiteB".into(), 10.0, 10.0, -780.0),
            ("BombsiteB".into(), 20.0, 20.0, -728.0),
        ];
        let rows = callout_medians(&pos, 3);
        // Sorted z: -780, -728, -700 -> the middle sample is -728.
        assert_eq!(rows[0].z, -728.0);
    }

    /// A `len / 2` regression would pick the UPPER middle (-350) here; the
    /// documented behaviour is the lower middle (-380) — a real sample, not
    /// an average of two.
    #[test]
    fn callout_medians_take_the_lower_middle_of_an_even_length_group() {
        let pos: Vec<(String, f32, f32, f32)> = vec![
            ("Mid".into(), -300.0, 0.0, 0.0),
            ("Mid".into(), -400.0, 0.0, 0.0),
            ("Mid".into(), -350.0, 0.0, 0.0),
            ("Mid".into(), -380.0, 0.0, 0.0),
        ];
        let rows = callout_medians(&pos, 4);
        assert_eq!(rows.len(), 1);
        // Sorted: -400, -380, -350, -300 -> lower middle is -380.
        assert_eq!(rows[0].x, -380.0);
    }

    /// The Report's "Not yet detected" line is derived from the catalog,
    /// never a literal: a class that ships disappears from it without
    /// anyone editing this file.
    #[test]
    fn classes_not_built_is_the_catalogs_unbuilt_list() {
        let ids = unbuilt_class_ids();
        assert!(
            !ids.contains(&8) && !ids.contains(&10),
            "classes 8 and 10 ship in V1.6: {ids:?}"
        );
        assert_eq!(ids, vec![12]);
    }

    #[test]
    fn stat_series_aligns_with_matches_and_leaves_holes() {
        let full = cf_store::store::MatchStatsRow {
            rounds_played: 10,
            kills: 12,
            deaths: 8,
            assists: 2,
            damage: 800,
            headshots: 6,
            kast_rounds: 7,
            entry_attempts: 4,
            entry_wins: 1,
            traded_deaths: 2,
            trade_kills: 1,
            trade_opportunities: 2,
            clutch_attempts: 2,
            clutch_wins: 1,
        };
        let s = stat_series(&[Some(full), None]);
        let kd = s.iter().find(|x| x.key == "kd").unwrap();
        assert_eq!(kd.values, vec![Some(1.5), None]);
        let adr = s.iter().find(|x| x.key == "adr").unwrap();
        assert_eq!(adr.values, vec![Some(80.0), None]);
        let entry = s.iter().find(|x| x.key == "entry").unwrap();
        assert_eq!(entry.values, vec![Some(25.0), None]); // wins / attempts, percent
        let trade = s.iter().find(|x| x.key == "trade").unwrap();
        assert_eq!(trade.values, vec![Some(25.0), None]); // traded deaths / deaths, percent
        assert_eq!(
            s.iter().map(|x| x.key.as_str()).collect::<Vec<_>>(),
            vec!["kd", "adr", "hs", "kast", "entry", "trade", "clutch"]
        );
    }
}
