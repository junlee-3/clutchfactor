//! D6 reference corpus: pro-demo positional occupancy grids and the D6
//! "unusual positioning" detector (PROMPT.md §5 D6). Per map/side/round-
//! phase, tallies corpus-player positions into a radar-pixel grid; compares
//! the tracked player's key moments against pooled cell density to surface
//! consistently low-density ("unusual", never "wrong" — §5 honesty rule)
//! positioning. Pure math only — ingestion, storage and the corpus screen
//! are a later task.

use std::collections::HashMap;

use cf_parser::model::Side;

use crate::config::CorpusCfg;
use crate::types::{Category, EvidenceRef, Insight};

const D6_DETECTOR_ID: &str = "D6_UNUSUAL_POSITIONING";
/// §5 D6: this measures unusualness, not wrongness — severity/confidence
/// are fixed, low-stakes constants, never scored as a hard rule violation.
const D6_SEVERITY: f32 = 0.5;
const D6_CONFIDENCE: f32 = 0.6;
/// Evidence entries per insight, so a pathological finding can't flood the
/// replay jump-list.
const D6_MAX_EVIDENCE: usize = 8;

/// Vendored awpy radar calibration data (attribution: assets/maps/ATTRIBUTION.md).
const MAP_DATA_JSON: &str = include_str!("../../../../assets/maps/map-data.json");

/// Round phase a position sample was taken at (PROMPT.md §5 D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase {
    FreezeEnd,
    Early,
    Mid,
    PostPlant,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::FreezeEnd => "freeze_end",
            Phase::Early => "early",
            Phase::Mid => "mid",
            Phase::PostPlant => "post_plant",
        }
    }
}

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

/// Stable ordering key for `Side` (it doesn't derive `Ord`).
fn side_rank(side: Side) -> u8 {
    match side {
        Side::Ct => 0,
        Side::T => 1,
    }
}

/// Per-map radar calibration — mirrors `src/replay/coords.ts`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapCalibration {
    pub pos_x: f32,
    pub pos_y: f32,
    pub scale: f32,
}

/// Look up a map's radar calibration from the embedded
/// `assets/maps/map-data.json`. Only `pos_x`/`pos_y`/`scale` are used —
/// `rotate`/`zoom`/`lower_level_max_units` don't factor into the world->
/// radar transform used by the frontend replay viewer.
pub fn calibration_for(map: &str) -> Option<MapCalibration> {
    let parsed: serde_json::Value = serde_json::from_str(MAP_DATA_JSON).ok()?;
    let entry = parsed.get(map)?;
    Some(MapCalibration {
        pos_x: entry.get("pos_x")?.as_f64()? as f32,
        pos_y: entry.get("pos_y")?.as_f64()? as f32,
        scale: entry.get("scale")?.as_f64()? as f32,
    })
}

/// World (x, y) -> occupancy-grid cell, matching `src/replay/coords.ts`'s
/// world->radar transform. `None` when the point falls outside the radar
/// image (radar px range is `0.0..1024.0`, exclusive, on both axes — radar
/// images are 1024x1024).
pub fn grid_cell(cal: &MapCalibration, grid: usize, x: f32, y: f32) -> Option<(usize, usize)> {
    let radar_px_x = (x - cal.pos_x) / cal.scale;
    let radar_px_y = (cal.pos_y - y) / cal.scale;
    if !(0.0..1024.0).contains(&radar_px_x) || !(0.0..1024.0).contains(&radar_px_y) {
        return None;
    }
    let cell_size = 1024.0 / grid as f32;
    let cx = (radar_px_x / cell_size) as usize;
    let cy = (radar_px_y / cell_size) as usize;
    Some((cx, cy))
}

/// One alive-player position sample at a phase moment, from a corpus demo.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseSample {
    pub map: String,
    pub side: Side,
    pub phase: Phase,
    pub x: f32,
    pub y: f32,
}

/// Tallied corpus positions for one (map, side, phase). `counts` is
/// `size * size`, row-major `[y][x]` (index = `cy * size + cx`).
#[derive(Debug, Clone, PartialEq)]
pub struct OccupancyGrid {
    pub map: String,
    pub side: Side,
    pub phase: Phase,
    pub size: usize,
    pub counts: Vec<u32>,
    pub demos: usize,
    pub samples: u64,
}

