//! The cf-parser boundary types (PROMPT.md §4): everything downstream crates
//! see. No demoparser2 types appear here.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Side {
    Ct,
    T,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RoundEndReason {
    TKilled,
    CtKilled,
    BombDefused,
    BombExploded,
    TargetSaved,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Round {
    /// 1-based, normalized (sequence-derived; validated against the demo's
    /// own round numbers where present).
    pub number: u32,
    /// Synthesized from the previous round's officially-ended tick (or 0 for
    /// round 1) when the demo carries no round_start (GOTV round 1 quirk).
    pub start_tick: i32,
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub officially_ended_tick: Option<i32>,
    pub winner: Side,
    pub reason: RoundEndReason,
    pub ct_steamids: Vec<u64>,
    pub t_steamids: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerMeta {
    pub steamid: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Kill {
    pub tick: i32,
    /// Normalized round number the kill occurred in (0 = before round 1).
    pub round: u32,
    pub attacker: Option<u64>,
    pub victim: u64,
    pub assister: Option<u64>,
    pub weapon: String,
    pub headshot: bool,
    pub penetrated: i32,
    pub thru_smoke: bool,
    pub attacker_blind: bool,
    pub assistedflash: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Blind {
    pub tick: i32,
    pub victim: u64,
    pub attacker: Option<u64>,
    pub duration: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GrenadeEvent {
    pub tick: i32,
    /// "flashbang" | "smoke" | "he" | "molotov_start" | "molotov_expire"
    pub kind: String,
    pub thrower: Option<u64>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BombEvent {
    pub tick: i32,
    /// "planted" | "defused" | "exploded"
    pub kind: String,
    pub player: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Shot {
    pub tick: i32,
    pub player: u64,
    pub weapon: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Hurt {
    pub tick: i32,
    pub victim: u64,
    pub attacker: Option<u64>,
    pub dmg_health: i32,
    pub weapon: String,
    pub hitgroup: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reload {
    pub tick: i32,
    pub player: u64,
}

/// Inventory snapshot at a targeted tick (deaths + round ends only).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InventorySample {
    pub tick: i32,
    pub steamid: u64,
    pub items: Vec<String>,
}

/// Column-oriented per-player samples, one row per (sampled tick, player).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TickTable {
    /// Keep every Nth tick (4 => ~16 Hz at 64-tick).
    pub sample_every: u32,
    pub tick: Vec<i32>,
    pub steamid: Vec<u64>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
    pub yaw: Vec<f32>,
    pub health: Vec<i32>,
    pub is_alive: Vec<bool>,
    pub team_num: Vec<i32>,
    pub active_weapon: Vec<Option<String>>,
    pub spotted: Vec<bool>,
    pub last_place: Vec<Option<String>>,
    pub is_scoped: Vec<bool>,
}

impl TickTable {
    pub fn len(&self) -> usize {
        self.tick.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tick.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MatchData {
    pub map: String,
    pub tickrate: f32,
    pub players: Vec<PlayerMeta>,
    pub rounds: Vec<Round>,
    pub kills: Vec<Kill>,
    pub blinds: Vec<Blind>,
    pub grenades: Vec<GrenadeEvent>,
    pub bomb_events: Vec<BombEvent>,
    pub shots: Vec<Shot>,
    pub hurts: Vec<Hurt>,
    pub reloads: Vec<Reload>,
    pub inventories: Vec<InventorySample>,
    pub ticks: TickTable,
}
