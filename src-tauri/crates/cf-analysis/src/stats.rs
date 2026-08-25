//! Coaching-first stats (docs/spec/stats-and-understanding.md §1). Computed
//! from the same event stream the detectors read, reusing their helpers, so
//! a stat and its coaching rule never disagree. Per-player per-round rows
//! (the scoreboard) plus the tracked player's match totals (Task 2).

use serde::{Deserialize, Serialize};

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::families::h14_entry::round_entries;
use crate::families::h2::killed_in;
use crate::play_ledger::RoundLedger;
use cf_parser::model::{Round, Side};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundPlayerStats {
    pub round: u32,
    pub steamid: u64,
    pub side: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub damage: u32,
    pub headshots: u32,
    pub survived: bool,
    pub traded: bool,
    pub entry: Option<String>,
}

fn side_str(s: Side) -> &'static str {
    match s {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

fn span_end(r: &Round) -> i32 {
    r.officially_ended_tick.unwrap_or(r.end_tick)
}

/// Rounds where the tracked player is on either roster.
pub fn rounds_played(ctx: &AnalysisContext) -> u32 {
    let t = ctx.tracked();
    ctx.data()
        .rounds
        .iter()
        .filter(|r| r.ct_steamids.contains(&t) || r.t_steamids.contains(&t))
        .count() as u32
}

/// One row per rostered player per round. `entry` is filled by
/// `match_stats` (Task 2) from H14's opening-duel finder.
pub fn round_player_rows(ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RoundPlayerStats> {
    let data = ctx.data();
    let commit_w = ctx.seconds(cfg.trade.commit_window_s);
    let mut out = vec![];
    for round in &data.rounds {
        let end = span_end(round);
        let roster: Vec<(u64, Side)> = round
            .ct_steamids
            .iter()
            .map(|s| (*s, Side::Ct))
            .chain(round.t_steamids.iter().map(|s| (*s, Side::T)))
            .collect();
        let side_of = |sid: u64| {
            roster
                .iter()
                .find(|(s, _)| *s == sid)
                .map(|(_, side)| *side)
        };
        for (sid, side) in &roster {
            let kills_in_round = data.kills.iter().filter(|k| k.round == round.number);
            let mut kills = 0;
            let mut headshots = 0;
            let mut assists = 0;
            let mut deaths = 0;
            let mut traded = false;
            for k in kills_in_round {
                let victim_enemy = side_of(k.victim).is_some_and(|vs| vs != *side);
                if k.attacker == Some(*sid) && k.victim != *sid && victim_enemy {
                    kills += 1;
                    if k.headshot {
                        headshots += 1;
                    }
                }
                if k.assister == Some(*sid) && victim_enemy {
                    assists += 1;
                }
                if k.victim == *sid {
                    deaths += 1;
                    if let Some(killer) = k.attacker.filter(|a| *a != *sid) {
                        if side_of(killer).is_some_and(|ks| ks != *side) {
                            traded = traded || killed_in(ctx, killer, k.tick, k.tick + commit_w);
                        }
                    }
                }
            }
            let damage: u32 = ctx
                .hurts_dealt_in(*sid, round.start_tick, end)
                .iter()
                .filter(|h| h.victim != *sid && side_of(h.victim).is_some_and(|vs| vs != *side))
                .map(|h| h.dmg_health.max(0) as u32)
                .sum();
            out.push(RoundPlayerStats {
                round: round.number,
                steamid: *sid,
                side: side_str(*side).to_string(),
                kills,
                deaths,
                assists,
                damage,
                headshots,
                survived: deaths == 0,
                traded,
                entry: None,
            });
        }
    }
    out
}

/// The tracked player's match totals (docs/spec/stats-and-understanding.md
/// §1): KAST, opening-duel entries (both sides, via H14's finder), trade
/// rate (from the play ledger) and clutch attempts/wins (kill-state
/// replay). Also fills `entry` on `rows` for BOTH opening-duel participants.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MatchStats {
    pub rounds_played: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub damage: u32,
    pub headshots: u32,
    pub kast_rounds: u32,
    pub entry_attempts: u32,
    pub entry_wins: u32,
    pub traded_deaths: u32,
    pub trade_kills: u32,
    pub trade_opportunities: u32,
    pub clutch_attempts: u32,
    pub clutch_wins: u32,
}

impl MatchStats {
    /// `None` when `deaths == 0` (would divide by zero) — the UI shows a bare kill count like "12-0" instead.
    pub fn kd(&self) -> Option<f32> {
        (self.deaths > 0).then(|| self.kills as f32 / self.deaths as f32)
    }
    /// `Some(0.0)` when rounds were played but no damage was dealt; `None` only when no rounds were played.
    pub fn adr(&self) -> Option<f32> {
        (self.rounds_played > 0)
            .then(|| (self.damage as f32 / self.rounds_played as f32 * 10.0).round() / 10.0)
    }
    /// `None` when there are no kills to take a headshot share of.
    pub fn hs_pct(&self) -> Option<u32> {
        (self.kills > 0).then(|| (self.headshots as f32 / self.kills as f32 * 100.0).round() as u32)
    }
    /// `None` when no rounds were played.
    pub fn kast_pct(&self) -> Option<u32> {
        (self.rounds_played > 0)
            .then(|| (self.kast_rounds as f32 / self.rounds_played as f32 * 100.0).round() as u32)
    }
}

/// Was the tracked player, at any point this round, the last one alive on
/// their side with at least one enemy alive? Kill-event state replay (the
/// ADR-0008 approach): rosters minus victims in tick order.
fn clutch_state(
    round: &Round,
    kills: &[&cf_parser::model::Kill],
    tracked: u64,
    side: Side,
) -> Option<u32> {
    let (mine, theirs) = match side {
        Side::Ct => (&round.ct_steamids, &round.t_steamids),
        Side::T => (&round.t_steamids, &round.ct_steamids),
    };
    let mut my_alive: Vec<u64> = mine.clone();
    let mut their_alive: Vec<u64> = theirs.clone();
    let mut sorted: Vec<&cf_parser::model::Kill> = kills.to_vec();
    sorted.sort_by_key(|k| k.tick);
    let mut best: Option<u32> = None;
    let check = |my: &Vec<u64>, th: &Vec<u64>, best: &mut Option<u32>| {
        if my.len() == 1 && my[0] == tracked && !th.is_empty() {
            let n = th.len() as u32;
            *best = Some(best.map_or(n, |b| b.max(n)));
        }
    };
    check(&my_alive, &their_alive, &mut best);
    for k in sorted {
        my_alive.retain(|s| *s != k.victim);
        their_alive.retain(|s| *s != k.victim);
        check(&my_alive, &their_alive, &mut best);
    }
    best
}

/// Fills `entry` on both opening-duel participants' rows and totals the
/// tracked player's match stats. Pure: reuses H14's `round_entries` (no
/// re-implementing opening-duel logic) and the play ledger's trade/
/// missed_trade plays (no re-implementing trade logic).
pub fn match_stats(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    rows: &mut [RoundPlayerStats],
    ledger: &[RoundLedger],
) -> MatchStats {
    let data = ctx.data();
    let tracked = ctx.tracked();
    let mut s = MatchStats {
        rounds_played: rounds_played(ctx),
        ..Default::default()
    };
    // Opening duels (H14's finder, both sides): mark both rows, count the
    // tracked player's attempts/wins.
    for e in round_entries(ctx, cfg) {
        let (winner, loser) = (
            e.killer,
            if e.killer == e.entry_player {
                e.opponent
            } else {
                e.entry_player
            },
        );
        for r in rows.iter_mut().filter(|r| r.round == e.round) {
            if r.steamid == winner {
                r.entry = Some("win".to_string());
            } else if r.steamid == loser {
                r.entry = Some("loss".to_string());
            }
        }
        if winner == tracked || loser == tracked {
            s.entry_attempts += 1;
            if winner == tracked {
                s.entry_wins += 1;
            }
        }
    }
    for r in rows.iter().filter(|r| r.steamid == tracked) {
        s.kills += r.kills;
        s.deaths += r.deaths;
        s.assists += r.assists;
        s.damage += r.damage;
        s.headshots += r.headshots;
        if r.kills > 0 || r.assists > 0 || r.survived || r.traded {
            s.kast_rounds += 1;
        }
        if r.traded {
            s.traded_deaths += 1;
        }
    }
    for l in ledger {
        for p in &l.plays {
            match p.kind.as_str() {
                "trade" => {
                    s.trade_kills += 1;
                    s.trade_opportunities += 1;
                }
                "missed_trade" => s.trade_opportunities += 1,
                _ => {}
            }
        }
    }
    for round in &data.rounds {
        let Some(side) = ctx.side_of(tracked, round.number) else {
            continue;
        };
        let kills: Vec<&cf_parser::model::Kill> = data
            .kills
            .iter()
            .filter(|k| k.round == round.number)
            .collect();
        if clutch_state(round, &kills, tracked, side).is_some() {
            s.clutch_attempts += 1;
            if round.winner == side {
                s.clutch_wins += 1;
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use crate::DetectorConfig;

    const ME: u64 = 1;
    const MATE: u64 = 2;
    const E1: u64 = 9;
    const E2: u64 = 10;

    fn base() -> Scenario {
        Scenario::new("de_mirage")
            .players_ct(&[ME, MATE])
            .players_t(&[E1, E2])
            .round(1, 1000, 5000)
            .round(2, 6000, 9000)
            .hold(ME, 1000, 9000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 9000, 500.0, 0.0, 0.0)
            .hold(E1, 1000, 9000, 3000.0, 0.0, 0.0)
            .hold(E2, 1000, 9000, 3200.0, 0.0, 0.0)
    }

    fn rows_for(data: &cf_parser::model::MatchData) -> Vec<RoundPlayerStats> {
        let ctx = AnalysisContext::new(data, ME);
        round_player_rows(&ctx, &DetectorConfig::default())
    }

    fn row(rows: &[RoundPlayerStats], round: u32, sid: u64) -> &RoundPlayerStats {
        rows.iter()
            .find(|r| r.round == round && r.steamid == sid)
            .expect("row")
    }

    #[test]
    fn one_row_per_rostered_player_per_round_with_sides() {
        let rows = rows_for(&base().build());
        assert_eq!(rows.len(), 8);
        assert_eq!(row(&rows, 1, ME).side, "CT");
        assert_eq!(row(&rows, 2, E1).side, "T");
        assert!(row(&rows, 1, ME).survived);
        assert_eq!(row(&rows, 1, ME).kills, 0);
    }

    #[test]
    fn kills_assists_damage_headshots_and_teamkills() {
        let data = base()
            .hurt(ME, E1, 2000, 60, "weapon_ak47")
            .hurt(ME, E1, 2010, 40, "weapon_ak47")
            .hurt(ME, MATE, 2020, 25, "weapon_ak47") // team damage: never counted
            .kill_full(Some(ME), E1, 1, 2010, "weapon_ak47", false, 0)
            .kill(ME, MATE, 1, 2500, "weapon_ak47") // teamkill: not a kill, MATE's death
            .build();
        // make the E1 kill a headshot
        let mut data = data;
        data.kills
            .iter_mut()
            .find(|k| k.victim == E1)
            .unwrap()
            .headshot = true;
        data.kills
            .iter_mut()
            .find(|k| k.victim == E1)
            .unwrap()
            .assister = Some(MATE);
        let rows = rows_for(&data);
        let me = row(&rows, 1, ME);
        assert_eq!(
            (me.kills, me.headshots, me.damage, me.deaths),
            (1, 1, 100, 0)
        );
        let mate = row(&rows, 1, MATE);
        assert_eq!((mate.assists, mate.deaths, mate.survived), (1, 1, false));
        let e1 = row(&rows, 1, E1);
        assert_eq!((e1.deaths, e1.survived), (1, false));
    }

    #[test]
    fn traded_death_uses_h2s_commit_window() {
        let data = base()
            .kill(E1, ME, 1, 3000, "weapon_ak47")
            .kill(MATE, E1, 1, 3100, "weapon_ak47") // 100 ticks < 128 (2 s at 64)
            .build();
        let rows = rows_for(&data);
        assert!(row(&rows, 1, ME).traded);
        let late = base()
            .kill(E1, ME, 1, 3000, "weapon_ak47")
            .kill(MATE, E1, 1, 3200, "weapon_ak47")
            .build();
        assert!(!row(&rows_for(&late), 1, ME).traded);
    }

    #[test]
    fn rounds_played_counts_rostered_rounds_only() {
        let data = base().build();
        let ctx = AnalysisContext::new(&data, ME);
        assert_eq!(rounds_played(&ctx), 2);
        let spectator = Scenario::new("de_mirage")
            .players_ct(&[MATE])
            .players_t(&[E1])
            .round(1, 1000, 5000)
            .build();
        let ctx = AnalysisContext::new(&spectator, ME);
        assert_eq!(rounds_played(&ctx), 0);
    }

    #[test]
    fn world_and_self_kills_are_deaths_only() {
        // World kill (attacker None) and a self-kill (attacker == victim):
        // both count as a death for the victim, never a kill, never traded.
        let data = base()
            .kill_full(None, ME, 1, 3000, "world", false, 0)
            .kill_full(Some(ME), ME, 2, 7000, "weapon_hegrenade", false, 0)
            .build();
        let rows = rows_for(&data);
        let r1 = row(&rows, 1, ME);
        assert_eq!((r1.kills, r1.deaths, r1.traded), (0, 1, false));
        let r2 = row(&rows, 2, ME);
        assert_eq!((r2.kills, r2.deaths, r2.traded), (0, 1, false));
    }

    use crate::play_ledger::build_ledger;

    fn stats_for(data: &cf_parser::model::MatchData) -> (MatchStats, Vec<RoundPlayerStats>) {
        let ctx = AnalysisContext::new(data, ME);
        let cfg = DetectorConfig::default();
        let mut rows = round_player_rows(&ctx, &cfg);
        let ledger = build_ledger(&ctx, &cfg, &[]);
        let s = match_stats(&ctx, &cfg, &mut rows, &ledger);
        (s, rows)
    }

    #[test]
    fn totals_and_derived_ratios() {
        let data = base()
            .kill(ME, E1, 1, 2000, "weapon_ak47")
            .hurt(ME, E1, 1990, 100, "weapon_ak47")
            .kill(E2, ME, 1, 2500, "weapon_ak47")
            .kill(ME, E1, 2, 7000, "weapon_ak47")
            .build();
        let (s, _) = stats_for(&data);
        assert_eq!(
            (s.rounds_played, s.kills, s.deaths, s.damage),
            (2, 2, 1, 100)
        );
        assert_eq!(s.kd(), Some(2.0));
        assert_eq!(s.adr(), Some(50.0));
        assert_eq!(s.hs_pct(), Some(0));
        let (empty, _) = stats_for(&base().build());
        assert_eq!(empty.kd(), None);
        assert_eq!(empty.hs_pct(), None);
        assert_eq!(empty.adr(), Some(0.0));
    }

    #[test]
    fn kast_counts_kill_assist_survival_or_traded_death() {
        // R1: died untraded, no kill/assist -> not KAST. R2: survived -> KAST.
        let data = base().kill(E1, ME, 1, 3000, "weapon_ak47").build();
        let (s, _) = stats_for(&data);
        assert_eq!((s.kast_rounds, s.kast_pct()), (1, Some(50)));
        let traded = base()
            .kill(E1, ME, 1, 3000, "weapon_ak47")
            .kill(MATE, E1, 1, 3060, "weapon_ak47")
            .build();
        assert_eq!(stats_for(&traded).0.kast_rounds, 2);
    }

    #[test]
    fn entry_uses_h14s_opening_duel_on_both_sides_and_marks_both_rows() {
        // R1 opening duel within 15 s: ME (CT) kills E1 -> attempt + win; E1 row gets "loss".
        // R2: first kill after the window -> no entry.
        let data = base()
            .kill(ME, E1, 1, 1000 + 64 * 10, "weapon_ak47")
            .kill(E2, ME, 2, 6000 + 64 * 40, "weapon_ak47")
            .build();
        let (s, rows) = stats_for(&data);
        assert_eq!((s.entry_attempts, s.entry_wins), (1, 1));
        assert_eq!(row(&rows, 1, ME).entry.as_deref(), Some("win"));
        assert_eq!(row(&rows, 1, E1).entry.as_deref(), Some("loss"));
        assert_eq!(row(&rows, 2, ME).entry, None);
        // Losing the opening duel is an attempt too.
        let lost = base().kill(E1, ME, 1, 1000 + 64 * 5, "weapon_ak47").build();
        let (s, rows) = stats_for(&lost);
        assert_eq!((s.entry_attempts, s.entry_wins), (1, 0));
        assert_eq!(row(&rows, 1, ME).entry.as_deref(), Some("loss"));
    }

    #[test]
    fn trade_rate_comes_from_the_ledger_and_the_death_rows() {
        let data = base()
            .kill(E1, MATE, 1, 3000, "weapon_ak47") // teammate dies 500 u from me
            .kill(ME, E1, 1, 3060, "weapon_ak47") // I trade -> trade play (Good)
            .kill(E2, MATE, 2, 7000, "weapon_ak47") // teammate dies, I do nothing -> missed_trade
            .kill(E2, ME, 2, 7500, "weapon_ak47") // my untraded death
            .build();
        let (s, _) = stats_for(&data);
        assert_eq!((s.trade_kills, s.trade_opportunities), (1, 2));
        assert_eq!((s.traded_deaths, s.deaths), (0, 1));
    }

    #[test]
    fn clutch_attempts_and_wins_from_the_kill_state_replay() {
        // R1: MATE dies at 2000 -> ME is last alive vs 2 (1v2); ME kills both -> win.
        // R2: ME dies first -> never a clutch.
        let data = base()
            .kill(E1, MATE, 1, 2000, "weapon_ak47")
            .kill(ME, E1, 1, 2500, "weapon_ak47")
            .kill(ME, E2, 1, 2600, "weapon_ak47")
            .kill(E1, ME, 2, 6500, "weapon_ak47")
            .build();
        let (s, _) = stats_for(&data);
        assert_eq!((s.clutch_attempts, s.clutch_wins), (1, 1));
        let lost = base()
            .kill(E1, MATE, 1, 2000, "weapon_ak47")
            .kill(E1, ME, 1, 2500, "weapon_ak47")
            .round_won_by(1, cf_parser::model::Side::T)
            .build();
        let (s, _) = stats_for(&lost);
        assert_eq!((s.clutch_attempts, s.clutch_wins), (1, 0));
    }

    #[test]
    fn clutch_attempt_when_starting_the_round_alone() {
        // ME is already alone against two enemies with no kills at all --
        // this exercises clutch_state's pre-loop check (the initial-roster
        // case), not the tick-by-tick replay.
        let data = Scenario::new("de_mirage")
            .players_ct(&[ME])
            .players_t(&[E1, E2])
            .round(1, 1000, 5000)
            .hold(ME, 1000, 5000, 0.0, 0.0, 0.0)
            .hold(E1, 1000, 5000, 3000.0, 0.0, 0.0)
            .hold(E2, 1000, 5000, 3200.0, 0.0, 0.0)
            .build();
        let (s, _) = stats_for(&data);
        assert_eq!(s.clutch_attempts, 1);
    }

    #[test]
    fn traded_accumulates_across_duplicate_victim_rows() {
        // Two Kill records with victim ME in one round: the first
        // (E1's kill) is never traded, the second (E2's kill) is. `traded`
        // is accumulated with OR across all of a sid's victim rows, so the
        // row ends up traded even though the first record alone would not.
        let data = base()
            .kill_full(Some(E1), ME, 1, 3000, "weapon_ak47", false, 0)
            .kill_full(Some(E2), ME, 1, 3100, "weapon_ak47", false, 0)
            .kill(MATE, E2, 1, 3150, "weapon_ak47")
            .build();
        assert!(row(&rows_for(&data), 1, ME).traded);
    }
}
