//! H1 — Man-Count Discipline (spec §2 H1; taxonomy class 8).
//!
//! `H1_DESPERATION_PEEK` (→ class 8): down bodies, the player walked into
//! the contact, and the clock was not demanding it. Spec target case: 3v4,
//! 40 s left, nothing forcing contact, walked into the open looking for the
//! equaliser.
//!
//! "Player initiated" has no geometry behind it here — the demo exposes no
//! per-pair visibility (spec §5.1) — so it is a *kinematic* proxy: over the
//! approach window the tracked player both closed on the killer and walked
//! at least as far as the killer did. If the killer was the one covering
//! ground, the tracked player was defending, not initiating, and the rule
//! stays silent (spec §2 H1's "the enemy was already pushing you"
//! suppression). Approximation → confidence capped at 0.6 (spec §4.2).
//!
//! Death-anchored flags follow the classifier convention: `tick` = the kill
//! tick, `steamid` = the victim (always the tracked player here).

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::families::h2::killed_in;
use crate::types::{Category, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::{Kill, Side};

pub struct H1ManCount;

const DESPERATION_PEEK: &str = "H1_DESPERATION_PEEK";

/// Kinematic proxy for "player initiated", no geometry behind it — spec §4.2
/// caps approximations here.
const CONF_KINEMATIC: f32 = 0.6;
/// Kinematics are sampled at 4 Hz across the approach window.
const SAMPLE_STEP_S: f32 = 0.25;
const INSIGHT_MIN_OCCURRENCES: usize = 2;
const INSIGHT_EVIDENCE_CAP: usize = 8;
/// How many distinct man contexts ("3v4") the insight names.
const INSIGHT_MAN_CONTEXT_CAP: usize = 3;

impl Detector for H1ManCount {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[DESPERATION_PEEK]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        ctx.tracked_deaths()
            .into_iter()
            .filter_map(|kill| desperation_peek(ctx, cfg, kill))
            .collect()
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let fs: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == DESPERATION_PEEK)
            .collect();
        if fs.len() < INSIGHT_MIN_OCCURRENCES {
            return vec![];
        }
        let confidence = fs.iter().map(|f| f.confidence).fold(1.0f32, f32::min);
        // The man counts are the finding ("3v4, 2v4"), so the insight carries
        // them for the caption — deduped in order, capped.
        let mut man_contexts: Vec<String> = vec![];
        for f in &fs {
            if let Some(m) = f.details["man_context"].as_str() {
                if !man_contexts.iter().any(|seen| seen == m) {
                    man_contexts.push(m.to_string());
                }
            }
        }
        man_contexts.truncate(INSIGHT_MAN_CONTEXT_CAP);
        let per_round: Vec<serde_json::Value> = fs
            .iter()
            .map(|f| json!({ "round": f.round, "tick": f.tick }))
            .collect();
        vec![Insight {
            detector: DESPERATION_PEEK.to_string(),
            category: Category::Deaths,
            severity: cfg.severity.h1_desperation_peek,
            confidence,
            round: 0,
            player: ctx.tracked(),
            title_data: json!({
                "count": fs.len(),
                "rule": DESPERATION_PEEK,
                "man_contexts": man_contexts,
            }),
            metrics: json!({ "count": fs.len(), "per_round": per_round }),
            evidence: fs
                .iter()
                .take(INSIGHT_EVIDENCE_CAP)
                .map(|f| f.evidence.clone())
                .collect(),
        }]
    }
}

