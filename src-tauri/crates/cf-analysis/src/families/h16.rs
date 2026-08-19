//! H16 — Utility Damage Exposure (spec §2 H16, sources class 2).
//!
//! Their utility killing you outright (opposite of H3, which is *your*
//! utility making you vulnerable). Volume warning (spec: ~0.8 % of deaths):
//! class 2 is rare by nature — this family is deliberately silent-biased and
//! near-silent on most matches. That is correct behavior.

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{Category, EvidenceRef, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::Hurt;

pub struct H16UtilityDamage;

const RULE_IDS: &[&str] = &["H16_DIED_TO_UTILITY_NO_DUEL", "H16_FIRE_LINGER"];

/// Hurt-event weapon names for lingering fire damage (unprefixed, as emitted
/// by `player_hurt` — verified parser facts, plan Global Constraints). These
/// are event identities, not tunable thresholds, hence not in DetectorConfig.
const FIRE_WEAPONS: &[&str] = &["inferno", "molotov", "incgrenade"];

/// Consecutive fire hurts closer than this belong to one burn episode. Fire
/// ticks land ~0.5 s apart while standing in flames, so a gap over a second
/// means the player left the fire (episode-grouping identity, not a tunable).
const EPISODE_GAP_S: f32 = 1.0;

impl Detector for H16UtilityDamage {
    fn rule_ids(&self) -> &'static [&'static str] {
        RULE_IDS
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut flags = detect_utility_deaths(ctx, cfg);
        flags.extend(detect_fire_linger(ctx, cfg));
        flags
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        _cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        // Silent-biased (spec H16 volume warning): no match insight below two
        // occurrences — near-silence on most matches is correct behavior.
        if flags.len() < 2 {
            return vec![];
        }
        let utility_deaths = flags
            .iter()
            .filter(|f| f.rule_id == "H16_DIED_TO_UTILITY_NO_DUEL")
            .count();
        let linger: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == "H16_FIRE_LINGER")
            .collect();
        let total_fire_damage: i64 = linger
            .iter()
            .filter_map(|f| f.details["total_damage"].as_i64())
            .sum();
        let severity = flags.iter().map(|f| f.severity).fold(0.0, f32::max);
        let confidence = flags.iter().map(|f| f.confidence).fold(1.0, f32::min);
        vec![Insight {
            detector: "H16_UTILITY_EXPOSURE".to_string(),
            category: Category::Utility,
            severity,
            confidence,
            round: 0, // match-level
            player: ctx.tracked(),
            title_data: serde_json::json!({
                "utility_deaths": utility_deaths,
                "fire_linger_episodes": linger.len(),
            }),
            metrics: serde_json::json!({
                "utility_deaths": utility_deaths,
                "fire_linger_episodes": linger.len(),
                "total_fire_damage": total_fire_damage,
            }),
            evidence: flags.iter().take(8).map(|f| f.evidence.clone()).collect(),
        }]
    }
}

