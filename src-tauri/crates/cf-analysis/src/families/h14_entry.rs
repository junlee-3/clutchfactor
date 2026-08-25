//! H14 — Opening-Duel / Entry Structure (D4; PROMPT.md §5 D4;
//! docs/spec/death-taxonomy.md §2-3).
//!
//! The round's opening duel = the first `Kill` (by tick) whose two
//! participants are on opposite, known sides, provided it lands within
//! `entry.opening_window_s` of freeze end (later first-kills are mid-round
//! picks, not entries). Its "entry player" is the T-side participant.
//!
//! - `H14_UNSUPPORTED_ENTRY` (flag, NOT death-anchored — anchor tick = the
//!   opening-kill tick, steamid = tracked): fires only when the tracked
//!   player WAS that round's entry player and had no living teammate within
//!   `entry.support_distance_u` (z-weighted) or sharing their `last_place` at
//!   that tick. Fires whether the entry won or lost the duel — the mistake is
//!   structural, not the trade outcome.
//! - `D4_ENTRY_PROFILE` (match-level insight, category Positioning): fires
//!   unconditionally once the tracked player made ≥3 T-side entries,
//!   summarizing their own entry record plus the team's, and how often the
//!   tracked player stood near a dying teammate-entry without trading.

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{Category, Insight, RuleFlag};
use crate::{evidence_around, Detector};
use cf_parser::model::Side;

use super::h2::committed;

pub struct H14EntryStructure;

const UNSUPPORTED: &str = "H14_UNSUPPORTED_ENTRY";

impl Detector for H14EntryStructure {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[UNSUPPORTED]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let tracked = ctx.tracked();
        let mut out = vec![];
        for entry in round_entries(ctx, cfg) {
            if entry.entry_player != tracked || entry.supported {
                continue;
            }
            let mut focus = vec![tracked, entry.opponent];
            if let Some((mate, _)) = entry.nearest_teammate {
                focus.push(mate);
            }
            out.push(RuleFlag {
                rule_id: UNSUPPORTED,
                round: entry.round,
                tick: entry.tick,
                steamid: tracked,
                confidence: 0.7,
                severity: cfg.severity.h14_unsupported_entry,
                details: json!({
                    "won": entry.won,
                    "opponent": entry.opponent.to_string(),
                    "nearest_teammate": entry.nearest_teammate.map(|(sid, _)| sid.to_string()),
                    "distance": entry.nearest_teammate.map(|(_, d)| d),
                }),
                evidence: evidence_around(ctx, entry.round, entry.tick, &focus),
            });
        }
        out
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let tracked = ctx.tracked();
        let all_entries = round_entries(ctx, cfg);
        let mine: Vec<&EntryInfo> = all_entries
            .iter()
            .filter(|e| e.entry_player == tracked)
            .collect();
        if mine.len() < 3 {
            return vec![];
        }
        let entries = mine.len();
        let entry_wins = mine.iter().filter(|e| e.won).count();
        let supported = mine.iter().filter(|e| e.supported).count();
        let unsupported = mine.iter().filter(|e| !e.supported).count();

        // "Team" entries: rounds where the entry (always T-side) was on
        // tracked's own side, i.e. tracked was T-side that round too — every
        // T player in a round shares the same T roster, so this is exactly
        // "a teammate (or tracked) made the entry."
        let team: Vec<&EntryInfo> = all_entries
            .iter()
            .filter(|e| ctx.side_of(tracked, e.round) == Some(Side::T))
            .collect();
        let team_entries = team.len();
        let team_entry_wins = team.iter().filter(|e| e.won).count();

        let commit_w = ctx.seconds(cfg.trade.commit_window_s);
        let non_trading_on_entries = team
            .iter()
            .filter(|e| e.entry_player != tracked && !e.won)
            .filter(|e| {
                let Some(mate_st) = ctx.state_at(e.entry_player, e.tick) else {
                    return false;
                };
                let Some(me) = ctx.state_at(tracked, e.tick) else {
                    return false;
                };
                if !me.is_alive {
                    return false;
                }
                let d = AnalysisContext::dist(&me, &mate_st, cfg.general.z_weight);
                if d > cfg.entry.support_distance_u {
                    return false;
                }
                !committed(ctx, tracked, e.killer, e.tick, e.tick + commit_w)
            })
            .count();

