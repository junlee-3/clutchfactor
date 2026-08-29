//! H4 Tier-1 — Peeking Geometry & Exposure (spec §2 H4, Tier 1 only).
//!
//! `H4_KILLED_WITHOUT_CONTACT` (class 5): died without ever being in the
//! fight — shot through smoke, wallbanged, or killed by an enemy the victim
//! never exchanged anything with. Caption care per spec: this is "you stood
//! in a line someone pre-fires for free", not "you were outplayed".
//! `H4_CAUGHT_IN_CROSSFIRE` (class 9): mid-duel with enemy A, killed by a
//! second enemy B from a clearly different direction.
//!
//! Tier 2 (kinematic, no geometry) — `H4_WIDE_PEEK_HELD_ANGLE` (class 10):
//! "whoever is further from the corner sees the other first" needs raycasts
//! we do not have, so the proxy is movement: the tracked player covered
//! ground toward an enemy who was holding still, engaged them, and lost.
//! Approximation → confidence capped at 0.6 (spec §4.2).

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::{AnalysisContext, PlayerState};
use crate::types::{Category, EvidenceRef, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::Kill;

pub struct H4Exposure;

const KILLED_WITHOUT_CONTACT: &str = "H4_KILLED_WITHOUT_CONTACT";
const CAUGHT_IN_CROSSFIRE: &str = "H4_CAUGHT_IN_CROSSFIRE";
const WIDE_PEEK_HELD_ANGLE: &str = "H4_WIDE_PEEK_HELD_ANGLE";

/// Event-exact signals (`thru_smoke`, `penetrated`) — the class-5 volume core
/// per spec §5.4.
const CONF_EVENT_EXACT: f32 = 0.95;
/// Inferred "never in contact" — no spotted-flag data in MVP, so low.
const CONF_NO_CONTACT: f32 = 0.6;
const CONF_CROSSFIRE: f32 = 0.8;
/// Kinematic swing-vs-hold proxy, no geometry behind it (spec §4.2).
const CONF_WIDE_PEEK: f32 = 0.6;
/// Movement is sampled at 4 Hz across the peek window. The tick table is
/// ~16 Hz, so this reads one sample in four: a jiggle-peek shorter than
/// 250 ms can fall between samples and read as a player standing still.
/// Deliberate (the plan fixes the step) — widen it here, not per rule.
const SAMPLE_STEP_S: f32 = 0.25;
const INSIGHT_MIN_OCCURRENCES: usize = 2;
const INSIGHT_EVIDENCE_CAP: usize = 8;

impl Detector for H4Exposure {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[
            KILLED_WITHOUT_CONTACT,
            CAUGHT_IN_CROSSFIRE,
            WIDE_PEEK_HELD_ANGLE,
        ]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut out = vec![];
        for kill in ctx.tracked_deaths() {
            out.extend(killed_without_contact(ctx, cfg, kill));
            out.extend(caught_in_crossfire(ctx, cfg, kill));
            out.extend(wide_peek_held_angle(ctx, cfg, kill));
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

        let wide: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == WIDE_PEEK_HELD_ANGLE)
            .collect();
        if wide.len() >= INSIGHT_MIN_OCCURRENCES {
            out.push(Insight {
                detector: WIDE_PEEK_HELD_ANGLE.to_string(),
                category: Category::Positioning,
                severity: cfg.severity.h4_wide_peek_held_angle,
                confidence: CONF_WIDE_PEEK,
                round: 0,
                player: ctx.tracked(),
                title_data: json!({ "count": wide.len() }),
                metrics: json!({ "count": wide.len() }),
                evidence: wide
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

/// Tier 2, class 10: the tracked player swung into a held angle and lost
/// the duel. Kinematic only — the victim covered ground toward a killer who
/// barely moved, the gap shrank, shots were traded, and it happened at a
/// distance where holding an angle is what actually beats you.
fn wide_peek_held_angle(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    kill: &Kill,
) -> Option<RuleFlag> {
    // Through smoke or through a wall was never an angle duel — class 5.
    if kill.thru_smoke || kill.penetrated > 0 {
        return None;
    }
    let killer = enemy_killer(ctx, kill)?;
    let killer_state = ctx.state_at(killer, kill.tick)?;
    if !killer_state.is_alive {
        return None;
    }
    let victim_state = ctx.state_at(kill.victim, kill.tick)?;

    let z = cfg.general.z_weight;
    let t0 = kill.tick - ctx.seconds(cfg.h4.peek_window_s);
    let step = ctx.seconds(SAMPLE_STEP_S);
    let victim_track = ctx.samples_in(kill.victim, t0, kill.tick, step)?;
    let killer_track = ctx.samples_in(killer, t0, kill.tick, step)?;
    let exposed_u = AnalysisContext::path_length(&victim_track, z);
    let killer_moved_u = AnalysisContext::path_length(&killer_track, z);
    if exposed_u < cfg.h4.exposure_min_u || killer_moved_u > cfg.h4.holder_max_u {
        return None;
    }
    let distance = AnalysisContext::dist(&victim_state, &killer_state, z);
    // The swing has to be *at* them.
    if AnalysisContext::dist(victim_track.first()?, killer_track.first()?, z) <= distance {
        return None;
    }
    // Point-blank is a scramble, not an angle duel.
    if distance < cfg.h4.wide_peek_min_dist_u {
        return None;
    }

    // A duel has to have happened: no engagement is class 5's or 13's story.
    let shots = ctx.shots_by_in(kill.victim, t0, kill.tick).len();
    let hit_the_killer = ctx
        .hurts_dealt_in(kill.victim, t0, kill.tick)
        .iter()
        .any(|h| h.victim == killer);
    if shots == 0 && !hit_the_killer {
        return None;
    }

    Some(RuleFlag {
        rule_id: WIDE_PEEK_HELD_ANGLE,
        round: kill.round,
        tick: kill.tick,
        steamid: kill.victim,
        confidence: CONF_WIDE_PEEK,
        severity: cfg.severity.h4_wide_peek_held_angle,
        details: json!({
            "exposed_u": exposed_u,
            "killer_moved_u": killer_moved_u,
            "distance": distance,
            "shots": shots,
            "place": victim_state.place,
            "killer_place": killer_state.place,
            "killer": killer.to_string(),
        }),
        evidence: evidence_around(ctx, kill.round, kill.tick, &[kill.victim, killer]),
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

    // ---- H4_WIDE_PEEK_HELD_ANGLE ----

    /// 1.5 s of peek window at 64 tick.
    const PEEK_WINDOW: i32 = 96;

    fn wide_peek_flags(flags: &[RuleFlag]) -> Vec<&RuleFlag> {
        flags
            .iter()
            .filter(|f| f.rule_id == WIDE_PEEK_HELD_ANGLE)
            .collect()
    }

    fn at(s: Scenario, sid: u64, tick: i32, x: f32, y: f32, place: &str) -> Scenario {
        s.waypoint_full(
            sid,
            tick,
            x,
            y,
            0.0,
            0.0,
            100,
            true,
            Some("weapon_ak47"),
            Some(place),
            false,
        )
    }

    /// The victim swings from the origin to `victim_to` across the peek
    /// window; the killer sits at `killer_x` and drifts `killer_travel`.
    /// Everyone else is parked far away.
    fn swing(victim_to: (f32, f32), killer_x: f32, killer_travel: f32) -> Scenario {
        let s = Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000)
            .hold(MATE, 1000, 3000, -5000.0, -5000.0, 0.0)
            .hold(ENEMY_B, 1000, 3000, 9000.0, 0.0, 0.0)
            .hold(ENEMY_C, 1000, 3000, 9000.0, 100.0, 0.0);
        let s = at(s, VICTIM, 1000, 0.0, 0.0, "Palace");
        let s = at(s, VICTIM, DEATH - PEEK_WINDOW, 0.0, 0.0, "Palace");
        let s = at(s, VICTIM, DEATH, victim_to.0, victim_to.1, "Palace");
        let s = at(s, ENEMY_A, 1000, killer_x, 0.0, "Jungle");
        let s = at(s, ENEMY_A, DEATH - PEEK_WINDOW, killer_x, 0.0, "Jungle");
        at(s, ENEMY_A, DEATH, killer_x + killer_travel, 0.0, "Jungle")
    }

    /// The target case: 200 u of swing into a killer holding 800 u away,
    /// with one shot fired on the way in.
    fn target_swing() -> Scenario {
        swing((200.0, 0.0), 1000.0, 0.0).shot(VICTIM, 1950, "weapon_ak47")
    }

    #[test]
    fn wide_peek_fires_when_the_victim_swung_into_a_holder() {
        let data = target_swing()
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        let flags = detect(&data);
        let wide = wide_peek_flags(&flags);
        assert_eq!(wide.len(), 1);
        let f = wide[0];
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, DEATH, "death-anchored: flag tick = kill tick");
        assert_eq!(f.steamid, VICTIM);
        assert!((f.confidence - 0.6).abs() < 1e-6, "kinematic proxy");
        assert_eq!(
            f.severity,
            DetectorConfig::default().severity.h4_wide_peek_held_angle
        );
        assert!((f.details["exposed_u"].as_f64().unwrap() - 200.0).abs() < 1.0);
        assert!(f.details["killer_moved_u"].as_f64().unwrap() < 1.0);
        assert!((f.details["distance"].as_f64().unwrap() - 800.0).abs() < 1.0);
        assert_eq!(f.details["shots"], 1);
        assert_eq!(f.details["place"], "Palace");
        assert_eq!(f.details["killer_place"], "Jungle");
        assert_eq!(f.details["killer"], ENEMY_A.to_string());
        assert_eq!(f.evidence.focus_players, vec![VICTIM, ENEMY_A]);
    }

    #[test]
    fn wide_peek_silent_when_the_victim_barely_moved() {
        let data = swing((50.0, 0.0), 1000.0, 0.0)
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_silent_when_the_killer_was_moving_too() {
        // 200 u from the killer: they were not holding an angle.
        let data = swing((200.0, 0.0), 1000.0, -200.0)
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_silent_when_the_victim_moved_away_from_the_killer() {
        let data = swing((-200.0, 0.0), 1000.0, 0.0)
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_silent_without_an_engagement() {
        // No shot and no damage: that death is class 5's or 13's to judge.
        let data = swing((200.0, 0.0), 1000.0, 0.0)
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_fires_on_damage_dealt_to_the_killer_without_a_shot_event() {
        let data = swing((200.0, 0.0), 1000.0, 0.0)
            .hurt(VICTIM, ENEMY_A, 1950, 30, "ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        let flags = detect(&data);
        let wide = wide_peek_flags(&flags);
        assert_eq!(wide.len(), 1);
        assert_eq!(wide[0].details["shots"], 0);
    }

    #[test]
    fn wide_peek_silent_at_point_blank_range() {
        // 100 u apart at the death is a scramble, not an angle duel.
        let data = swing((200.0, 0.0), 300.0, 0.0)
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_silent_through_smoke_or_a_wall() {
        for (thru_smoke, penetrated) in [(true, 0), (false, 2)] {
            let data = target_swing()
                .kill_full(
                    Some(ENEMY_A),
                    VICTIM,
                    1,
                    DEATH,
                    "ak47",
                    thru_smoke,
                    penetrated,
                )
                .build();
            assert!(
                wide_peek_flags(&detect(&data)).is_empty(),
                "thru_smoke {thru_smoke} / penetrated {penetrated} is class 5's"
            );
        }
    }

    #[test]
    fn wide_peek_silent_when_the_window_opens_before_the_first_sample() {
        // The victim's track starts 64 ticks before the death, inside the
        // 1.5 s peek window: no sample at the window start, no swing to
        // measure. Every other gate would pass.
        let s = Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000);
        let s = at(s, VICTIM, DEATH - 64, 0.0, 0.0, "Palace");
        let s = at(s, VICTIM, DEATH, 200.0, 0.0, "Palace");
        let s = at(s, ENEMY_A, 1000, 1000.0, 0.0, "Jungle");
        let s = at(s, ENEMY_A, DEATH - PEEK_WINDOW, 1000.0, 0.0, "Jungle");
        let s = at(s, ENEMY_A, DEATH, 1000.0, 0.0, "Jungle");
        let data = s
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&data)).is_empty());
    }

    #[test]
    fn wide_peek_requires_a_living_enemy_killer() {
        let team = target_swing().kill(MATE, VICTIM, 1, DEATH, "m4a1").build();
        assert!(wide_peek_flags(&detect(&team)).is_empty());

        // Same swing, but the killer's own last sample says dead.
        let s = Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000);
        let s = at(s, VICTIM, 1000, 0.0, 0.0, "Palace");
        let s = at(s, VICTIM, DEATH - PEEK_WINDOW, 0.0, 0.0, "Palace");
        let s = at(s, VICTIM, DEATH, 200.0, 0.0, "Palace");
        let s = at(s, ENEMY_A, 1000, 1000.0, 0.0, "Jungle");
        let s = at(s, ENEMY_A, DEATH - PEEK_WINDOW, 1000.0, 0.0, "Jungle");
        let dead_killer = s
            .waypoint_full(
                ENEMY_A, DEATH, 1000.0, 0.0, 0.0, 0.0, 0, false, None, None, false,
            )
            .shot(VICTIM, 1950, "weapon_ak47")
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        assert!(wide_peek_flags(&detect(&dead_killer)).is_empty());
    }

    #[test]
    fn wide_peek_insight_fires_at_two_occurrences() {
        let mut s = Scenario::new("de_test")
            .players_ct(&[VICTIM, MATE])
            .players_t(&[ENEMY_A, ENEMY_B, ENEMY_C])
            .round(1, 1000, 5000)
            .round(2, 6000, 10000);
        for (start, death) in [(1000, DEATH), (6000, 7000)] {
            s = at(s, VICTIM, start, 0.0, 0.0, "Palace");
            s = at(s, VICTIM, death - PEEK_WINDOW, 0.0, 0.0, "Palace");
            s = at(s, VICTIM, death, 200.0, 0.0, "Palace");
            s = at(s, ENEMY_A, start, 1000.0, 0.0, "Jungle");
            s = at(s, ENEMY_A, death - PEEK_WINDOW, 1000.0, 0.0, "Jungle");
            s = at(s, ENEMY_A, death, 1000.0, 0.0, "Jungle");
            s = s.shot(VICTIM, death - 50, "weapon_ak47").kill(
                ENEMY_A,
                VICTIM,
                if death == DEATH { 1 } else { 2 },
                death,
                "ak47",
            );
        }
        let data = s.build();
        let ctx = AnalysisContext::new(&data, VICTIM);
        let cfg = DetectorConfig::default();
        let flags = H4Exposure.detect(&ctx, &cfg);
        assert_eq!(wide_peek_flags(&flags).len(), 2);
        let insights = H4Exposure.insights(&ctx, &cfg, &flags);
        let wide: Vec<&Insight> = insights
            .iter()
            .filter(|i| i.detector == WIDE_PEEK_HELD_ANGLE)
            .collect();
        assert_eq!(wide.len(), 1);
        assert_eq!(wide[0].category, Category::Positioning);
        assert_eq!(wide[0].round, 0, "match-level");
        assert_eq!(wide[0].severity, cfg.severity.h4_wide_peek_held_angle);
        assert!((wide[0].confidence - 0.6).abs() < 1e-6);
        assert_eq!(wide[0].title_data["count"], 2);
        assert_eq!(wide[0].metrics["count"], 2);
        assert_eq!(wide[0].evidence.len(), 2);
    }

    #[test]
    fn wide_peek_insight_suppressed_at_one_occurrence() {
        let data = target_swing()
            .kill(ENEMY_A, VICTIM, 1, DEATH, "ak47")
            .build();
        let ctx = AnalysisContext::new(&data, VICTIM);
        let cfg = DetectorConfig::default();
        let flags = H4Exposure.detect(&ctx, &cfg);
        assert_eq!(wide_peek_flags(&flags).len(), 1);
        assert!(!H4Exposure
            .insights(&ctx, &cfg, &flags)
            .iter()
            .any(|i| i.detector == WIDE_PEEK_HELD_ANGLE));
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
