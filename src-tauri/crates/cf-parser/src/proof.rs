//! M0 parser proof: kill feed + round ends from a real demo.
//! Throwaway-adjacent — M1 replaces this with the full MatchData model.

use std::path::Path;

use ahash::AHashMap;
use demoparser::first_pass::parser_settings::{create_mmap, ParserInputs};
use demoparser::parse_demo::{Parser, ParsingMode};
use demoparser::second_pass::game_events::GameEvent;
use demoparser::second_pass::parser_settings::create_huffman_lookup_table;
use demoparser::second_pass::variants::Variant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct KillEntry {
    pub tick: i32,
    pub attacker: String,
    pub victim: String,
    pub weapon: String,
    pub headshot: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoundEnd {
    pub tick: i32,
    pub winner: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProofSummary {
    pub map: String,
    pub kills: Vec<KillEntry>,
    pub round_ends: Vec<RoundEnd>,
}

pub fn field(ev: &GameEvent, name: &str) -> Option<Variant> {
    ev.fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| f.data.clone())
}

pub fn field_str(ev: &GameEvent, name: &str) -> Option<String> {
    match field(ev, name) {
        Some(Variant::String(s)) => Some(s),
        _ => None,
    }
}

pub fn field_bool(ev: &GameEvent, name: &str) -> Option<bool> {
    match field(ev, name) {
        Some(Variant::Bool(b)) => Some(b),
        _ => None,
    }
}

pub fn winner_label(v: &Variant) -> String {
    match v {
        Variant::U32(2) | Variant::I32(2) => "T".to_string(),
        Variant::U32(3) | Variant::I32(3) => "CT".to_string(),
        Variant::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

pub fn parse_proof_summary(path: &Path) -> Result<ProofSummary, String> {
    let huf = create_huffman_lookup_table();
    let inputs = ParserInputs {
        real_name_to_og_name: AHashMap::default(),
        wanted_players: vec![],
        wanted_player_props: vec![],
        wanted_other_props: vec![],
        wanted_prop_states: AHashMap::default(),
        wanted_ticks: vec![],
        wanted_events: vec!["player_death".to_string(), "round_end".to_string()],
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
    let mmap = create_mmap(path.to_string_lossy().into_owned())
        .map_err(|e| format!("mmap failed: {e:?}"))?;
    let mut parser = Parser::new(inputs, ParsingMode::Normal);
    let output = parser
        .parse_demo(&mmap)
        .map_err(|e| format!("parse failed: {e:?}"))?;

    let map = output
        .header
        .as_ref()
        .and_then(|h| h.get("map_name").cloned())
        .unwrap_or_else(|| "<unknown>".to_string());

    let mut kills = vec![];
    let mut round_ends = vec![];
    for ev in &output.game_events {
        match ev.name.as_str() {
            "player_death" => kills.push(KillEntry {
                tick: ev.tick,
                attacker: field_str(ev, "attacker_name").unwrap_or_else(|| "<world>".into()),
                victim: field_str(ev, "user_name").unwrap_or_else(|| "<unknown>".into()),
                weapon: field_str(ev, "weapon").unwrap_or_else(|| "<unknown>".into()),
                headshot: field_bool(ev, "headshot").unwrap_or(false),
            }),
            "round_end" => round_ends.push(RoundEnd {
                tick: ev.tick,
                winner: field(ev, "winner")
                    .map(|v| winner_label(&v))
                    .unwrap_or_else(|| "<unknown>".into()),
            }),
            _ => {}
        }
    }
    Ok(ProofSummary {
        map,
        kills,
        round_ends,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use demoparser::second_pass::game_events::{EventField, GameEvent};
    use demoparser::second_pass::variants::Variant;

    fn ev(fields: Vec<(&str, Option<Variant>)>) -> GameEvent {
        GameEvent {
            name: "player_death".to_string(),
            tick: 100,
            fields: fields
                .into_iter()
                .map(|(n, d)| EventField {
                    name: n.to_string(),
                    data: d,
                })
                .collect(),
        }
    }

    #[test]
    fn field_str_extracts_string_variant() {
        let e = ev(vec![(
            "attacker_name",
            Some(Variant::String("dev1ce".into())),
        )]);
        assert_eq!(field_str(&e, "attacker_name").as_deref(), Some("dev1ce"));
    }

    #[test]
    fn field_str_none_when_missing_or_wrong_type() {
        let e = ev(vec![("headshot", Some(Variant::Bool(true)))]);
        assert_eq!(field_str(&e, "attacker_name"), None);
        assert_eq!(field_str(&e, "headshot"), None);
    }

    #[test]
    fn field_bool_extracts() {
        let e = ev(vec![("headshot", Some(Variant::Bool(true)))]);
        assert_eq!(field_bool(&e, "headshot"), Some(true));
    }

    #[test]
    fn winner_label_maps_team_numbers() {
        assert_eq!(winner_label(&Variant::U32(3)), "CT");
        assert_eq!(winner_label(&Variant::U32(2)), "T");
        assert_eq!(winner_label(&Variant::I32(3)), "CT");
        assert_eq!(winner_label(&Variant::String("CT".into())), "CT");
    }
}
