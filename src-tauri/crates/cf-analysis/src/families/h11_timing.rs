//! D5 — Timing & Rotation (PROMPT.md §5 D5; spec `docs/spec/death-taxonomy.md`
//! §3, H1/H11 mapping table). Two rule shapes share this module:
//!
//! - `H11_EARLY_AGGRESSIVE_DEATH` (flag only — NOT a class source; class 8 is
//!   reserved for H1): the tracked player died within `early_aggression_s`
//!   of freeze_end, having travelled `min_spawn_distance_u`+ from their
//!   freeze-end position, with no teammate close enough to support.
//! - `H6_PUSH_WITHOUT_INFO` (→ class 11): the same shape as above, AND the
//!   team's information proxy is empty in [freeze_end, death) — no enemy
//!   ever spotted, no cross-team damage, no enemy shot fired. Silence across
//!   all three channels reads as "nobody knew this push was coming."
//! - `H11_SLOW_ROTATION` (flag only, round-anchored not death-anchored): a
//!   CT stayed outside `rotate_radius_u` of the plant past `plant_tick +
//!   rotate_max_s` while alive, in a round the team lost.
//!
//! Death-anchored flags follow the classifier convention (classify.rs):
//! `tick` = kill tick, `steamid` = victim. `H11_SLOW_ROTATION` breaks that
//! convention deliberately — it is not about a death.

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{Category, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::{Kill, Round, Side};

pub struct H11Timing;

const EARLY_AGGRESSIVE: &str = "H11_EARLY_AGGRESSIVE_DEATH";
const PUSH_WITHOUT_INFO: &str = "H6_PUSH_WITHOUT_INFO";
const SLOW_ROTATION: &str = "H11_SLOW_ROTATION";
const RULE_IDS: &[&str] = &[EARLY_AGGRESSIVE, PUSH_WITHOUT_INFO, SLOW_ROTATION];

impl Detector for H11Timing {
    fn rule_ids(&self) -> &'static [&'static str] {
        RULE_IDS
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut out = detect_early_aggressive_deaths(ctx, cfg);
        out.extend(detect_push_without_info(ctx, cfg));
        out.extend(detect_slow_rotation(ctx, cfg));
        out
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        _cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let early = flags
            .iter()
            .filter(|f| f.rule_id == EARLY_AGGRESSIVE)
            .count();
        let slow = flags.iter().filter(|f| f.rule_id == SLOW_ROTATION).count();
        let push = flags
            .iter()
            .filter(|f| f.rule_id == PUSH_WITHOUT_INFO)
            .count();
        // Gate on the two H11 rules only (spec: "≥2 flags across the two H11
        // rules") — H6_PUSH_WITHOUT_INFO still shows up in the metrics, it
        // just doesn't count toward opening the insight.
        if early + slow < 2 {
            return vec![];
        }
        let severity = flags.iter().map(|f| f.severity).fold(0.0f32, f32::max);
        let confidence = flags.iter().map(|f| f.confidence).fold(1.0f32, f32::min);
        vec![Insight {
            detector: "D5_TIMING".to_string(),
            category: Category::Timing,
            severity,
            confidence,
            round: 0, // match-level
            player: ctx.tracked(),
            title_data: json!({
                "early_aggressive_deaths": early,
                "slow_rotations": slow,
                "push_without_info": push,
            }),
            metrics: json!({
                "early_aggressive_deaths": early,
                "slow_rotations": slow,
                "push_without_info": push,
            }),
            evidence: flags.iter().take(8).map(|f| f.evidence.clone()).collect(),
        }]
    }
}

/// The round (by number) that `kill.round`/a plant tick belongs to.
fn round_of<'a>(ctx: &'a AnalysisContext, number: u32) -> Option<&'a Round> {
    ctx.data().rounds.iter().find(|r| r.number == number)
}

