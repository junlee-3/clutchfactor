//! H4 Tier-1 — Peeking Geometry & Exposure (spec §2 H4, Tier 1 only).
//!
//! `H4_KILLED_WITHOUT_CONTACT` (class 5): died without ever being in the
//! fight — shot through smoke, wallbanged, or killed by an enemy the victim
//! never exchanged anything with. Caption care per spec: this is "you stood
//! in a line someone pre-fires for free", not "you were outplayed".
//! `H4_CAUGHT_IN_CROSSFIRE` (class 9): mid-duel with enemy A, killed by a
//! second enemy B from a clearly different direction.

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::{AnalysisContext, PlayerState};
use crate::types::{Category, EvidenceRef, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::Kill;

pub struct H4Exposure;

const KILLED_WITHOUT_CONTACT: &str = "H4_KILLED_WITHOUT_CONTACT";
const CAUGHT_IN_CROSSFIRE: &str = "H4_CAUGHT_IN_CROSSFIRE";

/// Event-exact signals (`thru_smoke`, `penetrated`) — the class-5 volume core
/// per spec §5.4.
const CONF_EVENT_EXACT: f32 = 0.95;
/// Inferred "never in contact" — no spotted-flag data in MVP, so low.
const CONF_NO_CONTACT: f32 = 0.6;
const CONF_CROSSFIRE: f32 = 0.8;
const INSIGHT_MIN_OCCURRENCES: usize = 2;
const INSIGHT_EVIDENCE_CAP: usize = 8;

impl Detector for H4Exposure {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[KILLED_WITHOUT_CONTACT, CAUGHT_IN_CROSSFIRE]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut out = vec![];
        for kill in ctx.tracked_deaths() {
            out.extend(killed_without_contact(ctx, cfg, kill));
            out.extend(caught_in_crossfire(ctx, cfg, kill));
        }
        out
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let mut out = vec![];

        let kwc: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == KILLED_WITHOUT_CONTACT)
            .collect();
        let variant_is = |f: &RuleFlag, v: &str| f.details["variant"] == v;
        let smoke = kwc.iter().filter(|f| variant_is(f, "smoke")).count();
        let wallbang = kwc.iter().filter(|f| variant_is(f, "wallbang")).count();
        let no_contact = kwc.iter().filter(|f| variant_is(f, "no_contact")).count();
        // Gate on the event-exact variants only — the inferred no-contact
        // variant is too weak to promote to a match-level insight by itself.
        if smoke + wallbang >= INSIGHT_MIN_OCCURRENCES {
            // Event-exact evidence first, inferred after, capped.
            let mut evidence: Vec<EvidenceRef> = kwc
                .iter()
                .filter(|f| !variant_is(f, "no_contact"))
                .chain(kwc.iter().filter(|f| variant_is(f, "no_contact")))
                .map(|f| f.evidence.clone())
                .collect();
            evidence.truncate(INSIGHT_EVIDENCE_CAP);
            out.push(Insight {
                detector: KILLED_WITHOUT_CONTACT.to_string(),
                category: Category::Positioning,
                severity: cfg.severity.h4_killed_without_contact,
                confidence: CONF_EVENT_EXACT,
                round: 0,
                player: ctx.tracked(),
                title_data: json!({
                    "smoke_deaths": smoke,
                    "wallbang_deaths": wallbang,
                }),
                metrics: json!({
                    "smoke_deaths": smoke,
                    "wallbang_deaths": wallbang,
                    "no_contact_deaths": no_contact,
                    "total_deaths": ctx.tracked_deaths().len(),
                }),
                evidence,
            });
        }

        let crossfire: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == CAUGHT_IN_CROSSFIRE)
            .collect();
        if crossfire.len() >= INSIGHT_MIN_OCCURRENCES {
            out.push(Insight {
                detector: CAUGHT_IN_CROSSFIRE.to_string(),
                category: Category::Positioning,
                severity: cfg.severity.h4_caught_in_crossfire,
                confidence: CONF_CROSSFIRE,
                round: 0,
                player: ctx.tracked(),
                title_data: json!({ "count": crossfire.len() }),
                metrics: json!({ "count": crossfire.len() }),
                evidence: crossfire
                    .iter()
                    .take(INSIGHT_EVIDENCE_CAP)
                    .map(|f| f.evidence.clone())
                    .collect(),
            });
        }

        out
    }
}

