//! Analysis output types. `EvidenceRef` is the §4 evidence contract: every
//! insight and death classification must be replayable.

/// Steamid64 overflows a JS number (2^53), so focus_players serialize as
/// strings — matching the frontend `EvidenceRef` type in src/lib/evidence.ts.
fn ser_steamids<S: serde::Serializer>(v: &[u64], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(v.iter().map(|id| id.to_string()))
}

fn de_steamids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<u64>, D::Error> {
    let raw: Vec<String> = serde::Deserialize::deserialize(d)?;
    raw.iter()
        .map(|s| s.parse::<u64>().map_err(serde::de::Error::custom))
        .collect()
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvidenceRef {
    pub round: u32,
    pub tick_start: i32,
    pub tick_end: i32,
    #[serde(serialize_with = "ser_steamids", deserialize_with = "de_steamids")]
    pub focus_players: Vec<u64>,
    pub camera_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Category {
    Deaths,
    Utility,
    Positioning,
    Timing,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Deaths => "deaths",
            Category::Utility => "utility",
            Category::Positioning => "positioning",
            Category::Timing => "timing",
        }
    }
}

/// One rule firing on one moment. `details` names the concrete facts the
/// caption needs (e.g. the teammate who didn't follow, with distance).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RuleFlag {
    pub rule_id: &'static str,
    pub round: u32,
    pub tick: i32,
    pub steamid: u64,
    pub confidence: f32,
    pub severity: f32,
    pub details: serde_json::Value,
    pub evidence: EvidenceRef,
}

/// One user-facing insight (match-level or moment-level).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Insight {
    /// The rule id (or family id for aggregates) that produced this.
    pub detector: String,
    pub category: Category,
    pub severity: f32,
    pub confidence: f32,
    /// 0 = match-level.
    pub round: u32,
    pub player: u64,
    pub title_data: serde_json::Value,
    pub metrics: serde_json::Value,
    pub evidence: Vec<EvidenceRef>,
}

/// One row per tracked-player death (spec §1: exactly one primary class,
/// secondary tags record every other rule that fired).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeathClassRow {
    pub round: u32,
    pub tick: i32,
    pub victim: u64,
    pub class_id: u8,
    pub class_source: String,
    pub secondary_tags: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct AnalysisOutput {
    pub flags: Vec<RuleFlag>,
    pub insights: Vec<Insight>,
    pub death_classes: Vec<DeathClassRow>,
}
