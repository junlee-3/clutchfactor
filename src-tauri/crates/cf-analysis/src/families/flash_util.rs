//! D2 flash effectiveness + D3 utility economy (PROMPT.md §5, spec §2 H5 /
//! §3 H6). Sources taxonomy class 3 via `H5_DIED_FLASHED` and flags the H6
//! utility-economy sins: team/self flashes, dead-time smokes, hoarded nades
//! at round end, and utility damage into teammates.
//!
//! Per-flash model: all `player_blind` events with the same (attacker, tick)
//! are one flashbang. A flash that blinded nobody produces no blind events
//! and is therefore invisible here (deliberate: we group blinds, not
//! detonates — bias to silence).
//!
//! `flash_groups`/`round_containing`/`is_utility_weapon` are `pub(crate)`
//! because the play ledger (`crate::play_ledger`) reuses them — the ledger
//! and the detectors must never drift.

use serde_json::json;
use std::collections::BTreeMap;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::evidence_around;
use crate::types::{Category, Insight, RuleFlag};
use crate::Detector;
use cf_parser::model::{MatchData, Round};

pub struct FlashAndUtility;

const RULE_IDS: &[&str] = &[
    "H5_DIED_FLASHED",
    "H6_FLASH_SELF_OR_TEAM",
    "H6_DEAD_TIME_SMOKE",
    "H6_UNUSED_UTIL_AT_ROUND_END",
    "H6_UTIL_TEAM_DAMAGE",
];

/// Kill/hurt weapon names that are always utility, unioned with
/// `cfg.util.utility_kill_weapons` so a config override can only widen the set.
const UTILITY_WEAPONS: &[&str] = &["hegrenade", "inferno", "molotov", "incgrenade"];

/// One flashbang thrown by the tracked player (grouped blind events).
pub(crate) struct FlashGroup {
    pub(crate) tick: i32,
    pub(crate) round: u32,
    /// Enemy victims blinded ≥ effective_s.
    pub(crate) enemies_effective: Vec<u64>,
    /// Teammate victims blinded ≥ effective_s (self excluded).
    pub(crate) teammates_blinded: Vec<u64>,
    /// Tracked player blinded themselves ≥ effective_s.
    pub(crate) self_blind: bool,
    /// An enemy blinded by this flash died to the tracked player's side (or
    /// with a flash assist credited to the tracked player) within the
    /// conversion window.
    pub(crate) converted: bool,
}

/// The round whose [start_tick, officially_ended_tick] range contains `tick`.
/// Falls back to end_tick when officially_ended is missing (old import) —
/// which silences dead-time detection for that round, per bias-to-silence.
pub(crate) fn round_containing(data: &MatchData, tick: i32) -> Option<&Round> {
    data.rounds
        .iter()
        .find(|r| r.start_tick <= tick && tick <= r.officially_ended_tick.unwrap_or(r.end_tick))
}

pub(crate) fn is_utility_weapon(weapon: &str, cfg: &DetectorConfig) -> bool {
    UTILITY_WEAPONS.contains(&weapon) || cfg.util.utility_kill_weapons.iter().any(|w| w == weapon)
}

