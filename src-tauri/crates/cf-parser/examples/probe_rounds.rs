//! Dev probe: dump round-boundary events (+ fields) from a demo to study
//! CS2 round quirks (warmup, restarts, round_officially_ended) before
//! building round normalization. Usage: probe_rounds <demo.dem>

use ahash::AHashMap;
use demoparser::first_pass::parser_settings::{create_mmap, ParserInputs};
use demoparser::parse_demo::{Parser, ParsingMode};
use demoparser::second_pass::parser_settings::create_huffman_lookup_table;

fn main() {
    let path = std::env::args().nth(1).expect("usage: probe_rounds <demo.dem>");
    let huf = create_huffman_lookup_table();
    let wanted = [
        "round_start",
        "round_freeze_end",
        "round_end",
        "round_officially_ended",
        "begin_new_match",
        "round_announce_match_start",
        "round_announce_warmup",
        "warmup_end",
        "announce_phase_end",
        "cs_win_panel_match",
        "round_announce_last_round_half",
    ];
    let inputs = ParserInputs {
        real_name_to_og_name: AHashMap::default(),
        wanted_players: vec![],
        wanted_player_props: vec![],
        wanted_other_props: vec![],
        wanted_prop_states: AHashMap::default(),
        wanted_ticks: vec![],
        wanted_events: wanted.iter().map(|s| s.to_string()).collect(),
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
    let mmap = create_mmap(path).expect("mmap");
    let mut parser = Parser::new(inputs, ParsingMode::Normal);
    let output = parser.parse_demo(&mmap).expect("parse");

    println!("events seen in demo: {:?}", output.game_events_counter);
    let mut evs: Vec<_> = output.game_events.iter().collect();
    evs.sort_by_key(|e| e.tick);
    for ev in evs {
        let fields: Vec<String> = ev
            .fields
            .iter()
            .map(|f| format!("{}={:?}", f.name, f.data))
            .collect();
        println!("[{:>7}] {:<28} {}", ev.tick, ev.name, fields.join(" "));
    }
}
