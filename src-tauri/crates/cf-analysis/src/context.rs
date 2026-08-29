//! AnalysisContext: prebuilt indexes + spatial/temporal helpers over one
//! match. Built once, shared by every rule family. Pure — no I/O.

use std::collections::HashMap;

use cf_parser::model::{Blind, Hurt, InventorySample, Kill, MatchData, Reload, Shot, Side};

use crate::config::PhaseCfg;

/// Where in the round a tick falls. Post-plant is a state, not a time:
/// once the bomb is down every prior boundary is irrelevant to coaching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundPhase {
    Freeze,
    Opening,
    Mid,
    Late,
    PostPlant,
}

/// A player's state at (or just before) a tick, from the 16 Hz sample table.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub tick: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub health: i32,
    pub is_alive: bool,
    pub team_num: i32,
    pub weapon: Option<String>,
    pub place: Option<String>,
    pub is_scoped: bool,
}

struct PlayerTrack {
    ticks: Vec<i32>,
    rows: Vec<usize>, // indexes into the TickTable columns
}

pub struct AnalysisContext<'a> {
    data: &'a MatchData,
    tracked: u64,
    tracks: HashMap<u64, PlayerTrack>,
    shots_by_player: HashMap<u64, Vec<&'a Shot>>,
    reloads_by_player: HashMap<u64, Vec<&'a Reload>>,
    hurts_by_victim: HashMap<u64, Vec<&'a Hurt>>,
    hurts_by_attacker: HashMap<u64, Vec<&'a Hurt>>,
    blinds_by_victim: HashMap<u64, Vec<&'a Blind>>,
    inventories: HashMap<u64, Vec<&'a InventorySample>>,
    sides: HashMap<(u32, u64), Side>,
}

impl<'a> AnalysisContext<'a> {
    pub fn new(data: &'a MatchData, tracked: u64) -> Self {
        let mut tracks: HashMap<u64, PlayerTrack> = HashMap::new();
        let t = &data.ticks;
        for i in 0..t.len() {
            let tr = tracks.entry(t.steamid[i]).or_insert_with(|| PlayerTrack {
                ticks: vec![],
                rows: vec![],
            });
            tr.ticks.push(t.tick[i]);
            tr.rows.push(i);
        }
        let mut shots_by_player: HashMap<u64, Vec<&Shot>> = HashMap::new();
        for s in &data.shots {
            shots_by_player.entry(s.player).or_default().push(s);
        }
        let mut reloads_by_player: HashMap<u64, Vec<&Reload>> = HashMap::new();
        for r in &data.reloads {
            reloads_by_player.entry(r.player).or_default().push(r);
        }
        let mut hurts_by_victim: HashMap<u64, Vec<&Hurt>> = HashMap::new();
        let mut hurts_by_attacker: HashMap<u64, Vec<&Hurt>> = HashMap::new();
        for h in &data.hurts {
            hurts_by_victim.entry(h.victim).or_default().push(h);
            if let Some(a) = h.attacker {
                hurts_by_attacker.entry(a).or_default().push(h);
            }
        }
        let mut blinds_by_victim: HashMap<u64, Vec<&Blind>> = HashMap::new();
        for b in &data.blinds {
            blinds_by_victim.entry(b.victim).or_default().push(b);
        }
        let mut inventories: HashMap<u64, Vec<&InventorySample>> = HashMap::new();
        for inv in &data.inventories {
            inventories.entry(inv.steamid).or_default().push(inv);
        }
        for v in inventories.values_mut() {
            v.sort_by_key(|i| i.tick);
        }
        let mut sides = HashMap::new();
        for r in &data.rounds {
            for s in &r.ct_steamids {
                sides.insert((r.number, *s), Side::Ct);
            }
            for s in &r.t_steamids {
                sides.insert((r.number, *s), Side::T);
            }
        }
        AnalysisContext {
            data,
            tracked,
            tracks,
            shots_by_player,
            reloads_by_player,
            hurts_by_victim,
            hurts_by_attacker,
            blinds_by_victim,
            inventories,
            sides,
        }
    }

    pub fn data(&self) -> &MatchData {
        self.data
    }

    pub fn tracked(&self) -> u64 {
        self.tracked
    }

    pub fn seconds(&self, s: f32) -> i32 {
        (s * self.data.tickrate).round() as i32
    }

