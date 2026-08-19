//! H2 — Trade Spacing & Line-of-Sight (D1; spec §2 H2; taxonomy classes 6/7).
//!
//! - `H2_ISOLATED_DEATH` (→ class 6): the tracked player died where no
//!   teammate could plausibly punish the killer — nearest living teammate
//!   beyond `trade.isolation_u`, in a different `last_place` (place equality
//!   is the no-geometry LOS proxy), and the killer survived the commit window.
//! - `H2_FAILED_TRADE` (flag only): a teammate died in trade range and the
//!   tracked player never committed while the killer was still alive.
//! - `H2_BAITED_TRADE` (→ class 7): the tracked player DID commit, died doing
//!   it, and no other teammate was in trade range of them. The one rule here
//!   that is not primarily the player's fault: severity is capped in config
//!   well below isolated, and the details must name the teammate who didn't
//!   follow (spec: without that the caption reads as blame).
//!
//! Death-anchored flags follow the classifier convention: `tick` = the kill
//! tick, `steamid` = the victim (always the tracked player here).

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{Category, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::Side;

pub struct H2TradeSpacing;

const ISOLATED: &str = "H2_ISOLATED_DEATH";
const FAILED: &str = "H2_FAILED_TRADE";
const BAITED: &str = "H2_BAITED_TRADE";

impl Detector for H2TradeSpacing {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[ISOLATED, FAILED, BAITED]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut flags = isolated_deaths(ctx, cfg);
        flags.extend(trade_follow_ups(ctx, cfg));
        flags
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let count = |id: &str| flags.iter().filter(|f| f.rule_id == id).count();
        // Failed + baited both recurring is a *team* spacing problem, not
        // individual fault (spec H2: never coach the user out of trading).
        let team_pattern = count(FAILED) >= 2 && count(BAITED) >= 2;
        let mut out = vec![];
        for (id, severity) in [
            (ISOLATED, cfg.severity.h2_isolated_death),
            (FAILED, cfg.severity.h2_failed_trade),
            (BAITED, cfg.severity.h2_baited_trade),
        ] {
            let fs: Vec<&RuleFlag> = flags.iter().filter(|f| f.rule_id == id).collect();
            if fs.len() < 2 {
                continue;
            }
            let confidence = fs.iter().map(|f| f.confidence).fold(1.0f32, f32::min);
            let mut title_data = json!({ "count": fs.len(), "rule": id });
            if team_pattern && (id == FAILED || id == BAITED) {
                title_data["team_pattern"] = json!(true);
            }
            let per_round: Vec<serde_json::Value> = fs
                .iter()
                .map(|f| json!({ "round": f.round, "tick": f.tick }))
                .collect();
            out.push(Insight {
                detector: id.to_string(),
                category: Category::Deaths,
                severity,
                confidence,
                round: 0,
                player: ctx.tracked(),
                title_data,
                metrics: json!({ "count": fs.len(), "per_round": per_round }),
                evidence: fs.iter().take(8).map(|f| f.evidence.clone()).collect(),
            });
        }
        out
    }
}

/// Was `sid` killed by anyone within [t0, t1]?
fn killed_in(ctx: &AnalysisContext, sid: u64, t0: i32, t1: i32) -> bool {
    ctx.data()
        .kills
        .iter()
        .any(|k| k.victim == sid && k.tick >= t0 && k.tick <= t1)
}

/// Did `sid` commit within [t0, t1] — fired any shot, or damaged the killer?
fn committed(ctx: &AnalysisContext, sid: u64, killer: u64, t0: i32, t1: i32) -> bool {
    !ctx.shots_by_in(sid, t0, t1).is_empty()
        || ctx
            .hurts_dealt_in(sid, t0, t1)
            .iter()
            .any(|h| h.victim == killer)
}

/// The killer's side must be known and opposite `my_side` (unknown → silent;
/// teamkill/self/world deaths belong to class 14, not to trade spacing).
fn enemy_of(ctx: &AnalysisContext, killer: u64, my_side: Side, round: u32) -> bool {
    ctx.side_of(killer, round).is_some_and(|s| s != my_side)
}

