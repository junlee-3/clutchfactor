//! Derive data/win_prob_v1.yaml from the OpenML 43430 ARFF (CC0).
//!
//! Usage:
//!   curl -sL https://openml.org/data/v1/download/22102255/CSGO-Round-Winner-Classification.arff -o /tmp/csgo.arff
//!   cargo run -p cf-analysis --example derive_winprob -- /tmp/csgo.arff > crates/cf-analysis/data/win_prob_v1.yaml
//!
//! Deterministic: same input → byte-identical output. Rows that fail to
//! parse are counted and reported on stderr; any bad row is a hard error
//! (the validation run had zero).
//!
//! One documented exception: rows with ct_players_alive or t_players_alive
//! outside 0..=5 are impossible under 5v5 rules (a single snapshot glitch
//! in the 2019-2020 source data, not a parse failure) and would overflow
//! the fixed 6x6 table shape. These are skipped and counted separately —
//! never silently folded into the table, never a hard error.

use std::collections::BTreeMap;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: derive_winprob <arff>");
    let text = std::fs::read_to_string(&path).expect("cannot read ARFF");
    let mut attrs: Vec<String> = vec![];
    let mut agg: BTreeMap<(u8, u8, bool), (u64, u64)> = BTreeMap::new(); // (ct,t,planted) -> (ct_wins, n)
    let mut in_data = false;
    let mut rows = 0u64;
    let mut out_of_range = 0u64;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        if !in_data {
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("@ATTRIBUTE") {
                attrs.push(
                    line.split_whitespace()
                        .nth(1)
                        .expect("attr name")
                        .to_string(),
                );
            } else if upper.starts_with("@DATA") {
                in_data = true;
            }
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        assert_eq!(cols.len(), attrs.len(), "malformed row: {line}");
        let get = |name: &str| cols[attrs.iter().position(|a| a == name).expect(name)].trim();
        let ct = get("ct_players_alive")
            .parse::<f32>()
            .expect("ct_players_alive") as u8;
        let t = get("t_players_alive")
            .parse::<f32>()
            .expect("t_players_alive") as u8;
        let planted = get("bomb_planted") == "True";
        let winner = get("round_winner").trim_matches(|c| c == '\'' || c == '"');
        assert!(winner == "CT" || winner == "T", "bad winner: {winner}");
        rows += 1;
        if ct > 5 || t > 5 {
            out_of_range += 1;
            continue;
        }
        let e = agg.entry((ct, t, planted)).or_insert((0, 0));
        e.1 += 1;
        if winner == "CT" {
            e.0 += 1;
        }
    }
    eprintln!(
        "rows={rows} out_of_range_skipped={out_of_range} cells={}",
        agg.len()
    );
    println!("# GENERATED — do not edit. See docs/adr/ADR-0006-win-probability-table.md.");
    println!("# Source: OpenML dataset 43430 \"CSGO-Round-Winner-Classification\" (CC0),");
    println!("# 122,410 snapshots @ 20 s from 700 pro-tournament demos (2019-2020,");
    println!("# Skybox CS:GO AI Challenge). p_ct = P(CT wins round | state at snapshot).");
    println!("# Regenerate: cargo run -p cf-analysis --example derive_winprob -- <arff>");
    println!("version: win_prob_v1");
    println!("cells:");
    for ((ct, t, planted), (wins, n)) in &agg {
        println!(
            "  - {{ ct: {ct}, t: {t}, planted: {planted}, p_ct: {:.6}, n: {n} }}",
            *wins as f64 / *n as f64
        );
    }
}