/// The killer, if there is one and they are on the enemy side. Missing side
/// data (e.g. warmup kills outside any round) → None, bias to silence.
fn enemy_killer(ctx: &AnalysisContext, kill: &Kill) -> Option<u64> {
    let killer = kill.attacker?;
    if killer == kill.victim {
        return None;
    }
    let killer_side = ctx.side_of(killer, kill.round)?;
    let victim_side = ctx.side_of(kill.victim, kill.round)?;
    (killer_side != victim_side).then_some(killer)
}

fn killed_without_contact(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    kill: &Kill,
) -> Option<RuleFlag> {
    let killer = enemy_killer(ctx, kill)?;
    // Variant (a) — event-exact, wins when both apply.
    let (variant, confidence) = if kill.thru_smoke {
        ("smoke", CONF_EVENT_EXACT)
    } else if kill.penetrated > 0 {
        ("wallbang", CONF_EVENT_EXACT)
    } else {
        // Variant (b) — inferred: the victim was never in the exchange.
        let t0 = kill.tick - ctx.seconds(cfg.h4.contactless_window_s);
        let fired = !ctx.shots_by_in(kill.victim, t0, kill.tick).is_empty();
        // Damage from the killer strictly before the killing moment means the
        // victim was IN the duel; the killing blow itself lands at the death
        // tick and must not count.
        let hit_by_killer_earlier = ctx
            .hurts_taken_in(kill.victim, t0, kill.tick)
            .iter()
            .any(|h| h.attacker == Some(killer) && h.tick != kill.tick);
        let dealt_any = !ctx.hurts_dealt_in(kill.victim, t0, kill.tick).is_empty();
        if fired || hit_by_killer_earlier || dealt_any {
            return None;
        }
        ("no_contact", CONF_NO_CONTACT)
    };
    Some(RuleFlag {
        rule_id: KILLED_WITHOUT_CONTACT,
        round: kill.round,
        tick: kill.tick,
        steamid: kill.victim,
        confidence,
        severity: cfg.severity.h4_killed_without_contact,
        details: json!({ "variant": variant, "weapon": kill.weapon }),
        evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim, killer]),
    })
}

fn caught_in_crossfire(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    kill: &Kill,
) -> Option<RuleFlag> {
    let killer = enemy_killer(ctx, kill)?;
    let killer_state = ctx.state_at(killer, kill.tick)?;
    if !killer_state.is_alive {
        return None;
    }
    let victim_state = ctx.state_at(kill.victim, kill.tick)?;
    let victim_side = ctx.side_of(kill.victim, kill.round)?;

    // Enemies the victim exchanged damage with (either direction) in the
    // engage window before death.
    let t0 = kill.tick - ctx.seconds(cfg.h4.crossfire_engage_window_s);
    let mut candidates: Vec<u64> = ctx
        .hurts_dealt_in(kill.victim, t0, kill.tick)
        .iter()
        .map(|h| h.victim)
        .chain(
            ctx.hurts_taken_in(kill.victim, t0, kill.tick)
                .iter()
                .filter_map(|h| h.attacker),
        )
        .collect();
    candidates.sort_unstable();
    candidates.dedup();

    // Pick the qualifying A with the widest angle — the clearest crossfire.
    let mut best: Option<(u64, f32)> = None;
    for a in candidates {
        if a == killer || a == kill.victim {
            continue;
        }
        if !ctx.side_of(a, kill.round).is_some_and(|s| s != victim_side) {
            continue;
        }
        let Some(a_state) = ctx.state_at(a, kill.tick) else {
            continue;
        };
        if !a_state.is_alive {
            continue;
        }
        let Some(angle) = xy_angle_deg(&victim_state, &a_state, &killer_state) else {
            continue;
        };
        if angle > cfg.h4.crossfire_min_angle_deg && best.is_none_or(|(_, b)| angle > b) {
            best = Some((a, angle));
        }
    }
    let (engaged, angle_deg) = best?;
    Some(RuleFlag {
        rule_id: CAUGHT_IN_CROSSFIRE,
        round: kill.round,
        tick: kill.tick,
        steamid: kill.victim,
        confidence: CONF_CROSSFIRE,
        severity: cfg.severity.h4_caught_in_crossfire,
        details: json!({
            "engaged_enemy": engaged.to_string(),
            "killer": killer.to_string(),
            "angle_deg": angle_deg,
        }),
        evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim, killer, engaged]),
    })
}

