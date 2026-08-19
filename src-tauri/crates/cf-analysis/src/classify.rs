//! The death taxonomy classifier (spec §1): every tracked-player death gets
//! exactly ONE primary class by priority order, plus secondary tags for every
//! other rule that fired on it.
//!
//! Convention for death-anchored flags (families MUST follow it): the flag's
//! `tick` is the kill tick and `steamid` is the victim. The classifier joins
//! on exactly that.

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{DeathClassRow, RuleFlag};
use cf_parser::model::Kill;

/// Priority-ordered (class_id, source rule ids). Order encodes causality
/// (spec §1 "why the order is the spec") — do not reorder casually; classes
/// 8/10/11/12 are reserved for families not yet built (H1/H4-T2/H6-info/H8).
const PRIORITY: &[(u8, &[&str])] = &[
    (1, &["H3_DIED_WITH_NADE_OUT", "H3_DIED_MID_SWITCH"]),
    (2, &["H16_DIED_TO_UTILITY_NO_DUEL"]),
    (3, &["H5_DIED_FLASHED"]),
    (4, &["H3_DIED_RELOADING", "H3_DIED_SCOPED_CLOSE"]),
    (5, &["H4_KILLED_WITHOUT_CONTACT"]),
    (6, &["H2_ISOLATED_DEATH"]),
    (7, &["H2_BAITED_TRADE"]),
    (9, &["H4_CAUGHT_IN_CROSSFIRE"]),
    (11, &["H6_PUSH_WITHOUT_INFO"]),
];

pub const CLASS_OUTAIMED_FAIR: u8 = 13;
pub const CLASS_SELF_OR_WORLD: u8 = 14;
pub const CLASS_UNCLASSIFIED: u8 = 15;

/// Non-player killer weapon names seen in real demos (spec §5.4).
const WORLD_WEAPONS: &[&str] = &[
    "world",
    "worldspawn",
    "planted_c4",
    "trigger_hurt",
    "env_fire",
];

/// Class 14: self-inflicted / world / teammate — never a coaching moment,
/// classified out before everything else (spec §1 principle 4).
fn is_self_or_world(ctx: &AnalysisContext, kill: &Kill) -> bool {
    let Some(attacker) = kill.attacker else {
        return true;
    };
    if attacker == kill.victim {
        return true;
    }
    if WORLD_WEAPONS.contains(&kill.weapon.as_str()) {
        return true;
    }
    // Teammate kill.
    matches!(
        (
            ctx.side_of(attacker, kill.round),
            ctx.side_of(kill.victim, kill.round)
        ),
        (Some(a), Some(v)) if a == v
    )
}

/// Class 13 vs 15: "outaimed in a fair duel" requires evidence the victim was
/// actually in the duel — they shot, or exchanged damage with the killer,
/// shortly before dying. Otherwise it's unclassified (15), which must stay
/// near-empty on real demos (spec §1).
fn had_duel(ctx: &AnalysisContext, cfg: &DetectorConfig, kill: &Kill) -> bool {
    let w = ctx.seconds(cfg.general.fallthrough_duel_window_s);
    let t0 = kill.tick - w;
    if !ctx.shots_by_in(kill.victim, t0, kill.tick).is_empty() {
        return true;
    }
    let Some(killer) = kill.attacker else {
        return false;
    };
    ctx.hurts_dealt_in(kill.victim, t0, kill.tick)
        .iter()
        .any(|h| h.victim == killer)
}

pub fn classify_deaths(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    flags: &[RuleFlag],
) -> Vec<DeathClassRow> {
    let tracked = ctx.tracked();
    let mut out = vec![];
    for kill in ctx.tracked_deaths() {
        let fired: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.steamid == tracked && f.tick == kill.tick)
            .collect();
        let fired_ids: Vec<&str> = fired.iter().map(|f| f.rule_id).collect();

        let (class_id, class_source, confidence) = if is_self_or_world(ctx, kill) {
            (
                CLASS_SELF_OR_WORLD,
                "H14_DIED_SELF_OR_WORLD".to_string(),
                1.0,
            )
        } else if let Some((class_id, flag)) = PRIORITY.iter().find_map(|(cid, ids)| {
            fired
                .iter()
                .find(|f| ids.contains(&f.rule_id))
                .map(|f| (*cid, *f))
        }) {
            (class_id, flag.rule_id.to_string(), flag.confidence)
        } else if had_duel(ctx, cfg, kill) {
            (CLASS_OUTAIMED_FAIR, "fallthrough".to_string(), 1.0)
        } else {
            (CLASS_UNCLASSIFIED, "fallthrough".to_string(), 1.0)
        };

        let secondary_tags: Vec<String> = fired_ids
            .iter()
            .filter(|id| **id != class_source)
            .map(|id| id.to_string())
            .collect();

        out.push(DeathClassRow {
            round: kill.round,
            tick: kill.tick,
            victim: kill.victim,
            class_id,
            class_source,
            secondary_tags,
            confidence,
        });
    }
    out
}

