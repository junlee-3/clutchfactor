//! Tauri commands. Snake_case names, typed payloads mirrored by hand in
//! `src/lib/ipc.ts` (keep the MIRROR CHECKLIST there in sync). Steamids are
//! strings on the wire.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use cf_analysis::corpus::{self, Phase, PhaseSample, TrackedMoment};
use cf_parser::extract::{parse_match, ImportStage};
use cf_parser::model::Side;
use cf_store::store::{GridRow, MatchDetail, RoundTicks};
use cf_store::{MatchSummary, Store, StoreError};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use tauri::State;

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

    send(on_progress, "parsing", 0.05, "Parsing demo");
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
    // §7 voice: say what happened and what to do next. A failed parse has
    // written nothing — the save only happens after this point.
    .map_err(|e| {
        format!(
            "Couldn't parse {file_name}: {e}. If this demo is from a different \
             game or the download was cut short, re-download it and try again."
        )
    })?;

    send(on_progress, "saving", 0.88, "Saving to library");
    let match_id = {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .save_match(&file_name, &file_hash, kind, &data)
            .map_err(|e| match e {
                StoreError::DuplicateImport => e.to_string(),
                other => format!("failed to save match: {other}"),
            })?
    };
    let (_, _, score_a, score_b) = cf_parser::extract::derive_score(&data.rounds);
    Ok((match_id, data, score_a, score_b))
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

    let tracked = {
        let store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store.tracked_steamid().map_err(|e| e.to_string())?
    };

    // Analysis needs a tracked player; after save_match the modal fallback
    // always yields one for a non-empty library.
    if let Some(tracked) = tracked.and_then(|t| t.parse::<u64>().ok()) {
        send(&on_progress, "analyzing", 0.92, "Running detectors");
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
        // D6 runs outside analyze(): it needs the corpus grids, which only
        // exist once pro demos were imported and built for this map.
        let has_grids = !store
            .load_grids(&map)
            .map_err(|e| e.to_string())?
            .is_empty();
        if has_grids {
            send(
                &on_progress,
                "analyzing",
                0.97,
                "Comparing positioning to corpus",
            );
            run_positioning(&mut store, match_id)?;
        }
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
}

/// Chronological deaths/class-13 series for the tracked player's own matches,
/// plus per-rule flag-count series aligned to the same match order. Single
/// events (total < 2) are dropped as noise (§7); the rest are capped to the
/// 8 largest totals.
#[tauri::command]
pub fn get_trends(state: State<'_, AppState>) -> Result<TrendsDto, String> {
    let store = state.store.lock().map_err(|_| "store lock poisoned")?;
    let Some(tracked) = store.tracked_steamid().map_err(|e| e.to_string())? else {
        return Ok(TrendsDto {
            matches: vec![],
            rules: vec![],
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

    Ok(TrendsDto { matches, rules })
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

fn threshold_rows(cfg: &cf_analysis::DetectorConfig) -> Vec<ThresholdRow> {
    let row = |name: &str, value: String, unit: &str| ThresholdRow {
        name: name.to_string(),
        value,
        unit: unit.to_string(),
    };
    vec![
        row("Trade window", format!("{}", cfg.trade.window_s), "s"),
        row(
            "Trade distance",
            format!("{}", cfg.trade.distance_u),
            "units",
        ),
        row(
            "Isolation distance",
            format!("{}", cfg.trade.isolation_u),
            "units",
        ),
        row("Effective flash", format!("{}", cfg.flash.effective_s), "s"),
        row(
            "Weapon-switch window",
            format!("{}", cfg.h3.switch_window_s),
            "s",
        ),
        row(
            "Early aggression cutoff",
            format!("{}", cfg.timing.early_aggression_s),
            "s",
        ),
        row(
            "Habit promotion",
            format!(
                "{} of last {}",
                cfg.habit.min_matches, cfg.habit.window_matches
            ),
            "matches",
        ),
        row(
            "Positioning corpus gate",
            format!("{}", cfg.corpus.min_demos_per_map),
            "demos per map",
        ),
    ]
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
    let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
    store.delete_match(match_id).map_err(|e| e.to_string())
}
