//! ScenarioBuilder: synthetic `MatchData` for detector tests (PROMPT.md §10.2).
//! Waypoints per player are densified into 16 Hz samples with linear
//! interpolation, so rules that read the tick table see realistic tracks.

use cf_parser::model::{
    Blind, BombEvent, GrenadeEvent, Hurt, InventorySample, Kill, MatchData, PlayerMeta, Reload,
    Round, RoundEndReason, Shot, Side, TickTable,
};

const TICKRATE: f32 = 64.0;
const SAMPLE_EVERY: i32 = 4;

struct Waypoint {
    tick: i32,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    health: i32,
    alive: bool,
    weapon: Option<String>,
    place: Option<String>,
    scoped: bool,
}

pub struct Scenario {
    map: String,
    ct: Vec<u64>,
    t: Vec<u64>,
    rounds: Vec<Round>,
    waypoints: Vec<(u64, Waypoint)>,
    kills: Vec<Kill>,
    blinds: Vec<Blind>,
    grenades: Vec<GrenadeEvent>,
    bombs: Vec<BombEvent>,
    shots: Vec<Shot>,
    hurts: Vec<Hurt>,
    reloads: Vec<Reload>,
    inventories: Vec<InventorySample>,
}

impl Scenario {
    pub fn new(map: &str) -> Self {
        Scenario {
            map: map.to_string(),
            ct: vec![],
            t: vec![],
            rounds: vec![],
            waypoints: vec![],
            kills: vec![],
            blinds: vec![],
            grenades: vec![],
            bombs: vec![],
            shots: vec![],
            hurts: vec![],
            reloads: vec![],
            inventories: vec![],
        }
    }

    pub fn players_ct(mut self, ids: &[u64]) -> Self {
        self.ct = ids.to_vec();
        self
    }

    pub fn players_t(mut self, ids: &[u64]) -> Self {
        self.t = ids.to_vec();
        self
    }

    /// Adds a round; freeze_end == start here (tests care about action time).
    pub fn round(mut self, number: u32, start_tick: i32, end_tick: i32) -> Self {
        self.rounds.push(Round {
            number,
            start_tick,
            freeze_end_tick: Some(start_tick),
            end_tick,
            officially_ended_tick: Some(end_tick + 128),
            winner: Side::Ct,
            reason: RoundEndReason::TKilled,
            ct_steamids: self.ct.clone(),
            t_steamids: self.t.clone(),
        });
        self
    }

    pub fn round_won_by(mut self, number: u32, winner: Side) -> Self {
        if let Some(r) = self.rounds.iter_mut().find(|r| r.number == number) {
            r.winner = winner;
        }
        self
    }