/// Share of deaths classified 13 — the spec's CI regression metric.
pub fn class_13_share(rows: &[DeathClassRow]) -> f32 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().filter(|r| r.class_id == 13).count() as f32 / rows.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use crate::types::EvidenceRef;

    fn flag(rule_id: &'static str, tick: i32, steamid: u64, confidence: f32) -> RuleFlag {
        RuleFlag {
            rule_id,
            round: 1,
            tick,
            steamid,
            confidence,
            severity: 0.5,
            details: serde_json::json!({}),
            evidence: EvidenceRef {
                round: 1,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![steamid],
                camera_hint: None,
            },
        }
    }

    fn base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .hold(1, 1000, 3000, 0.0, 0.0, 0.0)
            .hold(3, 1000, 3000, 500.0, 0.0, 0.0)
    }

    #[test]
    fn priority_causality_molly_kill_beats_isolation() {
        let data = base().kill(3, 1, 1, 2000, "inferno").build();
        let ctx = AnalysisContext::new(&data, 1);
        let cfg = DetectorConfig::default();
        let flags = vec![
            flag("H16_DIED_TO_UTILITY_NO_DUEL", 2000, 1, 0.8),
            flag("H2_ISOLATED_DEATH", 2000, 1, 0.75),
        ];
        let rows = classify_deaths(&ctx, &cfg, &flags);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].class_id, 2);
        assert_eq!(rows[0].class_source, "H16_DIED_TO_UTILITY_NO_DUEL");
        assert_eq!(rows[0].secondary_tags, vec!["H2_ISOLATED_DEATH"]);
        assert!((rows[0].confidence - 0.8).abs() < 0.01);
    }

    #[test]
    fn class_14_wins_over_everything() {
        // Self-kill with own HE while an isolation flag also fired.
        let data = base().kill(1, 1, 1, 2000, "hegrenade").build();
        let ctx = AnalysisContext::new(&data, 1);
        let rows = classify_deaths(
            &ctx,
            &DetectorConfig::default(),
            &[flag("H2_ISOLATED_DEATH", 2000, 1, 0.75)],
        );
        assert_eq!(rows[0].class_id, 14);
        assert_eq!(rows[0].class_source, "H14_DIED_SELF_OR_WORLD");
    }

    #[test]
    fn teammate_and_world_kills_are_class_14() {
        let team = base().kill(2, 1, 1, 2000, "m4a1").build();
        let ctx = AnalysisContext::new(&team, 1);
        assert_eq!(
            classify_deaths(&ctx, &DetectorConfig::default(), &[])[0].class_id,
            14
        );
        let world = base()
            .kill_full(None, 1, 1, 2000, "world", false, 0)
            .build();
        let ctx = AnalysisContext::new(&world, 1);
        assert_eq!(
            classify_deaths(&ctx, &DetectorConfig::default(), &[])[0].class_id,
            14
        );
    }

    #[test]
    fn fallthrough_splits_13_fair_duel_from_15_unclassified() {
        // Victim shot back just before dying → 13.
        let dueling = base()
            .shot(1, 1950, "weapon_ak47")
            .kill(3, 1, 1, 2000, "ak47")
            .build();
        let ctx = AnalysisContext::new(&dueling, 1);
        let rows = classify_deaths(&ctx, &DetectorConfig::default(), &[]);
        assert_eq!(rows[0].class_id, 13);

        // No shots, no damage exchange, no rule fired → 15.
        let silent = base().kill(3, 1, 1, 2000, "ak47").build();
        let ctx = AnalysisContext::new(&silent, 1);
        let rows = classify_deaths(&ctx, &DetectorConfig::default(), &[]);
        assert_eq!(rows[0].class_id, 15);
    }

    #[test]
    fn damage_dealt_to_killer_also_counts_as_duel() {
        let data = base()
            .hurt(1, 3, 1950, 20, "ak47")
            .kill(3, 1, 1, 2000, "ak47")
            .build();
        let ctx = AnalysisContext::new(&data, 1);
        let rows = classify_deaths(&ctx, &DetectorConfig::default(), &[]);
        assert_eq!(rows[0].class_id, 13);
    }

    #[test]
    fn push_without_info_classifies_as_11_below_crossfire() {
        let data = base().kill(3, 1, 1, 2000, "ak47").build();
        let ctx = AnalysisContext::new(&data, 1);
        let cfg = DetectorConfig::default();
        // Crossfire (class 9) outranks pushed-without-info (class 11).
        let both = vec![
            flag("H4_CAUGHT_IN_CROSSFIRE", 2000, 1, 0.8),
            flag("H6_PUSH_WITHOUT_INFO", 2000, 1, 0.6),
        ];
        let rows = classify_deaths(&ctx, &cfg, &both);
        assert_eq!(rows[0].class_id, 9);
        assert_eq!(rows[0].secondary_tags, vec!["H6_PUSH_WITHOUT_INFO"]);
        // Alone it sources class 11.
        let alone = vec![flag("H6_PUSH_WITHOUT_INFO", 2000, 1, 0.6)];
        let rows = classify_deaths(&ctx, &cfg, &alone);
        assert_eq!(rows[0].class_id, 11);
        assert_eq!(rows[0].class_source, "H6_PUSH_WITHOUT_INFO");
    }

    #[test]
    fn class_13_share_metric() {
        let rows = vec![
            DeathClassRow {
                round: 1,
                tick: 1,
                victim: 1,
                class_id: 13,
                class_source: "fallthrough".into(),
                secondary_tags: vec![],
                confidence: 1.0,
            },
            DeathClassRow {
                round: 1,
                tick: 2,
                victim: 1,
                class_id: 6,
                class_source: "H2_ISOLATED_DEATH".into(),
                secondary_tags: vec![],
                confidence: 0.75,
            },
        ];
        assert!((class_13_share(&rows) - 0.5).abs() < 0.01);
    }
}
