//! Dev tool + M3 verification: parse a demo, run the detectors, print every
//! insight, rule flag, and the death-class table with the class-13 share.
//! Usage: print_insights <demo.dem> <tracked_steamid64> [--json <out.json>]

use std::collections::BTreeMap;
use std::path::PathBuf;

use cf_analysis::{analyze, classify::class_13_share, DetectorConfig};
use cf_parser::extract::parse_match;

const CLASS_NAMES: &[(u8, &str)] = &[
    (1, "caught in utility animation"),
    (2, "caught in grenade damage (no duel)"),
    (3, "blinded / flashed out"),
    (4, "caught reloading or unscoped"),
    (5, "no-engagement death"),
    (6, "isolated & untradeable"),
    (7, "baited / unsupported trade"),
    (8, "over-peek in man disadvantage [not built]"),
    (9, "crossfire death"),
    (10, "lost angle-advantage duel [not built]"),
    (11, "pushed without info [not built]"),
    (12, "repeat-hotspot death [not built]"),
    (13, "outaimed in fair duel"),
    (14, "self / world / teammate"),
    (15, "unclassified"),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let demo = PathBuf::from(
        args.first()
            .expect("usage: print_insights <demo.dem> <steamid64>"),
    );
    let tracked: u64 = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .expect("second arg: tracked steamid64");
    let json_out = args
        .iter()
        .position(|a| a == "--json")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);
    let golden_out = args
        .iter()
        .position(|a| a == "--golden")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from);

    let mut progress = |_s: cf_parser::extract::ImportStage, _p: f32| {};
    let data = parse_match(&demo, 4, &mut progress).expect("parse failed");
    let name_of = |sid: u64| {
        data.players
            .iter()
            .find(|p| p.steamid == sid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| sid.to_string())
    };
    let cfg = DetectorConfig::default();
    let out = analyze(&data, tracked, &cfg);

    println!(
        "== {} | tracked: {} | deaths: {} ==",
        data.map,
        name_of(tracked),
        out.death_classes.len()
    );

    println!("\n-- death classes --");
    let mut dist: BTreeMap<u8, usize> = BTreeMap::new();
    for d in &out.death_classes {
        *dist.entry(d.class_id).or_default() += 1;
        let name = CLASS_NAMES
            .iter()
            .find(|(id, _)| *id == d.class_id)
            .map(|(_, n)| *n)
            .unwrap_or("?");
        println!(
            "r{:>2} [tick {:>7}] class {:>2} ({name}) via {} conf {:.2} tags {:?}",
            d.round, d.tick, d.class_id, d.class_source, d.confidence, d.secondary_tags
        );
    }
    println!("\n-- class distribution --");
    for (id, count) in &dist {
        let name = CLASS_NAMES
            .iter()
            .find(|(cid, _)| cid == id)
            .map(|(_, n)| *n)
            .unwrap_or("?");
        println!("class {id:>2} ({name}): {count}");
    }
    println!(
        "class-13 share: {:.1}%  (CI regression metric)",
        class_13_share(&out.death_classes) * 100.0
    );

    println!("\n-- rule flags ({}) --", out.flags.len());
    let mut by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &out.flags {
        *by_rule.entry(f.rule_id).or_default() += 1;
    }
    for (rule, count) in &by_rule {
        println!("{rule}: {count}");
    }

    println!("\n-- insights ({}) --", out.insights.len());
    for i in &out.insights {
        println!(
            "[{}] {:?} sev {:.2} conf {:.2} r{} | title {} | metrics {} | {} evidence refs",
            i.detector,
            i.category,
            i.severity,
            i.confidence,
            i.round,
            i.title_data,
            i.metrics,
            i.evidence.len()
        );
    }

    if let Some(p) = json_out {
        std::fs::write(&p, serde_json::to_string_pretty(&out).unwrap()).unwrap();
        println!("\nanalysis json written: {}", p.display());
    }
    if let Some(p) = golden_out {
        let golden = cf_analysis::types::AnalysisGolden::from_output(&out);
        std::fs::write(&p, serde_json::to_string_pretty(&golden).unwrap()).unwrap();
        println!("golden written: {}", p.display());
    }
}