fn isolated_deaths(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let commit_w = ctx.seconds(cfg.trade.commit_window_s);
    let mut out = vec![];
    for kill in ctx.tracked_deaths() {
        let Some(my_side) = ctx.side_of(tracked, kill.round) else {
            continue;
        };
        let Some(killer) = kill.attacker else {
            continue;
        };
        if !enemy_of(ctx, killer, my_side, kill.round) {
            continue;
        }
        let Some(me) = ctx.state_at(tracked, kill.tick) else {
            continue;
        };
        // Traded: the killer was punished in time — spacing did its job.
        if killed_in(ctx, killer, kill.tick, kill.tick + commit_w) {
            continue;
        }

        let mates = ctx.teammates_alive_at(tracked, kill.round, kill.tick);
        let (confidence, mate_focus, details) = if mates.is_empty() {
            // Last alive is still isolated (spec H2 has no clutch exemption),
            // but blame is less certain.
            (
                0.6,
                None,
                json!({
                    "nearest_teammate": null,
                    "distance": null,
                    "place": me.place,
                    "teammate_place": null,
                }),
            )
        } else {
            let Some((mate, dist)) =
                ctx.nearest_teammate(tracked, kill.round, kill.tick, cfg.general.z_weight)
            else {
                continue;
            };
            if dist <= cfg.trade.isolation_u {
                continue;
            }
            let mate_place = ctx.state_at(mate, kill.tick).and_then(|s| s.place);
            // Place equality is the no-geometry LOS proxy: same place means
            // the teammate plausibly saw the killer → silent. Either place
            // missing counts as different (per plan Task 3).
            if me.place.is_some() && me.place == mate_place {
                continue;
            }
            (
                0.75, // LOS approximated → capped per spec (≤ 0.75)
                Some(mate),
                json!({
                    "nearest_teammate": mate.to_string(),
                    "distance": dist,
                    "place": me.place,
                    "teammate_place": mate_place,
                }),
            )
        };
        let mut focus = vec![tracked, killer];
        focus.extend(mate_focus);
        out.push(RuleFlag {
            rule_id: ISOLATED,
            round: kill.round,
            tick: kill.tick,
            steamid: tracked,
            confidence,
            severity: cfg.severity.h2_isolated_death,
            details,
            evidence: evidence_around(ctx, kill.round, kill.tick, &focus),
        });
    }
    out
}

