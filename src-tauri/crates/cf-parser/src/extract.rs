//! One-pass demo extraction: demoparser2 output → `MatchData`.
//!
//! The only module (besides dev examples) that touches demoparser2 types;
//! everything it returns is `crate::model` (PROMPT.md §4 boundary rule).

use std::collections::HashMap;
use std::path::Path;

use ahash::AHashMap;
use demoparser::first_pass::parser_settings::{create_mmap, rm_user_friendly_names, ParserInputs};
use demoparser::parse_demo::{DemoOutput, Parser, ParsingMode};
use demoparser::second_pass::game_events::GameEvent;
use demoparser::second_pass::parser_settings::create_huffman_lookup_table;
use demoparser::second_pass::variants::{VarVec, Variant};

use crate::model::{
    Blind, BombEvent, GrenadeEvent, Hurt, InventorySample, Kill, MatchData, PlayerMeta, Reload,
    Round, Shot, Side, TickTable,
};
use crate::rounds::{normalize_rounds, RawReason, RawRoundEvent, RawWinner};

/// CS2 runs a fixed 64-tick simulation and the demo header carries no rate.
const TICKRATE: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStage {
    Reading,
    Parsing,
    Normalizing,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to read demo: {0}")]
    Io(String),
    #[error("demo parse failed: {0}")]
    Demo(String),
    #[error("demo contains no usable match data: {0}")]
    Empty(String),
    #[error("the parser crashed on this file — it is corrupt or not a CS2 demo")]
    Crashed,
}

const WANTED_EVENTS: &[&str] = &[
    "round_start",
    "round_freeze_end",
    "round_end",
    "round_officially_ended",
    "cs_win_panel_match",
    "player_death",
    "player_blind",
    "flashbang_detonate",
    "smokegrenade_detonate",
    "smokegrenade_expired",
    "hegrenade_detonate",
    "inferno_startburn",
    "inferno_expire",
    "bomb_planted",
    "bomb_defused",
    "bomb_exploded",
    "weapon_fire",
    "weapon_reload",
    "player_hurt",
];

const WANTED_PROPS: &[&str] = &[
    "X",
    "Y",
    "Z",
    "yaw",
    "health",
    "is_alive",
    "team_num",
    "weapon_name", // string name of active weapon (active_weapon is a raw entity handle)
    "spotted",
    "last_place_name",
    "is_scoped",
];

// ---- game-event field helpers (tolerant: wrong/missing → None) ----

fn field(ev: &GameEvent, name: &str) -> Option<Variant> {
    ev.fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| f.data.clone())
}

fn field_str(ev: &GameEvent, name: &str) -> Option<String> {
    match field(ev, name) {
        Some(Variant::String(s)) => Some(s),
        _ => None,
    }
}

fn field_bool(ev: &GameEvent, name: &str) -> Option<bool> {
    match field(ev, name) {
        Some(Variant::Bool(b)) => Some(b),
        _ => None,
    }
}

fn field_i32(ev: &GameEvent, name: &str) -> Option<i32> {
    match field(ev, name) {
        Some(Variant::I32(v)) => Some(v),
        Some(Variant::U32(v)) => i32::try_from(v).ok(),
        _ => None,
    }
}

fn field_f32(ev: &GameEvent, name: &str) -> Option<f32> {
    match field(ev, name) {
        Some(Variant::F32(v)) => Some(v),
        _ => None,
    }
}

/// Event enrichment emits steamids as stringified u64 (verified in
/// game_events.rs::create_player_steamid_field); accept raw u64 too.
fn field_steamid(ev: &GameEvent, name: &str) -> Option<u64> {
    match field(ev, name) {
        Some(Variant::String(s)) => s.parse::<u64>().ok().filter(|v| *v > 0),
        Some(Variant::U64(v)) if v > 0 => Some(v),
        _ => None,
    }
}

// ---- public API ----

