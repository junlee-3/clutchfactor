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

#[derive(serde::Serialize)]
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
fn insight_from_row(row: &cf_store::store::InsightRow) -> Option<cf_analysis::Insight> {
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

#[tauri::command]
pub fn get_match_report(
    state: State<'_, AppState>,
    match_id: i64,
) -> Result<Option<MatchReport>, String> {
    use cf_narrator::{CoachingNarrator, TemplateNarrator};
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
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
        classes_not_built: vec![8, 10, 12],
    }))
}

#[tauri::command]
pub fn get_habits(state: State<'_, AppState>) -> Result<Vec<HabitReport>, String> {
    use cf_analysis::habits::{death_hotspots, promote_habits, DeathPoint, HabitInput};
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let Some(tracked) = store.tracked_steamid().map_err(|e| e.to_string())? else {
        return Ok(vec![]);
    };
    let cfg = cf_analysis::DetectorConfig::default();

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
            if let Some(e) = c
                .first_evidence_json
                .as_ref()
                .and_then(|j| serde_json::from_str::<cf_analysis::EvidenceRef>(j).ok())
            {
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
            let n = cf_narrator::narrate_habit(
                &h.rule_id,
                h.matches_hit,
                h.window,
                h.total,
                &serde_json::json!({}),
            );
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
        .death_positions(&tracked)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| DeathPoint {
            match_id: p.match_id,
            map: p.map,
            round: p.round,
            tick: p.tick,
            x: p.x,
            y: p.y,
        })
        .collect();
    let tracked_u64 = tracked.parse::<u64>().ok();
    // One card per map: several clusters on the same map read as duplicate
    // titles and crowd out rule habits — keep the deadliest cluster only.
    let mut seen_maps = std::collections::HashSet::new();
    for hs in death_hotspots(&points, &cfg.habit) {
        if !seen_maps.insert(hs.map.clone()) {
            continue;
        }
        let n = cf_narrator::narrate_habit(
            "H4_REPEAT_HOTSPOT",
            hs.matches,
            cfg.habit.window_matches,
            hs.deaths as u32,
            &serde_json::json!({ "map": hs.map, "deaths": hs.deaths, "matches": hs.matches }),
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
