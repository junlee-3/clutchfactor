//! Tauri commands. Snake_case names, typed payloads mirrored by hand in
//! `src/lib/ipc.ts` (keep the MIRROR CHECKLIST there in sync). Steamids are
//! strings on the wire.

use std::path::PathBuf;
use std::sync::Mutex;

use cf_parser::extract::{parse_match, ImportStage};
use cf_store::store::{MatchDetail, RoundTicks};
use cf_store::{MatchSummary, Store, StoreError};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::State;

/// Per ADR-0002.
const SAMPLE_EVERY: u32 = 4;

pub struct AppState {
    pub store: Mutex<Store>,
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

#[tauri::command]
pub async fn import_demo(
    state: State<'_, AppState>,
    path: String,
    on_progress: Channel<ProgressEvent>,
) -> Result<ImportResult, String> {
    let file = PathBuf::from(&path);
    let file_name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| "not a file path".to_string())?;

    send(&on_progress, "hashing", 0.0, "Hashing demo file");
    let hash_path = file.clone();
    let file_hash = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let bytes = std::fs::read(&hash_path).map_err(|e| format!("cannot read demo: {e}"))?;
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    })
    .await
    .map_err(|e| e.to_string())??;

    // Reject duplicates before the expensive parse (save_match re-checks).
    {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        if store.has_file_hash(&file_hash).map_err(|e| e.to_string())? {
            return Err(StoreError::DuplicateImport.to_string());
        }
    }

    send(&on_progress, "parsing", 0.05, "Parsing demo");
    let parse_path = file.clone();
    let progress_channel = on_progress.clone();
    let data = tauri::async_runtime::spawn_blocking(move || {
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
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    send(&on_progress, "saving", 0.88, "Saving to library");
    let (match_id, map, score_a, score_b, tracked) = {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        let id = store
            .save_match(&file_name, &file_hash, &data)
            .map_err(|e| match e {
                StoreError::DuplicateImport => e.to_string(),
                other => format!("failed to save match: {other}"),
            })?;
        let (_, _, wa, wb) = cf_parser::extract::derive_score(&data.rounds);
        let tracked = store.tracked_steamid().map_err(|e| e.to_string())?;
        (id, data.map.clone(), wa, wb, tracked)
    };

    // Analysis needs a tracked player; after save_match the modal fallback
    // always yields one for a non-empty library.
    if let Some(tracked) = tracked.and_then(|t| t.parse::<u64>().ok()) {
        send(&on_progress, "analyzing", 0.92, "Running detectors");
        let cfg = cf_analysis::DetectorConfig::default();
        let analysis = tauri::async_runtime::spawn_blocking(move || {
            cf_analysis::analyze(&data, tracked, &cfg)
        })
        .await
        .map_err(|e| e.to_string())?;
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .save_analysis(match_id, &analysis)
            .map_err(|e| e.to_string())?;
    }

    send(&on_progress, "done", 1.0, "Import complete");
    Ok(ImportResult {
        match_id,
        map,
        score_a,
        score_b,
    })
}

#[tauri::command]
pub fn list_matches(state: State<'_, AppState>) -> Result<Vec<MatchSummary>, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    store.list_matches().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn tracked_player(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    store.tracked_steamid().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_match_detail(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Option<MatchDetail>, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    store.match_detail(match_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_round_ticks(
    state: State<'_, AppState>,
    match_id: i64,
    round: u32,
) -> Result<RoundTicks, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    store
        .round_ticks(match_id, round)
        .map_err(|e| e.to_string())
}
