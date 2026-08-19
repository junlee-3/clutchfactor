//! Golden tests: full MatchData extraction diffed against committed
//! snapshots. Each skips (passes) when its demo isn't present — CI never
//! needs real demos (PROMPT.md §10.3).

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn check_golden(demo_rel: &str, golden_rel: &str) {
    let demo = repo_root().join(demo_rel);
    let golden_path = repo_root().join(golden_rel);
    if !demo.exists() {
        eprintln!("fixture demo {demo_rel} not present — skipping golden test");
        return;
    }
    let mut progress = |_stage: cf_parser::extract::ImportStage, _pct: f32| {};
    let data = cf_parser::extract::parse_match(&demo, 4, &mut progress).expect("parse failed");
    let actual = serde_json::to_string_pretty(&cf_parser::extract::golden_from(&data)).unwrap();
    let expected = std::fs::read_to_string(&golden_path).expect("golden missing");
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "parser output diverged from {golden_rel}"
    );
}

#[test]
fn mm_demo_matches_golden() {
    check_golden(
        "fixtures/own/mirage-tie-18-8-2026.dem",
        "fixtures/goldens/mirage-tie.match.json",
    );
}

#[test]
fn gotv_demo_matches_golden() {
    check_golden(
        "fixtures/public/navi-javelins-vs-9-pandas-fearless-m1-mirage.dem",
        "fixtures/goldens/navi-javelins-mirage.match.json",
    );
}