    /// Nearest sample at or before `tick` (None before the first sample).
    pub fn state_at(&self, steamid: u64, tick: i32) -> Option<PlayerState> {
        let tr = self.tracks.get(&steamid)?;
        let idx = tr.ticks.partition_point(|t| *t <= tick);
        if idx == 0 {
            return None;
        }
        let row = tr.rows[idx - 1];
        let t = &self.data.ticks;
        Some(PlayerState {
            tick: t.tick[row],
            x: t.x[row],
            y: t.y[row],
            z: t.z[row],
            yaw: t.yaw[row],
            health: t.health[row],
            is_alive: t.is_alive[row],
            team_num: t.team_num[row],
            weapon: t.active_weapon[row].clone(),
            place: t.last_place[row].clone(),
            is_scoped: t.is_scoped.get(row).copied().unwrap_or(false),
        })
    }

    pub fn side_of(&self, steamid: u64, round: u32) -> Option<Side> {
        self.sides.get(&(round, steamid)).copied()
    }

    /// 3D distance with the configured vertical weight applied to Δz
    /// (spec H2 refinement: 800 u apart with 200 u height difference is not
    /// tradeable spacing).
    pub fn dist(a: &PlayerState, b: &PlayerState, z_weight: f32) -> f32 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = (a.z - b.z) * z_weight;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// `steamid`'s samples across [t0, t1], stepping `step` ticks and always
    /// ending on `t1`. None when the player has no sample at `t0` — a rule
    /// that cannot see the start of its window stays silent (spec §4.1).
    pub fn samples_in(
        &self,
        steamid: u64,
        t0: i32,
        t1: i32,
        step: i32,
    ) -> Option<Vec<PlayerState>> {
        let mut out = vec![self.state_at(steamid, t0)?];
        let step = step.max(1);
        let mut t = t0 + step;
        while t < t1 {
            if let Some(st) = self.state_at(steamid, t) {
                out.push(st);
            }
            t += step;
        }
        if t1 > t0 {
            out.extend(self.state_at(steamid, t1));
        }
        Some(out)
    }

    /// Ground covered along a sampled track, same vertical weighting as
    /// `dist`.
    pub fn path_length(track: &[PlayerState], z_weight: f32) -> f32 {
        track
            .windows(2)
            .map(|w| Self::dist(&w[0], &w[1], z_weight))
            .sum()
    }

    pub fn teammates_alive_at(
        &self,
        steamid: u64,
        round: u32,
        tick: i32,
    ) -> Vec<(u64, PlayerState)> {
        self.on_side_alive_at(steamid, round, tick, true)
    }

    pub fn enemies_alive_at(&self, steamid: u64, round: u32, tick: i32) -> Vec<(u64, PlayerState)> {
        self.on_side_alive_at(steamid, round, tick, false)
    }

    fn on_side_alive_at(
        &self,
        steamid: u64,
        round: u32,
        tick: i32,
        same: bool,
    ) -> Vec<(u64, PlayerState)> {
        let Some(my_side) = self.side_of(steamid, round) else {
            return vec![];
        };
        self.data
            .players
            .iter()
            .filter(|p| p.steamid != steamid)
            .filter(|p| {
                self.side_of(p.steamid, round)
                    .is_some_and(|s| (s == my_side) == same)
            })
            .filter_map(|p| {
                let st = self.state_at(p.steamid, tick)?;
                st.is_alive.then_some((p.steamid, st))
            })
            .collect()
    }

    pub fn nearest_teammate(
        &self,
        steamid: u64,
        round: u32,
        tick: i32,
        z_weight: f32,
    ) -> Option<(u64, f32)> {
        let me = self.state_at(steamid, tick)?;
        self.teammates_alive_at(steamid, round, tick)
            .into_iter()
            .map(|(sid, st)| (sid, Self::dist(&me, &st, z_weight)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    pub fn nearest_enemy(
        &self,
        steamid: u64,
        round: u32,
        tick: i32,
        z_weight: f32,
    ) -> Option<(u64, f32)> {
        let me = self.state_at(steamid, tick)?;
        self.enemies_alive_at(steamid, round, tick)
            .into_iter()
            .map(|(sid, st)| (sid, Self::dist(&me, &st, z_weight)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    fn window<'b, T>(
        items: Option<&'b Vec<&'a T>>,
        tick_of: impl Fn(&T) -> i32,
        t0: i32,
        t1: i32,
    ) -> Vec<&'b &'a T> {
        match items {
            Some(v) => v
                .iter()
                .filter(|i| {
                    let t = tick_of(i);
                    t >= t0 && t <= t1
                })
                .collect(),
            None => vec![],
        }
    }

    pub fn shots_by_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&&'a Shot> {
        Self::window(self.shots_by_player.get(&steamid), |s| s.tick, t0, t1)
    }

    pub fn reloads_by_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&&'a Reload> {
        Self::window(self.reloads_by_player.get(&steamid), |r| r.tick, t0, t1)
    }

    pub fn hurts_taken_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&&'a Hurt> {
        Self::window(self.hurts_by_victim.get(&steamid), |h| h.tick, t0, t1)
    }

    pub fn hurts_dealt_in(&self, steamid: u64, t0: i32, t1: i32) -> Vec<&&'a Hurt> {
        Self::window(self.hurts_by_attacker.get(&steamid), |h| h.tick, t0, t1)
    }

    /// The enemy-attributed blind window covering `tick`, if any.
    pub fn blind_window_at(&self, steamid: u64, tick: i32) -> Option<&'a Blind> {
        let blinds = self.blinds_by_victim.get(&steamid)?;
        blinds
            .iter()
            .filter(|b| b.tick <= tick && tick <= b.tick + (b.duration * self.data.tickrate) as i32)
            .max_by_key(|b| b.tick)
            .copied()
    }