/// Tally `samples` into one grid per (map, side, phase). `demos_per_map`
/// supplies the corpus-size gate input `unusual_positions` reads later.
/// Samples that fall outside the radar image, or on an uncalibrated map,
/// are silently dropped — corpus ingestion is imperfect, this never panics.
pub fn build_grids(
    samples: &[PhaseSample],
    demos_per_map: &HashMap<String, usize>,
    cfg: &CorpusCfg,
) -> Vec<OccupancyGrid> {
    let mut grids: HashMap<(String, Side, Phase), OccupancyGrid> = HashMap::new();
    for s in samples {
        let Some(cal) = calibration_for(&s.map) else {
            continue;
        };
        let Some((cx, cy)) = grid_cell(&cal, cfg.grid_size, s.x, s.y) else {
            continue;
        };
        let key = (s.map.clone(), s.side, s.phase);
        let grid = grids.entry(key).or_insert_with(|| OccupancyGrid {
            map: s.map.clone(),
            side: s.side,
            phase: s.phase,
            size: cfg.grid_size,
            counts: vec![0; cfg.grid_size * cfg.grid_size],
            demos: *demos_per_map.get(&s.map).unwrap_or(&0),
            samples: 0,
        });
        let idx = cy * grid.size + cx;
        grid.counts[idx] = grid.counts[idx].saturating_add(1);
        grid.samples += 1;
    }
    let mut out: Vec<OccupancyGrid> = grids.into_values().collect();
    out.sort_by(|a, b| {
        a.map
            .cmp(&b.map)
            .then(side_rank(a.side).cmp(&side_rank(b.side)))
            .then(a.phase.cmp(&b.phase))
    });
    out
}

/// Sum of counts over the `(2*neighborhood+1)^2` cells centered on `cell`,
/// clamped at grid edges — out-of-grid neighbor cells simply don't
/// contribute (no wraparound).
pub fn pooled_density(grid: &OccupancyGrid, cell: (usize, usize), neighborhood: usize) -> u32 {
    let (cx, cy) = (cell.0 as isize, cell.1 as isize);
    let n = neighborhood as isize;
    let size = grid.size as isize;
    let mut sum = 0u32;
    for dy in -n..=n {
        let y = cy + dy;
        if y < 0 || y >= size {
            continue;
        }
        for dx in -n..=n {
            let x = cx + dx;
            if x < 0 || x >= size {
                continue;
            }
            sum = sum.saturating_add(grid.counts[y as usize * grid.size + x as usize]);
        }
    }
    sum
}

/// The `pct`-th percentile (nearest-rank) of pooled densities across every
/// cell in `grid` whose pooled density is non-zero. `0` when the grid has
/// no non-zero pooled cells (empty/all-zero grid).
pub fn low_density_threshold(grid: &OccupancyGrid, pct: f32, neighborhood: usize) -> u32 {
    let mut densities: Vec<u32> = (0..grid.size)
        .flat_map(|cy| (0..grid.size).map(move |cx| (cx, cy)))
        .map(|cell| pooled_density(grid, cell, neighborhood))
        .filter(|&d| d != 0)
        .collect();
    if densities.is_empty() {
        return 0;
    }
    densities.sort_unstable();
    let n = densities.len();
    let rank = ((pct / 100.0) * n as f32).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    densities[idx]
}

/// One tracked-player position sample at a key moment in the analyzed demo.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedMoment {
    pub round: u32,
    pub tick: i32,
    pub side: Side,
    pub phase: Phase,
    pub x: f32,
    pub y: f32,
}

/// Recurring low-density positioning for one (side, phase): every
/// qualifying moment (pooled density <= the grid's threshold), in input
/// order.
#[derive(Debug, Clone, PartialEq)]
pub struct PositioningFinding {
    pub phase: Phase,
    pub side: Side,
    pub rounds: Vec<u32>,
    pub ticks: Vec<i32>,
    pub cells: Vec<(usize, usize)>,
    pub pooled_densities: Vec<u32>,
    pub threshold: u32,
}

