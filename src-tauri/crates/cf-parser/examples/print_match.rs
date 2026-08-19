//! Print a MatchData summary from a real demo.
//! Usage: cargo run -p cf-parser --release --example print_match -- <demo.dem> [--golden <out.json>] [--sample N]

use std::path::PathBuf;

use cf_parser::extract::{derive_score, golden_from, parse_match, ImportStage};
use cf_parser::model::Side;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let demo = PathBuf::from(
        args.first()
            .expect("usage: print_match <demo.dem> [--golden out.json] [--sample N]"),
    );
    let flag = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let golden_out = flag("--golden").map(PathBuf::from);
    let sample_every: u32 = flag("--sample").and_then(|s| s.parse().ok()).unwrap_or(4);

    let mut progress = |stage: ImportStage, pct: f32| eprintln!("{stage:?} {:.0}%", pct * 100.0);
    let data = parse_match(&demo, sample_every, &mut progress).expect("parse failed");

    println!(
        "map: {}  tickrate: {}  sample_every: {}",
        data.map, data.tickrate, data.ticks.sample_every
    );
    println!("-- players ({}) --", data.players.len());
    for p in &data.players {
        println!("  {:<20} {}", p.name, p.steamid);
    }
    let (ra, rb, wa, wb) = derive_score(&data.rounds);
    println!("-- rounds ({}) --", data.rounds.len());
    for r in &data.rounds {
        println!(
            "round {:>2}: {:?} wins ({:?})  freeze_end {:?}  end {}  [ct {} | t {}]",
            r.number,
            r.winner,
            r.reason,
            r.freeze_end_tick,
            r.end_tick,
            r.ct_steamids.len(),
            r.t_steamids.len()
        );
    }
    let name_of = |sid: &u64| {
        data.players
            .iter()
            .find(|p| p.steamid == *sid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| sid.to_string())
    };
    println!(
        "score: [{}] {} - {} [{}]",
        ra.iter().map(name_of).collect::<Vec<_>>().join(", "),
        wa,
        wb,
        rb.iter().map(name_of).collect::<Vec<_>>().join(", ")
    );
    let ct_r1 = data.rounds.first().map(|r| r.winner == Side::Ct);
    let _ = ct_r1;
    println!(
        "events: kills {} | blinds {} | grenades {} | bombs {} | tick rows {}",
        data.kills.len(),
        data.blinds.len(),
        data.grenades.len(),
        data.bomb_events.len(),
        data.ticks.len()
    );

    if let Some(p) = golden_out {
        let golden = golden_from(&data);
        std::fs::write(&p, serde_json::to_string_pretty(&golden).unwrap()).unwrap();
        println!("golden written: {}", p.display());
    }
}