/// demoparser2 skips per-tick prop collection whenever `wanted_events` is
/// non-empty (verified: collect_entities gate at the pinned rev), so a demo
/// takes two passes — events first, then the tick table — exactly like the
/// upstream Python bindings do.
///
/// Panic boundary: demoparser2 indexes into the raw byte buffer and panics
/// (rather than returning `Err`) on some malformed/truncated inputs — e.g. a
/// 10-byte garbage file panics with "range end index 16 out of range for
/// slice of length 10" deep inside its first pass. Per the crate boundary
/// rule (no demoparser2 types *or behavior* leak past cf-parser), that panic
/// is caught here and converted into `ParseError::Crashed`. `AssertUnwindSafe`
/// is sound: on unwind we discard the entire in-progress `parse_match_inner`
/// call (mmap, parser state, partially-built `MatchData`) and return `Err` —
/// nothing half-mutated is ever read back.
pub fn parse_match(
    path: &Path,
    sample_every: u32,
    progress: &mut dyn FnMut(ImportStage, f32),
) -> Result<MatchData, ParseError> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parse_match_inner(path, sample_every, progress)
    })) {
        Ok(result) => result,
        Err(payload) => {
            eprintln!(
                "cf-parser: demoparser2 panicked while parsing {}: {}",
                path.display(),
                panic_payload_message(&payload)
            );
            Err(ParseError::Crashed)
        }
    }
}

/// Best-effort extraction of a panic payload's message for logging only —
/// never surfaced to users (see `ParseError::Crashed`'s fixed §7 message).
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn parse_match_inner(
    path: &Path,
    sample_every: u32,
    progress: &mut dyn FnMut(ImportStage, f32),
) -> Result<MatchData, ParseError> {
    progress(ImportStage::Reading, 0.0);
    let mmap = create_mmap(path.to_string_lossy().into_owned())
        .map_err(|e| ParseError::Io(format!("{e:?}")))?;
    let huf = create_huffman_lookup_table();

    progress(ImportStage::Parsing, 0.05);
    let events_output = {
        let inputs = ParserInputs {
            real_name_to_og_name: AHashMap::default(),
            wanted_players: vec![],
            wanted_player_props: vec![],
            wanted_other_props: vec![],
            wanted_prop_states: AHashMap::default(),
            wanted_ticks: vec![],
            wanted_events: WANTED_EVENTS.iter().map(|s| s.to_string()).collect(),
            parse_ents: true,
            parse_projectiles: false,
            parse_grenades: false,
            only_header: false,
            only_convars: false,
            huffman_lookup_table: &huf,
            order_by_steamid: false,
            list_props: false,
            fallback_bytes: None,
        };
        let mut parser = Parser::new(inputs, ParsingMode::Normal);
        parser
            .parse_demo(&mmap)
            .map_err(|e| ParseError::Demo(format!("{e:?}")))?
    };

    progress(ImportStage::Parsing, 0.45);
    let ticks_output = {
        let friendly: Vec<String> = WANTED_PROPS.iter().map(|s| s.to_string()).collect();
        let real_props =
            rm_user_friendly_names(&friendly).map_err(|e| ParseError::Demo(format!("{e:?}")))?;
        let mut real_name_to_og_name = AHashMap::default();
        for (real, og) in real_props.iter().zip(friendly.iter()) {
            real_name_to_og_name.insert(real.clone(), og.clone());
        }
        // only_header: true mirrors the upstream Python parse_ticks call.
        let inputs = ParserInputs {
            real_name_to_og_name,
            wanted_players: vec![],
            wanted_player_props: real_props,
            wanted_other_props: vec![],
            wanted_prop_states: AHashMap::default(),
            wanted_ticks: vec![],
            wanted_events: vec![],
            parse_ents: true,
            parse_projectiles: false,
            parse_grenades: false,
            only_header: true,
            only_convars: false,
            huffman_lookup_table: &huf,
            order_by_steamid: false,
            list_props: false,
            fallback_bytes: None,
        };
        let mut parser = Parser::new(inputs, ParsingMode::Normal);
        parser
            .parse_demo(&mmap)
            .map_err(|e| ParseError::Demo(format!("{e:?}")))?
    };

    progress(ImportStage::Normalizing, 0.8);
    let mut data = build_match_data(&events_output, &ticks_output, sample_every)?;

    // Third targeted pass: inventory snapshots at death + round-end ticks only
    // (the per-tick inventory prop is a StringVec — far too heavy for the full
    // tick table, tiny for ~200 targeted ticks).
    progress(ImportStage::Parsing, 0.85);
    // Sample shortly BEFORE each death too: at the death tick itself the
    // dying player's inventory is already dropped/cleared (verified on real
    // demos — death-tick samples never show the victim's grenades).
    let pre_death = (0.25 * TICKRATE) as i32;
    let mut wanted_ticks: Vec<i32> = data
        .kills
        .iter()
        .flat_map(|k| [k.tick, k.tick - pre_death])
        .chain(data.rounds.iter().map(|r| r.end_tick))
        .collect();
    wanted_ticks.sort_unstable();
    wanted_ticks.dedup();
    let inv_output = {
        let friendly = vec!["inventory".to_string()];
        let real_props =
            rm_user_friendly_names(&friendly).map_err(|e| ParseError::Demo(format!("{e:?}")))?;
        let mut real_name_to_og_name = AHashMap::default();
        for (real, og) in real_props.iter().zip(friendly.iter()) {
            real_name_to_og_name.insert(real.clone(), og.clone());
        }
        let inputs = ParserInputs {
            real_name_to_og_name,
            wanted_players: vec![],
            wanted_player_props: real_props,
            wanted_other_props: vec![],
            wanted_prop_states: AHashMap::default(),
            wanted_ticks,
            wanted_events: vec![],
            parse_ents: true,
            parse_projectiles: false,
            parse_grenades: false,
            only_header: true,
            only_convars: false,
            huffman_lookup_table: &huf,
            order_by_steamid: false,
            list_props: false,
            fallback_bytes: None,
        };
        let mut parser = Parser::new(inputs, ParsingMode::Normal);
        parser
            .parse_demo(&mmap)
            .map_err(|e| ParseError::Demo(format!("{e:?}")))?
    };
    data.inventories = extract_inventories(&inv_output)?;

    progress(ImportStage::Normalizing, 0.9);
    Ok(data)
}