/// H2_FAILED_TRADE + H2_BAITED_TRADE: both start from a teammate dying in
/// trade range of the tracked player and split on whether they committed.
fn trade_follow_ups(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
    let tracked = ctx.tracked();
    let commit_w = ctx.seconds(cfg.trade.commit_window_s);
    let trade_w = ctx.seconds(cfg.trade.window_s);
    let z = cfg.general.z_weight;
    let mut out: Vec<RuleFlag> = vec![];
    for kill in &ctx.data().kills {
        if kill.victim == tracked {
            continue;
        }
        let Some(my_side) = ctx.side_of(tracked, kill.round) else {
            continue;
        };
        if ctx.side_of(kill.victim, kill.round) != Some(my_side) {
            continue;
        }
        let Some(killer) = kill.attacker else {
            continue;
        };
        if !enemy_of(ctx, killer, my_side, kill.round) {
            continue;
        }
        let td = kill.tick;
        let Some(me) = ctx.state_at(tracked, td) else {
            continue;
        };
        if !me.is_alive {
            continue;
        }
        let Some(mate_st) = ctx.state_at(kill.victim, td) else {
            continue;
        };
        let dist = AnalysisContext::dist(&me, &mate_st, z);
        if dist > cfg.trade.distance_u {
            continue;
        }

        if !committed(ctx, tracked, killer, td, td + commit_w) {
            // Passive — but only chargeable while the killer was still alive
            // to be traded.
            if killed_in(ctx, killer, td, td + commit_w) {
                continue;
            }
            out.push(RuleFlag {
                rule_id: FAILED,
                round: kill.round,
                tick: td,
                steamid: tracked,
                confidence: 0.7,
                severity: cfg.severity.h2_failed_trade,
                details: json!({
                    "teammate": kill.victim.to_string(),
                    "killer": killer.to_string(),
                    "distance": dist,
                }),
                evidence: evidence_around(ctx, kill.round, td, &[tracked, kill.victim, killer]),
            });
            continue;
        }

        // Committed. Baited only if the tracked player died doing it...
        let Some(my_death) = ctx.data().kills.iter().find(|k| {
            k.victim == tracked && k.round == kill.round && k.tick >= td && k.tick <= td + trade_w
        }) else {
            continue;
        };
        // ...with nobody else in trade range of THEM at their death.
        let mates = ctx.teammates_alive_at(tracked, kill.round, my_death.tick);
        if mates.is_empty() {
            continue; // 1vX: nobody could have followed — not a bait.
        }
        let Some(me_dead) = ctx.state_at(tracked, my_death.tick) else {
            continue;
        };
        let with_dist: Vec<(u64, f32)> = mates
            .iter()
            .map(|(sid, st)| (*sid, AnalysisContext::dist(&me_dead, st, z)))
            .collect();
        if with_dist.iter().any(|(_, d)| *d <= cfg.trade.distance_u) {
            continue; // support was in range → the trade attempt was covered.
        }
        // Spec: evidence must NAME the teammate who didn't follow. Prefer the
        // nearest teammate who also failed to commit; if all committed, the
        // nearest one still stands (they were out of trade range regardless).
        let non_follower = with_dist
            .iter()
            .filter(|(sid, _)| !committed(ctx, *sid, killer, td, td + commit_w))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .or_else(|| with_dist.iter().min_by(|a, b| a.1.total_cmp(&b.1)));
        let Some((nf, nf_dist)) = non_follower.copied() else {
            continue;
        };
        // One baited flag per tracked death: keep the most recent teammate
        // death as its cause.
        out.retain(|f| !(f.rule_id == BAITED && f.tick == my_death.tick));
        out.push(RuleFlag {
            rule_id: BAITED,
            round: kill.round,
            tick: my_death.tick,
            steamid: tracked,
            confidence: 0.7,
            severity: cfg.severity.h2_baited_trade,
            details: json!({
                "non_following_teammate": nf.to_string(),
                "their_distance": nf_dist,
                "dead_teammate": kill.victim.to_string(),
                "killer": killer.to_string(),
            }),
            evidence: evidence_around(
                ctx,
                kill.round,
                my_death.tick,
                &[tracked, kill.victim, killer, nf],
            ),
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

    fn detect_all(data: &MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H2TradeSpacing.detect(&ctx, &DetectorConfig::default())
    }

    fn only<'a>(flags: &'a [RuleFlag], id: &str) -> Vec<&'a RuleFlag> {
        flags.iter().filter(|f| f.rule_id == id).collect()
    }

    /// Stationary alive player on y = z = 0 with an optional place name.
    fn hold_place(
        s: Scenario,
        sid: u64,
        t0: i32,
        t1: i32,
        x: f32,
        place: Option<&str>,
    ) -> Scenario {
        s.waypoint_full(
            sid,
            t0,
            x,
            0.0,
            0.0,
            0.0,
            100,
            true,
            Some("weapon_ak47"),
            place,
            false,
        )
        .waypoint_full(
            sid,
            t1,
            x,
            0.0,
            0.0,
            0.0,
            100,
            true,
            Some("weapon_ak47"),
            place,
            false,
        )
    }

    /// Corpse samples from just after `tick` until `until`.
    fn dead_after(s: Scenario, sid: u64, tick: i32, x: f32, until: i32) -> Scenario {
        s.waypoint_full(sid, tick + 1, x, 0.0, 0.0, 0.0, 0, false, None, None, false)
            .waypoint_full(sid, until, x, 0.0, 0.0, 0.0, 0, false, None, None, false)
    }

    /// Tracked (CT 1) + one teammate (CT 2) holding still all round.
    fn isolated_base(mate_x: f32, my_place: Option<&str>, mate_place: Option<&str>) -> Scenario {
        let s = Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[4, 5])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, my_place);
        hold_place(s, 2, 1000, 3000, mate_x, mate_place)
    }

    // ---- H2_ISOLATED_DEATH ----

    #[test]
    fn isolated_fires_when_nearest_teammate_far_in_different_place() {
        let data = isolated_base(1200.0, Some("SiteA"), Some("Mid"))
            .kill(4, 1, 1, 2000, "ak47")
            .build();
        let flags = detect_all(&data);
        let iso = only(&flags, ISOLATED);
        assert_eq!(iso.len(), 1);
        let f = iso[0];
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, 2000, "death-anchored: flag tick = kill tick");
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.75).abs() < 0.001);
        let cfg = DetectorConfig::default();
        assert_eq!(f.severity, cfg.severity.h2_isolated_death);
        assert_eq!(f.details["nearest_teammate"], json!("2"));
        assert!((f.details["distance"].as_f64().unwrap() - 1200.0).abs() < 1.0);
        assert_eq!(f.details["place"], json!("SiteA"));
        assert_eq!(f.details["teammate_place"], json!("Mid"));
    }

    #[test]
    fn isolated_suppressed_when_teammate_within_isolation_radius() {
        let data = isolated_base(500.0, Some("SiteA"), Some("Mid"))
            .kill(4, 1, 1, 2000, "ak47")
            .build();
        assert!(only(&detect_all(&data), ISOLATED).is_empty());
    }

    #[test]
    fn isolated_suppressed_when_teammate_shares_place() {
        // Same place = LOS proxy says the teammate could trade → silent,
        // even at 1200 u.
        let data = isolated_base(1200.0, Some("Mid"), Some("Mid"))
            .kill(4, 1, 1, 2000, "ak47")
            .build();
        assert!(only(&detect_all(&data), ISOLATED).is_empty());
    }

    #[test]
    fn isolated_suppressed_when_killer_traded_within_commit_window() {
        let data = isolated_base(1200.0, Some("SiteA"), Some("Mid"))
            .kill(4, 1, 1, 2000, "ak47")
            .kill(2, 4, 1, 2050, "ak47") // teammate traded the killer 50 ticks later
            .build();
        assert!(only(&detect_all(&data), ISOLATED).is_empty());
    }

    #[test]
    fn isolated_fires_at_reduced_confidence_when_no_teammates_alive() {
        let s = Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[4, 5])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        // Teammate 2 died long before, far away.
        let s = hold_place(s, 2, 1000, 1200, 5000.0, None);
        let s = dead_after(s, 2, 1200, 5000.0, 3000);
        let data = s
            .kill(4, 2, 1, 1200, "ak47")
            .kill(4, 1, 1, 2000, "ak47")
            .build();
        let flags = detect_all(&data);
        let iso = only(&flags, ISOLATED);
        assert_eq!(iso.len(), 1);
        assert!((iso[0].confidence - 0.6).abs() < 0.001);
        assert!(iso[0].details["nearest_teammate"].is_null());
        assert!(iso[0].details["distance"].is_null());
    }

    // ---- H2_FAILED_TRADE ----

    /// Tracked (CT 1) alive at origin; teammate (CT 2) dies at x=`mate_x`.
    fn failed_base(mate_x: f32) -> Scenario {
        let s = Scenario::new("de_test")
            .players_ct(&[1, 2, 3])
            .players_t(&[4, 5])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        let s = hold_place(s, 2, 1000, 2000, mate_x, None);
        dead_after(s, 2, 2000, mate_x, 3000)
    }

    #[test]
    fn failed_trade_fires_when_tracked_never_commits() {
        let data = failed_base(400.0).kill(4, 2, 1, 2000, "ak47").build();
        let flags = detect_all(&data);
        assert_eq!(flags.len(), 1, "only the failed trade fires");
        let f = &flags[0];
        assert_eq!(f.rule_id, FAILED);
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, 2000, "anchored on the teammate's death tick");
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.7).abs() < 0.001);
        let cfg = DetectorConfig::default();
        assert_eq!(f.severity, cfg.severity.h2_failed_trade);
        assert_eq!(f.details["teammate"], json!("2"));
        assert_eq!(f.details["killer"], json!("4"));
        assert!((f.details["distance"].as_f64().unwrap() - 400.0).abs() < 1.0);
    }

    #[test]
    fn failed_trade_suppressed_when_tracked_shoots_within_window() {
        let data = failed_base(400.0)
            .kill(4, 2, 1, 2000, "ak47")
            .shot(1, 2100, "weapon_ak47") // within the 2 s commit window
            .build();
        assert!(only(&detect_all(&data), FAILED).is_empty());
    }

    #[test]
    fn failed_trade_suppressed_when_killer_died_anyway() {
        // Someone else traded the killer inside the window: nothing to charge.
        let data = failed_base(400.0)
            .kill(4, 2, 1, 2000, "ak47")
            .kill(3, 4, 1, 2100, "ak47")
            .build();
        assert!(only(&detect_all(&data), FAILED).is_empty());
    }

    #[test]
    fn failed_trade_suppressed_when_teammate_died_out_of_trade_range() {
        let data = failed_base(900.0).kill(4, 2, 1, 2000, "ak47").build();
        assert!(only(&detect_all(&data), FAILED).is_empty());
    }

    // ---- H2_BAITED_TRADE ----

    /// Teammate 2 dies at 2000 (400 u from tracked); tracked commits (damages
    /// the killer) and dies at 2100; third teammate 3 holds at `third_x`.
    fn baited_data(third_x: f32) -> MatchData {
        let s = Scenario::new("de_test")
            .players_ct(&[1, 2, 3])
            .players_t(&[4, 5])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 2100, 0.0, None);
        let s = dead_after(s, 1, 2100, 0.0, 3000);
        let s = hold_place(s, 2, 1000, 2000, 400.0, None);
        let s = dead_after(s, 2, 2000, 400.0, 3000);
        let s = hold_place(s, 3, 1000, 3000, third_x, None);
        s.kill(4, 2, 1, 2000, "ak47")
            .hurt(1, 4, 2050, 25, "ak47")
            .kill(4, 1, 1, 2100, "ak47")
            .build()
    }

    #[test]
    fn baited_fires_and_names_non_following_teammate() {
        let data = baited_data(1500.0);
        let flags = detect_all(&data);
        let baited = only(&flags, BAITED);
        assert_eq!(baited.len(), 1);
        let f = baited[0];
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, 2100, "anchored on the TRACKED player's death");
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.7).abs() < 0.001);
        let cfg = DetectorConfig::default();
        assert_eq!(f.severity, cfg.severity.h2_baited_trade);
        assert_eq!(f.details["non_following_teammate"], json!("3"));
        assert!((f.details["their_distance"].as_f64().unwrap() - 1500.0).abs() < 1.0);
        assert_eq!(f.details["dead_teammate"], json!("2"));
        assert_eq!(f.details["killer"], json!("4"));
        // Tracked committed, so no failed-trade on the same moment.
        assert!(only(&flags, FAILED).is_empty());
        // The isolated rule also fires on this death (teammate 3 was 1500 u
        // away); the classifier resolves 6-over-7 by priority.
        assert_eq!(only(&flags, ISOLATED).len(), 1);
    }

    #[test]
    fn baited_suppressed_when_third_teammate_in_trade_range() {
        let data = baited_data(300.0);
        assert!(only(&detect_all(&data), BAITED).is_empty());
    }

    #[test]
    fn baited_suppressed_when_no_other_teammate_alive() {
        // Same story with no third teammate: a 1v X, not a bait.
        let s = Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[4, 5])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 2100, 0.0, None);
        let s = dead_after(s, 1, 2100, 0.0, 3000);
        let s = hold_place(s, 2, 1000, 2000, 400.0, None);
        let s = dead_after(s, 2, 2000, 400.0, 3000);
        let data = s
            .kill(4, 2, 1, 2000, "ak47")
            .hurt(1, 4, 2050, 25, "ak47")
            .kill(4, 1, 1, 2100, "ak47")
            .build();
        assert!(only(&detect_all(&data), BAITED).is_empty());
    }

    // ---- insights ----

    fn syn(rule_id: &'static str, round: u32, tick: i32, confidence: f32) -> RuleFlag {
        RuleFlag {
            rule_id,
            round,
            tick,
            steamid: TRACKED,
            confidence,
            severity: 0.5,
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
            .players_ct(&[1])
            .players_t(&[4])
            .round(1, 1000, 5000)
            .build()
    }

    fn insights_for(data: &MatchData, flags: &[RuleFlag]) -> Vec<Insight> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H2TradeSpacing.insights(&ctx, &DetectorConfig::default(), flags)
    }

    #[test]
    fn insight_requires_two_occurrences_and_aggregates() {
        let data = insight_ctx_data();
        let flags = vec![
            syn(ISOLATED, 1, 2000, 0.75),
            syn(ISOLATED, 3, 4000, 0.6),
            syn(FAILED, 2, 3000, 0.7), // only once → no insight
        ];
        let ins = insights_for(&data, &flags);
        assert_eq!(ins.len(), 1);
        let i = &ins[0];
        assert_eq!(i.detector, ISOLATED);
        assert_eq!(i.category, Category::Deaths);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        let cfg = DetectorConfig::default();
        assert_eq!(i.severity, cfg.severity.h2_isolated_death);
        assert!(
            (i.confidence - 0.6).abs() < 0.001,
            "min of flag confidences"
        );
        assert_eq!(i.title_data["count"], json!(2));
        assert_eq!(i.title_data["rule"], json!(ISOLATED));
        assert!(i.title_data.get("team_pattern").is_none());
        assert_eq!(i.metrics["count"], json!(2));
        assert_eq!(i.metrics["per_round"].as_array().unwrap().len(), 2);
        assert_eq!(i.evidence.len(), 2);
    }

    #[test]
    fn insights_mark_team_pattern_when_failed_and_baited_both_recur() {
        let data = insight_ctx_data();
        let flags = vec![
            syn(FAILED, 1, 2000, 0.7),
            syn(FAILED, 2, 3000, 0.7),
            syn(BAITED, 3, 4000, 0.7),
            syn(BAITED, 4, 5000, 0.7),
        ];
        let ins = insights_for(&data, &flags);
        assert_eq!(ins.len(), 2);
        for i in &ins {
            assert_eq!(
                i.title_data["team_pattern"],
                json!(true),
                "{} must carry the team-pattern marker",
                i.detector
            );
        }

        // Baited only once → no team pattern on the failed-trade insight.
        let flags = vec![
            syn(FAILED, 1, 2000, 0.7),
            syn(FAILED, 2, 3000, 0.7),
            syn(BAITED, 3, 4000, 0.7),
        ];
        let ins = insights_for(&data, &flags);
        assert_eq!(ins.len(), 1);
        assert_eq!(ins[0].detector, FAILED);
        assert!(ins[0].title_data.get("team_pattern").is_none());
    }

    #[test]
    fn insight_evidence_capped_at_eight() {
        let data = insight_ctx_data();
        let flags: Vec<RuleFlag> = (0..9)
            .map(|n| syn(ISOLATED, n + 1, 2000 + 500 * n as i32, 0.75))
            .collect();
        let ins = insights_for(&data, &flags);
        assert_eq!(ins.len(), 1);
        assert_eq!(ins[0].title_data["count"], json!(9));
        assert_eq!(ins[0].evidence.len(), 8);
    }
}
