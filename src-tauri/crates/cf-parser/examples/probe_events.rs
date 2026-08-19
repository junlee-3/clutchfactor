//! Dev probe: print the first N instances of arbitrary events with all fields.
//! Usage: probe_events <demo.dem> <event1,event2,...> [N]

use ahash::AHashMap;
use demoparser::first_pass::parser_settings::{create_mmap, ParserInputs};
use demoparser::parse_demo::{Parser, ParsingMode};
use demoparser::second_pass::parser_settings::create_huffman_lookup_table;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: probe_events <demo.dem> <events> [N]");
    let events: Vec<String> = args
        .next()
        .expect("comma-separated event names")
        .split(',')
        .map(|s| s.to_string())
        .collect();
    let n: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(5);

    let huf = create_huffman_lookup_table();
    let inputs = ParserInputs {
        real_name_to_og_name: AHashMap::default(),
        wanted_players: vec![],
        wanted_player_props: vec![],
        wanted_other_props: vec![],
        wanted_prop_states: AHashMap::default(),
        wanted_ticks: vec![],
        wanted_events: events.clone(),
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

    for name in &events {
        let mut count = 0usize;
        let total = output.game_events.iter().filter(|e| &e.name == name).count();
        println!("== {name} (total {total}) ==");
        for ev in output.game_events.iter().filter(|e| &e.name == name) {
            if count >= n {
                break;
            }
            let fields: Vec<String> = ev
                .fields
                .iter()
                .map(|f| format!("{}={:?}", f.name, f.data))
                .collect();
            println!("[{:>7}] {}", ev.tick, fields.join(" "));
            count += 1;
        }
    }
}