fn extract_inventories(output: &DemoOutput) -> Result<Vec<InventorySample>, ParseError> {
    let by_name: HashMap<&str, &VarVec> = output
        .prop_controller
        .prop_infos
        .iter()
        .filter_map(|pi| {
            output
                .df
                .get(&pi.id)
                .and_then(|c| c.data.as_ref())
                .map(|d| (pi.prop_friendly_name.as_str(), d))
        })
        .collect();
    let (Some(VarVec::I32(ticks)), Some(VarVec::U64(steamids)), Some(VarVec::StringVec(items))) = (
        by_name.get("tick").copied(),
        by_name.get("steamid").copied(),
        by_name.get("inventory").copied(),
    ) else {
        // Inventory columns absent (very old demo?) — silence-bias: no samples.
        return Ok(vec![]);
    };
    let mut out = vec![];
    for i in 0..ticks.len() {
        let (Some(tick), Some(steamid)) = (ticks[i], steamids[i]) else {
            continue;
        };
        if steamid == 0 {
            continue;
        }
        out.push(InventorySample {
            tick,
            steamid,
            items: items[i].clone(),
        });
    }
    out.sort_by_key(|s| (s.tick, s.steamid));
    Ok(out)
}

fn build_match_data(
    events_output: &DemoOutput,
    ticks_output: &DemoOutput,
    sample_every: u32,
) -> Result<MatchData, ParseError> {
    let map = events_output
        .header
        .as_ref()
        .or(ticks_output.header.as_ref())
        .and_then(|h| h.get("map_name").cloned())
        .unwrap_or_else(|| "<unknown>".to_string());

    let players = extract_players(events_output);
    if players.is_empty() {
        return Err(ParseError::Empty("no players found".into()));
    }

    let mut rounds = normalize_rounds(&raw_round_events(&events_output.game_events));
    if rounds.is_empty() {
        return Err(ParseError::Empty("no completed rounds found".into()));
    }

    let ticks = build_tick_table(ticks_output, sample_every)?;
    assign_sides(&mut rounds, &ticks);

    let (kills, blinds, grenades, bomb_events, shots, hurts, reloads) =
        extract_events(&events_output.game_events, &rounds);

    Ok(MatchData {
        map,
        tickrate: TICKRATE,
        players,
        rounds,
        kills,
        blinds,
        grenades,
        bomb_events,
        shots,
        hurts,
        reloads,
        inventories: vec![], // filled by the targeted third pass in parse_match
        ticks,
    })
}