/// Group `moments` by (side, phase); a moment "qualifies" when its cell's
/// pooled density is at or below that grid's `low_density_pct` threshold. A
/// finding emits only once qualifying moments reach `cfg.min_recurrences`
/// (approximations bias toward silence — false negative >> false positive).
/// Any (side, phase) whose corpus grid has fewer than `cfg.min_demos_per_map`
/// demos — or has no grid at all — is skipped entirely: the corpus-size
/// honesty gate (§5 D6).
pub fn unusual_positions(
    moments: &[TrackedMoment],
    grids: &[OccupancyGrid],
    map: &str,
    cfg: &CorpusCfg,
) -> Vec<PositioningFinding> {
    let mut keys: Vec<(Side, Phase)> = vec![];
    for m in moments {
        let key = (m.side, m.phase);
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    let mut out = vec![];
    for (side, phase) in keys {
        let Some(grid) = grids
            .iter()
            .find(|g| g.map == map && g.side == side && g.phase == phase)
        else {
            continue;
        };
        if grid.demos < cfg.min_demos_per_map {
            continue;
        }
        let Some(cal) = calibration_for(map) else {
            continue;
        };
        let threshold = low_density_threshold(grid, cfg.low_density_pct, cfg.neighborhood);

        let mut rounds = vec![];
        let mut ticks = vec![];
        let mut cells = vec![];
        let mut pooled_densities = vec![];
        for m in moments
            .iter()
            .filter(|m| m.side == side && m.phase == phase)
        {
            let Some(cell) = grid_cell(&cal, grid.size, m.x, m.y) else {
                continue;
            };
            let density = pooled_density(grid, cell, cfg.neighborhood);
            if density <= threshold {
                rounds.push(m.round);
                ticks.push(m.tick);
                cells.push(cell);
                pooled_densities.push(density);
            }
        }
        if rounds.len() >= cfg.min_recurrences {
            out.push(PositioningFinding {
                phase,
                side,
                rounds,
                ticks,
                cells,
                pooled_densities,
                threshold,
            });
        }
    }
    out
}

/// D6 insights from qualifying findings (PROMPT.md §5 D6). One insight per
/// finding: `title_data`/`metrics` carry the structured facts a narrator
/// renders as "reference players rarely hold this position here" — never a
/// hard-rule "wrongness" claim. Evidence is one `EvidenceRef` per qualifying
/// moment, capped at `D6_MAX_EVIDENCE`.
///
/// `cfg` is accepted for interface symmetry with the rest of the corpus
/// pipeline; every threshold this step needs is already baked into
/// `findings` by `unusual_positions`.
pub fn d6_insights(
    findings: &[PositioningFinding],
    map: &str,
    tracked: u64,
    total_rounds: u32,
    tickrate: f32,
    _cfg: &CorpusCfg,
) -> Vec<Insight> {
    findings
        .iter()
        .map(|f| {
            let side = side_str(f.side);
            let phase = f.phase.as_str();
            let camera_hint = Some(format!("heatmap:{map}:{side}:{phase}"));
            let evidence = f
                .rounds
                .iter()
                .zip(&f.ticks)
                .take(D6_MAX_EVIDENCE)
                .map(|(&round, &tick)| EvidenceRef {
                    round,
                    // Same 5 s pre-roll / 2 s post-roll as evidence_around
                    // elsewhere in cf-analysis; tickrate comes from the match
                    // being analyzed, never a constant.
                    tick_start: tick - (5.0 * tickrate) as i32,
                    tick_end: tick + (2.0 * tickrate) as i32,
                    focus_players: vec![tracked],
                    camera_hint: camera_hint.clone(),
                })
                .collect();
            Insight {
                detector: D6_DETECTOR_ID.to_string(),
                category: Category::Positioning,
                severity: D6_SEVERITY,
                confidence: D6_CONFIDENCE,
                round: 0,
                player: tracked,
                title_data: serde_json::json!({
                    "phase": phase,
                    "side": side,
                    "count": f.rounds.len(),
                    "map": map,
                }),
                metrics: serde_json::json!({
                    "rounds": f.rounds,
                    "threshold": f.threshold,
                    "densities": f.pooled_densities,
                    "rounds_analyzed": total_rounds,
                }),
                evidence,
            }
        })
        .collect()
}

/// The phase-sample ticks for one round (PROMPT.md §5 D6 phases): freeze-end,
/// early and mid offsets from `freeze_end_tick`; post-plant offset from the
/// plant. A moment past `end_tick` is dropped (the round was already over),
/// and a plant at or before the mid offset preempts the mid sample — the
/// round has switched to post-plant positioning by then.
pub fn phase_moments(
    freeze_end_tick: i32,
    end_tick: i32,
    plant_tick: Option<i32>,
    tickrate: f32,
    cfg: &CorpusCfg,
) -> Vec<(Phase, i32)> {
    let at = |s: f32| freeze_end_tick + (s * tickrate) as i32;
    let mut out = Vec::new();
    let mut push = |phase: Phase, tick: i32| {
        if tick <= end_tick {
            out.push((phase, tick));
        }
    };
    push(Phase::FreezeEnd, at(cfg.freeze_sample_s));
    push(Phase::Early, at(cfg.early_s));
    let mid_tick = at(cfg.mid_s);
    if !matches!(plant_tick, Some(p) if p <= mid_tick) {
        push(Phase::Mid, mid_tick);
    }
    if let Some(p) = plant_tick {
        push(Phase::PostPlant, p + (cfg.post_plant_s * tickrate) as i32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_parser::model::Side;
    use std::collections::HashMap;

    fn cfg() -> CorpusCfg {
        crate::config::DetectorConfig::default().corpus
    }

    fn world_for(cal: &MapCalibration, px: f32, py: f32) -> (f32, f32) {
        (cal.pos_x + px * cal.scale, cal.pos_y - py * cal.scale)
    }

    #[test]
    fn grid_cell_known_mirage_point_and_out_of_bounds() {
        let cal = calibration_for("de_mirage").expect("mirage calibration present");
        assert_eq!(cal.pos_x, -3230.0);
        assert_eq!(cal.pos_y, 1713.0);
        assert_eq!(cal.scale, 5.0);

        // (pos_x, pos_y) is radar px (0,0) -> cell (0,0).
        assert_eq!(grid_cell(&cal, 128, cal.pos_x, cal.pos_y), Some((0, 0)));

        // radar px (500, 500) -> cell (62, 62) for a 128-grid (8 px/cell).
        let (x, y) = world_for(&cal, 500.0, 500.0);
        assert_eq!(grid_cell(&cal, 128, x, y), Some((62, 62)));

        // Just outside the radar (negative px) -> None.
        assert_eq!(grid_cell(&cal, 128, cal.pos_x - 10.0, cal.pos_y), None);

        // Exactly at the upper edge (radar px == 1024, exclusive) -> None.
        let (edge_x, _) = world_for(&cal, 1024.0, 0.0);
        assert_eq!(grid_cell(&cal, 128, edge_x, cal.pos_y), None);
    }

    #[test]
    fn build_grids_counts_demos_and_tallies_samples() {
        let c = cfg();
        let cal = calibration_for("de_mirage").unwrap();
        let mut demos = HashMap::new();
        demos.insert("de_mirage".to_string(), 12);

        let (xa, ya) = world_for(&cal, 84.0, 84.0); // cell (10, 10)
        let (xb, yb) = world_for(&cal, 300.0, 10.0); // different cell

        let samples = vec![
            PhaseSample {
                map: "de_mirage".to_string(),
                side: Side::Ct,
                phase: Phase::Early,
                x: xa,
                y: ya,
            },
            PhaseSample {
                map: "de_mirage".to_string(),
                side: Side::Ct,
                phase: Phase::Early,
                x: xa,
                y: ya,
            },
            PhaseSample {
                map: "de_mirage".to_string(),
                side: Side::T,
                phase: Phase::Mid,
                x: xb,
                y: yb,
            },
        ];

        let grids = build_grids(&samples, &demos, &c);
        assert_eq!(grids.len(), 2);

        let ct_early = grids
            .iter()
            .find(|g| g.side == Side::Ct && g.phase == Phase::Early)
            .expect("ct/early grid built");
        assert_eq!(ct_early.map, "de_mirage");
        assert_eq!(ct_early.demos, 12);
        assert_eq!(ct_early.samples, 2);
        assert_eq!(ct_early.counts.iter().sum::<u32>(), 2);
        assert_eq!(ct_early.counts[10 * ct_early.size + 10], 2);

        let t_mid = grids
            .iter()
            .find(|g| g.side == Side::T && g.phase == Phase::Mid)
            .expect("t/mid grid built");
        assert_eq!(t_mid.demos, 12);
        assert_eq!(t_mid.samples, 1);
    }

    fn small_grid(size: usize, counts: Vec<u32>, demos: usize) -> OccupancyGrid {
        assert_eq!(counts.len(), size * size);
        OccupancyGrid {
            map: "de_mirage".to_string(),
            side: Side::Ct,
            phase: Phase::Early,
            size,
            samples: counts.iter().map(|&c| c as u64).sum(),
            counts,
            demos,
        }
    }

    #[test]
    fn pooled_density_clamps_at_edges() {
        // 4x4 grid, values 1..=16 row-major.
        let g = small_grid(4, (1..=16).collect(), 10);

        // Corner cell (0,0), neighborhood 1: only the in-bounds quadrant
        // (0,0)=1 (1,0)=2 (0,1)=5 (1,1)=6 contributes.
        assert_eq!(pooled_density(&g, (0, 0), 1), 1 + 2 + 5 + 6);

        // Interior cell (2,2), neighborhood 1: full 3x3 window.
        assert_eq!(
            pooled_density(&g, (2, 2), 1),
            6 + 7 + 8 + 10 + 11 + 12 + 14 + 15 + 16
        );

        // neighborhood 0 is just the cell itself.
        assert_eq!(pooled_density(&g, (3, 3), 0), 16);
    }

    #[test]
    fn threshold_percentile_uniform_spiked_and_empty() {
        // Uniform: every pooled cell equal -> that value at any percentile.
        let uniform = small_grid(4, vec![1; 16], 10);
        assert_eq!(low_density_threshold(&uniform, 5.0, 0), 1);
        assert_eq!(low_density_threshold(&uniform, 95.0, 0), 1);

        // Distinct values 1..=100 (10x10, neighborhood 0 = raw counts):
        // nearest-rank 5th percentile of 100 values is the 5th smallest.
        let spiked = small_grid(10, (1..=100).collect(), 10);
        assert_eq!(low_density_threshold(&spiked, 5.0, 0), 5);
        assert_eq!(low_density_threshold(&spiked, 1.0, 0), 1);
        assert_eq!(low_density_threshold(&spiked, 100.0, 0), 100);

        // Empty/all-zero grid -> 0.
        let empty = small_grid(4, vec![0; 16], 10);
        assert_eq!(low_density_threshold(&empty, 5.0, 1), 0);
    }

    fn moment(round: u32, tick: i32, side: Side, phase: Phase, x: f32, y: f32) -> TrackedMoment {
        TrackedMoment {
            round,
            tick,
            side,
            phase,
            x,
            y,
        }
    }

    #[test]
    fn silence_gate_below_min_demos_produces_no_findings() {
        let c = cfg();
        let cal = calibration_for("de_mirage").unwrap();
        // Every cell density 0 except a corner, low threshold -> everything
        // would qualify, but demos=7 < default min_demos_per_map=8.
        let grid = small_grid(4, vec![0; 16], 7);
        let (x, y) = world_for(&cal, 10.0, 10.0);
        let moments: Vec<TrackedMoment> = (1..=5)
            .map(|r| moment(r, 1000 + r as i32, Side::Ct, Phase::Early, x, y))
            .collect();
        let findings = unusual_positions(&moments, &[grid], "de_mirage", &c);
        assert!(
            findings.is_empty(),
            "corpus below min_demos_per_map must stay silent"
        );
    }

    #[test]
    fn recurrence_gate_needs_min_recurrences_qualifying_moments() {
        let c = cfg(); // min_recurrences = 3
        let cal = calibration_for("de_mirage").unwrap();
        // Mostly-zero grid with min_demos_per_map met -> any sampled cell
        // has pooled density 0, so every moment qualifies (0 <= threshold).
        let grid = small_grid(4, vec![0; 16], c.min_demos_per_map);
        let (x, y) = world_for(&cal, 10.0, 10.0);

        let two: Vec<TrackedMoment> = (1..=2)
            .map(|r| moment(r, 1000 + r as i32, Side::Ct, Phase::Early, x, y))
            .collect();
        assert!(
            unusual_positions(&two, std::slice::from_ref(&grid), "de_mirage", &c).is_empty(),
            "2 qualifying moments < min_recurrences(3) must stay silent"
        );

        let three: Vec<TrackedMoment> = (1..=3)
            .map(|r| moment(r, 1000 + r as i32, Side::Ct, Phase::Early, x, y))
            .collect();
        let findings = unusual_positions(&three, &[grid], "de_mirage", &c);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rounds, vec![1, 2, 3]);
        assert_eq!(findings[0].side, Side::Ct);
        assert_eq!(findings[0].phase, Phase::Early);
        assert_eq!(findings[0].threshold, 0);
        assert_eq!(findings[0].pooled_densities, vec![0, 0, 0]);
    }

    fn finding(rounds: Vec<u32>, ticks: Vec<i32>) -> PositioningFinding {
        let n = rounds.len();
        PositioningFinding {
            phase: Phase::Mid,
            side: Side::T,
            rounds,
            ticks,
            cells: vec![(1, 1); n],
            pooled_densities: vec![0; n],
            threshold: 0,
        }
    }

    #[test]
    fn d6_insights_fields_evidence_shape_and_camera_hint() {
        let c = cfg();
        let f = finding(vec![4, 9, 12], vec![5000, 9000, 15000]);
        let insights = d6_insights(&[f], "de_mirage", 76561199228328773, 24, 64.0, &c);
        assert_eq!(insights.len(), 1);
        let ins = &insights[0];

        assert_eq!(ins.detector, "D6_UNUSUAL_POSITIONING");
        assert_eq!(ins.category, crate::types::Category::Positioning);
        assert_eq!(ins.severity, 0.5);
        assert_eq!(ins.confidence, 0.6);
        assert_eq!(ins.round, 0, "match-level insight");
        assert_eq!(ins.player, 76561199228328773);

        assert_eq!(ins.title_data["phase"], "mid");
        assert_eq!(ins.title_data["side"], "T");
        assert_eq!(ins.title_data["count"], 3);
        assert_eq!(ins.title_data["map"], "de_mirage");

        assert_eq!(ins.metrics["rounds"], serde_json::json!([4, 9, 12]));
        assert_eq!(ins.metrics["threshold"], 0);
        assert_eq!(ins.metrics["densities"], serde_json::json!([0, 0, 0]));
        assert_eq!(ins.metrics["rounds_analyzed"], 24);

        assert_eq!(ins.evidence.len(), 3);
        let e0 = &ins.evidence[0];
        assert_eq!(e0.round, 4);
        assert_eq!(e0.tick_start, 5000 - 320);
        assert_eq!(e0.tick_end, 5000 + 128);
        assert_eq!(e0.focus_players, vec![76561199228328773]);
        assert_eq!(e0.camera_hint, Some("heatmap:de_mirage:T:mid".to_string()));
    }

    #[test]
    fn d6_insights_caps_evidence_at_eight() {
        let c = cfg();
        let rounds: Vec<u32> = (1..=10).collect();
        let ticks: Vec<i32> = (1..=10).map(|r| r * 1000).collect();
        let f = finding(rounds, ticks);
        let insights = d6_insights(&[f], "de_mirage", 1, 24, 64.0, &c);
        assert_eq!(insights[0].evidence.len(), 8);
        assert_eq!(
            insights[0].title_data["count"], 10,
            "count reports all qualifying moments, not the evidence cap"
        );
    }

    #[test]
    fn unusual_positions_and_d6_insights_are_deterministic() {
        let c = cfg();
        let cal = calibration_for("de_mirage").unwrap();
        let grid = small_grid(4, vec![0; 16], c.min_demos_per_map);
        let (x, y) = world_for(&cal, 10.0, 10.0);
        let moments: Vec<TrackedMoment> = (1..=4)
            .map(|r| moment(r, 1000 + r as i32, Side::Ct, Phase::Early, x, y))
            .collect();

        let f1 = unusual_positions(&moments, std::slice::from_ref(&grid), "de_mirage", &c);
        let f2 = unusual_positions(&moments, &[grid], "de_mirage", &c);
        assert_eq!(f1, f2);

        let i1 = d6_insights(&f1, "de_mirage", 42, 30, 64.0, &c);
        let i2 = d6_insights(&f2, "de_mirage", 42, 30, 64.0, &c);
        assert_eq!(i1, i2);
    }

    #[test]
    fn phase_moments_full_round_without_plant() {
        // 64-tick, freeze end at 1000, round runs long: all pre-plant phases.
        let m = phase_moments(1000, 10000, None, 64.0, &cfg());
        assert_eq!(
            m,
            vec![
                (Phase::FreezeEnd, 1064), // +1.0 s
                (Phase::Early, 1640),     // +10 s
                (Phase::Mid, 3240),       // +35 s
            ]
        );
    }

    #[test]
    fn phase_moments_plant_preempts_mid_and_adds_post_plant() {
        // Plant at 2500 (before the +35 s mid moment at 3240): mid is
        // skipped, post-plant sampled at plant + 5 s.
        let m = phase_moments(1000, 10000, Some(2500), 64.0, &cfg());
        assert_eq!(
            m,
            vec![
                (Phase::FreezeEnd, 1064),
                (Phase::Early, 1640),
                (Phase::PostPlant, 2820),
            ]
        );
    }

    #[test]
    fn phase_moments_drops_moments_after_round_end() {
        // Round over at 1500: only freeze-end survives; a late plant's
        // post-plant moment lands past end and is dropped too.
        let m = phase_moments(1000, 1500, Some(1400), 64.0, &cfg());
        assert_eq!(m, vec![(Phase::FreezeEnd, 1064)]);
    }
}
