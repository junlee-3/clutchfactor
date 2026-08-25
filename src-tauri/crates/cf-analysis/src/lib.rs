//! Detectors: pure functions over MatchData -> Vec<Insight>. No I/O (PROMPT.md §4).
//!
//! Structure (docs/plans/M3-detectors.md, docs/spec/death-taxonomy.md):
//! `AnalysisContext` (indexes/helpers) → rule families in `families/` emit
//! `RuleFlag`s + `Insight`s → `classify` assigns each tracked-player death
//! exactly one taxonomy class by priority, with secondary tags.

pub mod classify;
pub mod config;
pub mod context;
pub mod corpus;
pub mod families;
pub mod habits;
pub mod play_ledger;
pub mod round_review;
pub mod scenario;
pub mod types;
pub mod winprob;

pub use config::DetectorConfig;
pub use context::{AnalysisContext, PlayerState, RoundPhase};
pub use types::{AnalysisOutput, Category, DeathClassRow, EvidenceRef, Insight, RuleFlag};

use cf_parser::model::MatchData;

/// A rule family: emits flags for everything it detects, then optional
/// match-level insights derived from those flags.
pub trait Detector {
    fn rule_ids(&self) -> &'static [&'static str];
    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag>;
    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight>;
}

/// Standard evidence window around a moment: 5 s before → 2 s after.
pub fn evidence_around(ctx: &AnalysisContext, round: u32, tick: i32, focus: &[u64]) -> EvidenceRef {
    EvidenceRef {
        round,
        tick_start: tick - ctx.seconds(5.0),
        tick_end: tick + ctx.seconds(2.0),
        focus_players: focus.to_vec(),
        camera_hint: None,
    }
}

/// Run every registered family + the classifier over one match.
pub fn analyze(data: &MatchData, tracked: u64, cfg: &DetectorConfig) -> AnalysisOutput {
    let ctx = AnalysisContext::new(data, tracked);
    let detectors = families::all();
    let mut flags = vec![];
    let mut insights = vec![];
    for d in &detectors {
        let f = d.detect(&ctx, cfg);
        insights.extend(d.insights(&ctx, cfg, &f));
        flags.extend(f);
    }
    let death_classes = classify::classify_deaths(&ctx, cfg, &flags);
    let ledger = play_ledger::build_ledger(&ctx, cfg, &flags);
    AnalysisOutput {
        flags,
        insights,
        death_classes,
        ledger,
    }
}
