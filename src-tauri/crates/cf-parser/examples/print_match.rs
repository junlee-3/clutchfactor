//! M0 proof: print kill feed + round scores from a real demo.
//! Usage: cargo run -p cf-parser --release --example print_match -- <demo.dem> [--golden <out.json>]

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let demo = PathBuf::from(
        args.next()
            .expect("usage: print_match <demo.dem> [--golden out.json]"),
    );
    let golden_out = match (args.next().as_deref(), args.next()) {
        (Some("--golden"), Some(p)) => Some(PathBuf::from(p)),
        _ => None,
    };

    let summary = cf_parser::proof::parse_proof_summary(&demo).expect("parse failed");

    println!("map: {}", summary.map);
    println!("-- players ({}) --", summary.players.len());
    for p in &summary.players {
        println!("  team {} | {:<20} | {}", p.team_number, p.name, p.steamid);
    }
    println!("-- kill feed ({} kills) --", summary.kills.len());
    for k in &summary.kills {
        let hs = if k.headshot { " (HS)" } else { "" };
        println!(
            "[tick {:>7}] {} -> {} [{}]{}",
            k.tick, k.attacker, k.victim, k.weapon, hs
        );
    }
    println!("-- rounds ({}) --", summary.round_ends.len());
    let (mut ct, mut t) = (0u32, 0u32);
    for (i, r) in summary.round_ends.iter().enumerate() {
        match r.winner.as_str() {
            "CT" => ct += 1,
            "T" => t += 1,
            _ => {}
        }
        println!(
            "round {:>2}: winner {}  (CT {} - {} T)  [tick {}]",
            i + 1,
            r.winner,
            ct,
            t,
            r.tick
        );
    }
    println!("final (by side, no half-swap accounting): CT-side wins {ct} - T-side wins {t}");

    if let Some(p) = golden_out {
        std::fs::write(&p, serde_json::to_string_pretty(&summary).unwrap()).unwrap();
        println!("golden written: {}", p.display());
    }
}