fn extract_players(output: &DemoOutput) -> Vec<PlayerMeta> {
    let md = if output.player_md.is_empty() {
        &output.roster
    } else {
        &output.player_md
    };
    let mut players: Vec<PlayerMeta> = md
        .iter()
        .filter_map(|p| {
            Some(PlayerMeta {
                steamid: p.steamid.filter(|s| *s > 0)?,
                name: p.name.clone()?,
            })
        })
        .collect();
    players.sort_by_key(|p| p.steamid);
    players.dedup_by_key(|p| p.steamid);
    players
}

fn raw_round_events(events: &[GameEvent]) -> Vec<RawRoundEvent> {
    let mut raw = vec![];
    for ev in events {
        match ev.name.as_str() {
            "round_start" => raw.push(RawRoundEvent::Start {
                tick: ev.tick,
                round: field_i32(ev, "round").and_then(|n| u32::try_from(n).ok()),
            }),
            "round_freeze_end" => raw.push(RawRoundEvent::FreezeEnd { tick: ev.tick }),
            "round_officially_ended" => raw.push(RawRoundEvent::OfficiallyEnded { tick: ev.tick }),
            "cs_win_panel_match" => raw.push(RawRoundEvent::WinPanelMatch { tick: ev.tick }),
            "round_end" => {
                let winner = match field(ev, "winner") {
                    Some(Variant::String(s)) => RawWinner::Str(s),
                    Some(Variant::I32(n)) => RawWinner::Num(n),
                    Some(Variant::U32(n)) => RawWinner::Num(n as i32),
                    // Undecodable → normalize_rounds drops the round.
                    _ => RawWinner::Num(-1),
                };
                let reason = match field(ev, "reason") {
                    Some(Variant::String(s)) => RawReason::Str(s),
                    Some(Variant::I32(n)) => RawReason::Num(n),
                    Some(Variant::U32(n)) => RawReason::Num(n as i32),
                    _ => RawReason::Num(-1),
                };
                raw.push(RawRoundEvent::End {
                    tick: ev.tick,
                    winner,
                    reason,
                });
            }
            _ => {}
        }
    }
    raw
}

/// Round containing `tick`, by start-tick boundaries (kills in the
/// post-round lull belong to the round that just ended). 0 = before round 1.
fn round_for_tick(rounds: &[Round], tick: i32) -> u32 {
    let idx = rounds.partition_point(|r| r.start_tick <= tick);
    idx as u32
}

type ExtractedEvents = (
    Vec<Kill>,
    Vec<Blind>,
    Vec<GrenadeEvent>,
    Vec<BombEvent>,
    Vec<Shot>,
    Vec<Hurt>,
    Vec<Reload>,
);