/// Shared H11_EARLY_AGGRESSIVE_DEATH gate — also required by
/// H6_PUSH_WITHOUT_INFO ("all H11_EARLY_AGGRESSIVE_DEATH conditions"): died
/// within `early_aggression_s` of freeze_end, travelled at least
/// `min_spawn_distance_u` (XY-only, per coordinator resolution — z ignored)
/// from the freeze-end position, and no living teammate within
/// `trade.distance_u` (z-weighted, reusing the trade-support threshold) at
/// the death tick. Returns (seconds_in, distance_from_spawn) on a hit.
fn early_aggro_facts(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    kill: &Kill,
) -> Option<(f32, f32)> {
    let round = round_of(ctx, kill.round)?;
    let freeze_end = round.freeze_end_tick?;
    let elapsed = kill.tick - freeze_end;
    if elapsed < 0 || elapsed > ctx.seconds(cfg.timing.early_aggression_s) {
        return None;
    }
    let spawn = ctx.state_at(kill.victim, freeze_end)?;
    let death = ctx.state_at(kill.victim, kill.tick)?;
    let dx = death.x - spawn.x;
    let dy = death.y - spawn.y;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < cfg.timing.min_spawn_distance_u {
        return None;
    }
    if let Some((_, mate_dist)) =
        ctx.nearest_teammate(kill.victim, kill.round, kill.tick, cfg.general.z_weight)
    {
        if mate_dist <= cfg.trade.distance_u {
            return None;
        }
    }
    Some((elapsed as f32 / ctx.data().tickrate, distance))
}

fn detect_early_aggressive_deaths(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    ctx.tracked_deaths()
        .into_iter()
        .filter_map(|kill| {
            let (seconds_in, distance) = early_aggro_facts(ctx, cfg, kill)?;
            Some(RuleFlag {
                rule_id: EARLY_AGGRESSIVE,
                round: kill.round,
                tick: kill.tick,
                steamid: kill.victim,
                confidence: 0.7,
                severity: cfg.severity.h11_early_aggressive_death,
                details: json!({
                    "seconds_in": seconds_in,
                    "distance_from_spawn": distance,
                }),
                evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim]),
            })
        })
        .collect()
}

/// Info-proxy for H6_PUSH_WITHOUT_INFO (coordinator ambiguity resolution):
/// silence across three channels in [freeze_end, kill.tick) reads as "the
/// team had no information". The window excludes the death tick itself —
/// the fatal shot/hurt IS the moment of death, not information the team had
/// beforehand (same "the killing blow doesn't count as prior contact"
/// precedent used by H4's contactless check).
fn info_proxy_empty(ctx: &AnalysisContext, kill: &Kill, freeze_end: i32) -> bool {
    let Some(my_side) = ctx.side_of(kill.victim, kill.round) else {
        return false; // bias to silence: unknown side can't confirm an empty proxy
    };

    // PlayerState (state_at) does not expose `spotted` — it lives only on
    // the raw TickTable columns, so read them directly here (plan Task 3).
    let tt = &ctx.data().ticks;
    let spotted_enemy = (0..tt.len()).any(|i| {
        tt.spotted[i]
            && tt.tick[i] >= freeze_end
            && tt.tick[i] < kill.tick
            && ctx
                .side_of(tt.steamid[i], kill.round)
                .is_some_and(|s| s != my_side)
    });
    if spotted_enemy {
        return false;
    }

    let cross_team_hurt = ctx.data().hurts.iter().any(|h| {
        h.tick >= freeze_end
            && h.tick < kill.tick
            && h.attacker.is_some_and(|a| {
                let (Some(att_side), Some(vic_side)) = (
                    ctx.side_of(a, kill.round),
                    ctx.side_of(h.victim, kill.round),
                ) else {
                    return false;
                };
                att_side != vic_side
            })
    });
    if cross_team_hurt {
        return false;
    }

    let enemy_shot = ctx.data().shots.iter().any(|s| {
        s.tick >= freeze_end
            && s.tick < kill.tick
            && ctx
                .side_of(s.player, kill.round)
                .is_some_and(|side| side != my_side)
    });

    !enemy_shot
}

