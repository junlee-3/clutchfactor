//! AnalysisContext: prebuilt indexes + spatial/temporal helpers over one
//! match. Built once, shared by every rule family. Pure — no I/O.

use std::collections::HashMap;

use cf_parser::model::{Blind, Hurt, InventorySample, Kill, MatchData, Reload, Shot, Side};

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
    inventories: HashMap<(i32, u64), &'a InventorySample>,
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
        let inventories = data
            .inventories
            .iter()
            .map(|inv| ((inv.tick, inv.steamid), inv))
            .collect();
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
    pub fn inventory_at(&self, steamid: u64, tick: i32) -> Option<&'a InventorySample> {
        self.inventories.get(&(tick, steamid)).copied()
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

    #[test]
    fn seconds_converts_via_tickrate() {
        let data = ctx_fixture();
        let ctx = AnalysisContext::new(&data, 1);
        assert_eq!(ctx.seconds(2.0), 128);
    }
}
