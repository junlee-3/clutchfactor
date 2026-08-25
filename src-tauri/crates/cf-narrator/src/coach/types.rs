//! Input the orchestrator assembles for the coach, and the coach's answers.
//! Every string here is ALREADY narrated (V1.2b captions, prettified
//! callouts, display names) — the model never sees raw JSON facts, and the
//! validator grounds against exactly the text the model saw.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct MatchInput {
    pub map: String,
    pub score: (u32, u32),
    pub tracked_name: String,
    pub tracked_result: Option<String>,
    /// Display names of every player in the match (both teams).
    pub roster: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayLine {
    pub tick: i32,
    /// "+12 s" into the round (after freeze end), rendered by the caller.
    pub clock: String,
    pub kind: String,
    pub headline: String,
    pub facts: Vec<String>,
    /// "good" | "bad" | "neutral" when the engine measured it.
    pub quality: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoundInput {
    pub round: u32,
    pub side: String,
    pub won: bool,
    pub verdict_label: String,
    /// Signed whole percent of win probability (ADR-0008 impact).
    pub impact_pct: i32,
    pub man_context: Option<String>,
    pub kills: u32,
    pub deaths: u32,
    pub plays: Vec<PlayLine>,
    /// Everyone's kills/bomb events this round, one narrated line each.
    pub timeline: Vec<String>,
    /// One line per earlier round of this match ("R5 · Quiet · won").
    pub prior_digest: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayComment {
    pub tick: i32,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundCommentary {
    pub round: u32,
    pub read: String,
    #[serde(default)]
    pub plays: Vec<PlayComment>,
    #[serde(default)]
    pub why_it_mattered: Option<String>,
    #[serde(default)]
    pub what_to_practise: Option<String>,
    #[serde(default)]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoundDigest {
    pub round: u32,
    pub verdict_label: String,
    pub won: bool,
    /// The validated per-round `read` (or the template why/practise line
    /// when the coach fell back).
    pub read: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SynthesisInput {
    pub match_input: MatchInput,
    pub rounds: Vec<RoundDigest>,
    /// Template insight narrations, "Title: body".
    pub insights: Vec<String>,
    /// Cross-match habit narrations, "Title: body".
    pub habits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchSynthesis {
    pub opening: String,
    #[serde(default)]
    pub work_on: Vec<String>,
}