fn detect_push_without_info(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    ctx.tracked_deaths()
        .into_iter()
        .filter_map(|kill| {
            let (seconds_in, distance) = early_aggro_facts(ctx, cfg, kill)?;
            let round = round_of(ctx, kill.round)?;
            let freeze_end = round.freeze_end_tick?;
            if !info_proxy_empty(ctx, kill, freeze_end) {
                return None;
            }
            Some(RuleFlag {
                rule_id: PUSH_WITHOUT_INFO,
                round: kill.round,
                tick: kill.tick,
                steamid: kill.victim,
                confidence: 0.6,
                severity: cfg.severity.h6_push_without_info,
                details: json!({
                    "seconds_in": seconds_in,
                    "distance_from_spawn": distance,
                }),
                evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim]),
            })
        })
        .collect()
}

/// `H11_SLOW_ROTATION`: round-anchored, not death-anchored. A CT who was
/// still outside `rotate_radius_u` of the plant spot at `plant_tick +
/// rotate_max_s` (clamped to round end), while alive both at plant and at
/// that deadline, in a round the team lost. Two-point check per coordinator
/// resolution — only the plant-tick and deadline-tick distances are sampled,
/// not the whole timeline in between.
fn detect_slow_rotation(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let z = cfg.general.z_weight;
    let mut out = vec![];
    for round in &ctx.data().rounds {
        let Some(side) = ctx.side_of(tracked, round.number) else {
            continue;
        };
        if side != Side::Ct {
            continue;
        }
        let span_end = round.officially_ended_tick.unwrap_or(round.end_tick);
        let Some(plant) = ctx
            .data()
            .bomb_events
            .iter()
            .find(|b| b.kind == "planted" && b.tick >= round.start_tick && b.tick <= span_end)
        else {
            continue;
        };
        let Some(planter) = plant.player else {
            continue;
        };
        let Some(plant_pos) = ctx.state_at(planter, plant.tick) else {
            continue; // planter has no sample -> silent
        };
        let Some(at_plant) = ctx.state_at(tracked, plant.tick) else {
            continue;
        };
        if !at_plant.is_alive {
            continue; // dead players can't rotate -> silent
        }
        let distance_at_plant = AnalysisContext::dist(&at_plant, &plant_pos, z);
        if distance_at_plant <= cfg.timing.rotate_radius_u {
            continue;
        }
        if round.winner == side {
            continue; // a won round's rotation choice was evidently fine
        }
        let flag_tick = (plant.tick + ctx.seconds(cfg.timing.rotate_max_s)).min(span_end);
        let Some(at_flag) = ctx.state_at(tracked, flag_tick) else {
            continue;
        };
        if !at_flag.is_alive {
            continue; // dead players can't rotate -> silent
        }
        let distance_at_flag = AnalysisContext::dist(&at_flag, &plant_pos, z);
        if distance_at_flag <= cfg.timing.rotate_radius_u {
            continue; // arrived in time
        }
        out.push(RuleFlag {
            rule_id: SLOW_ROTATION,
            round: round.number,
            tick: flag_tick,
            steamid: tracked,
            confidence: 0.65,
            severity: cfg.severity.h11_slow_rotation,
            details: json!({
                "seconds_late_or_never": serde_json::Value::Null,
                "distance_at_plant": distance_at_plant,
            }),
            evidence: evidence_around(ctx, round.number, flag_tick, &[tracked]),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use crate::types::EvidenceRef;
    use cf_parser::model::MatchData;

    const TRACKED: u64 = 1;
    const MATE: u64 = 2;
    const ENEMY: u64 = 3;
    const PLANTER: u64 = 4;

    fn detect(data: &MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H11Timing.detect(&ctx, &DetectorConfig::default())
    }

    fn only<'a>(flags: &'a [RuleFlag], id: &str) -> Vec<&'a RuleFlag> {
        flags.iter().filter(|f| f.rule_id == id).collect()
    }

    // ---------------- H11_EARLY_AGGRESSIVE_DEATH ----------------

    /// CT tracked + teammate, T enemy. Round 1: 1000..6000, freeze_end=1000.
    fn early_base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[TRACKED, MATE])
            .players_t(&[ENEMY])
            .round(1, 1000, 6000)
    }

    #[test]
    fn early_aggressive_fires_within_window_far_from_spawn_no_support() {
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 900.0, 0.0, 0.0) // 15 s later, 900 u away
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0) // far: no support
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build();
        let flags = detect(&data);
        let f = only(&flags, EARLY_AGGRESSIVE);
        assert_eq!(f.len(), 1);
        let flag = f[0];
        assert_eq!(flag.round, 1);
        assert_eq!(flag.tick, 1960, "death-anchored: flag tick = kill tick");
        assert_eq!(flag.steamid, TRACKED);
        assert!((flag.confidence - 0.7).abs() < 1e-6);
        let cfg = DetectorConfig::default();
        assert_eq!(flag.severity, cfg.severity.h11_early_aggressive_death);
        assert!((flag.details["seconds_in"].as_f64().unwrap() - 15.0).abs() < 0.05);
        assert!((flag.details["distance_from_spawn"].as_f64().unwrap() - 900.0).abs() < 1.0);
        assert!(flag.evidence.focus_players.contains(&TRACKED));
    }

    #[test]
    fn early_aggressive_suppressed_after_early_aggression_window() {
        // 25 s after freeze_end: past the 20 s default cutoff.
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 2600, 900.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .kill(ENEMY, TRACKED, 1, 2600, "ak47")
            .build();
        assert!(only(&detect(&data), EARLY_AGGRESSIVE).is_empty());
    }

    #[test]
    fn early_aggressive_suppressed_near_spawn() {
        // 500 u < min_spawn_distance_u (750).
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 500.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build();
        assert!(only(&detect(&data), EARLY_AGGRESSIVE).is_empty());
    }

    #[test]
    fn early_aggressive_suppressed_with_teammate_close() {
        // Teammate 500 u from the death spot: within trade.distance_u (700).
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 900.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, 1400.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build();
        assert!(only(&detect(&data), EARLY_AGGRESSIVE).is_empty());
    }

    // ---------------- H6_PUSH_WITHOUT_INFO ----------------

    /// Satisfies every H11_EARLY_AGGRESSIVE_DEATH condition; no shots/hurts/
    /// spotted events are added, so the info-proxy is empty by construction.
    fn qualifying_push_data() -> MatchData {
        early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 900.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build()
    }

    /// Flips `spotted` true on the sample row for `sid` at `tick` (scenario.rs
    /// densifies every waypoint into 16 Hz TickTable rows but always writes
    /// spotted=false; there is no builder knob for it, so tests mutate the
    /// public `data.ticks.spotted` vec directly post-build, per plan Task 3).
    fn mark_spotted(data: &mut MatchData, sid: u64, tick: i32) {
        let idx = data
            .ticks
            .steamid
            .iter()
            .zip(&data.ticks.tick)
            .position(|(s, t)| *s == sid && *t == tick)
            .expect("sample must exist at the given tick for mutation");
        data.ticks.spotted[idx] = true;
    }

    #[test]
    fn push_without_info_and_early_aggressive_both_fire_when_conditions_and_info_proxy_hold() {
        let data = qualifying_push_data();
        let flags = detect(&data);
        assert_eq!(
            only(&flags, EARLY_AGGRESSIVE).len(),
            1,
            "H6_PUSH_WITHOUT_INFO requires H11_EARLY_AGGRESSIVE_DEATH's conditions \
             to have fired too"
        );
        let f = only(&flags, PUSH_WITHOUT_INFO);
        assert_eq!(f.len(), 1);
        let flag = f[0];
        assert_eq!(flag.round, 1);
        assert_eq!(flag.tick, 1960);
        assert_eq!(flag.steamid, TRACKED);
        assert!((flag.confidence - 0.6).abs() < 1e-6);
        let cfg = DetectorConfig::default();
        assert_eq!(flag.severity, cfg.severity.h6_push_without_info);
        assert!((flag.details["seconds_in"].as_f64().unwrap() - 15.0).abs() < 0.05);
        assert!((flag.details["distance_from_spawn"].as_f64().unwrap() - 900.0).abs() < 1.0);
        assert!(flag.evidence.focus_players.contains(&TRACKED));
    }

    #[test]
    fn push_without_info_suppressed_when_enemy_spotted_before_death() {
        let mut data = qualifying_push_data();
        mark_spotted(&mut data, ENEMY, 1500);
        let flags = detect(&data);
        assert!(only(&flags, PUSH_WITHOUT_INFO).is_empty());
        assert_eq!(
            only(&flags, EARLY_AGGRESSIVE).len(),
            1,
            "spotting doesn't affect the early-aggressive-death flag"
        );
    }

    #[test]
    fn push_without_info_suppressed_when_team_exchanged_damage_first() {
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 900.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .hurt(TRACKED, ENEMY, 1500, 20, "ak47")
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build();
        assert!(only(&detect(&data), PUSH_WITHOUT_INFO).is_empty());
    }

    #[test]
    fn push_without_info_suppressed_when_enemy_fired_before_death() {
        let data = early_base()
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint(TRACKED, 1960, 900.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, -2000.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 6000, 5000.0, 0.0, 0.0)
            .shot(ENEMY, 1500, "weapon_ak47")
            .kill(ENEMY, TRACKED, 1, 1960, "ak47")
            .build();
        assert!(only(&detect(&data), PUSH_WITHOUT_INFO).is_empty());
    }

    // ---------------- H11_SLOW_ROTATION ----------------

    /// CT tracked (+ mate) vs T planter, all holding position. Round 1:
    /// 1000..6000 (officially ends at 6128 per Scenario::round). Plant @2000.
    fn rotation_base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[TRACKED, MATE])
            .players_t(&[PLANTER])
            .round(1, 1000, 6000)
            .hold(PLANTER, 1000, 6000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, 0.0, 0.0, 0.0)
    }

    #[test]
    fn slow_rotation_fires_when_ct_alive_far_never_arrives_round_lost() {
        let data = rotation_base()
            .hold(TRACKED, 1000, 6000, 2000.0, 0.0, 0.0) // stays far all round
            .bomb("planted", PLANTER, 2000)
            .round_won_by(1, Side::T)
            .build();
        let flags = detect(&data);
        let f = only(&flags, SLOW_ROTATION);
        assert_eq!(f.len(), 1);
        let flag = f[0];
        assert_eq!(flag.round, 1);
        assert_eq!(
            flag.tick,
            2000 + 64 * 25,
            "tick = plant_tick + rotate_max_s"
        );
        assert_eq!(flag.steamid, TRACKED);
        assert!((flag.confidence - 0.65).abs() < 1e-6);
        let cfg = DetectorConfig::default();
        assert_eq!(flag.severity, cfg.severity.h11_slow_rotation);
        assert!(flag.details["seconds_late_or_never"].is_null());
        assert!((flag.details["distance_at_plant"].as_f64().unwrap() - 2000.0).abs() < 1.0);
        assert!(flag.evidence.focus_players.contains(&TRACKED));
    }

    #[test]
    fn slow_rotation_suppressed_when_round_won() {
        // Default Scenario::round() winner is Ct == tracked's side here.
        let data = rotation_base()
            .hold(TRACKED, 1000, 6000, 2000.0, 0.0, 0.0)
            .bomb("planted", PLANTER, 2000)
            .build();
        assert!(only(&detect(&data), SLOW_ROTATION).is_empty());
    }

    #[test]
    fn slow_rotation_suppressed_when_arrives_in_time() {
        // Far at plant (2000), but reaches the plant site by plant+25s.
        let data = rotation_base()
            .waypoint(TRACKED, 1000, 2000.0, 0.0, 0.0)
            .waypoint(TRACKED, 3600, 0.0, 0.0, 0.0)
            .bomb("planted", PLANTER, 2000)
            .round_won_by(1, Side::T)
            .build();
        assert!(only(&detect(&data), SLOW_ROTATION).is_empty());
    }

    #[test]
    fn slow_rotation_suppressed_when_tracked_dead_at_plant() {
        let data = rotation_base()
            .waypoint_full(
                TRACKED,
                1000,
                2000.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_ak47"),
                None,
                false,
            )
            .waypoint_full(
                TRACKED, 1500, 2000.0, 0.0, 0.0, 0.0, 0, false, None, None, false,
            )
            .bomb("planted", PLANTER, 2000)
            .round_won_by(1, Side::T)
            .build();
        assert!(only(&detect(&data), SLOW_ROTATION).is_empty());
    }

    #[test]
    fn slow_rotation_suppressed_when_not_ct() {
        // Tracked is T this round: the rule only concerns CT rotation.
        let data = Scenario::new("de_test")
            .players_ct(&[PLANTER])
            .players_t(&[TRACKED, MATE])
            .round(1, 1000, 6000)
            .hold(PLANTER, 1000, 6000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 6000, 0.0, 0.0, 0.0)
            .hold(TRACKED, 1000, 6000, 2000.0, 0.0, 0.0)
            .bomb("planted", PLANTER, 2000)
            .round_won_by(1, Side::T)
            .build();
        assert!(only(&detect(&data), SLOW_ROTATION).is_empty());
    }

    // ---------------- insights ----------------

    fn syn(
        rule_id: &'static str,
        round: u32,
        tick: i32,
        confidence: f32,
        severity: f32,
    ) -> RuleFlag {
        RuleFlag {
            rule_id,
            round,
            tick,
            steamid: TRACKED,
            confidence,
            severity,
            details: json!({}),
            evidence: EvidenceRef {
                round,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![TRACKED],
                camera_hint: None,
            },
        }
    }

    fn insight_ctx_data() -> MatchData {
        Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[ENEMY])
            .round(1, 1000, 5000)
            .build()
    }

    #[test]
    fn insight_fires_at_two_h11_flags_with_metrics() {
        let data = insight_ctx_data();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = vec![
            syn(
                EARLY_AGGRESSIVE,
                1,
                2000,
                0.7,
                cfg.severity.h11_early_aggressive_death,
            ),
            syn(SLOW_ROTATION, 2, 3000, 0.65, cfg.severity.h11_slow_rotation),
        ];
        let insights = H11Timing.insights(&ctx, &cfg, &flags);
        assert_eq!(insights.len(), 1);
        let i = &insights[0];
        assert_eq!(i.detector, "D5_TIMING");
        assert_eq!(i.category, Category::Timing);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        assert_eq!(i.metrics["early_aggressive_deaths"], 1);
        assert_eq!(i.metrics["slow_rotations"], 1);
        assert_eq!(i.metrics["push_without_info"], 0);
        assert_eq!(i.evidence.len(), 2);
    }

    #[test]
    fn insight_suppressed_when_only_one_h11_rule_fired() {
        // One H11 rule + one H6 rule: two flags total, but H6 doesn't count
        // toward the "two H11 rules" gate.
        let data = insight_ctx_data();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = vec![
            syn(
                EARLY_AGGRESSIVE,
                1,
                2000,
                0.7,
                cfg.severity.h11_early_aggressive_death,
            ),
            syn(
                PUSH_WITHOUT_INFO,
                1,
                2000,
                0.6,
                cfg.severity.h6_push_without_info,
            ),
        ];
        assert!(H11Timing.insights(&ctx, &cfg, &flags).is_empty());
    }

    #[test]
    fn insight_evidence_capped_at_eight() {
        let data = insight_ctx_data();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags: Vec<RuleFlag> = (0..9)
            .map(|n| {
                syn(
                    EARLY_AGGRESSIVE,
                    n + 1,
                    2000 + 500 * n as i32,
                    0.7,
                    cfg.severity.h11_early_aggressive_death,
                )
            })
            .collect();
        let insights = H11Timing.insights(&ctx, &cfg, &flags);
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].evidence.len(), 8);
    }
}