fn extract_events(events: &[GameEvent], rounds: &[Round]) -> ExtractedEvents {
    let mut kills = vec![];
    let mut blinds = vec![];
    let mut grenades = vec![];
    let mut bombs = vec![];
    let mut shots = vec![];
    let mut hurts = vec![];
    let mut reloads = vec![];
    for ev in events {
        match ev.name.as_str() {
            "player_death" => {
                let Some(victim) = field_steamid(ev, "user_steamid") else {
                    continue;
                };
                kills.push(Kill {
                    tick: ev.tick,
                    round: round_for_tick(rounds, ev.tick),
                    attacker: field_steamid(ev, "attacker_steamid"),
                    victim,
                    assister: field_steamid(ev, "assister_steamid"),
                    weapon: field_str(ev, "weapon").unwrap_or_else(|| "<unknown>".into()),
                    headshot: field_bool(ev, "headshot").unwrap_or(false),
                    penetrated: field_i32(ev, "penetrated").unwrap_or(0),
                    thru_smoke: field_bool(ev, "thrusmoke").unwrap_or(false),
                    attacker_blind: field_bool(ev, "attackerblind").unwrap_or(false),
                    assistedflash: field_bool(ev, "assistedflash").unwrap_or(false),
                });
            }
            "player_blind" => {
                let Some(victim) = field_steamid(ev, "user_steamid") else {
                    continue;
                };
                blinds.push(Blind {
                    tick: ev.tick,
                    victim,
                    attacker: field_steamid(ev, "attacker_steamid"),
                    duration: field_f32(ev, "blind_duration").unwrap_or(0.0),
                });
            }
            "flashbang_detonate"
            | "smokegrenade_detonate"
            | "smokegrenade_expired"
            | "hegrenade_detonate"
            | "inferno_startburn"
            | "inferno_expire" => {
                let kind = match ev.name.as_str() {
                    "flashbang_detonate" => "flashbang",
                    "smokegrenade_detonate" => "smoke",
                    "smokegrenade_expired" => "smoke_expired",
                    "hegrenade_detonate" => "he",
                    "inferno_startburn" => "molotov_start",
                    _ => "molotov_expire",
                };
                grenades.push(GrenadeEvent {
                    tick: ev.tick,
                    kind: kind.to_string(),
                    thrower: field_steamid(ev, "user_steamid"),
                    x: field_f32(ev, "x").unwrap_or(0.0),
                    y: field_f32(ev, "y").unwrap_or(0.0),
                    z: field_f32(ev, "z").unwrap_or(0.0),
                });
            }
            "weapon_fire" => {
                let Some(player) = field_steamid(ev, "user_steamid") else {
                    continue;
                };
                shots.push(Shot {
                    tick: ev.tick,
                    player,
                    weapon: field_str(ev, "weapon").unwrap_or_default(),
                });
            }
            "weapon_reload" => {
                let Some(player) = field_steamid(ev, "user_steamid") else {
                    continue;
                };
                reloads.push(Reload {
                    tick: ev.tick,
                    player,
                });
            }
            "player_hurt" => {
                let Some(victim) = field_steamid(ev, "user_steamid") else {
                    continue;
                };
                hurts.push(Hurt {
                    tick: ev.tick,
                    victim,
                    attacker: field_steamid(ev, "attacker_steamid"),
                    dmg_health: field_i32(ev, "dmg_health").unwrap_or(0),
                    weapon: field_str(ev, "weapon").unwrap_or_default(),
                    hitgroup: field_str(ev, "hitgroup").unwrap_or_default(),
                });
            }
            "bomb_planted" | "bomb_defused" | "bomb_exploded" => {
                let kind = ev.name.trim_start_matches("bomb_");
                bombs.push(BombEvent {
                    tick: ev.tick,
                    kind: kind.to_string(),
                    player: field_steamid(ev, "user_steamid"),
                });
            }
            _ => {}
        }
    }
    kills.sort_by_key(|k| k.tick);
    blinds.sort_by_key(|b| b.tick);
    grenades.sort_by_key(|g| g.tick);
    bombs.sort_by_key(|b| b.tick);
    shots.sort_by_key(|s| s.tick);
    hurts.sort_by_key(|h| h.tick);
    reloads.sort_by_key(|r| r.tick);
    (kills, blinds, grenades, bombs, shots, hurts, reloads)
}

// ---- tick table ----

