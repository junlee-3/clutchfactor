//! Golden test: parse the fixture demo and diff against the committed snapshot.
//! Skips (passes) when the demo isn't present — CI never needs real demos (PROMPT.md §10.3).

use std::path::Path;

#[test]
fn proof_summary_matches_golden() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let demo = root.join("fixtures/public/navi-javelins-vs-9-pandas-fearless-m1-mirage.dem");
    let golden_path = root.join("fixtures/goldens/navi-javelins-mirage.proof.json");
    if !demo.exists() {
        eprintln!("fixture demo not present — skipping golden test");
        return;
    }
    let summary = cf_parser::proof::parse_proof_summary(&demo).expect("parse failed");
    let actual = serde_json::to_string_pretty(&summary).unwrap();
    let expected = std::fs::read_to_string(&golden_path).expect("golden missing");
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "parser output diverged from golden"
    );
}