    /// Exact-tick inventory sample (present only at death/round-end ticks).
    /// Nearest inventory sample at-or-shortly-before `tick`. Samples exist
    /// only at targeted ticks (deaths, ~0.25 s pre-death, round ends); the
    /// half-second lookback covers the pre-death sample without ever reading
    /// a different moment's inventory. At the death tick itself the victim's
    /// items are already dropped — hence the pre-death sampling.
    /// Empty samples are skipped: a living player always holds at least a
    /// knife, so `[]` is the already-dropped-items death artifact (verified
    /// on real demos — the death-tick sample is always empty).
    pub fn inventory_at(&self, steamid: u64, tick: i32) -> Option<&'a InventorySample> {
        let max_back = (0.5 * self.data.tickrate) as i32;
        let v = self.inventories.get(&steamid)?;
        let idx = v.partition_point(|i| i.tick <= tick);
        v[..idx]
            .iter()
            .rev()
            .take_while(|i| tick - i.tick <= max_back)
            .find(|i| !i.items.is_empty())
            .copied()
    }

    /// (CT alive, T alive) at `tick`, replayed from kill events over the
    /// round's rosters — never tick samples, so a synthetic or sparsely
    /// sampled track can't misreport a death (a kill on the tick itself is
    /// already counted; callers wanting the board *before* a death pass
    /// `kill.tick - 1`). None when the round number is unknown.
    pub fn alive_counts_at(&self, round: u32, tick: i32) -> Option<(usize, usize)> {
        let r = self.data.rounds.iter().find(|r| r.number == round)?;
        let alive = |roster: &[u64]| {
            let dead = self
                .data
                .kills
                .iter()
                .filter(|k| k.round == round && k.tick <= tick && roster.contains(&k.victim))
                .count();
            roster.len().saturating_sub(dead)
        };
        Some((alive(&r.ct_steamids), alive(&r.t_steamids)))
    }

    /// Bodies up (+) or down (−) at `tick` from `steamid`'s side. None when
    /// the player has no side in the round, or the round is unknown.
    pub fn man_advantage(&self, steamid: u64, round: u32, tick: i32) -> Option<i32> {
        let (ct, t) = self.alive_counts_at(round, tick)?;
        let (mine, theirs) = match self.side_of(steamid, round)? {
            Side::Ct => (ct, t),
            Side::T => (t, ct),
        };
        Some(mine as i32 - theirs as i32)
    }

    pub fn kill_of(&self, victim: u64, round: u32) -> Option<&'a Kill> {
        self.data
            .kills
            .iter()
            .find(|k| k.victim == victim && k.round == round)
    }

    /// All deaths of the tracked player, in tick order.
    pub fn tracked_deaths(&self) -> Vec<&'a Kill> {
        self.data
            .kills
            .iter()
            .filter(|k| k.victim == self.tracked)
            .collect()
    }

    /// Phase of `round` at `tick`. None when the round number is unknown.
    /// Freeze-end falls back to start_tick for rounds where the demo lacks
    /// the freeze event (parser normalizes most, not all).
    pub fn phase_at(&self, round: u32, tick: i32, cfg: &PhaseCfg) -> Option<RoundPhase> {
        let r = self.data.rounds.iter().find(|r| r.number == round)?;
        let span_end = r.officially_ended_tick.unwrap_or(r.end_tick);
        let planted = self
            .data
            .bomb_events
            .iter()
            .find(|b| b.kind == "planted" && b.tick >= r.start_tick && b.tick <= span_end);
        if let Some(p) = planted {
            if tick >= p.tick {
                return Some(RoundPhase::PostPlant);
            }
        }
        let freeze_end = r.freeze_end_tick.unwrap_or(r.start_tick);
        if tick < freeze_end {
            return Some(RoundPhase::Freeze);
        }
        let elapsed = tick - freeze_end;
        if elapsed < self.seconds(cfg.opening_end_s) {
            Some(RoundPhase::Opening)
        } else if elapsed < self.seconds(cfg.mid_end_s) {
            Some(RoundPhase::Mid)
        } else {
            Some(RoundPhase::Late)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;

    fn ctx_fixture() -> MatchData {
        Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .waypoint(1, 1100, 0.0, 0.0, 0.0)
            .waypoint(1, 2000, 900.0, 0.0, 0.0)
            .waypoint(2, 1100, 100.0, 0.0, 0.0)
            .waypoint(3, 1100, 2000.0, 0.0, 0.0)
            .shot(1, 1500, "weapon_ak47")
            .hurt(1, 3, 1520, 27, "ak47")
            .build()
    }

    #[test]
    fn state_at_returns_nearest_sample_at_or_before() {
        let data = ctx_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert!(ctx.state_at(1, 1099).is_none() || ctx.state_at(1, 1099).unwrap().tick <= 1099);
        let s = ctx.state_at(1, 1500).unwrap();
        assert!(s.tick <= 1500);
        assert!(s.x >= 0.0 && s.x <= 900.0);
        let exact = ctx.state_at(1, 2000).unwrap();
        assert_eq!(exact.x, 900.0);
    }

    #[test]
    fn nearest_teammate_and_enemy_with_z_weight() {
        let data = ctx_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        let (mate, d) = ctx.nearest_teammate(1, 1, 1100, 2.0).unwrap();
        assert_eq!(mate, 2);
        assert!((d - 100.0).abs() < 1.0);
        let (enemy, de) = ctx.nearest_enemy(1, 1, 1100, 2.0).unwrap();
        assert_eq!(enemy, 3);
        assert!((de - 2000.0).abs() < 1.0);
    }

    #[test]
    fn dist_weights_vertical_difference() {
        let a = PlayerState {
            tick: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            health: 100,
            is_alive: true,
            team_num: 3,
            weapon: None,
            place: None,
            is_scoped: false,
        };
        let mut b = a.clone();
        b.z = 200.0;
        assert!((AnalysisContext::dist(&a, &b, 2.0) - 400.0).abs() < 0.01);
    }

    #[test]
    fn event_windows_slice_correctly() {
        let data = ctx_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert_eq!(ctx.shots_by_in(1, 1400, 1600).len(), 1);
        assert_eq!(ctx.shots_by_in(1, 1501, 1600).len(), 0);
        assert_eq!(ctx.hurts_dealt_in(1, 1500, 1540).len(), 1);
        assert_eq!(ctx.hurts_taken_in(3, 1500, 1540).len(), 1);
    }

    /// CT [1,2] vs T [3,4]; T3 dies at 1500, CT1 at 2000.
    fn alive_fixture() -> MatchData {
        Scenario::new("de_test")
            .players_ct(&[1, 2])
            .players_t(&[3, 4])
            .round(1, 1000, 5000)
            .kill(1, 3, 1, 1500, "ak47")
            .kill(4, 1, 1, 2000, "ak47")
            .build()
    }

    #[test]
    fn alive_counts_replay_kill_events_over_the_round_rosters() {
        let data = alive_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert_eq!(ctx.alive_counts_at(1, 1400), Some((2, 2)));
        assert_eq!(
            ctx.alive_counts_at(1, 1500),
            Some((2, 1)),
            "a kill on the tick itself is already counted"
        );
        assert_eq!(ctx.alive_counts_at(1, 1999), Some((2, 1)));
        assert_eq!(ctx.alive_counts_at(1, 2000), Some((1, 1)));
        assert_eq!(ctx.alive_counts_at(7, 1500), None, "unknown round");
    }

    #[test]
    fn man_advantage_is_mine_minus_theirs_from_the_players_side() {
        let data = alive_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert_eq!(ctx.man_advantage(1, 1, 1999), Some(1), "CT up one body");
        assert_eq!(
            ctx.man_advantage(4, 1, 1999),
            Some(-1),
            "same board, T side"
        );
        assert_eq!(ctx.man_advantage(1, 1, 2000), Some(0), "after the death");
        assert_eq!(
            ctx.man_advantage(99, 1, 1999),
            None,
            "a player with no side in the round is silent"
        );
    }

    #[test]
    fn samples_in_walks_the_window_and_always_ends_on_the_last_tick() {
        // Player 1 walks 0 -> 640 u over ticks 1000..1064.
        let data = Scenario::new("de_test")
            .players_ct(&[1])
            .players_t(&[2])
            .round(1, 1000, 5000)
            .waypoint(1, 1000, 0.0, 0.0, 0.0)
            .waypoint(1, 1064, 640.0, 0.0, 0.0)
            .build();
        let ctx = AnalysisContext::new(&data, 1);
        let walk = ctx.samples_in(1, 1000, 1064, 16).expect("samples");
        assert_eq!(
            walk.iter().map(|s| s.tick).collect::<Vec<_>>(),
            vec![1000, 1016, 1032, 1048, 1064]
        );
        assert!((AnalysisContext::path_length(&walk, 2.0) - 640.0).abs() < 1.0);

        // A step that doesn't divide the window still ends on t1.
        let ragged = ctx.samples_in(1, 1000, 1060, 16).expect("samples");
        assert_eq!(ragged.last().map(|s| s.tick), Some(1060));

        assert!(
            ctx.samples_in(1, 900, 1064, 16).is_none(),
            "no sample at the window start is silence, not a guess"
        );
    }

    #[test]
    fn seconds_converts_via_tickrate() {
        let data = ctx_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert_eq!(ctx.seconds(2.0), 128);
    }
}

#[cfg(test)]
mod phase_tests {
    use super::*;
    use crate::config::DetectorConfig;
    use crate::scenario::Scenario;

    const P: u64 = 1;

    #[test]
    fn phase_at_walks_freeze_opening_mid_late_and_post_plant() {
        // round(1, 1000, 20000) sets freeze_end = Some(1000).
        let data = Scenario::new("de_test")
            .players_ct(&[P])
            .players_t(&[2])
            .round(1, 1000, 20000)
            .bomb("planted", 2, 8000)
            .build();
        let ctx = AnalysisContext::new(&data, P);
        let cfg = DetectorConfig::default();
        // Before freeze end.
        assert_eq!(ctx.phase_at(1, 900, &cfg.phase), Some(RoundPhase::Freeze));
        // 10 s in (tick 1640 at 64 tick) — opening (< 20 s).
        assert_eq!(ctx.phase_at(1, 1640, &cfg.phase), Some(RoundPhase::Opening));
        // 30 s in (tick 2920) — mid... but plant at 8000 is later; still Mid.
        assert_eq!(ctx.phase_at(1, 2920, &cfg.phase), Some(RoundPhase::Mid));
        // 60 s in (tick 4840) — late, still before the plant tick.
        assert_eq!(ctx.phase_at(1, 4840, &cfg.phase), Some(RoundPhase::Late));
        // At/after the plant — post-plant wins regardless of clock.
        assert_eq!(
            ctx.phase_at(1, 8000, &cfg.phase),
            Some(RoundPhase::PostPlant)
        );
        assert_eq!(
            ctx.phase_at(1, 12000, &cfg.phase),
            Some(RoundPhase::PostPlant)
        );
    }

    #[test]
    fn phase_boundaries_are_configurable_and_exact() {
        let data = Scenario::new("de_test")
            .players_ct(&[P])
            .players_t(&[2])
            .round(1, 1000, 20000)
            .build();
        let ctx = AnalysisContext::new(&data, P);
        let cfg = DetectorConfig::default();
        // Exactly 20.0 s after freeze end (tick 1000 + 1280): first Mid tick.
        assert_eq!(
            ctx.phase_at(1, 1000 + 1280, &cfg.phase),
            Some(RoundPhase::Mid)
        );
        assert_eq!(
            ctx.phase_at(1, 1000 + 1279, &cfg.phase),
            Some(RoundPhase::Opening)
        );
        // Exactly 50.0 s (tick 1000 + 3200): first Late tick.
        assert_eq!(
            ctx.phase_at(1, 1000 + 3200, &cfg.phase),
            Some(RoundPhase::Late)
        );
    }

    #[test]
    fn phase_at_unknown_round_is_none() {
        let data = Scenario::new("de_test")
            .players_ct(&[P])
            .players_t(&[2])
            .round(1, 1000, 20000)
            .build();
        let ctx = AnalysisContext::new(&data, P);
        assert_eq!(
            ctx.phase_at(7, 1500, &DetectorConfig::default().phase),
            None
        );
    }
}