fn build_tick_table(output: &DemoOutput, sample_every: u32) -> Result<TickTable, ParseError> {
    let sample_every = sample_every.max(1);
    let by_name: HashMap<&str, &VarVec> = output
        .prop_controller
        .prop_infos
        .iter()
        .filter_map(|pi| {
            output
                .df
                .get(&pi.id)
                .and_then(|c| c.data.as_ref())
                .map(|d| (pi.prop_friendly_name.as_str(), d))
        })
        .collect();

    let col = |name: &str| -> Result<&VarVec, ParseError> {
        by_name
            .get(name)
            .copied()
            .ok_or_else(|| ParseError::Demo(format!("tick column '{name}' missing")))
    };

    let ticks = match col("tick")? {
        VarVec::I32(v) => v,
        other => {
            return Err(ParseError::Demo(format!(
                "tick column has unexpected type {other:?}"
            )))
        }
    };
    let steamids = match col("steamid")? {
        VarVec::U64(v) => v,
        other => {
            return Err(ParseError::Demo(format!(
                "steamid column has unexpected type {other:?}"
            )))
        }
    };
    let f32_col = |name: &str| -> Result<&Vec<Option<f32>>, ParseError> {
        match col(name)? {
            VarVec::F32(v) => Ok(v),
            other => Err(ParseError::Demo(format!(
                "column '{name}' has unexpected type {other:?}"
            ))),
        }
    };
    let x = f32_col("X")?;
    let y = f32_col("Y")?;
    let z = f32_col("Z")?;
    let yaw = f32_col("yaw")?;
    // Small ints arrive as I32 or U32 depending on the underlying netvar.
    let int_col = |name: &str| -> Result<Vec<Option<i32>>, ParseError> {
        match col(name)? {
            VarVec::I32(v) => Ok(v.clone()),
            VarVec::U32(v) => Ok(v.iter().map(|o| o.map(|x| x as i32)).collect()),
            other => Err(ParseError::Demo(format!(
                "column '{name}' has unexpected int type {other:?}"
            ))),
        }
    };
    let health = int_col("health")?;
    let bool_col = |name: &str| -> Result<&Vec<Option<bool>>, ParseError> {
        match col(name)? {
            VarVec::Bool(v) => Ok(v),
            other => Err(ParseError::Demo(format!(
                "column '{name}' has unexpected type {other:?}"
            ))),
        }
    };
    let is_alive = bool_col("is_alive")?;
    let spotted = bool_col("spotted")?;
    let is_scoped = bool_col("is_scoped")?;
    let team_num = int_col("team_num")?;
    let str_col = |name: &str| -> Result<&Vec<Option<String>>, ParseError> {
        match col(name)? {
            VarVec::String(v) => Ok(v),
            other => Err(ParseError::Demo(format!(
                "column '{name}' has unexpected type {other:?}"
            ))),
        }
    };
    let active_weapon = str_col("weapon_name")?;
    let last_place = str_col("last_place_name")?;

    let n = ticks.len();
    let mut t = TickTable {
        sample_every,
        ..Default::default()
    };
    for i in 0..n {
        let Some(tick) = ticks[i] else { continue };
        if tick % sample_every as i32 != 0 {
            continue;
        }
        let Some(steamid) = steamids[i].filter(|s| *s > 0) else {
            continue;
        };
        t.tick.push(tick);
        t.steamid.push(steamid);
        t.x.push(x[i].unwrap_or(0.0));
        t.y.push(y[i].unwrap_or(0.0));
        t.z.push(z[i].unwrap_or(0.0));
        t.yaw.push(yaw[i].unwrap_or(0.0));
        t.health.push(health[i].unwrap_or(0));
        t.is_alive.push(is_alive[i].unwrap_or(false));
        t.team_num.push(team_num[i].unwrap_or(0));
        t.active_weapon.push(active_weapon[i].clone());
        t.spotted.push(spotted[i].unwrap_or(false));
        t.last_place.push(last_place[i].clone());
        t.is_scoped.push(is_scoped[i].unwrap_or(false));
    }
    Ok(t)
}

/// Fill each round's ct/t steamid lists from the modal `team_num` of each
/// player's samples inside [freeze_end (or start), end].
fn assign_sides(rounds: &mut [Round], ticks: &TickTable) {
    for round in rounds.iter_mut() {
        let lo = round.freeze_end_tick.unwrap_or(round.start_tick);
        let hi = round.end_tick;
        let mut counts: HashMap<u64, HashMap<i32, u32>> = HashMap::new();
        for i in 0..ticks.len() {
            let tk = ticks.tick[i];
            if tk < lo || tk > hi {
                continue;
            }
            let team = ticks.team_num[i];
            if team != 2 && team != 3 {
                continue;
            }
            *counts
                .entry(ticks.steamid[i])
                .or_default()
                .entry(team)
                .or_default() += 1;
        }
        let mut ct = vec![];
        let mut t = vec![];
        for (steamid, teams) in counts {
            let modal = teams.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k);
            match modal {
                Some(3) => ct.push(steamid),
                Some(2) => t.push(steamid),
                _ => {}
            }
        }
        ct.sort_unstable();
        t.sort_unstable();
        round.ct_steamids = ct;
        round.t_steamids = t;
    }
}

// ---- score & identity ----