    /// Position waypoint (yaw 0, healthy, alive, rifle out). Samples between
    /// consecutive waypoints of the same player are lerped at 16 Hz.
    pub fn waypoint(self, sid: u64, tick: i32, x: f32, y: f32, z: f32) -> Self {
        self.waypoint_full(
            sid,
            tick,
            x,
            y,
            z,
            0.0,
            100,
            true,
            Some("weapon_ak47"),
            None,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn waypoint_full(
        mut self,
        sid: u64,
        tick: i32,
        x: f32,
        y: f32,
        z: f32,
        yaw: f32,
        health: i32,
        alive: bool,
        weapon: Option<&str>,
        place: Option<&str>,
        scoped: bool,
    ) -> Self {
        self.waypoints.push((
            sid,
            Waypoint {
                tick,
                x,
                y,
                z,
                yaw,
                health,
                alive,
                weapon: weapon.map(|s| s.to_string()),
                place: place.map(|s| s.to_string()),
                scoped,
            },
        ));
        self
    }

    /// Stationary player: two waypoints spanning [t0, t1].
    pub fn hold(self, sid: u64, t0: i32, t1: i32, x: f32, y: f32, z: f32) -> Self {
        self.waypoint(sid, t0, x, y, z).waypoint(sid, t1, x, y, z)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn kill_full(
        mut self,
        attacker: Option<u64>,
        victim: u64,
        round: u32,
        tick: i32,
        weapon: &str,
        thru_smoke: bool,
        penetrated: i32,
    ) -> Self {
        self.kills.push(Kill {
            tick,
            round,
            attacker,
            victim,
            assister: None,
            weapon: weapon.to_string(),
            headshot: false,
            penetrated,
            thru_smoke,
            attacker_blind: false,
            assistedflash: false,
        });
        self
    }

    pub fn kill(self, attacker: u64, victim: u64, round: u32, tick: i32, weapon: &str) -> Self {
        self.kill_full(Some(attacker), victim, round, tick, weapon, false, 0)
    }

    pub fn blind(mut self, attacker: u64, victim: u64, tick: i32, duration: f32) -> Self {
        self.blinds.push(Blind {
            tick,
            victim,
            attacker: Some(attacker),
            duration,
        });
        self
    }

    pub fn grenade(mut self, kind: &str, thrower: u64, tick: i32, x: f32, y: f32) -> Self {
        self.grenades.push(GrenadeEvent {
            tick,
            kind: kind.to_string(),
            thrower: Some(thrower),
            x,
            y,
            z: 0.0,
        });
        self
    }

    pub fn shot(mut self, sid: u64, tick: i32, weapon: &str) -> Self {
        self.shots.push(Shot {
            tick,
            player: sid,
            weapon: weapon.to_string(),
        });
        self
    }

    pub fn hurt(mut self, attacker: u64, victim: u64, tick: i32, dmg: i32, weapon: &str) -> Self {
        self.hurts.push(Hurt {
            tick,
            victim,
            attacker: Some(attacker),
            dmg_health: dmg,
            weapon: weapon.to_string(),
            hitgroup: "chest".to_string(),
        });
        self
    }

    pub fn reload(mut self, sid: u64, tick: i32) -> Self {
        self.reloads.push(Reload { tick, player: sid });
        self
    }

    pub fn inventory(mut self, sid: u64, tick: i32, items: &[&str]) -> Self {
        self.inventories.push(InventorySample {
            tick,
            steamid: sid,
            items: items.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    pub fn bomb(mut self, kind: &str, player: u64, tick: i32) -> Self {
        self.bombs.push(BombEvent {
            tick,
            kind: kind.to_string(),
            player: Some(player),
        });
        self
    }

    pub fn build(mut self) -> MatchData {
        let mut ticks = TickTable {
            sample_every: SAMPLE_EVERY as u32,
            ..Default::default()
        };
        // Densify per player.
        let mut players: Vec<u64> = self.ct.iter().chain(self.t.iter()).copied().collect();
        players.sort_unstable();
        let mut rows: Vec<(i32, u64, Waypoint)> = vec![];
        for sid in &players {
            let mut wps: Vec<&Waypoint> = self
                .waypoints
                .iter()
                .filter(|(s, _)| s == sid)
                .map(|(_, w)| w)
                .collect();
            wps.sort_by_key(|w| w.tick);
            for pair in wps.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let mut t = a.tick - a.tick.rem_euclid(SAMPLE_EVERY) + SAMPLE_EVERY;
                if a.tick % SAMPLE_EVERY == 0 {
                    t = a.tick;
                }
                while t < b.tick {
                    let f = (t - a.tick) as f32 / (b.tick - a.tick).max(1) as f32;
                    rows.push((
                        t,
                        *sid,
                        Waypoint {
                            tick: t,
                            x: a.x + (b.x - a.x) * f,
                            y: a.y + (b.y - a.y) * f,
                            z: a.z + (b.z - a.z) * f,
                            yaw: a.yaw,
                            health: a.health,
                            alive: a.alive,
                            weapon: a.weapon.clone(),
                            place: a.place.clone(),
                            scoped: a.scoped,
                        },
                    ));
                    t += SAMPLE_EVERY;
                }
            }
            // Final waypoint sample (and single-waypoint players).
            if let Some(last) = wps.last() {
                rows.push((
                    last.tick,
                    *sid,
                    Waypoint {
                        tick: last.tick,
                        x: last.x,
                        y: last.y,
                        z: last.z,
                        yaw: last.yaw,
                        health: last.health,
                        alive: last.alive,
                        weapon: last.weapon.clone(),
                        place: last.place.clone(),
                        scoped: last.scoped,
                    },
                ));
            }
        }
        rows.sort_by_key(|(t, sid, _)| (*t, *sid));
        for (tick, sid, w) in rows {
            ticks.tick.push(tick);
            ticks.steamid.push(sid);
            ticks.x.push(w.x);
            ticks.y.push(w.y);
            ticks.z.push(w.z);
            ticks.yaw.push(w.yaw);
            ticks.health.push(w.health);
            ticks.is_alive.push(w.alive);
            ticks
                .team_num
                .push(if self.ct.contains(&sid) { 3 } else { 2 });
            ticks.active_weapon.push(w.weapon);
            ticks.spotted.push(false);
            ticks.last_place.push(w.place);
            ticks.is_scoped.push(w.scoped);
        }

        self.kills.sort_by_key(|k| k.tick);
        self.hurts.sort_by_key(|h| h.tick);
        self.shots.sort_by_key(|s| s.tick);

        MatchData {
            map: self.map,
            tickrate: TICKRATE,
            players: players
                .iter()
                .map(|s| PlayerMeta {
                    steamid: *s,
                    name: format!("p{s}"),
                })
                .collect(),
            rounds: self.rounds,
            kills: self.kills,
            blinds: self.blinds,
            grenades: self.grenades,
            bomb_events: self.bombs,
            shots: self.shots,
            hurts: self.hurts,
            reloads: self.reloads,
            inventories: self.inventories,
            ticks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densifies_waypoints_into_lerped_samples() {
        let data = Scenario::new("de_test")
            .players_ct(&[1])
            .players_t(&[2])
            .round(1, 1000, 2000)
            .waypoint(1, 1000, 0.0, 0.0, 0.0)
            .waypoint(1, 1064, 640.0, 0.0, 0.0)
            .waypoint(2, 1000, 50.0, 50.0, 0.0)
            .build();
        // Player 1: samples every 4 ticks from 1000..=1064 → 17 rows.
        let p1_rows = data.ticks.steamid.iter().filter(|s| **s == 1).count();
        assert_eq!(p1_rows, 17);
        // Mid-way sample lerped.
        let idx = data
            .ticks
            .tick
            .iter()
            .zip(&data.ticks.steamid)
            .position(|(t, s)| *t == 1032 && *s == 1)
            .unwrap();
        assert!((data.ticks.x[idx] - 320.0).abs() < 0.01);
        // Sides via team_num.
        assert_eq!(data.rounds[0].ct_steamids, vec![1]);
    }
}