/// `H16_DIED_TO_UTILITY_NO_DUEL` → class 2: an enemy's grenade/fire killed
/// the tracked player who never contested — no shot fired in the prior
/// `no_shot_window_s`, no damage dealt in the prior `no_contact_window_s`.
/// Non-enemy attackers (None / self / teammate) are class-14 territory and
/// stay silent here.
fn detect_utility_deaths(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let mut flags = vec![];
    for kill in ctx.tracked_deaths() {
        if !cfg.util.utility_kill_weapons.contains(&kill.weapon) {
            continue;
        }
        let Some(attacker) = kill.attacker else {
            continue;
        };
        if attacker == kill.victim {
            continue;
        }
        // Bias to silence: unknown sides suppress, not fire.
        let (Some(att_side), Some(vic_side)) = (
            ctx.side_of(attacker, kill.round),
            ctx.side_of(kill.victim, kill.round),
        ) else {
            continue;
        };
        if att_side == vic_side {
            continue;
        }
        let shot_t0 = kill.tick - ctx.seconds(cfg.h16.no_shot_window_s);
        if !ctx.shots_by_in(kill.victim, shot_t0, kill.tick).is_empty() {
            continue;
        }
        let contact_t0 = kill.tick - ctx.seconds(cfg.h16.no_contact_window_s);
        if !ctx
            .hurts_dealt_in(kill.victim, contact_t0, kill.tick)
            .is_empty()
        {
            continue;
        }
        flags.push(RuleFlag {
            rule_id: "H16_DIED_TO_UTILITY_NO_DUEL",
            round: kill.round,
            tick: kill.tick, // death-anchored: kill tick + victim steamid
            steamid: kill.victim,
            confidence: 0.8,
            severity: cfg.severity.h16_died_to_utility_no_duel,
            details: serde_json::json!({
                "weapon": kill.weapon,
                "attacker": attacker.to_string(),
            }),
            evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim, attacker]),
        });
    }
    flags
}

