//! Analysis golden tests: full parse + analyze on fixture demos, snapshot of
//! rule counts / class distribution / class-13 share. Skip when the demo is
//! absent — CI never needs real demos (PROMPT.md §10.3).

use std::path::{Path, PathBuf};

use cf_analysis::types::AnalysisGolden;
use cf_analysis::{analyze, DetectorConfig};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn check(demo_rel: &str, tracked: u64, golden_rel: &str) {
    let demo = repo_root().join(demo_rel);
    let golden_path = repo_root().join(golden_rel);
    if !demo.exists() {
        eprintln!("fixture demo {demo_rel} not present — skipping analysis golden");
        return;
    }
    let mut progress = |_s: cf_parser::extract::ImportStage, _p: f32| {};
    let data = cf_parser::extract::parse_match(&demo, 4, &mut progress).expect("parse failed");
    let out = analyze(&data, tracked, &DetectorConfig::default());
    let actual = serde_json::to_string_pretty(&AnalysisGolden::from_output(&out)).unwrap();
    let expected = std::fs::read_to_string(&golden_path).expect("golden missing");
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "analysis output diverged from {golden_rel}"
    );
}

#[test]
fn mm_demo_analysis_matches_golden() {
    check(
        "fixtures/own/mirage-tie-18-8-2026.dem",
        76561199228328773, // misosoupy3 — the owner
        "fixtures/goldens/mirage-tie.analysis.json",
    );
}

#[test]
fn gotv_demo_analysis_matches_golden() {
    check(
        "fixtures/public/navi-javelins-vs-9-pandas-fearless-m1-mirage.dem",
        76561198266290430, // vicu (from the match golden roster)
        "fixtures/goldens/navi-javelins-mirage.analysis.json",
    );
}