/// (roster_a, roster_b, wins_a, wins_b): roster A = CT side of round 1.
/// Each round's win is attributed to whichever roster overlaps the winning
/// side more (robust to leavers/substitutes).
pub fn derive_score(rounds: &[Round]) -> (Vec<u64>, Vec<u64>, u32, u32) {
    let Some(first) = rounds.first() else {
        return (vec![], vec![], 0, 0);
    };
    let roster_a: Vec<u64> = first.ct_steamids.clone();
    let roster_b: Vec<u64> = first.t_steamids.clone();
    let overlap = |side: &[u64], roster: &[u64]| side.iter().filter(|s| roster.contains(s)).count();
    let mut wins_a = 0;
    let mut wins_b = 0;
    for r in rounds {
        let winning_side = match r.winner {
            Side::Ct => &r.ct_steamids,
            Side::T => &r.t_steamids,
        };
        let a = overlap(winning_side, &roster_a);
        let b = overlap(winning_side, &roster_b);
        if a > b {
            wins_a += 1;
        } else if b > a {
            wins_b += 1;
        }
        // a == b (both zero — empty side data): unattributable, count neither.
    }
    (roster_a, roster_b, wins_a, wins_b)
}

/// Steamids ordered by how many matches they appear in (desc), then by id for
/// determinism. First entry is the best tracked-player candidate.
pub fn detect_tracked_candidates(players_per_match: &[Vec<u64>]) -> Vec<u64> {
    let mut counts: HashMap<u64, u32> = HashMap::new();
    for m in players_per_match {
        for s in m {
            *counts.entry(*s).or_default() += 1;
        }
    }
    let mut out: Vec<(u64, u32)> = counts.into_iter().collect();
    out.sort_by(|(ida, ca), (idb, cb)| cb.cmp(ca).then(ida.cmp(idb)));
    out.into_iter().map(|(id, _)| id).collect()
}

// ---- golden snapshot ----