/// `H16_FIRE_LINGER` (flag only, no class — fires without a death): the
/// tracked player stayed in fire. Consecutive fire hurts with gaps
/// ≤ `EPISODE_GAP_S` form one burn episode; it flags when cumulative damage
/// exceeds `fire_linger_dmg` AND the burn kept going past the first
/// `fire_linger_s` (getting out fast is fine, lingering is the mistake).
fn detect_fire_linger(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let gap = ctx.seconds(EPISODE_GAP_S);
    let min_linger = ctx.seconds(cfg.h16.fire_linger_s);
    let mut flags = vec![];
    for round in &ctx.data().rounds {
        let span_end = round.officially_ended_tick.unwrap_or(round.end_tick);
        let mut fire_hurts: Vec<&Hurt> = ctx
            .hurts_taken_in(tracked, round.start_tick, span_end)
            .into_iter()
            .copied()
            .filter(|h| FIRE_WEAPONS.contains(&h.weapon.as_str()))
            .collect();
        fire_hurts.sort_by_key(|h| h.tick);

        let mut episodes: Vec<Vec<&Hurt>> = vec![];
        for h in fire_hurts {
            match episodes.last_mut() {
                Some(ep) if h.tick - ep.last().expect("non-empty episode").tick <= gap => {
                    ep.push(h)
                }
                _ => episodes.push(vec![h]),
            }
        }

        for ep in episodes {
            let total: i32 = ep.iter().map(|h| h.dmg_health).sum();
            let first = ep.first().expect("non-empty episode").tick;
            let last = ep.last().expect("non-empty episode").tick;
            if total > cfg.h16.fire_linger_dmg && last - first >= min_linger {
                flags.push(RuleFlag {
                    rule_id: "H16_FIRE_LINGER",
                    round: round.number,
                    tick: first,
                    steamid: tracked,
                    confidence: 0.85,
                    severity: cfg.severity.h16_fire_linger,
                    details: serde_json::json!({
                        "total_damage": total,
                        "duration_s": (last - first) as f32 / ctx.data().tickrate,
                        "round": round.number,
                    }),
                    // Standard 5 s lead-in, but keep the whole episode in frame.
                    evidence: EvidenceRef {
                        round: round.number,
                        tick_start: first - ctx.seconds(5.0),
                        tick_end: last + ctx.seconds(2.0),
                        focus_players: vec![tracked],
                        camera_hint: None,
                    },
                });
            }
        }
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;

    const TRACKED: u64 = 1;

    /// CT 1 (tracked) + 2 vs T 3 + 4, round 1 ticks 1000..5000, everyone
    /// holding a position for the whole round.
    fn base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .hold(1, 1000, 5000, 0.0, 0.0, 0.0)
            .hold(2, 1000, 5000, 200.0, 0.0, 0.0)
            .hold(3, 1000, 5000, 800.0, 0.0, 0.0)
            .hold(4, 1000, 5000, 1000.0, 0.0, 0.0)
    }

    fn detect(data: &cf_parser::model::MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H16UtilityDamage.detect(&ctx, &DetectorConfig::default())
    }

    fn utility_death_flags(flags: &[RuleFlag]) -> Vec<&RuleFlag> {
        flags
            .iter()
            .filter(|f| f.rule_id == "H16_DIED_TO_UTILITY_NO_DUEL")
            .collect()
    }

    fn fire_linger_flags(flags: &[RuleFlag]) -> Vec<&RuleFlag> {
        flags
            .iter()
            .filter(|f| f.rule_id == "H16_FIRE_LINGER")
            .collect()
    }

    // --- H16_DIED_TO_UTILITY_NO_DUEL ---

    #[test]
    fn utility_death_fires_on_enemy_inferno_with_no_duel() {
        // Enemy 3's molotov kills tracked at 2000; tracked never shot nor
        // dealt damage in the whole round.
        let data = base().kill(3, TRACKED, 1, 2000, "inferno").build();
        let flags = detect(&data);
        let ud = utility_death_flags(&flags);
        assert_eq!(ud.len(), 1);
        let f = ud[0];
        // Death-anchored convention: tick = kill tick, steamid = victim.
        assert_eq!(f.tick, 2000);
        assert_eq!(f.steamid, TRACKED);
        assert_eq!(f.round, 1);
        assert!((f.confidence - 0.8).abs() < 1e-6);
        assert!(
            (f.severity
                - DetectorConfig::default()
                    .severity
                    .h16_died_to_utility_no_duel)
                .abs()
                < 1e-6
        );
        assert_eq!(f.details["weapon"], "inferno");
        assert_eq!(f.details["attacker"], "3");
        assert!(f.evidence.tick_start < 2000 && f.evidence.tick_end > 2000);
        assert!(f.evidence.focus_players.contains(&TRACKED));
        assert!(f.evidence.focus_players.contains(&3));
    }

    #[test]
    fn utility_death_suppressed_when_victim_fired_recently() {
        // Tracked fired 1 s before the molly kill — they were in a fight.
        let data = base()
            .shot(TRACKED, 1936, "weapon_ak47")
            .kill(3, TRACKED, 1, 2000, "inferno")
            .build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn utility_death_suppressed_when_victim_dealt_damage_recently() {
        // Tracked damaged an enemy 1 s before dying to the HE — contact existed.
        let data = base()
            .hurt(TRACKED, 3, 1936, 20, "ak47")
            .kill(3, TRACKED, 1, 2000, "hegrenade")
            .build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn utility_death_suppressed_for_non_utility_weapon() {
        let data = base().kill(3, TRACKED, 1, 2000, "ak47").build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn utility_death_suppressed_when_attacker_is_teammate() {
        // Teammate 2's HE kills tracked — class 14 territory, not ours.
        let data = base().kill(2, TRACKED, 1, 2000, "hegrenade").build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn utility_death_suppressed_when_attacker_missing_or_self() {
        // No attacker (own molly burned out under them, world-attributed).
        let data = base()
            .kill_full(None, TRACKED, 1, 2000, "inferno", false, 0)
            .build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
        // Self-kill.
        let data = base().kill(TRACKED, TRACKED, 1, 2000, "hegrenade").build();
        assert!(utility_death_flags(&detect(&data)).is_empty());
    }

    // --- H16_FIRE_LINGER ---

    /// 5 fire hurts x 7 dmg, 0.5 s apart → 35 dmg over 2 s: fires.
    fn burn_episode(s: Scenario, attacker: u64, first_tick: i32) -> Scenario {
        let mut s = s;
        for i in 0..5 {
            s = s.hurt(attacker, TRACKED, first_tick + i * 32, 7, "inferno");
        }
        s
    }

    #[test]
    fn fire_linger_fires_on_sustained_burn() {
        let data = burn_episode(base(), 3, 2000).build();
        let flags = detect(&data);
        let fl = fire_linger_flags(&flags);
        assert_eq!(fl.len(), 1);
        let f = fl[0];
        assert_eq!(f.tick, 2000, "flag anchored at first hurt of the episode");
        assert_eq!(f.steamid, TRACKED);
        assert_eq!(f.round, 1);
        assert!((f.confidence - 0.85).abs() < 1e-6);
        assert!((f.severity - DetectorConfig::default().severity.h16_fire_linger).abs() < 1e-6);
        assert_eq!(f.details["total_damage"], 35);
        assert!((f.details["duration_s"].as_f64().unwrap() - 2.0).abs() < 0.01);
        assert_eq!(f.details["round"], 1);
    }

    #[test]
    fn fire_linger_suppressed_when_total_damage_at_or_below_threshold() {
        // 4 hurts x 5 dmg over 1.5 s = 20, not > fire_linger_dmg (20).
        let mut s = base();
        for i in 0..4 {
            s = s.hurt(3, TRACKED, 2000 + i * 32, 5, "molotov");
        }
        assert!(fire_linger_flags(&detect(&s.build())).is_empty());
    }

    #[test]
    fn fire_linger_suppressed_when_player_got_out_fast() {
        // 30 dmg but all inside the first second (0.5 s span < fire_linger_s).
        let data = base()
            .hurt(3, TRACKED, 2000, 10, "inferno")
            .hurt(3, TRACKED, 2016, 10, "inferno")
            .hurt(3, TRACKED, 2032, 10, "inferno")
            .build();
        assert!(fire_linger_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn fire_linger_ignores_non_fire_damage() {
        // Sustained rifle damage is not a burn episode.
        let mut s = base();
        for i in 0..5 {
            s = s.hurt(3, TRACKED, 2000 + i * 32, 20, "ak47");
        }
        assert!(fire_linger_flags(&detect(&s.build())).is_empty());
    }

    #[test]
    fn fire_linger_two_separate_episodes_produce_two_flags() {
        // Episode A ends at 2128; episode B starts at 2300 (gap 172 ticks
        // > 1 s = 64 ticks) → two distinct flags.
        let data = burn_episode(burn_episode(base(), 3, 2000), 3, 2300).build();
        let flags = detect(&data);
        let fl = fire_linger_flags(&flags);
        assert_eq!(fl.len(), 2);
        assert_eq!(fl[0].tick, 2000);
        assert_eq!(fl[1].tick, 2300);
    }

    // --- insights ---

    #[test]
    fn no_insight_below_two_flags() {
        let data = base().kill(3, TRACKED, 1, 2000, "inferno").build();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = H16UtilityDamage.detect(&ctx, &cfg);
        assert_eq!(flags.len(), 1);
        assert!(H16UtilityDamage.insights(&ctx, &cfg, &flags).is_empty());
    }

    #[test]
    fn insight_fires_at_two_flags_with_metrics_and_evidence() {
        // One burn episode (2000..2128) + one utility death later (3000).
        let data = burn_episode(base(), 3, 2000)
            .kill(3, TRACKED, 1, 3000, "hegrenade")
            .build();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = H16UtilityDamage.detect(&ctx, &cfg);
        assert_eq!(flags.len(), 2);
        let insights = H16UtilityDamage.insights(&ctx, &cfg, &flags);
        assert_eq!(insights.len(), 1);
        let i = &insights[0];
        assert_eq!(i.detector, "H16_UTILITY_EXPOSURE");
        assert_eq!(i.category, Category::Utility);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        assert_eq!(i.metrics["utility_deaths"], 1);
        assert_eq!(i.metrics["fire_linger_episodes"], 1);
        assert_eq!(i.metrics["total_fire_damage"], 35);
        assert_eq!(i.evidence.len(), 2);
    }
}
