//! Coaching-first stats (docs/spec/stats-and-understanding.md §1). Computed
//! from the same event stream the detectors read, reusing their helpers, so
//! a stat and its coaching rule never disagree. Per-player per-round rows
//! (the scoreboard) plus the tracked player's match totals (Task 2).

use serde::{Deserialize, Serialize};

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::families::h2::killed_in;
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
                            traded = killed_in(ctx, killer, k.tick, k.tick + commit_w);
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
}