#[derive(Debug, serde::Serialize)]
pub struct GoldenRound {
    pub number: u32,
    pub winner: Side,
    pub reason: crate::model::RoundEndReason,
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub ct_count: usize,
    pub t_count: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct MatchGolden {
    pub map: String,
    pub tickrate: f32,
    pub sample_every: u32,
    pub players: Vec<String>, // "steamid name", sorted
    pub rounds: Vec<GoldenRound>,
    pub wins_a: u32,
    pub wins_b: u32,
    pub roster_a_len: usize,
    pub roster_b_len: usize,
    pub kills: usize,
    pub blinds: usize,
    pub grenades: usize,
    pub bomb_events: usize,
    pub tick_rows: usize,
    pub shots: usize,
    pub hurts: usize,
    pub reloads: usize,
    pub inventories: usize,
    pub first_kill: Option<String>,
    pub last_kill: Option<String>,
}

pub fn golden_from(data: &MatchData) -> MatchGolden {
    let (ra, rb, wa, wb) = derive_score(&data.rounds);
    let name_of = |sid: u64| -> String {
        data.players
            .iter()
            .find(|p| p.steamid == sid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "<unknown>".into())
    };
    let kill_line = |k: &Kill| {
        format!(
            "[{}] {} -> {} [{}]",
            k.tick,
            k.attacker.map(name_of).unwrap_or_else(|| "<world>".into()),
            name_of(k.victim),
            k.weapon
        )
    };
    MatchGolden {
        map: data.map.clone(),
        tickrate: data.tickrate,
        sample_every: data.ticks.sample_every,
        players: data
            .players
            .iter()
            .map(|p| format!("{} {}", p.steamid, p.name))
            .collect(),
        rounds: data
            .rounds
            .iter()
            .map(|r| GoldenRound {
                number: r.number,
                winner: r.winner,
                reason: r.reason.clone(),
                freeze_end_tick: r.freeze_end_tick,
                end_tick: r.end_tick,
                ct_count: r.ct_steamids.len(),
                t_count: r.t_steamids.len(),
            })
            .collect(),
        wins_a: wa,
        wins_b: wb,
        roster_a_len: ra.len(),
        roster_b_len: rb.len(),
        kills: data.kills.len(),
        blinds: data.blinds.len(),
        grenades: data.grenades.len(),
        bomb_events: data.bomb_events.len(),
        tick_rows: data.ticks.len(),
        shots: data.shots.len(),
        hurts: data.hurts.len(),
        reloads: data.reloads.len(),
        inventories: data.inventories.len(),
        first_kill: data.kills.first().map(kill_line),
        last_kill: data.kills.last().map(kill_line),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RoundEndReason;

    fn round(number: u32, winner: Side, ct: Vec<u64>, t: Vec<u64>) -> Round {
        Round {
            number,
            start_tick: number as i32 * 1000,
            freeze_end_tick: Some(number as i32 * 1000 + 100),
            end_tick: number as i32 * 1000 + 900,
            officially_ended_tick: None,
            winner,
            reason: RoundEndReason::TKilled,
            ct_steamids: ct,
            t_steamids: t,
        }
    }

    #[test]
    fn derive_score_follows_rosters_through_halftime_swap() {
        // Roster A (1,2) starts CT and wins round 1; sides swap in round 2
        // and roster A (now T) wins again; roster B wins round 3.
        let rounds = vec![
            round(1, Side::Ct, vec![1, 2], vec![3, 4]),
            round(2, Side::T, vec![3, 4], vec![1, 2]),
            round(3, Side::Ct, vec![3, 4], vec![1, 2]),
        ];
        let (ra, rb, wa, wb) = derive_score(&rounds);
        assert_eq!(ra, vec![1, 2]);
        assert_eq!(rb, vec![3, 4]);
        assert_eq!((wa, wb), (2, 1));
    }

    #[test]
    fn derive_score_survives_a_substitute() {
        // Player 2 leaves, 9 joins roster A's side mid-match.
        let rounds = vec![
            round(1, Side::Ct, vec![1, 2], vec![3, 4]),
            round(2, Side::Ct, vec![1, 9], vec![3, 4]),
        ];
        let (_, _, wa, wb) = derive_score(&rounds);
        assert_eq!((wa, wb), (2, 0));
    }

    #[test]
    fn derive_score_skips_unattributable_rounds() {
        let rounds = vec![
            round(1, Side::Ct, vec![1, 2], vec![3, 4]),
            round(2, Side::T, vec![], vec![]), // no side data
        ];
        let (_, _, wa, wb) = derive_score(&rounds);
        assert_eq!((wa, wb), (1, 0));
    }

    #[test]
    fn tracked_candidates_ordered_by_frequency_then_id() {
        let matches = vec![vec![10, 20, 30], vec![10, 20], vec![10, 40]];
        let c = detect_tracked_candidates(&matches);
        assert_eq!(c[0], 10);
        assert_eq!(c[1], 20);
        assert_eq!(&c[2..], &[30, 40]);
    }

    /// Regression for the corrupt-import panic (Task 10 fix): a truncated /
    /// non-demo file must surface as a friendly `ParseError`, never as an
    /// unwinding panic from demoparser2. Before the `catch_unwind` boundary
    /// was added, demoparser2 indexed past the end of the 10-byte buffer
    /// and panicked with "range end index 16 out of range for slice of
    /// length 10", aborting this test.
    #[test]
    fn parse_match_on_garbage_bytes_returns_err_not_panic() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cf-parser-garbage-{}-{}.dem",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"not a demo").expect("write garbage fixture");

        let mut noop = |_stage: ImportStage, _pct: f32| {};
        let result = parse_match(&path, 4, &mut noop);

        let _ = std::fs::remove_file(&path);

        let err = result.expect_err("garbage bytes must not parse successfully");
        let msg = err.to_string();
        assert!(
            msg.contains("corrupt or not a CS2 demo"),
            "expected a §7-style corrupt-demo message, got: {msg}"
        );
    }

    #[test]
    fn round_for_tick_boundaries() {
        let rounds = vec![
            round(1, Side::Ct, vec![], vec![]), // start 1000
            round(2, Side::Ct, vec![], vec![]), // start 2000
        ];
        assert_eq!(round_for_tick(&rounds, 500), 0);
        assert_eq!(round_for_tick(&rounds, 1000), 1);
        assert_eq!(round_for_tick(&rounds, 1999), 1);
        assert_eq!(round_for_tick(&rounds, 2500), 2);
    }
}