/// Angle A→victim→B in the XY plane, degrees. None when either arm is
/// degenerate (coincident positions).
fn xy_angle_deg(victim: &PlayerState, a: &PlayerState, b: &PlayerState) -> Option<f32> {
    let (ax, ay) = (a.x - victim.x, a.y - victim.y);
    let (bx, by) = (b.x - victim.x, b.y - victim.y);
    let na = (ax * ax + ay * ay).sqrt();
    let nb = (bx * bx + by * by).sqrt();
    if na < 1.0 || nb < 1.0 {
        return None;
    }
    let cos = ((ax * bx + ay * by) / (na * nb)).clamp(-1.0, 1.0);
    Some(cos.acos().to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;

    const VICTIM: u64 = 1;
    const MATE: u64 = 2;
    const ENEMY_A: u64 = 3; // due north of the victim
    const ENEMY_B: u64 = 4; // due east of the victim
    const ENEMY_C: u64 = 5; // almost the same direction as B

    const DEATH: i32 = 2000;

    /// Everyone held in place across round 1 (tickrate 64; 2 s window = 128
    /// ticks, so [1872, 2000] before the death at 2000).
    fn base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000)
            .hold(VICTIM, 1000, 3000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 3000, -500.0, -500.0, 0.0)
            .hold(ENEMY_A, 1000, 3000, 0.0, 1000.0, 0.0)
            .hold(ENEMY_B, 1000, 3000, 1000.0, 0.0, 0.0)
            .hold(ENEMY_C, 1000, 3000, 1000.0, 100.0, 0.0)
    }

    fn detect(data: &cf_parser::model::MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, VICTIM);
        H4Exposure.detect(&ctx, &DetectorConfig::default())
    }

    fn kwc_flags(flags: &[RuleFlag]) -> Vec<&RuleFlag> {
        flags
            .iter()
            .filter(|f| f.rule_id == KILLED_WITHOUT_CONTACT)
            .collect()
    }

    fn crossfire_flags(flags: &[RuleFlag]) -> Vec<&RuleFlag> {
        flags
            .iter()
            .filter(|f| f.rule_id == CAUGHT_IN_CROSSFIRE)
            .collect()
    }

    // ---- H4_KILLED_WITHOUT_CONTACT ----

    #[test]
    fn thru_smoke_kill_fires_smoke_variant_at_high_confidence() {
        let data = base()
            .kill_full(Some(ENEMY_A), VICTIM, 1, DEATH, "ak47", true, 0)
            .build();
        let flags = detect(&data);
        let kwc = kwc_flags(&flags);
        assert_eq!(kwc.len(), 1);
        let f = kwc[0];
        assert_eq!(f.tick, DEATH);
        assert_eq!(f.steamid, VICTIM);
        assert_eq!(f.round, 1);
        assert!((f.confidence - 0.95).abs() < 1e-6);
        assert_eq!(f.details["variant"], "smoke");
        assert_eq!(f.details["weapon"], "ak47");
        assert_eq!(f.evidence.focus_players, vec![VICTIM, ENEMY_A]);
    }

    #[test]
    fn penetrated_kill_fires_wallbang_variant() {
        let data = base()
            .kill_full(Some(ENEMY_A), VICTIM, 1, DEATH, "awp", false, 2)
            .build();
        let flags = detect(&data);
        let kwc = kwc_flags(&flags);
        assert_eq!(kwc.len(), 1);
        assert_eq!(kwc[0].details["variant"], "wallbang");
        assert!((kwc[0].confidence - 0.95).abs() < 1e-6);
    }

    #[test]
    fn smoke_variant_wins_when_both_smoke_and_penetration_set() {
        let data = base()
            .kill_full(Some(ENEMY_A), VICTIM, 1, DEATH, "ak47", true, 1)
            .build();
        let flags = detect(&data);
        let kwc = kwc_flags(&flags);
        assert_eq!(kwc.len(), 1, "exactly one flag per death");
        assert_eq!(kwc[0].details["variant"], "smoke");
    }

    #[test]
    fn contactless_fires_no_contact_variant_at_low_confidence() {
        // Victim silent; the killing blow's hurt at the death tick itself
        // must not count as prior contact.
        let data = base()
            .hurt(ENEMY_A, VICTIM, DEATH, 100, "ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        let flags = detect(&data);
        let kwc = kwc_flags(&flags);
        assert_eq!(kwc.len(), 1);
        let f = kwc[0];
        assert!((f.confidence - 0.6).abs() < 1e-6);
        assert_eq!(f.details["variant"], "no_contact");
        assert_eq!(f.details["weapon"], "ak47");
        assert_eq!(f.evidence.focus_players, vec![VICTIM, ENEMY_A]);
    }

    #[test]
    fn contactless_suppressed_when_victim_fired_within_window() {
        let data = base()
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(kwc_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn contactless_still_fires_when_shot_was_before_window() {
        // 2 s window at 64 tick = [1872, 2000]; a shot at 1800 is outside.
        let data = base()
            .shot(VICTIM, 1800, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        let flags = detect(&data);
        assert_eq!(kwc_flags(&flags).len(), 1);
    }

    #[test]
    fn contactless_suppressed_when_killer_damaged_victim_earlier_in_window() {
        // They were IN the duel — took damage from the killer before dying.
        let data = base()
            .hurt(ENEMY_A, VICTIM, 1950, 20, "ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(kwc_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn contactless_suppressed_when_victim_dealt_damage_in_window() {
        let data = base()
            .hurt(VICTIM, ENEMY_B, 1950, 30, "ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(kwc_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn killed_without_contact_requires_live_enemy_killer() {
        // Teammate kill → not this rule's business (class 14 territory).
        let team = base().kill(MATE, VICTIM, 1, DEATH, "m4a1").build();
        assert!(kwc_flags(&detect(&team)).is_empty());
        // No attacker at all.
        let world = base()
            .kill_full(None, VICTIM, 1, DEATH, "world", false, 0)
            .build();
        assert!(kwc_flags(&detect(&world)).is_empty());
    }

    // ---- H4_CAUGHT_IN_CROSSFIRE ----

    #[test]
    fn crossfire_fires_when_engaged_enemy_and_killer_are_wide_apart() {
        // Victim damaged A (north), then B (east) killed them: 90° apart.
        let data = base()
            .hurt(VICTIM, ENEMY_A, 1950, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, DEATH, "ak47")
            .build();
        let flags = detect(&data);
        let cf = crossfire_flags(&flags);
        assert_eq!(cf.len(), 1);
        let f = cf[0];
        assert_eq!(f.tick, DEATH);
        assert_eq!(f.steamid, VICTIM);
        assert!((f.confidence - 0.8).abs() < 1e-6);
        assert_eq!(f.details["engaged_enemy"], ENEMY_A.to_string());
        assert_eq!(f.details["killer"], ENEMY_B.to_string());
        let angle = f.details["angle_deg"].as_f64().unwrap();
        assert!((angle - 90.0).abs() < 1.0, "angle was {angle}");
        assert_eq!(f.evidence.focus_players, vec![VICTIM, ENEMY_B, ENEMY_A]);
    }

    #[test]
    fn crossfire_counts_damage_taken_from_the_engaged_enemy_too() {
        let data = base()
            .hurt(ENEMY_A, VICTIM, 1950, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, DEATH, "ak47")
            .build();
        let cf_all = detect(&data);
        let cf = crossfire_flags(&cf_all);
        assert_eq!(cf.len(), 1);
        assert_eq!(cf[0].details["engaged_enemy"], ENEMY_A.to_string());
    }

    #[test]
    fn crossfire_suppressed_when_angle_below_threshold() {
        // C (1000, 100) and B (1000, 0) are ~5.7° apart from the victim.
        let data = base()
            .hurt(VICTIM, ENEMY_C, 1950, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(crossfire_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn crossfire_suppressed_when_engaged_enemy_is_the_killer() {
        let data = base()
            .hurt(VICTIM, ENEMY_B, 1950, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(crossfire_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn crossfire_suppressed_when_engaged_enemy_dead_at_death_tick() {
        let data = base()
            .hurt(VICTIM, ENEMY_A, 1950, 25, "ak47")
            .waypoint_full(
                ENEMY_A,
                1990,
                0.0,
                1000.0,
                0.0,
                0.0,
                0,
                false,
                Some("weapon_ak47"),
                None,
                false,
            )
            .kill(ENEMY_B, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(crossfire_flags(&detect(&data)).is_empty());
    }

    // ---- insights ----

    /// Two rounds with everyone held in place in each; the tracked player
    /// dies once per round.
    fn two_round_base() -> Scenario {
        let mut s = Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000)
            .round(2, 6000, 10000);
        for (t0, t1) in [(1000, 3000), (6000, 8000)] {
            s = s
                .hold(VICTIM, t0, t1, 0.0, 0.0, 0.0)
                .hold(MATE, t0, t1, -500.0, -500.0, 0.0)
                .hold(ENEMY_A, t0, t1, 0.0, 1000.0, 0.0)
                .hold(ENEMY_B, t0, t1, 1000.0, 0.0, 0.0)
                .hold(ENEMY_C, t0, t1, 1000.0, 100.0, 0.0);
        }
        s
    }

    fn run_insights(data: &cf_parser::model::MatchData) -> Vec<Insight> {
        let ctx = AnalysisContext::new(data, VICTIM);
        let cfg = DetectorConfig::default();
        let flags = H4Exposure.detect(&ctx, &cfg);
        H4Exposure.insights(&ctx, &cfg, &flags)
    }

    #[test]
    fn without_contact_insight_fires_at_two_event_exact_deaths() {
        let data = two_round_base()
            .kill_full(Some(ENEMY_A), VICTIM, 1, 2500, "ak47", true, 0)
            .kill_full(Some(ENEMY_B), VICTIM, 2, 7000, "awp", false, 3)
            .build();
        let insights = run_insights(&data);
        assert_eq!(insights.len(), 1);
        let i = &insights[0];
        assert_eq!(i.detector, KILLED_WITHOUT_CONTACT);
        assert_eq!(i.category, Category::Positioning);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, VICTIM);
        assert_eq!(i.metrics["smoke_deaths"], 1);
        assert_eq!(i.metrics["wallbang_deaths"], 1);
        assert_eq!(i.metrics["no_contact_deaths"], 0);
        assert_eq!(i.metrics["total_deaths"], 2);
        assert_eq!(i.evidence.len(), 2);
    }

    #[test]
    fn without_contact_insight_not_gated_open_by_inferred_variant() {
        // One smoke death + one inferred no-contact death: the no-contact
        // variant doesn't count toward the ≥2 event-exact gate.
        let data = two_round_base()
            .kill_full(Some(ENEMY_A), VICTIM, 1, 2500, "ak47", true, 0)
            .kill(ENEMY_B, VICTIM, 2, 7000, "ak47")
            .build();
        let ctx = AnalysisContext::new(&data, VICTIM);
        let cfg = DetectorConfig::default();
        let flags = H4Exposure.detect(&ctx, &cfg);
        assert_eq!(kwc_flags(&flags).len(), 2, "both deaths flagged");
        assert!(H4Exposure.insights(&ctx, &cfg, &flags).is_empty());
    }

    #[test]
    fn crossfire_insight_fires_at_two_occurrences() {
        let data = two_round_base()
            .hurt(VICTIM, ENEMY_A, 2450, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, 2500, "ak47")
            .hurt(VICTIM, ENEMY_A, 6950, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 2, 7000, "ak47")
            .build();
        let insights = run_insights(&data);
        let cf: Vec<&Insight> = insights
            .iter()
            .filter(|i| i.detector == CAUGHT_IN_CROSSFIRE)
            .collect();
        assert_eq!(cf.len(), 1);
        assert_eq!(cf[0].category, Category::Positioning);
        assert_eq!(cf[0].metrics["count"], 2);
        assert_eq!(cf[0].evidence.len(), 2);
    }

    #[test]
    fn crossfire_insight_suppressed_at_one_occurrence() {
        let data = two_round_base()
            .hurt(VICTIM, ENEMY_A, 2450, 25, "ak47")
            .kill(ENEMY_B, VICTIM, 1, 2500, "ak47")
            .build();
        let insights = run_insights(&data);
        assert!(!insights.iter().any(|i| i.detector == CAUGHT_IN_CROSSFIRE));
    }
}
