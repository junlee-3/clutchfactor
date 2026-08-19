//! Store: the app's only door to SQLite. Steamids cross this boundary as
//! strings (steamid64 doesn't fit in a JS number — convention holds through
//! IPC to the frontend).

use std::path::Path;

use cf_parser::extract::derive_score;
#[cfg(test)]
use cf_parser::model::{Hurt, InventorySample, Reload, Shot};
use cf_parser::model::{MatchData, RoundEndReason, Side};
use rusqlite::{params, Connection};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("this demo is already imported (same file hash)")]
    DuplicateImport,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchSummary {
    pub id: i64,
    pub file_name: String,
    pub map: String,
    pub imported_at: String,
    pub rounds: u32,
    pub score_a: u32,
    pub score_b: u32,
    pub tracked_steamid: Option<String>,
    /// "win" | "loss" | "tie" from the tracked player's perspective; None if
    /// the tracked player didn't play this match.
    pub tracked_result: Option<String>,
    pub tracked_kills: Option<u32>,
    pub tracked_deaths: Option<u32>,
    pub tracked_hs_pct: Option<f32>,
}

// ---- read models for the replay viewer (mirrored in src/lib/ipc.ts) ----

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundInfo {
    pub number: u32,
    pub start_tick: i32,
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub officially_ended_tick: Option<i32>,
    pub winner: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct KillInfo {
    pub round: u32,
    pub tick: i32,
    pub attacker: Option<String>,
    pub victim: String,
    pub assister: Option<String>,
    pub weapon: String,
    pub headshot: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GrenadeInfo {
    pub tick: i32,
    pub kind: String,
    pub thrower: Option<String>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BombInfo {
    pub tick: i32,
    pub kind: String,
    pub player: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PlayerInfo {
    pub steamid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundSideInfo {
    pub number: u32,
    pub steamid: String,
    pub side: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchDetail {
    pub id: i64,
    pub map: String,
    pub tickrate: f32,
    pub sample_every: u32,
    pub score_a: u32,
    pub score_b: u32,
    pub players: Vec<PlayerInfo>,
    pub rounds: Vec<RoundInfo>,
    pub kills: Vec<KillInfo>,
    pub grenades: Vec<GrenadeInfo>,
    pub bomb_events: Vec<BombInfo>,
    pub round_sides: Vec<RoundSideInfo>,
}

/// Columnar, sorted by (tick, steamid);
/// range = [round.start_tick, officially_ended ?? end_tick].
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct RoundTicks {
    pub tick: Vec<i32>,
    pub steamid: Vec<String>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
    pub yaw: Vec<f32>,
    pub health: Vec<i32>,
    pub is_alive: Vec<bool>,
    pub team_num: Vec<i32>,
    pub active_weapon: Vec<Option<String>>,
    pub last_place: Vec<Option<String>>,
}

pub struct Store {
    conn: Connection,
}

fn side_str(s: Side) -> &'static str {
    match s {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

fn reason_str(r: &RoundEndReason) -> String {
    match r {
        RoundEndReason::TKilled => "t_killed".into(),
        RoundEndReason::CtKilled => "ct_killed".into(),
        RoundEndReason::BombDefused => "bomb_defused".into(),
        RoundEndReason::BombExploded => "bomb_exploded".into(),
        RoundEndReason::TargetSaved => "target_saved".into(),
        RoundEndReason::Other(s) => s.clone(),
    }
}

impl Store {
    /// Opens (creating if needed) and migrates the database.
    pub fn open(db_path: &Path) -> Result<Store, StoreError> {
        let mut conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::migrations::migrate(&mut conn)?;
        Ok(Store { conn })
    }

    pub fn has_file_hash(&self, file_hash: &str) -> Result<bool, StoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM matches WHERE file_hash = ?1)",
            [file_hash],
            |r| r.get(0),
        )?)
    }

    pub fn save_match(
        &mut self,
        file_name: &str,
        file_hash: &str,
        data: &MatchData,
    ) -> Result<i64, StoreError> {
        if self.has_file_hash(file_hash)? {
            return Err(StoreError::DuplicateImport);
        }

        let (roster_a, roster_b, wins_a, wins_b) = derive_score(&data.rounds);
        let roster_json = |r: &[u64]| {
            serde_json::to_string(&r.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .expect("roster json")
        };

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO matches (file_name, file_hash, map, tickrate, imported_at,
                                  sample_every, score_a, score_b, roster_a_json, roster_b_json)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), ?5, ?6, ?7, ?8, ?9)",
            params![
                file_name,
                file_hash,
                data.map,
                data.tickrate,
                data.ticks.sample_every,
                wins_a,
                wins_b,
                roster_json(&roster_a),
                roster_json(&roster_b),
            ],
        )?;
        let match_id = tx.last_insert_rowid();

        {
            let mut st =
                tx.prepare("INSERT INTO players (match_id, steamid, name) VALUES (?1, ?2, ?3)")?;
            for p in &data.players {
                st.execute(params![match_id, p.steamid.to_string(), p.name])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO rounds (match_id, number, start_tick, freeze_end_tick, end_tick,
                                     officially_ended_tick, winner, reason)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            let mut st_side = tx.prepare(
                "INSERT INTO round_sides (match_id, number, steamid, side) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for r in &data.rounds {
                st.execute(params![
                    match_id,
                    r.number,
                    r.start_tick,
                    r.freeze_end_tick,
                    r.end_tick,
                    r.officially_ended_tick,
                    side_str(r.winner),
                    reason_str(&r.reason),
                ])?;
                for s in &r.ct_steamids {
                    st_side.execute(params![match_id, r.number, s.to_string(), "CT"])?;
                }
                for s in &r.t_steamids {
                    st_side.execute(params![match_id, r.number, s.to_string(), "T"])?;
                }
            }

            let mut st = tx.prepare(
                "INSERT INTO kills (match_id, round, tick, attacker, victim, assister, weapon,
                                    headshot, penetrated, thru_smoke, attacker_blind, assistedflash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )?;
            for k in &data.kills {
                st.execute(params![
                    match_id,
                    k.round,
                    k.tick,
                    k.attacker.map(|a| a.to_string()),
                    k.victim.to_string(),
                    k.assister.map(|a| a.to_string()),
                    k.weapon,
                    k.headshot,
                    k.penetrated,
                    k.thru_smoke,
                    k.attacker_blind,
                    k.assistedflash,
                ])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO blinds (match_id, tick, victim, attacker, duration)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for b in &data.blinds {
                st.execute(params![
                    match_id,
                    b.tick,
                    b.victim.to_string(),
                    b.attacker.map(|a| a.to_string()),
                    b.duration,
                ])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO grenades (match_id, tick, kind, thrower, x, y, z)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for g in &data.grenades {
                st.execute(params![
                    match_id,
                    g.tick,
                    g.kind,
                    g.thrower.map(|t| t.to_string()),
                    g.x,
                    g.y,
                    g.z,
                ])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO bomb_events (match_id, tick, kind, player) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for b in &data.bomb_events {
                st.execute(params![
                    match_id,
                    b.tick,
                    b.kind,
                    b.player.map(|p| p.to_string())
                ])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO tick_samples (match_id, steamid, tick, x, y, z, yaw, health,
                                           is_alive, team_num, active_weapon, spotted, last_place,
                                           is_scoped)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            let t = &data.ticks;
            for i in 0..t.len() {
                st.execute(params![
                    match_id,
                    t.steamid[i].to_string(),
                    t.tick[i],
                    t.x[i],
                    t.y[i],
                    t.z[i],
                    t.yaw[i],
                    t.health[i],
                    t.is_alive[i],
                    t.team_num[i],
                    t.active_weapon[i],
                    t.spotted[i],
                    t.last_place[i],
                    t.is_scoped.get(i).copied(),
                ])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO shots (match_id, tick, player, weapon) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for s in &data.shots {
                st.execute(params![match_id, s.tick, s.player.to_string(), s.weapon])?;
            }

            let mut st = tx.prepare(
                "INSERT INTO hurts (match_id, tick, victim, attacker, dmg_health, weapon, hitgroup)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for h in &data.hurts {
                st.execute(params![
                    match_id,
                    h.tick,
                    h.victim.to_string(),
                    h.attacker.map(|a| a.to_string()),
                    h.dmg_health,
                    h.weapon,
                    h.hitgroup,
                ])?;
            }

            let mut st =
                tx.prepare("INSERT INTO reloads (match_id, tick, player) VALUES (?1, ?2, ?3)")?;
            for r in &data.reloads {
                st.execute(params![match_id, r.tick, r.player.to_string()])?;
            }

            let mut st = tx.prepare(
                "INSERT OR REPLACE INTO inventories (match_id, tick, steamid, items_json)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for inv in &data.inventories {
                st.execute(params![
                    match_id,
                    inv.tick,
                    inv.steamid.to_string(),
                    serde_json::to_string(&inv.items).expect("items json"),
                ])?;
            }
        }
        tx.commit()?;
        Ok(match_id)
    }

    pub fn list_matches(&self) -> Result<Vec<MatchSummary>, StoreError> {
        let tracked = self.tracked_steamid()?;
        let mut st = self.conn.prepare(
            "SELECT id, file_name, map, imported_at, score_a, score_b,
                    roster_a_json, roster_b_json,
                    (SELECT COUNT(*) FROM rounds r WHERE r.match_id = m.id) AS rounds
             FROM matches m ORDER BY imported_at DESC, id DESC",
        )?;
        let rows = st.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, u32>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, u32>(8)?,
            ))
        })?;

        let mut out = vec![];
        for row in rows {
            let (id, file_name, map, imported_at, score_a, score_b, ra_json, rb_json, rounds) =
                row?;
            let mut summary = MatchSummary {
                id,
                file_name,
                map,
                imported_at,
                rounds,
                score_a,
                score_b,
                tracked_steamid: tracked.clone(),
                tracked_result: None,
                tracked_kills: None,
                tracked_deaths: None,
                tracked_hs_pct: None,
            };
            if let Some(tid) = &tracked {
                let played: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM players WHERE match_id = ?1 AND steamid = ?2)",
                    params![id, tid],
                    |r| r.get(0),
                )?;
                if played {
                    let ra: Vec<String> = serde_json::from_str(&ra_json).unwrap_or_default();
                    let rb: Vec<String> = serde_json::from_str(&rb_json).unwrap_or_default();
                    let (own, opp) = if ra.contains(tid) {
                        (score_a, score_b)
                    } else if rb.contains(tid) {
                        (score_b, score_a)
                    } else {
                        (score_a, score_b) // substitute edge: fall back to A
                    };
                    summary.tracked_result = Some(
                        match own.cmp(&opp) {
                            std::cmp::Ordering::Greater => "win",
                            std::cmp::Ordering::Less => "loss",
                            std::cmp::Ordering::Equal => "tie",
                        }
                        .to_string(),
                    );
                    let (kills, hs): (u32, u32) = self.conn.query_row(
                        "SELECT COUNT(*), COALESCE(SUM(headshot), 0) FROM kills
                         WHERE match_id = ?1 AND attacker = ?2 AND victim != ?2",
                        params![id, tid],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    let deaths: u32 = self.conn.query_row(
                        "SELECT COUNT(*) FROM kills WHERE match_id = ?1 AND victim = ?2",
                        params![id, tid],
                        |r| r.get(0),
                    )?;
                    summary.tracked_kills = Some(kills);
                    summary.tracked_deaths = Some(deaths);
                    summary.tracked_hs_pct = if kills > 0 {
                        Some(hs as f32 / kills as f32 * 100.0)
                    } else {
                        None
                    };
                }
            }
            out.push(summary);
        }
        Ok(out)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let v = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn match_detail(&self, id: i64) -> Result<Option<MatchDetail>, StoreError> {
        let head = self
            .conn
            .query_row(
                "SELECT map, tickrate, sample_every, score_a, score_b FROM matches WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, f32>(1)?,
                        r.get::<_, u32>(2)?,
                        r.get::<_, u32>(3)?,
                        r.get::<_, u32>(4)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((map, tickrate, sample_every, score_a, score_b)) = head else {
            return Ok(None);
        };

        let mut st = self
            .conn
            .prepare("SELECT steamid, name FROM players WHERE match_id = ?1 ORDER BY steamid")?;
        let players = st
            .query_map([id], |r| {
                Ok(PlayerInfo {
                    steamid: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT number, start_tick, freeze_end_tick, end_tick, officially_ended_tick,
                    winner, reason
             FROM rounds WHERE match_id = ?1 ORDER BY number",
        )?;
        let rounds = st
            .query_map([id], |r| {
                Ok(RoundInfo {
                    number: r.get(0)?,
                    start_tick: r.get(1)?,
                    freeze_end_tick: r.get(2)?,
                    end_tick: r.get(3)?,
                    officially_ended_tick: r.get(4)?,
                    winner: r.get(5)?,
                    reason: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT round, tick, attacker, victim, assister, weapon, headshot
             FROM kills WHERE match_id = ?1 ORDER BY tick",
        )?;
        let kills = st
            .query_map([id], |r| {
                Ok(KillInfo {
                    round: r.get(0)?,
                    tick: r.get(1)?,
                    attacker: r.get(2)?,
                    victim: r.get(3)?,
                    assister: r.get(4)?,
                    weapon: r.get(5)?,
                    headshot: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT tick, kind, thrower, x, y, z FROM grenades WHERE match_id = ?1 ORDER BY tick",
        )?;
        let grenades = st
            .query_map([id], |r| {
                Ok(GrenadeInfo {
                    tick: r.get(0)?,
                    kind: r.get(1)?,
                    thrower: r.get(2)?,
                    x: r.get(3)?,
                    y: r.get(4)?,
                    z: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT tick, kind, player FROM bomb_events WHERE match_id = ?1 ORDER BY tick",
        )?;
        let bomb_events = st
            .query_map([id], |r| {
                Ok(BombInfo {
                    tick: r.get(0)?,
                    kind: r.get(1)?,
                    player: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT number, steamid, side FROM round_sides WHERE match_id = ?1
             ORDER BY number, steamid",
        )?;
        let round_sides = st
            .query_map([id], |r| {
                Ok(RoundSideInfo {
                    number: r.get(0)?,
                    steamid: r.get(1)?,
                    side: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(MatchDetail {
            id,
            map,
            tickrate,
            sample_every,
            score_a,
            score_b,
            players,
            rounds,
            kills,
            grenades,
            bomb_events,
            round_sides,
        }))
    }

    pub fn round_ticks(&self, id: i64, round: u32) -> Result<RoundTicks, StoreError> {
        let range = self
            .conn
            .query_row(
                "SELECT start_tick, COALESCE(officially_ended_tick, end_tick)
                 FROM rounds WHERE match_id = ?1 AND number = ?2",
                params![id, round],
                |r| Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((lo, hi)) = range else {
            return Ok(RoundTicks::default());
        };

        let mut st = self.conn.prepare(
            "SELECT tick, steamid, x, y, z, yaw, health, is_alive, team_num,
                    active_weapon, last_place
             FROM tick_samples
             WHERE match_id = ?1 AND tick BETWEEN ?2 AND ?3
             ORDER BY tick, steamid",
        )?;
        let mut out = RoundTicks::default();
        let mut rows = st.query(params![id, lo, hi])?;
        while let Some(r) = rows.next()? {
            out.tick.push(r.get(0)?);
            out.steamid.push(r.get(1)?);
            out.x.push(r.get(2)?);
            out.y.push(r.get(3)?);
            out.z.push(r.get(4)?);
            out.yaw.push(r.get(5)?);
            out.health.push(r.get(6)?);
            out.is_alive.push(r.get(7)?);
            out.team_num.push(r.get(8)?);
            out.active_weapon.push(r.get(9)?);
            out.last_place.push(r.get(10)?);
        }
        Ok(out)
    }

    /// Tracked player: explicit setting wins; otherwise the steamid appearing
    /// in the most imported matches (PROMPT.md §13 M1 identity detection).
    pub fn tracked_steamid(&self) -> Result<Option<String>, StoreError> {
        if let Some(v) = self.get_setting("tracked_steamid")? {
            return Ok(Some(v));
        }
        let modal = self
            .conn
            .query_row(
                "SELECT steamid FROM players
                 GROUP BY steamid
                 ORDER BY COUNT(DISTINCT match_id) DESC, steamid ASC
                 LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(modal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_parser::model::{
        Blind, Kill, MatchData, PlayerMeta, Round, RoundEndReason, Side, TickTable,
    };

    fn round(number: u32, winner: Side, ct: Vec<u64>, t: Vec<u64>) -> Round {
        Round {
            number,
            start_tick: number as i32 * 1000,
            freeze_end_tick: Some(number as i32 * 1000 + 100),
            end_tick: number as i32 * 1000 + 900,
            officially_ended_tick: Some(number as i32 * 1000 + 950),
            winner,
            reason: RoundEndReason::TKilled,
            ct_steamids: ct,
            t_steamids: t,
        }
    }

    fn kill(tick: i32, round: u32, attacker: u64, victim: u64, headshot: bool) -> Kill {
        Kill {
            tick,
            round,
            attacker: Some(attacker),
            victim,
            assister: None,
            weapon: "ak47".into(),
            headshot,
            penetrated: 0,
            thru_smoke: false,
            attacker_blind: false,
            assistedflash: false,
        }
    }

    /// Roster A = {1, 2} (CT round 1), roster B = {3, 4}. A wins 2-1.
    fn sample_match() -> MatchData {
        let mut ticks = TickTable {
            sample_every: 4,
            ..Default::default()
        };
        for (tick, steamid) in [(1100, 1u64), (1100, 3u64), (2100, 1u64)] {
            ticks.tick.push(tick);
            ticks.steamid.push(steamid);
            ticks.x.push(100.0);
            ticks.y.push(-50.0);
            ticks.z.push(0.0);
            ticks.yaw.push(90.0);
            ticks.health.push(100);
            ticks.is_alive.push(true);
            ticks.team_num.push(3);
            ticks.active_weapon.push(Some("weapon_ak47".into()));
            ticks.spotted.push(false);
            ticks.last_place.push(Some("BombsiteA".into()));
            ticks.is_scoped.push(false);
        }
        MatchData {
            map: "de_mirage".into(),
            tickrate: 64.0,
            players: vec![
                PlayerMeta {
                    steamid: 1,
                    name: "alice".into(),
                },
                PlayerMeta {
                    steamid: 2,
                    name: "bob".into(),
                },
                PlayerMeta {
                    steamid: 3,
                    name: "carol".into(),
                },
                PlayerMeta {
                    steamid: 4,
                    name: "dave".into(),
                },
            ],
            rounds: vec![
                round(1, Side::Ct, vec![1, 2], vec![3, 4]),
                round(2, Side::T, vec![3, 4], vec![1, 2]),
                round(3, Side::Ct, vec![3, 4], vec![1, 2]),
            ],
            kills: vec![
                kill(1200, 1, 1, 3, true),
                kill(1300, 1, 1, 4, false),
                kill(2200, 2, 3, 1, false),
            ],
            blinds: vec![Blind {
                tick: 1150,
                victim: 3,
                attacker: Some(1),
                duration: 2.4,
            }],
            grenades: vec![],
            bomb_events: vec![],
            shots: vec![Shot {
                tick: 1190,
                player: 1,
                weapon: "weapon_usp_silencer".into(),
            }],
            hurts: vec![Hurt {
                tick: 1195,
                victim: 3,
                attacker: Some(1),
                dmg_health: 27,
                weapon: "usp_silencer".into(),
                hitgroup: "chest".into(),
            }],
            reloads: vec![Reload {
                tick: 1400,
                player: 1,
            }],
            inventories: vec![InventorySample {
                tick: 2200,
                steamid: 1,
                items: vec!["Flashbang".into(), "Smoke Grenade".into()],
            }],
            ticks,
        }
    }

    fn open_tmp() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("test.db")).unwrap();
        (dir, store)
    }

    #[test]
    fn migrations_apply_fresh_and_are_idempotent_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 1);
        }
        // Reopen: migrations must not re-apply / error.
        let store = Store::open(&path).unwrap();
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 1);
    }

    #[test]
    fn save_and_list_roundtrip_with_tracked_stats() {
        let (_dir, mut store) = open_tmp();
        store.set_setting("tracked_steamid", "1").unwrap();
        let id = store
            .save_match("m1.dem", "hash-1", &sample_match())
            .unwrap();
        assert!(id > 0);
        let list = store.list_matches().unwrap();
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!(m.map, "de_mirage");
        assert_eq!(m.rounds, 3);
        assert_eq!((m.score_a, m.score_b), (2, 1));
        assert_eq!(m.tracked_steamid.as_deref(), Some("1"));
        assert_eq!(m.tracked_result.as_deref(), Some("win"));
        assert_eq!(m.tracked_kills, Some(2));
        assert_eq!(m.tracked_deaths, Some(1));
        assert_eq!(m.tracked_hs_pct, Some(50.0));
    }

    #[test]
    fn duplicate_hash_rejected() {
        let (_dir, mut store) = open_tmp();
        store
            .save_match("m1.dem", "hash-1", &sample_match())
            .unwrap();
        let err = store.save_match("m1-copy.dem", "hash-1", &sample_match());
        assert!(matches!(err, Err(StoreError::DuplicateImport)));
        assert_eq!(store.list_matches().unwrap().len(), 1);
    }

    #[test]
    fn settings_roundtrip_and_overwrite() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(store.get_setting("k").unwrap(), None);
        store.set_setting("k", "v1").unwrap();
        store.set_setting("k", "v2").unwrap();
        assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn tracked_falls_back_to_modal_steamid_across_matches() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(store.tracked_steamid().unwrap(), None);
        store.save_match("m1.dem", "h1", &sample_match()).unwrap();
        let mut second = sample_match();
        // Second match: only players 1 and 9 — player 1 now appears twice.
        second.players = vec![
            PlayerMeta {
                steamid: 1,
                name: "alice".into(),
            },
            PlayerMeta {
                steamid: 9,
                name: "eve".into(),
            },
        ];
        store.save_match("m2.dem", "h2", &second).unwrap();
        assert_eq!(store.tracked_steamid().unwrap().as_deref(), Some("1"));
    }

    #[test]
    fn match_detail_returns_full_read_model() {
        let (_dir, mut store) = open_tmp();
        let id = store.save_match("m1.dem", "h1", &sample_match()).unwrap();
        let d = store.match_detail(id).unwrap().expect("detail");
        assert_eq!(d.map, "de_mirage");
        assert_eq!((d.score_a, d.score_b), (2, 1));
        assert_eq!(d.players.len(), 4);
        assert_eq!(d.rounds.len(), 3);
        assert_eq!(d.rounds[0].winner, "CT");
        assert_eq!(d.rounds[0].reason, "t_killed");
        assert_eq!(d.kills.len(), 3);
        assert_eq!(d.kills[0].attacker.as_deref(), Some("1"));
        assert_eq!(d.kills[0].victim, "3");
        // 3 rounds × 4 players per side rows
        assert_eq!(d.round_sides.len(), 12);
        assert!(d
            .round_sides
            .iter()
            .any(|s| s.number == 2 && s.steamid == "1" && s.side == "T"));
    }

    #[test]
    fn match_detail_none_for_unknown_id() {
        let (_dir, store) = open_tmp();
        assert!(store.match_detail(999).unwrap().is_none());
    }

    #[test]
    fn round_ticks_returns_only_in_range_rows_sorted() {
        let (_dir, mut store) = open_tmp();
        let id = store.save_match("m1.dem", "h1", &sample_match()).unwrap();
        // Round 1 range: start 1000 → officially_ended 1950. Samples at 1100 (×2 players).
        let rt = store.round_ticks(id, 1).unwrap();
        assert_eq!(rt.tick, vec![1100, 1100]);
        assert_eq!(rt.steamid, vec!["1".to_string(), "3".to_string()]);
        assert_eq!(rt.x[0], 100.0);
        assert!(rt.is_alive[0]);
        // Round 2 range: 2000 → 2950. Sample at 2100 (player 1 only).
        let rt2 = store.round_ticks(id, 2).unwrap();
        assert_eq!(rt2.tick, vec![2100]);
        // Unknown round → empty, not error.
        assert!(store.round_ticks(id, 99).unwrap().tick.is_empty());
    }

    #[test]
    fn migration_2_analysis_tables_and_rule_inputs_persist() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 2);
        let id = store.save_match("m1.dem", "h1", &sample_match()).unwrap();
        let q = |sql: &str| -> u32 { store.conn.query_row(sql, [id], |r| r.get(0)).unwrap() };
        assert_eq!(q("SELECT COUNT(*) FROM shots WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM hurts WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM reloads WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM inventories WHERE match_id = ?1"), 1);
        let items: String = store
            .conn
            .query_row(
                "SELECT items_json FROM inventories WHERE match_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(items.contains("Flashbang"));
        let scoped: Option<bool> = store
            .conn
            .query_row(
                "SELECT is_scoped FROM tick_samples WHERE match_id = ?1 LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scoped, Some(false));
    }

    #[test]
    fn tick_samples_persist() {
        let (_dir, mut store) = open_tmp();
        let id = store.save_match("m1.dem", "h1", &sample_match()).unwrap();
        let n: u32 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tick_samples WHERE match_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 3);
    }
}