        let unsupported_flags: Vec<&RuleFlag> =
            flags.iter().filter(|f| f.rule_id == UNSUPPORTED).collect();

        vec![Insight {
            detector: "D4_ENTRY_PROFILE".to_string(),
            category: Category::Positioning,
            severity: cfg.severity.h14_unsupported_entry,
            confidence: 0.7,
            round: 0,
            player: tracked,
            title_data: json!({ "entries": entries }),
            metrics: json!({
                "entries": entries,
                "entry_wins": entry_wins,
                "supported": supported,
                "unsupported": unsupported,
                "team_entries": team_entries,
                "team_entry_wins": team_entry_wins,
                "non_trading_on_entries": non_trading_on_entries,
            }),
            evidence: unsupported_flags
                .iter()
                .take(8)
                .map(|f| f.evidence.clone())
                .collect(),
        }]
    }
}

/// One round's opening duel, resolved to its T-side "entry player".
struct EntryInfo {
    round: u32,
    tick: i32,
    entry_player: u64,
    opponent: u64,
    killer: u64,
    won: bool,
    supported: bool,
    nearest_teammate: Option<(u64, f32)>,
}

/// Every round's opening duel (spec: first kill with both sides known,
/// within the opening window), resolved to its entry player and support
/// status. Rounds with no qualifying kill, or with the entry player's
/// position unknown, are silently omitted (bias to silence).
fn round_entries(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<EntryInfo> {
    let mut out = vec![];
    for round in &ctx.data().rounds {
        let window_end = round.freeze_end_tick.unwrap_or(round.start_tick)
            + ctx.seconds(cfg.entry.opening_window_s);
        let opening = ctx
            .data()
            .kills
            .iter()
            .filter(|k| k.round == round.number)
            .filter(|k| {
                let Some(atk) = k.attacker else {
                    return false;
                };
                match (
                    ctx.side_of(atk, round.number),
                    ctx.side_of(k.victim, round.number),
                ) {
                    (Some(a), Some(v)) => a != v,
                    _ => false,
                }
            })
            .min_by_key(|k| k.tick);
        let Some(kill) = opening else {
            continue;
        };
        if kill.tick > window_end {
            continue;
        }
        let attacker = kill.attacker.expect("filtered above");
        let entry_player = if ctx.side_of(attacker, round.number) == Some(Side::T) {
            attacker
        } else {
            kill.victim
        };
        let opponent = if entry_player == attacker {
            kill.victim
        } else {
            attacker
        };
        let won = entry_player == attacker;
        let Some(me) = ctx.state_at(entry_player, kill.tick) else {
            continue;
        };
        let mates = ctx.teammates_alive_at(entry_player, round.number, kill.tick);
        let mut nearest: Option<(u64, f32)> = None;
        let mut supported = false;
        for (sid, st) in &mates {
            let d = AnalysisContext::dist(&me, st, cfg.general.z_weight);
            let same_place = me.place.is_some() && me.place == st.place;
            if d <= cfg.entry.support_distance_u || same_place {
                supported = true;
            }
            if nearest.is_none_or(|(_, nd)| d < nd) {
                nearest = Some((*sid, d));
            }
        }
        out.push(EntryInfo {
            round: round.number,
            tick: kill.tick,
            entry_player,
            opponent,
            killer: attacker,
            won,
            supported,
            nearest_teammate: nearest,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use cf_parser::model::MatchData;

    const TRACKED: u64 = 1;

    fn detect_all(data: &MatchData) -> Vec<RuleFlag> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H14EntryStructure.detect(&ctx, &DetectorConfig::default())
    }

    fn only<'a>(flags: &'a [RuleFlag], id: &str) -> Vec<&'a RuleFlag> {
        flags.iter().filter(|f| f.rule_id == id).collect()
    }

    fn insights_for(data: &MatchData, flags: &[RuleFlag]) -> Vec<Insight> {
        let ctx = AnalysisContext::new(data, TRACKED);
        H14EntryStructure.insights(&ctx, &DetectorConfig::default(), flags)
    }

    fn insights_from_scratch(data: &MatchData) -> Vec<Insight> {
        let flags = detect_all(data);
        insights_for(data, &flags)
    }

    /// Stationary alive player, optional `last_place`.
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

    // ---- entry detection ----

    #[test]
    fn entry_detection_uses_first_kill_only_second_kill_ignored() {
        // Round's first kill (tick 1500): CT 10 kills T 2 → entry player = 2
        // (not tracked). A later kill (tick 1700) has tracked (T) killing a
        // CT — if that were mistakenly treated as the opening duel, tracked
        // would fire unsupported (their only teammate, 2, is already dead).
        // It must not: only the round's first qualifying kill counts.
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        let s = hold_place(s, 2, 1000, 1500, 2000.0, None);
        let s = hold_place(s, 10, 1000, 3000, 500.0, None);
        let s = hold_place(s, 11, 1000, 3000, 900.0, None);
        let data = s
            .kill(10, 2, 1, 1500, "ak47")
            .kill(1, 11, 1, 1700, "ak47")
            .build();
        assert!(only(&detect_all(&data), UNSUPPORTED).is_empty());
    }

    #[test]
    fn unsupported_fires_when_entry_wins_and_names_nearest_teammate() {
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        let s = hold_place(s, 2, 1000, 3000, 1200.0, None); // unsupported (>700u, no shared place)
        let s = hold_place(s, 10, 1000, 3000, 50.0, None);
        let s = hold_place(s, 11, 1000, 3000, 900.0, None);
        let data = s.kill(1, 10, 1, 1500, "ak47").build(); // tracked (T) kills 10 (CT): win
        let flags = detect_all(&data);
        let f = only(&flags, UNSUPPORTED);
        assert_eq!(f.len(), 1);
        let f = f[0];
        assert_eq!(f.round, 1);
        assert_eq!(f.tick, 1500, "anchored on the opening-kill tick");
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.7).abs() < 0.001);
        let cfg = DetectorConfig::default();
        assert_eq!(f.severity, cfg.severity.h14_unsupported_entry);
        assert_eq!(f.details["won"], json!(true));
        assert_eq!(f.details["opponent"], json!("10"));
        assert_eq!(f.details["nearest_teammate"], json!("2"));
        assert!((f.details["distance"].as_f64().unwrap() - 1200.0).abs() < 1.0);
    }

    #[test]
    fn unsupported_fires_when_entry_loses() {
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        let s = hold_place(s, 2, 1000, 3000, 1200.0, None);
        let s = hold_place(s, 10, 1000, 3000, 50.0, None);
        let data = s.kill(10, 1, 1, 1500, "ak47").build(); // 10 (CT) kills tracked (T): loss
        let flags = detect_all(&data);
        let f = only(&flags, UNSUPPORTED);
        assert_eq!(f.len(), 1);
        let f = f[0];
        assert_eq!(f.tick, 1500);
        assert_eq!(f.details["won"], json!(false));
        assert_eq!(
            f.details["opponent"],
            json!("10"),
            "killer named as opponent"
        );
        assert_eq!(f.details["nearest_teammate"], json!("2"));
        assert!((f.details["distance"].as_f64().unwrap() - 1200.0).abs() < 1.0);
    }

    #[test]
    fn supported_suppressed_when_teammate_within_400u() {
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, None);
        let s = hold_place(s, 2, 1000, 3000, 400.0, None); // within 700u
        let s = hold_place(s, 10, 1000, 3000, 50.0, None);
        let data = s.kill(1, 10, 1, 1500, "ak47").build();
        assert!(only(&detect_all(&data), UNSUPPORTED).is_empty());
    }

    #[test]
    fn supported_suppressed_when_teammate_shares_last_place_despite_distance() {
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 0.0, Some("SiteA"));
        let s = hold_place(s, 2, 1000, 3000, 1200.0, Some("SiteA")); // far, but same place
        let s = hold_place(s, 10, 1000, 3000, 50.0, None);
        let data = s.kill(1, 10, 1, 1500, "ak47").build();
        assert!(only(&detect_all(&data), UNSUPPORTED).is_empty());
    }

    #[test]
    fn silent_when_tracked_was_not_the_entry_player() {
        // Round's opening duel is between teammate 2 and CT 10; tracked (1)
        // is not a participant at all.
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 5000);
        let s = hold_place(s, 1, 1000, 3000, 5000.0, None);
        let s = hold_place(s, 2, 1000, 3000, 0.0, None);
        let s = hold_place(s, 10, 1000, 3000, 50.0, None);
        let data = s.kill(2, 10, 1, 1500, "ak47").build();
        assert!(only(&detect_all(&data), UNSUPPORTED).is_empty());
    }

    fn three_round_t_side_base() -> Scenario {
        let s = Scenario::new("de_test")
            .players_ct(&[10, 11])
            .players_t(&[1, 2])
            .round(1, 1000, 3000)
            .round(2, 4000, 6000)
            .round(3, 7000, 9000);
        let s = hold_place(s, 1, 1000, 9000, 0.0, None);
        let s = hold_place(s, 2, 1000, 9000, 400.0, None);
        let s = hold_place(s, 10, 1000, 9000, 900.0, None);
        let s = hold_place(s, 11, 1000, 9000, 950.0, None);
        s.kill(1, 10, 1, 1500, "ak47")
            .kill(1, 10, 2, 4500, "ak47")
            .kill(1, 10, 3, 7500, "ak47")
    }

    #[test]
    fn ct_side_round_silent_for_flag_and_excluded_from_team_metrics() {
        // Rounds 1-3: tracked is T and the (supported) entry player each
        // round. Round 4: sides flip — tracked is CT, and dies as the
        // *opposing* side's opening duel; that round must not fire the flag
        // and must not count toward team_entries (tracked's team was not the
        // one entering).
        let s = three_round_t_side_base();
        let data = s
            .players_ct(&[1, 11])
            .players_t(&[10, 2])
            .round(4, 10000, 12000)
            .hold(10, 10000, 12000, 50.0, 0.0, 0.0)
            .hold(1, 10000, 12000, 0.0, 0.0, 0.0)
            .hold(2, 10000, 12000, 400.0, 0.0, 0.0)
            .kill(10, 1, 4, 10500, "ak47") // T (10, now T-side) kills tracked (now CT)
            .build();
        let flags = detect_all(&data);
        assert!(
            only(&flags, UNSUPPORTED).is_empty(),
            "flag silent: tracked was not the entry player"
        );
        let ins = insights_from_scratch(&data);
        assert_eq!(ins.len(), 1, "3 personal entries still gate the insight");
        assert_eq!(ins[0].metrics["team_entries"], json!(3), "round 4 excluded");
    }

    // ---- insights: gating ----

    fn entries_scenario(n: u32) -> MatchData {
        let mut s = Scenario::new("de_test")
            .players_ct(&[10])
            .players_t(&[1, 2]);
        for i in 0..n {
            let start = 1000 + i as i32 * 3000;
            let end = start + 2000;
            let tick = start + 500;
            s = s
                .round(i + 1, start, end)
                .hold(1, start, end, 0.0, 0.0, 0.0)
                .hold(2, start, end, 300.0, 0.0, 0.0)
                .hold(10, start, end, 900.0, 0.0, 0.0)
                .kill(1, 10, i + 1, tick, "ak47");
        }
        s.build()
    }

    #[test]
    fn insight_requires_at_least_three_entries() {
        let two = entries_scenario(2);
        assert!(
            insights_from_scratch(&two).is_empty(),
            "2 entries: no insight"
        );

        let three = entries_scenario(3);
        let ins = insights_from_scratch(&three);
        assert_eq!(ins.len(), 1, "3 entries: insight fires");
        let i = &ins[0];
        assert_eq!(i.detector, "D4_ENTRY_PROFILE");
        assert_eq!(i.category, Category::Positioning);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        let cfg = DetectorConfig::default();
        assert_eq!(i.severity, cfg.severity.h14_unsupported_entry);
        assert!((i.confidence - 0.7).abs() < 0.001);
    }

    // ---- insights: aggregate metrics ----

    #[test]
    fn insight_metrics_aggregate_all_fields() {
        let s = Scenario::new("de_test")
            .players_ct(&[10])
            .players_t(&[1, 2]);
        let s = s
            .round(1, 1000, 3000)
            .round(2, 4000, 6000)
            .round(3, 7000, 9000)
            .round(4, 10000, 12000);
        let s = hold_place(s, 1, 1000, 12000, 0.0, None);
        let s = hold_place(s, 10, 1000, 12000, 900.0, None);
        // teammate 2: close in round 1 (supported), far in rounds 2-3
        // (unsupported), position irrelevant in round 4 (its own entry).
        let s = hold_place(s, 2, 1000, 3000, 400.0, None);
        let s = hold_place(s, 2, 4000, 9000, 1200.0, None);
        let s = hold_place(s, 2, 10000, 12000, 50.0, None);
        let data = s
            .kill(1, 10, 1, 1500, "ak47") // tracked entry: win, supported
            .kill(10, 1, 2, 4500, "ak47") // tracked entry: loss, unsupported
            .kill(1, 10, 3, 7500, "ak47") // tracked entry: win, unsupported
            .kill(2, 10, 4, 10500, "ak47") // teammate entry: win
            .build();

        let flags = detect_all(&data);
        assert_eq!(only(&flags, UNSUPPORTED).len(), 2, "rounds 2 and 3");

        let ins = insights_for(&data, &flags);
        assert_eq!(ins.len(), 1);
        let m = &ins[0].metrics;
        assert_eq!(m["entries"], json!(3));
        assert_eq!(m["entry_wins"], json!(2));
        assert_eq!(m["supported"], json!(1));
        assert_eq!(m["unsupported"], json!(2));
        assert_eq!(m["team_entries"], json!(4));
        assert_eq!(m["team_entry_wins"], json!(3));
        assert_eq!(m["non_trading_on_entries"], json!(0));
        assert_eq!(ins[0].evidence.len(), 2, "unsupported-entry evidence only");
    }

    // ---- insights: non_trading_on_entries ----

    fn non_trading_base(tracked_r4_x: f32, mate_r4_x: f32) -> Scenario {
        let s = Scenario::new("de_test")
            .players_ct(&[10])
            .players_t(&[1, 2]);
        let s = s
            .round(1, 1000, 3000)
            .round(2, 4000, 6000)
            .round(3, 7000, 9000)
            .round(4, 10000, 12000);
        let s = hold_place(s, 1, 1000, 9000, 0.0, None);
        let s = hold_place(s, 2, 1000, 9000, 300.0, None);
        let s = hold_place(s, 10, 1000, 12000, 900.0, None);
        let s = hold_place(s, 1, 10000, 12000, tracked_r4_x, None);
        let s = hold_place(s, 2, 10000, 12000, mate_r4_x, None);
        s.kill(1, 10, 1, 1500, "ak47")
            .kill(1, 10, 2, 4500, "ak47")
            .kill(1, 10, 3, 7500, "ak47")
            .kill(10, 2, 4, 10500, "ak47") // teammate 2 dies as round-4 entry
    }

    #[test]
    fn non_trading_on_entries_counts_when_near_and_uncommitted() {
        let data = non_trading_base(1000.0, 1000.0).build();
        let ins = insights_from_scratch(&data);
        assert_eq!(ins.len(), 1);
        assert_eq!(ins[0].metrics["non_trading_on_entries"], json!(1));
    }

    #[test]
    fn non_trading_on_entries_excludes_when_tracked_commits() {
        let data = non_trading_base(1000.0, 1000.0)
            .shot(1, 10520, "weapon_ak47") // within the 2s commit window after 10500
            .build();
        let ins = insights_from_scratch(&data);
        assert_eq!(ins.len(), 1);
        assert_eq!(ins[0].metrics["non_trading_on_entries"], json!(0));
    }

    #[test]
    fn non_trading_on_entries_excludes_when_tracked_too_far() {
        let data = non_trading_base(1000.0, 3500.0).build(); // 2500u apart
        let ins = insights_from_scratch(&data);
        assert_eq!(ins.len(), 1);
        assert_eq!(ins[0].metrics["non_trading_on_entries"], json!(0));
    }
}