fn desperation_peek(ctx: &AnalysisContext, cfg: &DetectorConfig, kill: &Kill) -> Option<RuleFlag> {
    let tracked = ctx.tracked();
    let my_side = ctx.side_of(tracked, kill.round)?;
    let killer = kill.attacker?;
    if killer == tracked
        || !ctx
            .side_of(killer, kill.round)
            .is_some_and(|s| s != my_side)
    {
        return None;
    }
    let killer_state = ctx.state_at(killer, kill.tick)?;
    if !killer_state.is_alive {
        return None;
    }
    let me = ctx.state_at(tracked, kill.tick)?;

    // The board BEFORE this death (same convention as the play ledger).
    let before = kill.tick - 1;
    let (my_alive, their_alive) = my_side_first(ctx.alive_counts_at(kill.round, before)?, my_side);
    if my_alive as i32 - their_alive as i32 > cfg.h1.disadvantage_max {
        return None;
    }
    // Last alive is a clutch, judged by H10 — not an over-peek (spec §2 H1).
    if my_alive < 2 {
        return None;
    }

    // Player initiated: you closed the gap, and you covered more ground than
    // the killer did. Either way round, no sample at the window start is
    // silence rather than a guess.
    let z = cfg.general.z_weight;
    let t0 = kill.tick - ctx.seconds(cfg.h1.approach_window_s);
    let step = ctx.seconds(SAMPLE_STEP_S);
    let my_track = ctx.samples_in(tracked, t0, kill.tick, step)?;
    let killer_track = ctx.samples_in(killer, t0, kill.tick, step)?;
    let closed_u = AnalysisContext::dist(my_track.first()?, killer_track.first()?, z)
        - AnalysisContext::dist(&me, &killer_state, z);
    let my_moved = AnalysisContext::path_length(&my_track, z);
    let killer_moved = AnalysisContext::path_length(&killer_track, z);
    if closed_u < cfg.h1.approach_min_u
        || my_moved < cfg.h1.approach_min_u
        || killer_moved > my_moved
    {
        return None;
    }

    let seconds_left = clock_left(ctx, cfg, kill, my_side)?;

    // The peek worked as intended when the killer is punished in time.
    if killed_in(
        ctx,
        killer,
        kill.tick,
        kill.tick + ctx.seconds(cfg.h1.traded_within_s),
    ) {
        return None;
    }

    Some(RuleFlag {
        rule_id: DESPERATION_PEEK,
        round: kill.round,
        tick: kill.tick,
        steamid: tracked,
        confidence: CONF_KINEMATIC,
        severity: cfg.severity.h1_desperation_peek,
        details: json!({
            "man_context": format!("{my_alive}v{their_alive}"),
            "my_alive": my_alive,
            "their_alive": their_alive,
            "closed_u": closed_u,
            "killer_moved_u": killer_moved,
            "seconds_left": seconds_left,
            "place": me.place,
            "killer": killer.to_string(),
        }),
        evidence: evidence_around(ctx, kill.round, kill.tick, &[tracked, killer]),
    })
}

/// (CT, T) alive counts reordered as (mine, theirs).
fn my_side_first(counts: (usize, usize), my_side: Side) -> (usize, usize) {
    match my_side {
        Side::Ct => counts,
        Side::T => (counts.1, counts.0),
    }
}