/// Group the tracked player's blind events by tick: one group per flashbang.
pub(crate) fn flash_groups(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<FlashGroup> {
    let tracked = ctx.tracked();
    let data = ctx.data();
    let mut by_tick: BTreeMap<i32, Vec<&cf_parser::model::Blind>> = BTreeMap::new();
    for b in &data.blinds {
        if b.attacker == Some(tracked) {
            by_tick.entry(b.tick).or_default().push(b);
        }
    }
    let conv_window = ctx.seconds(cfg.flash.conversion_window_s);
    let mut out = vec![];
    for (tick, blinds) in by_tick {
        let Some(round) = round_containing(data, tick) else {
            continue; // outside any round → can't attribute sides; silence
        };
        let rn = round.number;
        let Some(my_side) = ctx.side_of(tracked, rn) else {
            continue;
        };
        let mut g = FlashGroup {
            tick,
            round: rn,
            enemies_effective: vec![],
            teammates_blinded: vec![],
            self_blind: false,
            converted: false,
        };
        // Enemy victims regardless of duration — conversion cross-check pool.
        let mut enemy_victims: Vec<u64> = vec![];
        for b in blinds {
            let effective = b.duration >= cfg.flash.effective_s;
            if b.victim == tracked {
                g.self_blind |= effective;
                continue;
            }
            match ctx.side_of(b.victim, rn) {
                Some(s) if s == my_side => {
                    if effective {
                        g.teammates_blinded.push(b.victim);
                    }
                }
                Some(_) => {
                    enemy_victims.push(b.victim);
                    if effective {
                        g.enemies_effective.push(b.victim);
                    }
                }
                None => {}
            }
        }
        g.converted = data.kills.iter().any(|k| {
            k.tick >= tick
                && k.tick <= tick + conv_window
                && enemy_victims.contains(&k.victim)
                && ((g.enemies_effective.contains(&k.victim)
                    && k.attacker.and_then(|a| ctx.side_of(a, rn)) == Some(my_side))
                    || (k.assistedflash && k.assister == Some(tracked)))
        });
        out.push(g);
    }
    out
}

/// H5_DIED_FLASHED — death-anchored (tick = kill tick, steamid = victim):
/// tracked player died inside an enemy-attributed effective blind window.
fn died_flashed(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let mut out = vec![];
    for kill in ctx.tracked_deaths() {
        let Some(blind) = ctx.blind_window_at(tracked, kill.tick) else {
            continue;
        };
        let Some(blinder) = blind.attacker else {
            continue;
        };
        let (Some(blinder_side), Some(victim_side)) = (
            ctx.side_of(blinder, kill.round),
            ctx.side_of(tracked, kill.round),
        ) else {
            continue;
        };
        if blinder_side == victim_side || blind.duration < cfg.flash.effective_s {
            continue;
        }
        out.push(RuleFlag {
            rule_id: "H5_DIED_FLASHED",
            round: kill.round,
            tick: kill.tick,
            steamid: tracked,
            confidence: 0.85,
            severity: cfg.severity.h5_died_flashed,
            details: json!({
                "blinder": blinder.to_string(),
                "blind_duration": blind.duration,
            }),
            evidence: evidence_around(ctx, kill.round, kill.tick, &[tracked, blinder]),
        });
    }
    out
}

/// H6_FLASH_SELF_OR_TEAM — a flash that effectively blinded ≥1 teammate or
/// self and zero enemies.
fn flash_self_or_team(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let mut out = vec![];
    for g in flash_groups(ctx, cfg) {
        let mut victims = g.teammates_blinded.clone();
        if g.self_blind {
            victims.push(tracked);
        }
        if victims.is_empty() || !g.enemies_effective.is_empty() {
            continue;
        }
        let mut focus = vec![tracked];
        focus.extend(&victims);
        out.push(RuleFlag {
            rule_id: "H6_FLASH_SELF_OR_TEAM",
            round: g.round,
            tick: g.tick,
            steamid: tracked,
            confidence: 0.9,
            severity: cfg.severity.h6_flash_self_or_team,
            details: json!({
                "teammates_blinded": g.teammates_blinded.len(),
                "victims": victims.iter().map(u64::to_string).collect::<Vec<_>>(),
            }),
            evidence: evidence_around(ctx, g.round, g.tick, &focus),
        });
    }
    out
}

/// H6_DEAD_TIME_SMOKE — smoke thrown by the tracked player after the round's
/// end_tick (but still inside the round's officially-ended range).
fn dead_time_smokes(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let data = ctx.data();
    let mut out = vec![];
    for ge in &data.grenades {
        if ge.kind != "smoke" || ge.thrower != Some(tracked) {
            continue;
        }
        let Some(r) = round_containing(data, ge.tick) else {
            continue;
        };
        if ge.tick <= r.end_tick {
            continue;
        }
        out.push(RuleFlag {
            rule_id: "H6_DEAD_TIME_SMOKE",
            round: r.number,
            tick: ge.tick,
            steamid: tracked,
            confidence: 0.9,
            severity: cfg.severity.h6_dead_time_smoke,
            details: json!({ "round": r.number }),
            evidence: evidence_around(ctx, r.number, ge.tick, &[tracked]),
        });
    }
    out
}

/// H6_UNUSED_UTIL_AT_ROUND_END — still alive at round end holding
/// ≥ min_unused_nades grenades. Dead players' nades are H3's business.
fn unused_util(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let mut out = vec![];
    for r in &ctx.data().rounds {
        let Some(inv) = ctx.inventory_at(tracked, r.end_tick) else {
            continue; // no sample (old import) → silence
        };
        let held: Vec<&String> = inv
            .items
            .iter()
            .filter(|i| cfg.util.grenade_items.contains(i))
            .collect();
        if held.len() < cfg.util.min_unused_nades {
            continue;
        }
        if !ctx
            .state_at(tracked, r.end_tick)
            .is_some_and(|s| s.is_alive)
        {
            continue;
        }
        out.push(RuleFlag {
            rule_id: "H6_UNUSED_UTIL_AT_ROUND_END",
            round: r.number,
            tick: r.end_tick,
            steamid: tracked,
            confidence: 0.85,
            severity: cfg.severity.h6_unused_util_at_round_end,
            details: json!({ "round": r.number, "held": held }),
            evidence: evidence_around(ctx, r.number, r.end_tick, &[tracked]),
        });
    }
    out
}

/// H6_UTIL_TEAM_DAMAGE — the tracked player's utility damaged a teammate.
fn util_team_damage(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let data = ctx.data();
    let mut out = vec![];
    for h in &data.hurts {
        if h.attacker != Some(tracked) || h.victim == tracked || h.dmg_health < 1 {
            continue;
        }
        if !is_utility_weapon(&h.weapon, cfg) {
            continue;
        }
        let Some(r) = round_containing(data, h.tick) else {
            continue;
        };
        let rn = r.number;
        let (Some(my_side), Some(victim_side)) =
            (ctx.side_of(tracked, rn), ctx.side_of(h.victim, rn))
        else {
            continue;
        };
        if my_side != victim_side {
            continue;
        }
        out.push(RuleFlag {
            rule_id: "H6_UTIL_TEAM_DAMAGE",
            round: rn,
            tick: h.tick,
            steamid: tracked,
            confidence: 0.95,
            severity: cfg.severity.h6_util_team_damage,
            details: json!({
                "victim": h.victim.to_string(),
                "damage": h.dmg_health,
                "weapon": h.weapon,
            }),
            evidence: evidence_around(ctx, rn, h.tick, &[tracked, h.victim]),
        });
    }
    out
}

/// D2 match-level flash report: always emitted once the tracked player threw
/// ≥3 flashes (that blinded anyone). Informational unless team flashes
/// outnumber effective ones.
fn flash_report(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Option<Insight> {
    let tracked = ctx.tracked();
    let groups = flash_groups(ctx, cfg);
    if groups.len() < 3 {
        return None;
    }
    let flashes = groups.len();
    let effective = groups
        .iter()
        .filter(|g| !g.enemies_effective.is_empty())
        .count();
    let team_flashes = groups
        .iter()
        .filter(|g| !g.teammates_blinded.is_empty())
        .count();
    let self_flashes = groups.iter().filter(|g| g.self_blind).count();
    let conversions = groups.iter().filter(|g| g.converted).count();
    let severity = if team_flashes > effective {
        cfg.severity.h6_flash_self_or_team
    } else {
        0.5
    };

    let mut evidence = vec![];
    // Worst team flash: most teammates (+self) effectively blinded.
    if let Some(worst) = groups
        .iter()
        .filter(|g| !g.teammates_blinded.is_empty() || g.self_blind)
        .max_by_key(|g| g.teammates_blinded.len() + usize::from(g.self_blind))
    {
        let mut focus = vec![tracked];
        focus.extend(&worst.teammates_blinded);
        evidence.push(evidence_around(ctx, worst.round, worst.tick, &focus));
    }
    // Best converted flash: most enemies effectively blinded.
    if let Some(best) = groups
        .iter()
        .filter(|g| g.converted)
        .max_by_key(|g| g.enemies_effective.len())
    {
        let mut focus = vec![tracked];
        focus.extend(&best.enemies_effective);
        evidence.push(evidence_around(ctx, best.round, best.tick, &focus));
    }
    evidence.truncate(6);

    Some(Insight {
        detector: "D2_FLASH_EFFECTIVENESS".to_string(),
        category: Category::Utility,
        severity,
        confidence: 0.9,
        round: 0,
        player: tracked,
        title_data: json!({
            "flashes": flashes,
            "effective": effective,
            "team_flashes": team_flashes,
            "conversions": conversions,
        }),
        metrics: json!({
            "flashes": flashes,
            "effective_rate": effective as f32 / flashes as f32,
            "team_flashes": team_flashes,
            "self_flashes": self_flashes,
            "conversions": conversions,
        }),
        evidence,
    })
}

/// Gate + shape for one flag-derived match-level aggregate insight.
struct AggregateSpec {
    rule_id: &'static str,
    min_events: usize,
    severity: f32,
    confidence: f32,
}

/// Aggregate a rule's flags into one match-level insight (evidence capped 6).
fn aggregate(
    tracked: u64,
    flags: &[RuleFlag],
    spec: &AggregateSpec,
    title_data: serde_json::Value,
    metrics: serde_json::Value,
) -> Option<Insight> {
    let matching: Vec<&RuleFlag> = flags.iter().filter(|f| f.rule_id == spec.rule_id).collect();
    if matching.len() < spec.min_events {
        return None;
    }
    let mut evidence: Vec<_> = matching.iter().map(|f| f.evidence.clone()).collect();
    evidence.truncate(6);
    Some(Insight {
        detector: spec.rule_id.to_string(),
        category: Category::Utility,
        severity: spec.severity,
        confidence: spec.confidence,
        round: 0,
        player: tracked,
        title_data,
        metrics,
        evidence,
    })
}

impl Detector for FlashAndUtility {
    fn rule_ids(&self) -> &'static [&'static str] {
        RULE_IDS
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut flags = died_flashed(ctx, cfg);
        flags.extend(flash_self_or_team(ctx, cfg));
        flags.extend(dead_time_smokes(ctx, cfg));
        flags.extend(unused_util(ctx, cfg));
        flags.extend(util_team_damage(ctx, cfg));
        flags
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let tracked = ctx.tracked();
        let mut out = vec![];
        if let Some(report) = flash_report(ctx, cfg) {
            out.push(report);
        }
        let team_dmg_events = flags
            .iter()
            .filter(|f| f.rule_id == "H6_UTIL_TEAM_DAMAGE")
            .count();
        let total_damage: i64 = flags
            .iter()
            .filter(|f| f.rule_id == "H6_UTIL_TEAM_DAMAGE")
            .filter_map(|f| f.details["damage"].as_i64())
            .sum();
        if let Some(agg) = aggregate(
            tracked,
            flags,
            &AggregateSpec {
                rule_id: "H6_UTIL_TEAM_DAMAGE",
                min_events: 2,
                severity: cfg.severity.h6_util_team_damage,
                confidence: 0.95,
            },
            json!({ "events": team_dmg_events, "total_damage": total_damage }),
            json!({ "events": team_dmg_events, "total_damage": total_damage }),
        ) {
            out.push(agg);
        }
        let hoard_rounds = flags
            .iter()
            .filter(|f| f.rule_id == "H6_UNUSED_UTIL_AT_ROUND_END")
            .count();
        if let Some(agg) = aggregate(
            tracked,
            flags,
            &AggregateSpec {
                rule_id: "H6_UNUSED_UTIL_AT_ROUND_END",
                min_events: 3,
                severity: cfg.severity.h6_unused_util_at_round_end,
                confidence: 0.85,
            },
            json!({ "rounds": hoard_rounds, "min_nades": cfg.util.min_unused_nades }),
            json!({ "rounds": hoard_rounds }),
        ) {
            out.push(agg);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use cf_parser::model::MatchData;

    const TRACKED: u64 = 1;

    /// CT [1 (tracked), 2], T [3, 4], one round 1000..5000 (officially ended
    /// 5128), everyone holding a position for the whole round.
    fn base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .hold(1, 1000, 5000, 0.0, 0.0, 0.0)
            .hold(2, 1000, 5000, 200.0, 0.0, 0.0)
            .hold(3, 1000, 5000, 1000.0, 0.0, 0.0)
            .hold(4, 1000, 5000, 1200.0, 0.0, 0.0)
    }

    fn detect(data: &MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        FlashAndUtility.detect(&ctx, &DetectorConfig::default())
    }

    fn insights_of(data: &MatchData) -> Vec<Insight> {
        let ctx = AnalysisContext::new(data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = FlashAndUtility.detect(&ctx, &cfg);
        FlashAndUtility.insights(&ctx, &cfg, &flags)
    }

    fn with_id<'f>(flags: &'f [RuleFlag], id: &str) -> Vec<&'f RuleFlag> {
        flags.iter().filter(|f| f.rule_id == id).collect()
    }

    // ---- H5_DIED_FLASHED -------------------------------------------------

    #[test]
    fn died_flashed_fires_inside_enemy_blind_window() {
        // Enemy 3 blinds tracked for 1.5 s at 1900; kill at 1950 (covered).
        let data = base()
            .blind(3, TRACKED, 1900, 1.5)
            .kill(3, TRACKED, 1, 1950, "ak47")
            .build();
        let flags = detect(&data);
        let h5 = with_id(&flags, "H5_DIED_FLASHED");
        assert_eq!(h5.len(), 1);
        let f = h5[0];
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, 1950, "death-anchored: tick is the kill tick");
        assert_eq!(f.steamid, TRACKED, "death-anchored: steamid is the victim");
        assert!((f.confidence - 0.85).abs() < 1e-6);
        assert_eq!(f.details["blinder"], "3");
        assert!((f.details["blind_duration"].as_f64().unwrap() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn died_flashed_suppressed_when_blinded_by_teammate() {
        let data = base()
            .blind(2, TRACKED, 1900, 1.5)
            .kill(3, TRACKED, 1, 1950, "ak47")
            .build();
        assert!(with_id(&detect(&data), "H5_DIED_FLASHED").is_empty());
    }

    #[test]
    fn died_flashed_suppressed_below_effective_duration() {
        // 0.9 s blind still covers the kill tick but is under effective_s 1.1.
        let data = base()
            .blind(3, TRACKED, 1900, 0.9)
            .kill(3, TRACKED, 1, 1950, "ak47")
            .build();
        assert!(with_id(&detect(&data), "H5_DIED_FLASHED").is_empty());
    }

    #[test]
    fn died_flashed_suppressed_when_death_outside_blind_window() {
        // Blind 1000..~1096; death at 2000 — long recovered.
        let data = base()
            .blind(3, TRACKED, 1000, 1.5)
            .kill(3, TRACKED, 1, 2000, "ak47")
            .build();
        assert!(with_id(&detect(&data), "H5_DIED_FLASHED").is_empty());
    }

    // ---- Flash grouping / effectiveness ----------------------------------

    #[test]
    fn blinds_at_same_tick_group_into_one_flash() {
        // Three blind events at tick 1500 = one flashbang (2 enemies + 1
        // teammate); a fourth at 2000 is a separate flash.
        let data = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .blind(TRACKED, 4, 1500, 2.0)
            .blind(TRACKED, 2, 1500, 1.5)
            .blind(TRACKED, 3, 2000, 1.5)
            .build();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let groups = flash_groups(&ctx, &DetectorConfig::default());
        assert_eq!(groups.len(), 2);
        let g = &groups[0];
        assert_eq!(g.tick, 1500);
        assert_eq!(g.round, 1);
        assert_eq!(g.enemies_effective.len(), 2);
        assert_eq!(g.teammates_blinded, vec![2]);
        assert!(!g.self_blind);
    }

    #[test]
    fn team_flash_flag_fires_when_only_teammates_blinded() {
        let data = base().blind(TRACKED, 2, 1500, 1.5).build();
        let flags = detect(&data);
        let tf = with_id(&flags, "H6_FLASH_SELF_OR_TEAM");
        assert_eq!(tf.len(), 1);
        let f = tf[0];
        assert_eq!(f.tick, 1500);
        assert_eq!(f.round, 1);
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.9).abs() < 1e-6);
        assert_eq!(f.details["teammates_blinded"], 1);
        assert_eq!(f.details["victims"][0], "2");
    }

    #[test]
    fn team_flash_flag_suppressed_when_enemy_also_effectively_blinded() {
        let data = base()
            .blind(TRACKED, 2, 1500, 1.5)
            .blind(TRACKED, 3, 1500, 1.5)
            .build();
        assert!(with_id(&detect(&data), "H6_FLASH_SELF_OR_TEAM").is_empty());
    }

    #[test]
    fn conversion_counted_when_blinded_enemy_dies_to_tracked_side_in_window() {
        // Enemy 3 blinded at 1500, killed by teammate 2 at 1560 (< 2 s = 128).
        let data = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .kill(2, 3, 1, 1560, "ak47")
            .build();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let groups = flash_groups(&ctx, &DetectorConfig::default());
        assert_eq!(groups.len(), 1);
        assert!(groups[0].converted);
    }

    #[test]
    fn conversion_not_counted_outside_window_or_from_enemy_side() {
        // Kill lands after the 2 s window (1500 + 128 = 1628).
        let late = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .kill(2, 3, 1, 1700, "ak47")
            .build();
        let ctx = AnalysisContext::new(&late, TRACKED);
        assert!(!flash_groups(&ctx, &DetectorConfig::default())[0].converted);

        // Killer is on the enemy's own side, not the thrower's.
        let wrong_side = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .kill(4, 3, 1, 1560, "ak47")
            .build();
        let ctx = AnalysisContext::new(&wrong_side, TRACKED);
        assert!(!flash_groups(&ctx, &DetectorConfig::default())[0].converted);
    }

    #[test]
    fn assistedflash_kill_credited_to_tracked_counts_as_conversion() {
        // Blind too short to be "effective", but the game credited the
        // tracked player with a flash assist on the kill — authoritative.
        let mut data = base()
            .blind(TRACKED, 3, 1500, 0.5)
            .kill(2, 3, 1, 1560, "ak47")
            .build();
        {
            let ctx = AnalysisContext::new(&data, TRACKED);
            assert!(
                !flash_groups(&ctx, &DetectorConfig::default())[0].converted,
                "ineffective blind alone is not a conversion"
            );
        }
        let k = data.kills.iter_mut().find(|k| k.victim == 3).unwrap();
        k.assistedflash = true;
        k.assister = Some(TRACKED);
        let ctx = AnalysisContext::new(&data, TRACKED);
        assert!(flash_groups(&ctx, &DetectorConfig::default())[0].converted);
    }

    // ---- H6_DEAD_TIME_SMOKE ----------------------------------------------

    #[test]
    fn dead_time_smoke_fires_after_round_end() {
        // Round ends at 5000, officially ends 5128; smoke at 5050 = dead time.
        let data = base().grenade("smoke", TRACKED, 5050, 0.0, 0.0).build();
        let flags = detect(&data);
        let dt = with_id(&flags, "H6_DEAD_TIME_SMOKE");
        assert_eq!(dt.len(), 1);
        assert_eq!(dt[0].tick, 5050);
        assert_eq!(dt[0].round, 1);
        assert_eq!(dt[0].steamid, TRACKED);
        assert!((dt[0].confidence - 0.9).abs() < 1e-6);
        assert_eq!(dt[0].details["round"], 1);
    }

    #[test]
    fn dead_time_smoke_suppressed_mid_round_or_wrong_kind_or_thrower() {
        let data = base()
            .grenade("smoke", TRACKED, 3000, 0.0, 0.0) // mid-round
            .grenade("flashbang", TRACKED, 5050, 0.0, 0.0) // not a smoke
            .grenade("smoke", 3, 5060, 0.0, 0.0) // not the tracked player
            .build();
        assert!(with_id(&detect(&data), "H6_DEAD_TIME_SMOKE").is_empty());
    }

    // ---- H6_UNUSED_UTIL_AT_ROUND_END --------------------------------------

    #[test]
    fn unused_util_fires_when_alive_with_two_nades_at_round_end() {
        let data = base()
            .inventory(
                TRACKED,
                5000,
                &["Flashbang", "Smoke Grenade", "Kevlar Vest"],
            )
            .build();
        let flags = detect(&data);
        let uu = with_id(&flags, "H6_UNUSED_UTIL_AT_ROUND_END");
        assert_eq!(uu.len(), 1);
        assert_eq!(uu[0].tick, 5000, "tick is the round end_tick");
        assert_eq!(uu[0].round, 1);
        assert_eq!(uu[0].steamid, TRACKED);
        assert!((uu[0].confidence - 0.85).abs() < 1e-6);
        assert_eq!(uu[0].details["round"], 1);
        assert_eq!(
            uu[0].details["held"],
            serde_json::json!(["Flashbang", "Smoke Grenade"]),
            "held lists only grenade items"
        );
    }

    #[test]
    fn unused_util_suppressed_when_dead_at_round_end() {
        // Dead players' leftover nades are H3_WASTED_UTILITY's business.
        let data = Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .waypoint(TRACKED, 1000, 0.0, 0.0, 0.0)
            .waypoint_full(
                TRACKED, 4000, 0.0, 0.0, 0.0, 0.0, 0, false, None, None, false,
            )
            .waypoint_full(
                TRACKED, 5000, 0.0, 0.0, 0.0, 0.0, 0, false, None, None, false,
            )
            .hold(2, 1000, 5000, 200.0, 0.0, 0.0)
            .hold(3, 1000, 5000, 1000.0, 0.0, 0.0)
            .hold(4, 1000, 5000, 1200.0, 0.0, 0.0)
            .inventory(TRACKED, 5000, &["Flashbang", "Smoke Grenade"])
            .build();
        assert!(with_id(&detect(&data), "H6_UNUSED_UTIL_AT_ROUND_END").is_empty());
    }

    #[test]
    fn unused_util_suppressed_with_one_nade_or_no_sample() {
        let one_nade = base()
            .inventory(TRACKED, 5000, &["Flashbang", "USP-S"])
            .build();
        assert!(with_id(&detect(&one_nade), "H6_UNUSED_UTIL_AT_ROUND_END").is_empty());

        // No inventory sample at all (old import) → silence.
        let no_sample = base().build();
        assert!(with_id(&detect(&no_sample), "H6_UNUSED_UTIL_AT_ROUND_END").is_empty());
    }

    // ---- H6_UTIL_TEAM_DAMAGE ----------------------------------------------

    #[test]
    fn util_team_damage_fires_on_teammate_molotov_hurt() {
        let data = base().hurt(TRACKED, 2, 2000, 12, "molotov").build();
        let flags = detect(&data);
        let td = with_id(&flags, "H6_UTIL_TEAM_DAMAGE");
        assert_eq!(td.len(), 1);
        let f = td[0];
        assert_eq!(f.tick, 2000);
        assert_eq!(f.round, 1);
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.95).abs() < 1e-6);
        assert_eq!(f.details["victim"], "2");
        assert_eq!(f.details["damage"], 12);
        assert_eq!(f.details["weapon"], "molotov");
    }

    #[test]
    fn util_team_damage_suppressed_on_enemy_self_or_gun_damage() {
        let data = base()
            .hurt(TRACKED, 3, 2000, 12, "molotov") // enemy: fine
            .hurt(TRACKED, TRACKED, 2100, 5, "hegrenade") // self-damage
            .hurt(TRACKED, 2, 2200, 20, "ak47") // team damage, but not utility
            .build();
        assert!(with_id(&detect(&data), "H6_UTIL_TEAM_DAMAGE").is_empty());
    }

    // ---- Insights ----------------------------------------------------------

    #[test]
    fn flash_report_requires_three_flashes() {
        let two = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .blind(TRACKED, 3, 2000, 2.0)
            .build();
        assert!(!insights_of(&two)
            .iter()
            .any(|i| i.detector == "D2_FLASH_EFFECTIVENESS"));

        let three = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .blind(TRACKED, 3, 2000, 2.0)
            .blind(TRACKED, 3, 2500, 2.0)
            .build();
        assert!(insights_of(&three)
            .iter()
            .any(|i| i.detector == "D2_FLASH_EFFECTIVENESS"));
    }

    #[test]
    fn flash_report_metrics_and_informational_severity() {
        // Flash A: effective + converted. Flash B: pure team flash.
        // Flash C: dud (enemy blinded 0.3 s).
        let data = base()
            .blind(TRACKED, 3, 1500, 2.0)
            .kill(2, 3, 1, 1560, "ak47")
            .blind(TRACKED, 2, 2000, 1.5)
            .blind(TRACKED, 4, 2500, 0.3)
            .build();
        let ins = insights_of(&data);
        let report = ins
            .iter()
            .find(|i| i.detector == "D2_FLASH_EFFECTIVENESS")
            .expect("flash report");
        assert_eq!(report.category, Category::Utility);
        assert_eq!(report.round, 0, "match-level");
        assert_eq!(report.player, TRACKED);
        assert_eq!(report.metrics["flashes"], 3);
        assert!((report.metrics["effective_rate"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-3);
        assert_eq!(report.metrics["team_flashes"], 1);
        assert_eq!(report.metrics["self_flashes"], 0);
        assert_eq!(report.metrics["conversions"], 1);
        // team_flashes (1) is not > effective flashes (1) → informational.
        assert!((report.severity - 0.5).abs() < 1e-6);
        // Evidence: worst team flash + best converted flash.
        assert_eq!(report.evidence.len(), 2);
        assert!(report.evidence.len() <= 6);
    }

    #[test]
    fn flash_report_severity_escalates_when_team_flashes_exceed_effective() {
        let data = base()
            .blind(TRACKED, 3, 1500, 2.0) // 1 effective
            .blind(TRACKED, 2, 2000, 1.5) // team flash
            .blind(TRACKED, 2, 2600, 1.5) // team flash
            .build();
        let ins = insights_of(&data);
        let report = ins
            .iter()
            .find(|i| i.detector == "D2_FLASH_EFFECTIVENESS")
            .expect("flash report");
        let cfg = DetectorConfig::default();
        assert!((report.severity - cfg.severity.h6_flash_self_or_team).abs() < 1e-6);
    }

    #[test]
    fn team_damage_aggregate_requires_two_events() {
        let one = base().hurt(TRACKED, 2, 2000, 12, "molotov").build();
        assert!(!insights_of(&one)
            .iter()
            .any(|i| i.detector == "H6_UTIL_TEAM_DAMAGE"));

        let two = base()
            .hurt(TRACKED, 2, 2000, 12, "molotov")
            .hurt(TRACKED, 2, 2200, 8, "hegrenade")
            .build();
        let ins = insights_of(&two);
        let agg = ins
            .iter()
            .find(|i| i.detector == "H6_UTIL_TEAM_DAMAGE")
            .expect("team damage aggregate");
        assert_eq!(agg.round, 0);
        assert_eq!(agg.metrics["events"], 2);
        assert_eq!(agg.metrics["total_damage"], 20);
        assert_eq!(agg.evidence.len(), 2);
    }

    #[test]
    fn unused_util_aggregate_requires_three_rounds() {
        fn multi_round(rounds_with_nades: u32) -> MatchData {
            let mut s = Scenario::new("de_test")
                .players_ct(&[1, 2])
                .players_t(&[3, 4])
                .round(1, 1000, 3000)
                .round(2, 3200, 5000)
                .round(3, 5200, 7000)
                .hold(1, 1000, 7000, 0.0, 0.0, 0.0)
                .hold(2, 1000, 7000, 200.0, 0.0, 0.0)
                .hold(3, 1000, 7000, 1000.0, 0.0, 0.0)
                .hold(4, 1000, 7000, 1200.0, 0.0, 0.0);
            for (i, end) in [3000, 5000, 7000].iter().enumerate() {
                if (i as u32) < rounds_with_nades {
                    s = s.inventory(TRACKED, *end, &["Flashbang", "Smoke Grenade"]);
                }
            }
            s.build()
        }

        assert!(!insights_of(&multi_round(2))
            .iter()
            .any(|i| i.detector == "H6_UNUSED_UTIL_AT_ROUND_END"));

        let ins = insights_of(&multi_round(3));
        let agg = ins
            .iter()
            .find(|i| i.detector == "H6_UNUSED_UTIL_AT_ROUND_END")
            .expect("unused util aggregate");
        assert_eq!(agg.round, 0);
        assert_eq!(agg.metrics["rounds"], 3);
        assert_eq!(agg.evidence.len(), 3);
    }
}