/// Seconds left on whichever clock is actually running at the death — the
/// bomb once it is down, the round otherwise — or None when that clock was
/// demanding the contact (spec §2 H1's two suppressions) or has already run
/// out, which means the derivation is wrong about this round and silence is
/// the safe answer (spec §4.1).
fn clock_left(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    kill: &Kill,
    my_side: Side,
) -> Option<f64> {
    let round = ctx.data().rounds.iter().find(|r| r.number == kill.round)?;
    let span_end = round.officially_ended_tick.unwrap_or(round.end_tick);
    let tickrate = f64::from(ctx.data().tickrate);
    // A plant after the death leaves the round clock as the running one.
    let planted = ctx.data().bomb_events.iter().find(|b| {
        b.kind == "planted"
            && b.tick >= round.start_tick
            && b.tick <= span_end
            && b.tick <= kill.tick
    });
    let left = match planted {
        Some(p) => f64::from(cfg.h1.bomb_timer_s) - f64::from(kill.tick - p.tick) / tickrate,
        None => {
            let freeze_end = round.freeze_end_tick.unwrap_or(round.start_tick);
            f64::from(cfg.h1.round_length_s) - f64::from(kill.tick - freeze_end) / tickrate
        }
    };
    if left < 0.0 {
        return None;
    }
    let forced = match (my_side, planted.is_some()) {
        (Side::Ct, true) => left < f64::from(cfg.h1.ct_retake_forced_s),
        (Side::T, false) => left < f64::from(cfg.h1.t_forced_s),
        _ => false,
    };
    if forced {
        return None;
    }
    Some((left * 10.0).round() / 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DetectorConfig;
    use crate::context::AnalysisContext;
    use crate::scenario::Scenario;
    use crate::types::RuleFlag;
    use crate::Detector;
    use cf_parser::model::MatchData;
    use serde_json::json;

    const TRACKED: u64 = 1;
    const MATE_A: u64 = 2;
    const MATE_B: u64 = 3;
    const KILLER: u64 = 4;
    const ENEMY_B: u64 = 5;
    const DEATH: i32 = 2000;
    const ROUND_START: i32 = 1000;
    /// 2.0 s of approach window at 64 tick.
    const WINDOW: i32 = 128;

    fn detect(data: &MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H1ManCount.detect(&ctx, &DetectorConfig::default())
    }

    /// Two waypoints before the window opens and one at the death, so the
    /// whole walk happens inside the approach window.
    fn walk(
        s: Scenario,
        sid: u64,
        from: (f32, f32),
        to: (f32, f32),
        death: i32,
        place: &str,
    ) -> Scenario {
        let at = |s: Scenario, tick: i32, p: (f32, f32)| {
            s.waypoint_full(
                sid,
                tick,
                p.0,
                p.1,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_ak47"),
                Some(place),
                false,
            )
        };
        let s = at(s, ROUND_START, from);
        let s = at(s, death - WINDOW, from);
        at(s, death, to)
    }

    /// 3 on the tracked player's side v 4 on the other — down one body. The
    /// tracked player walks from the origin to `victim_to`; the killer holds
    /// (or walks) between `killer_from` and `killer_to` on the x axis.
    fn peek(
        death: i32,
        victim_to: (f32, f32),
        killer_from: f32,
        killer_to: f32,
        tracked_is_ct: bool,
    ) -> Scenario {
        let mine = [TRACKED, MATE_A, MATE_B];
        let theirs = [KILLER, ENEMY_B, 6, 7];
        let s = if tracked_is_ct {
            Scenario::new("de_test")
                .players_ct(&mine)
                .players_t(&theirs)
        } else {
            Scenario::new("de_test")
                .players_t(&mine)
                .players_ct(&theirs)
        }
        .round(1, ROUND_START, 12000);
        let s = walk(s, TRACKED, (0.0, 0.0), victim_to, death, "Palace");
        walk(
            s,
            KILLER,
            (killer_from, 0.0),
            (killer_to, 0.0),
            death,
            "Jungle",
        )
    }

    /// The spec's target case: down a body, clock wide open, the tracked
    /// player walks 400 u at a killer who never moves.
    fn target_case() -> Scenario {
        peek(DEATH, (400.0, 0.0), 1000.0, 1000.0, true)
    }

    fn died(s: Scenario) -> MatchData {
        s.kill(KILLER, TRACKED, 1, DEATH, "ak47").build()
    }

    #[test]
    fn fires_when_down_bodies_and_the_player_walked_into_it() {
        let data = died(target_case());
        let flags = detect(&data);
        assert_eq!(flags.len(), 1);
        let f = &flags[0];
        assert_eq!(f.rule_id, DESPERATION_PEEK);
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, DEATH, "death-anchored: flag tick = kill tick");
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.6).abs() < 1e-6, "kinematic proxy");
        assert_eq!(
            f.severity,
            DetectorConfig::default().severity.h1_desperation_peek
        );
        assert_eq!(f.details["man_context"], json!("3v4"));
        assert_eq!(f.details["my_alive"], json!(3));
        assert_eq!(f.details["their_alive"], json!(4));
        assert!((f.details["closed_u"].as_f64().unwrap() - 400.0).abs() < 1.0);
        assert!(f.details["killer_moved_u"].as_f64().unwrap() < 1.0);
        // 115 s round − (2000 − 1000)/64 elapsed.
        assert!((f.details["seconds_left"].as_f64().unwrap() - 99.4).abs() < 0.05);
        assert_eq!(f.details["place"], json!("Palace"));
        assert_eq!(f.details["killer"], json!("4"));
        assert_eq!(f.evidence.focus_players, vec![TRACKED, KILLER]);
    }

    #[test]
    fn silent_when_the_man_count_is_level() {
        // The tracked player traded one down first: 3v3 at the death.
        let data = target_case()
            .kill(TRACKED, ENEMY_B, 1, 1500, "ak47")
            .kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_up_bodies() {
        let data = target_case()
            .kill(TRACKED, ENEMY_B, 1, 1400, "ak47")
            .kill(MATE_A, 6, 1, 1500, "ak47")
            .kill(MATE_A, 7, 1, 1600, "ak47")
            .kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_last_alive() {
        // 1v4 is a clutch, judged by H10 — not an over-peek (spec §2 H1).
        let data = target_case()
            .kill(KILLER, MATE_A, 1, 1400, "ak47")
            .kill(KILLER, MATE_B, 1, 1500, "ak47")
            .kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_the_player_did_not_close_on_the_killer() {
        // 400 u of walking, but sideways: the gap never shrank.
        let data = died(peek(DEATH, (0.0, 400.0), 1000.0, 1000.0, true));
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_the_killer_covered_more_ground_than_the_player() {
        // Killer walks 400 u in, tracked player 200 u: they were pushing
        // you, so you were defending (spec §2 H1 suppression).
        let data = died(peek(DEATH, (200.0, 0.0), 1000.0, 600.0, true));
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_ct_side_and_the_bomb_leaves_no_time_for_the_retake() {
        // Planted at 2000, death at 4000: 8.8 s of bomb left.
        let data = peek(4000, (400.0, 0.0), 1000.0, 1000.0, true)
            .bomb("planted", ENEMY_B, 2000)
            .kill(KILLER, TRACKED, 1, 4000, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn fires_with_the_bomb_down_while_the_retake_still_has_time() {
        // Planted at 3800, death at 4000: 36.9 s of bomb left, and the bomb
        // timer — not the round clock — is what "seconds left" means now.
        let data = peek(4000, (400.0, 0.0), 1000.0, 1000.0, true)
            .bomb("planted", ENEMY_B, 3800)
            .kill(KILLER, TRACKED, 1, 4000, "ak47")
            .build();
        let flags = detect(&data);
        assert_eq!(flags.len(), 1);
        assert!((flags[0].details["seconds_left"].as_f64().unwrap() - 36.9).abs() < 0.05);
    }

    #[test]
    fn silent_when_t_side_and_the_round_clock_is_running_out() {
        // Death at 8000: 109 s elapsed, 5.6 s of round left.
        let data = peek(8000, (400.0, 0.0), 1000.0, 1000.0, false)
            .kill(KILLER, TRACKED, 1, 8000, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn fires_on_t_side_while_the_round_clock_is_open() {
        let data = peek(DEATH, (400.0, 0.0), 1000.0, 1000.0, false)
            .kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .build();
        assert_eq!(detect(&data).len(), 1);
    }

    #[test]
    fn silent_when_the_derived_clock_has_already_run_out() {
        // No plant, 117 s elapsed: the round clock says the data is odd
        // (post-plant round with no plant event) — bias to silence.
        let data = peek(8500, (400.0, 0.0), 1000.0, 1000.0, true)
            .kill(KILLER, TRACKED, 1, 8500, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_the_killer_was_traded_inside_two_seconds() {
        // The peek worked as intended (spec §2 H1).
        let data = target_case()
            .kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .kill(MATE_A, KILLER, 1, DEATH + 100, "ak47")
            .build();
        assert!(detect(&data).is_empty());
    }

    #[test]
    fn silent_when_the_killer_is_not_a_known_enemy() {
        let teamkill = target_case()
            .kill(MATE_A, TRACKED, 1, DEATH, "m4a1")
            .build();
        assert!(detect(&teamkill).is_empty());
        let world = target_case()
            .kill_full(None, TRACKED, 1, DEATH, "world", false, 0)
            .build();
        assert!(detect(&world).is_empty());
    }

    #[test]
    fn silent_when_the_killer_is_not_alive_at_the_kill_tick() {
        // A corpse cannot be the enemy you walked at — odd state data
        // biases to silence.
        let s = Scenario::new("de_test")
            .players_ct(&[TRACKED, MATE_A, MATE_B])
            .players_t(&[KILLER, ENEMY_B, 6, 7])
            .round(1, ROUND_START, 12000);
        let s = walk(s, TRACKED, (0.0, 0.0), (400.0, 0.0), DEATH, "Palace");
        let alive = |s: Scenario, tick: i32| {
            s.waypoint_full(
                KILLER,
                tick,
                1000.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_ak47"),
                Some("Jungle"),
                false,
            )
        };
        let s = alive(s, ROUND_START);
        let s = alive(s, DEATH - WINDOW);
        let s = s.waypoint_full(
            KILLER, DEATH, 1000.0, 0.0, 0.0, 0.0, 0, false, None, None, false,
        );
        let data = s.kill(KILLER, TRACKED, 1, DEATH, "ak47").build();
        assert!(detect(&data).is_empty());
    }

    // ---- insights ----

    /// Two rounds, one desperation peek in each.
    fn two_peeks() -> MatchData {
        let mine = [TRACKED, MATE_A, MATE_B];
        let theirs = [KILLER, ENEMY_B, 6, 7];
        let mut s = Scenario::new("de_test")
            .players_ct(&mine)
            .players_t(&theirs)
            .round(1, ROUND_START, 12000)
            .round(2, 20000, 30000);
        for (start, death) in [(ROUND_START, DEATH), (20000, 21000)] {
            for (sid, from, to, place) in [
                (TRACKED, (0.0, 0.0), (400.0, 0.0), "Palace"),
                (KILLER, (1000.0, 0.0), (1000.0, 0.0), "Jungle"),
            ] {
                let at = |s: Scenario, tick: i32, p: (f32, f32)| {
                    s.waypoint_full(
                        sid,
                        tick,
                        p.0,
                        p.1,
                        0.0,
                        0.0,
                        100,
                        true,
                        Some("weapon_ak47"),
                        Some(place),
                        false,
                    )
                };
                s = at(s, start, from);
                s = at(s, death - WINDOW, from);
                s = at(s, death, to);
            }
        }
        s.kill(KILLER, TRACKED, 1, DEATH, "ak47")
            .kill(KILLER, TRACKED, 2, 21000, "ak47")
            .build()
    }

    #[test]
    fn insight_fires_at_two_occurrences_and_carries_the_man_contexts() {
        let data = two_peeks();
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = H1ManCount.detect(&ctx, &cfg);
        assert_eq!(flags.len(), 2);
        let insights = H1ManCount.insights(&ctx, &cfg, &flags);
        assert_eq!(insights.len(), 1);
        let i = &insights[0];
        assert_eq!(i.detector, DESPERATION_PEEK);
        assert_eq!(i.category, crate::types::Category::Deaths);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        assert_eq!(i.severity, cfg.severity.h1_desperation_peek);
        assert!((i.confidence - 0.6).abs() < 1e-6);
        assert_eq!(i.title_data["count"], json!(2));
        assert_eq!(i.title_data["rule"], json!(DESPERATION_PEEK));
        assert_eq!(i.title_data["man_contexts"], json!(["3v4"]));
        assert_eq!(i.metrics["count"], json!(2));
        assert_eq!(i.metrics["per_round"].as_array().unwrap().len(), 2);
        assert_eq!(i.evidence.len(), 2);
    }

    #[test]
    fn insight_suppressed_at_one_occurrence() {
        let data = died(target_case());
        let ctx = AnalysisContext::new(&data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = H1ManCount.detect(&ctx, &cfg);
        assert_eq!(flags.len(), 1);
        assert!(H1ManCount.insights(&ctx, &cfg, &flags).is_empty());
    }
}
