//! Store: the app's only door to SQLite. Steamids cross this boundary as
//! strings (steamid64 doesn't fit in a JS number — convention holds through
//! IPC to the frontend).

use std::path::Path;

use cf_parser::extract::derive_score;
#[cfg(test)]
use cf_parser::model::{Hurt, InventorySample, Reload, Shot};
use cf_parser::model::{MatchData, RoundEndReason, Side};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("this demo is already imported (same file hash)")]
    DuplicateImport,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchSummary {
    pub id: i64,
    pub file_name: String,
    pub map: String,
    pub imported_at: String,
    pub rounds: u32,
    pub score_a: u32,
    pub score_b: u32,
    pub tracked_steamid: Option<String>,
    /// "win" | "loss" | "tie" from the tracked player's perspective; None if
    /// the tracked player didn't play this match.
    pub tracked_result: Option<String>,
    pub tracked_kills: Option<u32>,
    pub tracked_deaths: Option<u32>,
    pub tracked_hs_pct: Option<f32>,
}

// ---- read models for the replay viewer (mirrored in src/lib/ipc.ts) ----

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundInfo {
    pub number: u32,
    pub start_tick: i32,
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub officially_ended_tick: Option<i32>,
    pub winner: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct KillInfo {
    pub round: u32,
    pub tick: i32,
    pub attacker: Option<String>,
    pub victim: String,
    pub assister: Option<String>,
    pub weapon: String,
    pub headshot: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GrenadeInfo {
    pub tick: i32,
    pub kind: String,
    pub thrower: Option<String>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BombInfo {
    pub tick: i32,
    pub kind: String,
    pub player: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PlayerInfo {
    pub steamid: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundSideInfo {
    pub number: u32,
    pub steamid: String,
    pub side: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct MatchDetail {
    pub id: i64,
    pub map: String,
    pub tickrate: f32,
    pub sample_every: u32,
    pub score_a: u32,
    pub score_b: u32,
    pub players: Vec<PlayerInfo>,
    pub rounds: Vec<RoundInfo>,
    pub kills: Vec<KillInfo>,
    pub grenades: Vec<GrenadeInfo>,
    pub bomb_events: Vec<BombInfo>,
    pub round_sides: Vec<RoundSideInfo>,
}

/// Columnar, sorted by (tick, steamid);
/// range = [round.start_tick, officially_ended ?? end_tick].
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct RoundTicks {
    pub tick: Vec<i32>,
    pub steamid: Vec<String>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
    pub yaw: Vec<f32>,
    pub health: Vec<i32>,
    pub is_alive: Vec<bool>,
    pub team_num: Vec<i32>,
    pub active_weapon: Vec<Option<String>>,
    pub last_place: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct InsightRow {
    pub detector: String,
    pub category: String,
    pub severity: f32,
    pub confidence: f32,
    pub round: u32,
    pub player: String,
    pub title_data_json: String,
    pub metrics_json: String,
    pub evidence_json: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RuleMatchCount {
    pub match_id: i64,
    pub map: String,
    pub imported_at: String,
    pub count: u32,
    pub first_evidence_json: Option<String>,
    /// Round/tick of the same first flag. Flags written before migration 3
    /// have a NULL `evidence_json`; these let the caller rebuild an
    /// `EvidenceRef` instead of dropping the habit's replay chip.
    pub first_round: Option<u32>,
    pub first_tick: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeathPos {
    pub match_id: i64,
    pub map: String,
    pub round: u32,
    pub tick: i32,
    pub x: f32,
    pub y: f32,
    pub place: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RoundStat {
    pub number: u32,
    pub freeze_end_tick: Option<i32>,
    pub winner: String,
    pub tracked_side: Option<String>,
    pub kills: u32,
    pub deaths: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeathClassDbRow {
    pub round: u32,
    pub tick: i32,
    pub victim: String,
    pub class_id: u8,
    pub class_source: String,
    pub secondary_tags_json: String,
    pub confidence: f32,
}

/// One stored rule-flag row for a match (issue #9 round-review backfill
/// input) — the same shape `save_analysis` writes, read back per-match
/// instead of aggregated across the corpus like `rule_counts_across_matches`.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FlagRow {
    pub rule_id: String,
    pub round: u32,
    pub tick: i32,
    pub steamid: String,
    pub severity: f32,
    pub confidence: f32,
    pub details_json: String,
}

/// A persisted round review row (issue #9 §7; ADR-0008). DB-shaped —
/// `verdict`/`attention` are already `as_str()`, `header`/`moments` are
/// JSON — conversion from `cf_analysis::round_review::RoundReview` happens
/// at the call site, keeping this crate's dependency surface unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundReviewRow {
    pub round: u32,
    pub impact: f32,
    pub verdict: String,   // snake_case verdict
    pub attention: String, // "none" | "dim" | "bright"
    pub selected: bool,
    pub pivotal_tick: i32,
    pub header_json: String,  // RoundHeader as JSON
    pub moments_json: String, // Vec<Moment> as JSON
    /// `cf_analysis::round_review::cfg_fingerprint` at the time this row was
    /// computed (migration 0007; V1.2 final-review fix wave, finding #5) —
    /// the caller compares it against the current fingerprint and recomputes
    /// on mismatch rather than serving a stale review. Existing pre-migration
    /// rows backfill to `""`, which never matches a real fingerprint.
    pub cfg_fingerprint: String,
}

/// One round's play ledger, as stored (structured JSON, narrated at serve time).
#[derive(Debug, Clone)]
pub struct RoundPlaysRow {
    pub round: u32,
    pub plays_json: String,
    pub timeline_json: String,
}

/// One cached coach answer (ADR-0010).
#[derive(Debug, Clone, PartialEq)]
pub struct CoachCacheRow {
    pub kind: String,
    pub round: u32,
    pub facts_hash: String,
    pub model: String,
    pub status: String,
    pub response_json: String,
    pub violations_json: String,
}

/// One own match's trend point (Trends screen chart, PROMPT.md M6).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct TrendMatchRow {
    pub match_id: i64,
    pub imported_at: String,
    pub map: String,
    pub deaths: u32,
    pub class13_pct: f32,
}

/// One (match, rule) flag count for the Trends screen's per-rule series.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RuleTrendCell {
    pub match_id: i64,
    pub rule_id: String,
    pub count: u32,
}

/// Own matches feed the tracked player's analytics; corpus matches feed the
/// reference grids only and stay invisible to library/habits/identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Own,
    Corpus,
}

impl MatchKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchKind::Own => "own",
            MatchKind::Corpus => "corpus",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CorpusMapCount {
    pub map: String,
    pub demos: u32,
}

/// One cached occupancy grid; `counts` is decoded from the LE-u32 row-major
/// blob (migration 5). Side/phase are strings at the DB boundary — commands
/// map them to/from `cf_analysis::corpus` enums.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GridRow {
    pub map: String,
    pub side: String,
    pub phase: String,
    pub size: usize,
    pub counts: Vec<u32>,
    pub demos: u32,
    pub samples: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GridStatus {
    pub map: String,
    pub side: String,
    pub phase: String,
    pub demos: u32,
    pub samples: u64,
    pub built_at: String,
}

/// A player's most recent sampled position at or before some tick.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPos {
    pub steamid: String,
    pub x: f32,
    pub y: f32,
    pub alive: bool,
}

pub struct Store {
    conn: Connection,
}

fn side_str(s: Side) -> &'static str {
    match s {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

fn counts_to_blob(counts: &[u32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(counts.len() * 4);
    for c in counts {
        b.extend_from_slice(&c.to_le_bytes());
    }
    b
}

fn blob_to_counts(blob: &[u8]) -> Vec<u32> {
    blob.as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect()
}

fn reason_str(r: &RoundEndReason) -> String {
    match r {
        RoundEndReason::TKilled => "t_killed".into(),
        RoundEndReason::CtKilled => "ct_killed".into(),
        RoundEndReason::BombDefused => "bomb_defused".into(),
        RoundEndReason::BombExploded => "bomb_exploded".into(),
        RoundEndReason::TargetSaved => "target_saved".into(),
        RoundEndReason::Other(s) => s.clone(),
    }
}

impl Store {
    /// Opens (creating if needed) and migrates the database.
    pub fn open(db_path: &Path) -> Result<Store, StoreError> {
        let mut conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        crate::migrations::migrate(&mut conn)?;
        Ok(Store { conn })
    }

    pub fn has_file_hash(&self, file_hash: &str) -> Result<bool, StoreError> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM matches WHERE file_hash = ?1)",
            [file_hash],
            |r| r.get(0),
        )?)
    }

    pub fn save_match(
        &mut self,
        file_name: &str,
        file_hash: &str,
        kind: MatchKind,
        data: &MatchData,
    ) -> Result<i64, StoreError> {
        if self.has_file_hash(file_hash)? {
            return Err(StoreError::DuplicateImport);
        }

        let (roster_a, roster_b, wins_a, wins_b) = derive_score(&data.rounds);
        let roster_json = |r: &[u64]| {
            serde_json::to_string(&r.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .expect("roster json")
        };

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO matches (file_name, file_hash, map, tickrate, imported_at,
                                  sample_every, score_a, score_b, roster_a_json, roster_b_json,
                                  kind)
             VALUES (?1, ?2, ?3, ?4, datetime('now'), ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                file_name,
                file_hash,
                data.map,
                data.tickrate,
                data.ticks.sample_every,
                wins_a,
                wins_b,
                roster_json(&roster_a),
                roster_json(&roster_b),
                kind.as_str(),
            ],
        )?;
        let match_id = tx.last_insert_rowid();

        insert_match_children(&tx, match_id, data)?;
        tx.commit()?;
        Ok(match_id)
    }

    pub fn list_matches(&self) -> Result<Vec<MatchSummary>, StoreError> {
        let tracked = self.tracked_steamid()?;
        let mut st = self.conn.prepare(
            "SELECT id, file_name, map, imported_at, score_a, score_b,
                    roster_a_json, roster_b_json,
                    (SELECT COUNT(*) FROM rounds r WHERE r.match_id = m.id) AS rounds
             FROM matches m WHERE m.kind = 'own' ORDER BY imported_at DESC, id DESC",
        )?;
        let rows = st.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, u32>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, u32>(8)?,
            ))
        })?;

        let mut out = vec![];
        for row in rows {
            let (id, file_name, map, imported_at, score_a, score_b, ra_json, rb_json, rounds) =
                row?;
            let mut summary = MatchSummary {
                id,
                file_name,
                map,
                imported_at,
                rounds,
                score_a,
                score_b,
                tracked_steamid: tracked.clone(),
                tracked_result: None,
                tracked_kills: None,
                tracked_deaths: None,
                tracked_hs_pct: None,
            };
            if let Some(tid) = &tracked {
                let played: bool = self.conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM players WHERE match_id = ?1 AND steamid = ?2)",
                    params![id, tid],
                    |r| r.get(0),
                )?;
                if played {
                    let ra: Vec<String> = serde_json::from_str(&ra_json).unwrap_or_default();
                    let rb: Vec<String> = serde_json::from_str(&rb_json).unwrap_or_default();
                    let (own, opp) = if ra.contains(tid) {
                        (score_a, score_b)
                    } else if rb.contains(tid) {
                        (score_b, score_a)
                    } else {
                        (score_a, score_b) // substitute edge: fall back to A
                    };
                    summary.tracked_result = Some(
                        match own.cmp(&opp) {
                            std::cmp::Ordering::Greater => "win",
                            std::cmp::Ordering::Less => "loss",
                            std::cmp::Ordering::Equal => "tie",
                        }
                        .to_string(),
                    );
                    let (kills, hs): (u32, u32) = self.conn.query_row(
                        "SELECT COUNT(*), COALESCE(SUM(headshot), 0) FROM kills
                         WHERE match_id = ?1 AND attacker = ?2 AND victim != ?2",
                        params![id, tid],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    let deaths: u32 = self.conn.query_row(
                        "SELECT COUNT(*) FROM kills WHERE match_id = ?1 AND victim = ?2",
                        params![id, tid],
                        |r| r.get(0),
                    )?;
                    summary.tracked_kills = Some(kills);
                    summary.tracked_deaths = Some(deaths);
                    summary.tracked_hs_pct = if kills > 0 {
                        Some(hs as f32 / kills as f32 * 100.0)
                    } else {
                        None
                    };
                }
            }
            out.push(summary);
        }
        Ok(out)
    }

    /// Persists a match's analysis output (replaces any previous run).
    pub fn save_analysis(
        &mut self,
        match_id: i64,
        out: &cf_analysis::AnalysisOutput,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM rule_flags WHERE match_id = ?1", [match_id])?;
        tx.execute("DELETE FROM insights WHERE match_id = ?1", [match_id])?;
        tx.execute("DELETE FROM death_class WHERE match_id = ?1", [match_id])?;
        tx.execute("DELETE FROM round_plays WHERE match_id = ?1", [match_id])?;
        {
            let mut st = tx.prepare(
                "INSERT INTO rule_flags (match_id, rule_id, round, tick, steamid, confidence,
                                         severity, details_json, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for f in &out.flags {
                st.execute(params![
                    match_id,
                    f.rule_id,
                    f.round,
                    f.tick,
                    f.steamid.to_string(),
                    f.confidence,
                    f.severity,
                    f.details.to_string(),
                    serde_json::to_string(&f.evidence).expect("flag evidence json"),
                ])?;
            }
            let mut st = tx.prepare(
                "INSERT INTO insights (match_id, detector, category, severity, confidence, round,
                                       player, title_data_json, metrics_json, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for i in &out.insights {
                st.execute(params![
                    match_id,
                    i.detector,
                    i.category.as_str(),
                    i.severity,
                    i.confidence,
                    i.round,
                    i.player.to_string(),
                    i.title_data.to_string(),
                    i.metrics.to_string(),
                    serde_json::to_string(&i.evidence).expect("evidence json"),
                ])?;
            }
            let mut st = tx.prepare(
                "INSERT INTO death_class (match_id, round, tick, victim, class_id, class_source,
                                          secondary_tags_json, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for d in &out.death_classes {
                st.execute(params![
                    match_id,
                    d.round,
                    d.tick,
                    d.victim.to_string(),
                    d.class_id,
                    d.class_source,
                    serde_json::to_string(&d.secondary_tags).expect("tags json"),
                    d.confidence,
                ])?;
            }
            let mut st = tx.prepare(
                "INSERT INTO round_plays (match_id, round, plays_json, timeline_json)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for r in &out.ledger {
                st.execute(params![
                    match_id,
                    r.round,
                    serde_json::to_string(&r.plays).expect("plays json"),
                    serde_json::to_string(&r.timeline).expect("timeline json"),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// All rule flags stored for one match (issue #9 round-review backfill
    /// input — the round-review engine wants a match's whole flag list, not
    /// a cross-corpus rule aggregate).
    pub fn flags_for_match(&self, match_id: i64) -> Result<Vec<FlagRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT rule_id, round, tick, steamid, severity, confidence, details_json
             FROM rule_flags WHERE match_id = ?1 ORDER BY round, tick",
        )?;
        let rows = st
            .query_map([match_id], |r| {
                Ok(FlagRow {
                    rule_id: r.get(0)?,
                    round: r.get(1)?,
                    tick: r.get(2)?,
                    steamid: r.get(3)?,
                    severity: r.get(4)?,
                    confidence: r.get(5)?,
                    details_json: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Persists a match's round-by-round reviews (issue #9 §7), replacing
    /// any previous run — the same DELETE+INSERT-in-one-tx model as
    /// `save_analysis`.
    pub fn save_round_reviews(
        &mut self,
        match_id: i64,
        rows: &[RoundReviewRow],
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM round_review WHERE match_id = ?1", [match_id])?;
        {
            let mut st = tx.prepare(
                "INSERT INTO round_review (match_id, round, impact, verdict, attention,
                                           selected, pivotal_tick, header_json, moments_json,
                                           cfg_fingerprint)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for r in rows {
                st.execute(params![
                    match_id,
                    r.round,
                    r.impact,
                    r.verdict,
                    r.attention,
                    r.selected,
                    r.pivotal_tick,
                    r.header_json,
                    r.moments_json,
                    r.cfg_fingerprint,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Loads a match's round-by-round reviews, ordered by round.
    pub fn load_round_reviews(&self, match_id: i64) -> Result<Vec<RoundReviewRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT round, impact, verdict, attention, selected, pivotal_tick,
                    header_json, moments_json, cfg_fingerprint
             FROM round_review WHERE match_id = ?1 ORDER BY round",
        )?;
        let rows = st
            .query_map([match_id], |r| {
                Ok(RoundReviewRow {
                    round: r.get(0)?,
                    impact: r.get(1)?,
                    verdict: r.get(2)?,
                    attention: r.get(3)?,
                    selected: r.get(4)?,
                    pivotal_tick: r.get(5)?,
                    header_json: r.get(6)?,
                    moments_json: r.get(7)?,
                    cfg_fingerprint: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// A match's stored play ledger, ordered by round (empty for imports
    /// that predate V1.2b — `re_analyze_match` fills it).
    pub fn load_round_plays(&self, match_id: i64) -> Result<Vec<RoundPlaysRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT round, plays_json, timeline_json FROM round_plays
             WHERE match_id = ?1 ORDER BY round",
        )?;
        let rows = st
            .query_map([match_id], |r| {
                Ok(RoundPlaysRow {
                    round: r.get(0)?,
                    plays_json: r.get(1)?,
                    timeline_json: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every distinct raw callout anyone stood in during the match (the
    /// position samples' `last_place`), sorted. The coach's known-callout
    /// set is this ∪ the ledger's places (V1.3 final-review fix #3): a place
    /// the coach names that nobody visited is an invention the validator
    /// can only catch if the place is in this set.
    pub fn distinct_places(&self, match_id: i64) -> Result<Vec<String>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT DISTINCT last_place FROM tick_samples
             WHERE match_id = ?1 AND last_place IS NOT NULL AND last_place != ''
             ORDER BY last_place",
        )?;
        let rows = st
            .query_map([match_id], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_coach_cache(
        &self,
        match_id: i64,
        kind: &str,
        round: u32,
    ) -> Result<Option<CoachCacheRow>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT kind, round, facts_hash, model, status, response_json, violations_json
                 FROM coach_cache WHERE match_id = ?1 AND kind = ?2 AND round = ?3",
                params![match_id, kind, round],
                |r| {
                    Ok(CoachCacheRow {
                        kind: r.get(0)?,
                        round: r.get(1)?,
                        facts_hash: r.get(2)?,
                        model: r.get(3)?,
                        status: r.get(4)?,
                        response_json: r.get(5)?,
                        violations_json: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn put_coach_cache(
        &mut self,
        match_id: i64,
        row: &CoachCacheRow,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO coach_cache (match_id, kind, round, facts_hash, model, status, response_json, violations_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(match_id, kind, round) DO UPDATE SET
               facts_hash = excluded.facts_hash, model = excluded.model, status = excluded.status,
               response_json = excluded.response_json, violations_json = excluded.violations_json,
               created_at = datetime('now')",
            params![match_id, row.kind, row.round, row.facts_hash, row.model, row.status, row.response_json, row.violations_json],
        )?;
        Ok(())
    }

    /// `kind`/`round` narrow the delete; both `None` clears the match.
    pub fn delete_coach_cache(
        &mut self,
        match_id: i64,
        kind: Option<&str>,
        round: Option<u32>,
    ) -> Result<(), StoreError> {
        match (kind, round) {
            (Some(k), Some(r)) => self.conn.execute(
                "DELETE FROM coach_cache WHERE match_id = ?1 AND kind = ?2 AND round = ?3",
                params![match_id, k, r],
            )?,
            (Some(k), None) => self.conn.execute(
                "DELETE FROM coach_cache WHERE match_id = ?1 AND kind = ?2",
                params![match_id, k],
            )?,
            _ => self
                .conn
                .execute("DELETE FROM coach_cache WHERE match_id = ?1", [match_id])?,
        };
        Ok(())
    }

    /// Records where a demo was imported from (V1.2b re-analyze input).
    pub fn set_source_path(&mut self, id: i64, path: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE matches SET source_path = ?2 WHERE id = ?1",
            params![id, path],
        )?;
        Ok(())
    }

    pub fn source_path(&self, id: i64) -> Result<Option<String>, StoreError> {
        let v = self
            .conn
            .query_row("SELECT source_path FROM matches WHERE id = ?1", [id], |r| {
                r.get::<_, Option<String>>(0)
            })
            .optional()?;
        Ok(v.flatten())
    }

    /// Per-match flag counts for one rule for the tracked player, newest
    /// match first, capped to `window` matches (§5A habit promotion input).
    pub fn rule_counts_across_matches(
        &self,
        tracked: &str,
        rule_id: &str,
        window: usize,
    ) -> Result<Vec<RuleMatchCount>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT m.id, m.map, m.imported_at,
                    (SELECT COUNT(*) FROM rule_flags f
                      WHERE f.match_id = m.id AND f.rule_id = ?2 AND f.steamid = ?1),
                    (SELECT f.evidence_json FROM rule_flags f
                      WHERE f.match_id = m.id AND f.rule_id = ?2 AND f.steamid = ?1
                      ORDER BY f.tick LIMIT 1),
                    (SELECT f.round FROM rule_flags f
                      WHERE f.match_id = m.id AND f.rule_id = ?2 AND f.steamid = ?1
                      ORDER BY f.tick LIMIT 1),
                    (SELECT f.tick FROM rule_flags f
                      WHERE f.match_id = m.id AND f.rule_id = ?2 AND f.steamid = ?1
                      ORDER BY f.tick LIMIT 1)
             FROM matches m
             WHERE m.kind = 'own'
             ORDER BY m.imported_at DESC, m.id DESC
             LIMIT ?3",
        )?;
        let rows = st
            .query_map(params![tracked, rule_id, window as i64], |r| {
                Ok(RuleMatchCount {
                    match_id: r.get(0)?,
                    map: r.get(1)?,
                    imported_at: r.get(2)?,
                    count: r.get(3)?,
                    first_evidence_json: r.get(4)?,
                    first_round: r.get(5)?,
                    first_tick: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Top callouts for one rule's flags across the recent window — feeds
    /// the "most often at Catwalk (5)" habit clause (issue #6 §3). First
    /// and only reader of rule_flags.details_json ('$.place'); flags
    /// without a place simply don't count (silence bias).
    pub fn rule_place_counts(
        &self,
        tracked: &str,
        rule_id: &str,
        window: usize,
    ) -> Result<Vec<(String, u32)>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT json_extract(f.details_json, '$.place') AS place, COUNT(*) AS n
             FROM rule_flags f
             WHERE f.steamid = ?1 AND f.rule_id = ?2
               AND f.match_id IN (SELECT id FROM matches WHERE kind = 'own'
                                  ORDER BY imported_at DESC, id DESC LIMIT ?3)
               AND place IS NOT NULL
             GROUP BY place
             ORDER BY n DESC, place ASC
             LIMIT 2",
        )?;
        let rows = st
            .query_map(params![tracked, rule_id, window as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Tracked player's death positions per map across all matches (from the
    /// nearest tick sample at or before each kill).
    ///
    /// The lookback is bounded to 10 s of the match's own tickrate — far
    /// wider than the ~16 Hz sampling interval, so it only excludes
    /// genuinely missing data (e.g. the first sample of the next round)
    /// rather than silently attributing a death to a stale sample from an
    /// earlier round (issue #6 §4). A death with no in-bound sample is
    /// dropped, never misplaced.
    pub fn death_positions(
        &self,
        tracked: &str,
        lookback_s: f32,
    ) -> Result<Vec<DeathPos>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT k.match_id, m.map, k.round, k.tick, t.x, t.y, t.last_place
             FROM kills k
             JOIN matches m ON m.id = k.match_id
             JOIN tick_samples t ON t.match_id = k.match_id AND t.steamid = k.victim
              AND t.tick = (SELECT MAX(tick) FROM tick_samples
                             WHERE match_id = k.match_id AND steamid = k.victim
                               AND tick <= k.tick
                               AND tick >= k.tick - CAST(?2 * m.tickrate AS INTEGER))
             WHERE k.victim = ?1 AND m.kind = 'own'
             ORDER BY k.match_id, k.tick",
        )?;
        let rows = st
            .query_map(params![tracked, lookback_s], |r| {
                Ok(DeathPos {
                    match_id: r.get(0)?,
                    map: r.get(1)?,
                    round: r.get(2)?,
                    tick: r.get(3)?,
                    x: r.get(4)?,
                    y: r.get(5)?,
                    place: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// (max severity, min confidence) of a rule's stored flags for the
    /// tracked player — habit scoring inputs, data-driven.
    pub fn rule_severity_confidence(
        &self,
        tracked: &str,
        rule_id: &str,
    ) -> Result<Option<(f32, f32)>, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT MAX(rf.severity), MIN(rf.confidence) FROM rule_flags rf
                 JOIN matches m ON m.id = rf.match_id AND m.kind = 'own'
                 WHERE rf.steamid = ?1 AND rf.rule_id = ?2",
                params![tracked, rule_id],
                |r| Ok((r.get::<_, Option<f32>>(0)?, r.get::<_, Option<f32>>(1)?)),
            )
            .map(|(s, c)| s.zip(c))?;
        Ok(row)
    }

    /// Distinct rule ids that ever flagged for the tracked player.
    pub fn flagged_rule_ids(&self, tracked: &str) -> Result<Vec<String>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT DISTINCT f.rule_id FROM rule_flags f
             JOIN matches m ON m.id = f.match_id
             WHERE f.steamid = ?1 AND m.kind = 'own' ORDER BY f.rule_id",
        )?;
        let rows = st
            .query_map([tracked], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Own matches chronologically with the tracked player's deaths and
    /// class-13 (outaimed in a fair duel — the taxonomy's good-news class)
    /// share — the Trends screen's x-axis and its baseline series.
    pub fn trend_matches(&self, tracked: &str) -> Result<Vec<TrendMatchRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT m.id, m.imported_at, m.map,
                    (SELECT COUNT(*) FROM kills k WHERE k.match_id = m.id AND k.victim = ?1),
                    (SELECT COUNT(*) FROM death_class dc WHERE dc.match_id = m.id),
                    (SELECT COUNT(*) FROM death_class dc
                      WHERE dc.match_id = m.id AND dc.class_id = 13)
             FROM matches m
             WHERE m.kind = 'own'
             ORDER BY m.imported_at ASC, m.id ASC",
        )?;
        let rows = st
            .query_map([tracked], |r| {
                let total_deaths: i64 = r.get(4)?;
                let class13: i64 = r.get(5)?;
                let class13_pct = if total_deaths > 0 {
                    100.0 * class13 as f32 / total_deaths as f32
                } else {
                    0.0
                };
                Ok(TrendMatchRow {
                    match_id: r.get(0)?,
                    imported_at: r.get(1)?,
                    map: r.get(2)?,
                    deaths: r.get(3)?,
                    class13_pct,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Per-match, per-rule flag counts for the tracked player across own
    /// matches — the Trends screen's per-rule series (unordered; the caller
    /// aligns cells to `trend_matches`' chronological order).
    pub fn rule_trend_counts(&self, tracked: &str) -> Result<Vec<RuleTrendCell>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT f.match_id, f.rule_id, COUNT(*)
             FROM rule_flags f
             JOIN matches m ON m.id = f.match_id
             WHERE f.steamid = ?1 AND m.kind = 'own'
             GROUP BY f.match_id, f.rule_id",
        )?;
        let rows = st
            .query_map([tracked], |r| {
                Ok(RuleTrendCell {
                    match_id: r.get(0)?,
                    rule_id: r.get(1)?,
                    count: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Per-round tracked K/D + winner for the report's timeline strip.
    pub fn per_round_stats(
        &self,
        match_id: i64,
        tracked: &str,
    ) -> Result<Vec<RoundStat>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT r.number, r.freeze_end_tick, r.winner,
                    (SELECT side FROM round_sides
                      WHERE match_id = r.match_id AND number = r.number AND steamid = ?2),
                    (SELECT COUNT(*) FROM kills k WHERE k.match_id = r.match_id
                      AND k.round = r.number AND k.attacker = ?2 AND k.victim != ?2),
                    (SELECT COUNT(*) FROM kills k WHERE k.match_id = r.match_id
                      AND k.round = r.number AND k.victim = ?2)
             FROM rounds r WHERE r.match_id = ?1 ORDER BY r.number",
        )?;
        let rows = st
            .query_map(params![match_id, tracked], |r| {
                Ok(RoundStat {
                    number: r.get(0)?,
                    freeze_end_tick: r.get(1)?,
                    winner: r.get(2)?,
                    tracked_side: r.get(3)?,
                    kills: r.get(4)?,
                    deaths: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Insight rows for a match, JSON fields as raw strings (UI decodes).
    pub fn insights_for_match(&self, match_id: i64) -> Result<Vec<InsightRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT detector, category, severity, confidence, round, player,
                    title_data_json, metrics_json, evidence_json
             FROM insights WHERE match_id = ?1
             ORDER BY severity * confidence DESC",
        )?;
        let rows = st
            .query_map([match_id], |r| {
                Ok(InsightRow {
                    detector: r.get(0)?,
                    category: r.get(1)?,
                    severity: r.get(2)?,
                    confidence: r.get(3)?,
                    round: r.get(4)?,
                    player: r.get(5)?,
                    title_data_json: r.get(6)?,
                    metrics_json: r.get(7)?,
                    evidence_json: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn death_classes_for_match(
        &self,
        match_id: i64,
    ) -> Result<Vec<DeathClassDbRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT round, tick, victim, class_id, class_source, secondary_tags_json, confidence
             FROM death_class WHERE match_id = ?1 ORDER BY tick",
        )?;
        let rows = st
            .query_map([match_id], |r| {
                Ok(DeathClassDbRow {
                    round: r.get(0)?,
                    tick: r.get(1)?,
                    victim: r.get(2)?,
                    class_id: r.get(3)?,
                    class_source: r.get(4)?,
                    secondary_tags_json: r.get(5)?,
                    confidence: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Corpus demo counts per map (kind='corpus' only).
    pub fn corpus_summary(&self) -> Result<Vec<CorpusMapCount>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT map, COUNT(*) FROM matches WHERE kind = 'corpus'
             GROUP BY map ORDER BY map",
        )?;
        let rows = st
            .query_map([], |r| {
                Ok(CorpusMapCount {
                    map: r.get(0)?,
                    demos: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Ids of corpus-kind matches on one map, oldest first.
    pub fn corpus_match_ids(&self, map: &str) -> Result<Vec<i64>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM matches WHERE kind = 'corpus' AND map = ?1 ORDER BY id")?;
        let rows = st
            .query_map([map], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Ids of the player's own matches on one map (D6 re-analysis after a
    /// corpus rebuild), oldest first.
    pub fn own_match_ids(&self, map: &str) -> Result<Vec<i64>, StoreError> {
        let mut st = self
            .conn
            .prepare("SELECT id FROM matches WHERE kind = 'own' AND map = ?1 ORDER BY id")?;
        let rows = st
            .query_map([map], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Lean per-match round list (corpus phase sampling — match_detail is
    /// too heavy to load per corpus demo).
    pub fn rounds_for_match(&self, id: i64) -> Result<Vec<RoundInfo>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT number, start_tick, freeze_end_tick, end_tick, officially_ended_tick,
                    winner, reason
             FROM rounds WHERE match_id = ?1 ORDER BY number",
        )?;
        let rows = st
            .query_map([id], |r| {
                Ok(RoundInfo {
                    number: r.get(0)?,
                    start_tick: r.get(1)?,
                    freeze_end_tick: r.get(2)?,
                    end_tick: r.get(3)?,
                    officially_ended_tick: r.get(4)?,
                    winner: r.get(5)?,
                    reason: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Lean (steamid, side) pairs for one round; side is "CT" | "T".
    pub fn sides_for_round(
        &self,
        id: i64,
        number: u32,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT steamid, side FROM round_sides
             WHERE match_id = ?1 AND number = ?2 ORDER BY steamid",
        )?;
        let rows = st
            .query_map(params![id, number], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Lean (map, tickrate) lookup without loading the full match detail.
    pub fn match_map_tickrate(&self, id: i64) -> Result<Option<(String, f64)>, StoreError> {
        let v = self
            .conn
            .query_row(
                "SELECT map, tickrate FROM matches WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    /// First "planted" bomb event inside the round's tick span, if any.
    pub fn bomb_plant_tick(&self, id: i64, round: u32) -> Result<Option<i32>, StoreError> {
        let v = self.conn.query_row(
            "SELECT MIN(be.tick) FROM bomb_events be
             JOIN rounds r ON r.match_id = be.match_id AND r.number = ?2
             WHERE be.match_id = ?1 AND be.kind = 'planted'
               AND be.tick BETWEEN r.start_tick AND r.end_tick",
            params![id, round],
            |r| r.get::<_, Option<i32>>(0),
        )?;
        Ok(v)
    }

    /// Every player's most recent sample in `min_tick..=tick` (players with
    /// no sample in that window are absent). `min_tick` is the round's
    /// start tick — without it, a player who disconnects keeps "standing"
    /// at their last sample for the rest of the match.
    pub fn positions_at(
        &self,
        id: i64,
        tick: i32,
        min_tick: i32,
    ) -> Result<Vec<PlayerPos>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT ts.steamid, ts.x, ts.y, ts.is_alive
             FROM tick_samples ts
             WHERE ts.match_id = ?1
               AND ts.tick = (SELECT MAX(t2.tick) FROM tick_samples t2
                              WHERE t2.match_id = ?1 AND t2.steamid = ts.steamid
                                AND t2.tick <= ?2 AND t2.tick >= ?3)
             ORDER BY ts.steamid",
        )?;
        let rows = st
            .query_map(params![id, tick, min_tick], |r| {
                Ok(PlayerPos {
                    steamid: r.get(0)?,
                    x: r.get::<_, f64>(1)? as f32,
                    y: r.get::<_, f64>(2)? as f32,
                    alive: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Upserts grid rows in one transaction; counts stored as an LE-u32
    /// row-major blob.
    pub fn save_grids(&mut self, grids: &[GridRow]) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        {
            let mut st = tx.prepare(
                "INSERT INTO corpus_grids (map, side, phase, size, counts, demos, samples, built_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
                 ON CONFLICT(map, side, phase) DO UPDATE SET
                   size = excluded.size, counts = excluded.counts, demos = excluded.demos,
                   samples = excluded.samples, built_at = excluded.built_at",
            )?;
            for g in grids {
                st.execute(params![
                    g.map,
                    g.side,
                    g.phase,
                    g.size as i64,
                    counts_to_blob(&g.counts),
                    g.demos,
                    g.samples as i64,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_grids(&self, map: &str) -> Result<Vec<GridRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT map, side, phase, size, counts, demos, samples
             FROM corpus_grids WHERE map = ?1 ORDER BY side, phase",
        )?;
        let rows = st
            .query_map([map], |r| {
                Ok(GridRow {
                    map: r.get(0)?,
                    side: r.get(1)?,
                    phase: r.get(2)?,
                    size: r.get::<_, i64>(3)? as usize,
                    counts: blob_to_counts(&r.get::<_, Vec<u8>>(4)?),
                    demos: r.get(5)?,
                    samples: r.get::<_, i64>(6)? as u64,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn grid_status(&self) -> Result<Vec<GridStatus>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT map, side, phase, demos, samples, built_at
             FROM corpus_grids ORDER BY map, side, phase",
        )?;
        let rows = st
            .query_map([], |r| {
                Ok(GridStatus {
                    map: r.get(0)?,
                    side: r.get(1)?,
                    phase: r.get(2)?,
                    demos: r.get(3)?,
                    samples: r.get::<_, i64>(4)? as u64,
                    built_at: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Replaces one detector's insights for a match (D6 re-analysis after a
    /// corpus rebuild) without touching other analysis rows.
    pub fn replace_detector_insights(
        &mut self,
        match_id: i64,
        detector: &str,
        insights: &[cf_analysis::Insight],
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM insights WHERE match_id = ?1 AND detector = ?2",
            params![match_id, detector],
        )?;
        {
            let mut st = tx.prepare(
                "INSERT INTO insights (match_id, detector, category, severity, confidence, round,
                                       player, title_data_json, metrics_json, evidence_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for i in insights {
                st.execute(params![
                    match_id,
                    i.detector,
                    i.category.as_str(),
                    i.severity,
                    i.confidence,
                    i.round,
                    i.player.to_string(),
                    i.title_data.to_string(),
                    i.metrics.to_string(),
                    serde_json::to_string(&i.evidence).expect("evidence json"),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let v = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    pub fn delete_setting(&mut self, key: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn match_detail(&self, id: i64) -> Result<Option<MatchDetail>, StoreError> {
        let head = self
            .conn
            .query_row(
                "SELECT map, tickrate, sample_every, score_a, score_b FROM matches WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, f32>(1)?,
                        r.get::<_, u32>(2)?,
                        r.get::<_, u32>(3)?,
                        r.get::<_, u32>(4)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((map, tickrate, sample_every, score_a, score_b)) = head else {
            return Ok(None);
        };

        let mut st = self
            .conn
            .prepare("SELECT steamid, name FROM players WHERE match_id = ?1 ORDER BY steamid")?;
        let players = st
            .query_map([id], |r| {
                Ok(PlayerInfo {
                    steamid: r.get(0)?,
                    name: r.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT number, start_tick, freeze_end_tick, end_tick, officially_ended_tick,
                    winner, reason
             FROM rounds WHERE match_id = ?1 ORDER BY number",
        )?;
        let rounds = st
            .query_map([id], |r| {
                Ok(RoundInfo {
                    number: r.get(0)?,
                    start_tick: r.get(1)?,
                    freeze_end_tick: r.get(2)?,
                    end_tick: r.get(3)?,
                    officially_ended_tick: r.get(4)?,
                    winner: r.get(5)?,
                    reason: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT round, tick, attacker, victim, assister, weapon, headshot
             FROM kills WHERE match_id = ?1 ORDER BY tick",
        )?;
        let kills = st
            .query_map([id], |r| {
                Ok(KillInfo {
                    round: r.get(0)?,
                    tick: r.get(1)?,
                    attacker: r.get(2)?,
                    victim: r.get(3)?,
                    assister: r.get(4)?,
                    weapon: r.get(5)?,
                    headshot: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT tick, kind, thrower, x, y, z FROM grenades WHERE match_id = ?1 ORDER BY tick",
        )?;
        let grenades = st
            .query_map([id], |r| {
                Ok(GrenadeInfo {
                    tick: r.get(0)?,
                    kind: r.get(1)?,
                    thrower: r.get(2)?,
                    x: r.get(3)?,
                    y: r.get(4)?,
                    z: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT tick, kind, player FROM bomb_events WHERE match_id = ?1 ORDER BY tick",
        )?;
        let bomb_events = st
            .query_map([id], |r| {
                Ok(BombInfo {
                    tick: r.get(0)?,
                    kind: r.get(1)?,
                    player: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut st = self.conn.prepare(
            "SELECT number, steamid, side FROM round_sides WHERE match_id = ?1
             ORDER BY number, steamid",
        )?;
        let round_sides = st
            .query_map([id], |r| {
                Ok(RoundSideInfo {
                    number: r.get(0)?,
                    steamid: r.get(1)?,
                    side: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(MatchDetail {
            id,
            map,
            tickrate,
            sample_every,
            score_a,
            score_b,
            players,
            rounds,
            kills,
            grenades,
            bomb_events,
            round_sides,
        }))
    }

    pub fn round_ticks(&self, id: i64, round: u32) -> Result<RoundTicks, StoreError> {
        let range = self
            .conn
            .query_row(
                "SELECT start_tick, COALESCE(officially_ended_tick, end_tick)
                 FROM rounds WHERE match_id = ?1 AND number = ?2",
                params![id, round],
                |r| Ok((r.get::<_, i32>(0)?, r.get::<_, i32>(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        let Some((lo, hi)) = range else {
            return Ok(RoundTicks::default());
        };

        let mut st = self.conn.prepare(
            "SELECT tick, steamid, x, y, z, yaw, health, is_alive, team_num,
                    active_weapon, last_place
             FROM tick_samples
             WHERE match_id = ?1 AND tick BETWEEN ?2 AND ?3
             ORDER BY tick, steamid",
        )?;
        let mut out = RoundTicks::default();
        let mut rows = st.query(params![id, lo, hi])?;
        while let Some(r) = rows.next()? {
            out.tick.push(r.get(0)?);
            out.steamid.push(r.get(1)?);
            out.x.push(r.get(2)?);
            out.y.push(r.get(3)?);
            out.z.push(r.get(4)?);
            out.yaw.push(r.get(5)?);
            out.health.push(r.get(6)?);
            out.is_alive.push(r.get(7)?);
            out.team_num.push(r.get(8)?);
            out.active_weapon.push(r.get(9)?);
            out.last_place.push(r.get(10)?);
        }
        Ok(out)
    }

    /// Tracked player: explicit setting wins; otherwise the steamid appearing
    /// in the most imported matches (PROMPT.md §13 M1 identity detection).
    /// Deletes a match and (via FK cascade) every child row — kills, ticks,
    /// analysis, the lot. Frees the file hash for re-import.
    pub fn delete_match(&mut self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM matches WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Display name for a steamid, from the most recent own match it
    /// appeared in.
    pub fn player_name(&self, steamid: &str) -> Result<Option<String>, StoreError> {
        let v = self
            .conn
            .query_row(
                "SELECT p.name FROM players p
                 JOIN matches m ON m.id = p.match_id AND m.kind = 'own'
                 WHERE p.steamid = ?1 ORDER BY m.id DESC LIMIT 1",
                [steamid],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(v)
    }

    pub fn tracked_steamid(&self) -> Result<Option<String>, StoreError> {
        if let Some(v) = self.get_setting("tracked_steamid")? {
            return Ok(Some(v));
        }
        let modal = self
            .conn
            .query_row(
                "SELECT p.steamid FROM players p
                 JOIN matches m ON m.id = p.match_id
                 WHERE m.kind = 'own'
                 GROUP BY p.steamid
                 ORDER BY COUNT(DISTINCT p.match_id) DESC, p.steamid ASC
                 LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        Ok(modal)
    }
}

/// Every child-table insert for one match — shared by `save_match` and
/// `replace_match_data` so a re-parse writes exactly what an import does.
fn insert_match_children(
    tx: &rusqlite::Transaction<'_>,
    match_id: i64,
    data: &MatchData,
) -> Result<(), StoreError> {
    let mut st = tx.prepare("INSERT INTO players (match_id, steamid, name) VALUES (?1, ?2, ?3)")?;
    for p in &data.players {
        st.execute(params![match_id, p.steamid.to_string(), p.name])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO rounds (match_id, number, start_tick, freeze_end_tick, end_tick,
                             officially_ended_tick, winner, reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    let mut st_side = tx.prepare(
        "INSERT INTO round_sides (match_id, number, steamid, side) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for r in &data.rounds {
        st.execute(params![
            match_id,
            r.number,
            r.start_tick,
            r.freeze_end_tick,
            r.end_tick,
            r.officially_ended_tick,
            side_str(r.winner),
            reason_str(&r.reason),
        ])?;
        for s in &r.ct_steamids {
            st_side.execute(params![match_id, r.number, s.to_string(), "CT"])?;
        }
        for s in &r.t_steamids {
            st_side.execute(params![match_id, r.number, s.to_string(), "T"])?;
        }
    }

    let mut st = tx.prepare(
        "INSERT INTO kills (match_id, round, tick, attacker, victim, assister, weapon,
                            headshot, penetrated, thru_smoke, attacker_blind, assistedflash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for k in &data.kills {
        st.execute(params![
            match_id,
            k.round,
            k.tick,
            k.attacker.map(|a| a.to_string()),
            k.victim.to_string(),
            k.assister.map(|a| a.to_string()),
            k.weapon,
            k.headshot,
            k.penetrated,
            k.thru_smoke,
            k.attacker_blind,
            k.assistedflash,
        ])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO blinds (match_id, tick, victim, attacker, duration)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for b in &data.blinds {
        st.execute(params![
            match_id,
            b.tick,
            b.victim.to_string(),
            b.attacker.map(|a| a.to_string()),
            b.duration,
        ])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO grenades (match_id, tick, kind, thrower, x, y, z)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for g in &data.grenades {
        st.execute(params![
            match_id,
            g.tick,
            g.kind,
            g.thrower.map(|t| t.to_string()),
            g.x,
            g.y,
            g.z,
        ])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO bomb_events (match_id, tick, kind, player) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for b in &data.bomb_events {
        st.execute(params![
            match_id,
            b.tick,
            b.kind,
            b.player.map(|p| p.to_string())
        ])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO tick_samples (match_id, steamid, tick, x, y, z, yaw, health,
                                   is_alive, team_num, active_weapon, spotted, last_place,
                                   is_scoped)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )?;
    let t = &data.ticks;
    for i in 0..t.len() {
        st.execute(params![
            match_id,
            t.steamid[i].to_string(),
            t.tick[i],
            t.x[i],
            t.y[i],
            t.z[i],
            t.yaw[i],
            t.health[i],
            t.is_alive[i],
            t.team_num[i],
            t.active_weapon[i],
            t.spotted[i],
            t.last_place[i],
            t.is_scoped.get(i).copied(),
        ])?;
    }

    let mut st =
        tx.prepare("INSERT INTO shots (match_id, tick, player, weapon) VALUES (?1, ?2, ?3, ?4)")?;
    for s in &data.shots {
        st.execute(params![match_id, s.tick, s.player.to_string(), s.weapon])?;
    }

    let mut st = tx.prepare(
        "INSERT INTO hurts (match_id, tick, victim, attacker, dmg_health, weapon, hitgroup)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for h in &data.hurts {
        st.execute(params![
            match_id,
            h.tick,
            h.victim.to_string(),
            h.attacker.map(|a| a.to_string()),
            h.dmg_health,
            h.weapon,
            h.hitgroup,
        ])?;
    }

    let mut st = tx.prepare("INSERT INTO reloads (match_id, tick, player) VALUES (?1, ?2, ?3)")?;
    for r in &data.reloads {
        st.execute(params![match_id, r.tick, r.player.to_string()])?;
    }

    let mut st = tx.prepare(
        "INSERT OR REPLACE INTO inventories (match_id, tick, steamid, items_json)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for inv in &data.inventories {
        st.execute(params![
            match_id,
            inv.tick,
            inv.steamid.to_string(),
            serde_json::to_string(&inv.items).expect("items json"),
        ])?;
    }

    Ok(())
}

/// Tables that hang off `matches(id)` and are rewritten by a re-parse.
const MATCH_CHILD_TABLES: &[&str] = &[
    "players",
    "rounds",
    "round_sides",
    "kills",
    "blinds",
    "grenades",
    "bomb_events",
    "tick_samples",
    "shots",
    "hurts",
    "reloads",
    "inventories",
];

/// Analysis tables — everything `save_analysis` and `save_round_reviews`
/// write. `replace_match_data` clears them in the SAME transaction as the
/// child-table swap (V1.2b final-review fix wave, #8), so a re-parse can
/// never commit new parsed rows beside an old analysis; the caller re-runs
/// the pipeline afterwards.
const MATCH_ANALYSIS_TABLES: &[&str] = &[
    "rule_flags",
    "insights",
    "death_class",
    "round_review",
    "round_plays",
    "coach_cache",
];

#[derive(Debug, Clone, PartialEq)]
pub struct MatchFile {
    pub file_name: String,
    pub file_hash: String,
    pub source_path: Option<String>,
}

impl Store {
    /// Re-parse support (V1.2b): replaces a match's parsed rows in place —
    /// same `id`, so report/replay URLs and cross-match keys survive — and
    /// clears the match's analysis rows (`MATCH_ANALYSIS_TABLES`) in the
    /// same transaction: after this commits the match is analysis-less,
    /// like a fresh import, never a new parse beside an old analysis.
    pub fn replace_match_data(&mut self, id: i64, data: &MatchData) -> Result<(), StoreError> {
        let (roster_a, roster_b, wins_a, wins_b) = derive_score(&data.rounds);
        let roster_json = |r: &[u64]| {
            serde_json::to_string(&r.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .expect("roster json")
        };
        let tx = self.conn.transaction()?;
        for table in MATCH_CHILD_TABLES.iter().chain(MATCH_ANALYSIS_TABLES) {
            tx.execute(&format!("DELETE FROM {table} WHERE match_id = ?1"), [id])?;
        }
        tx.execute(
            "UPDATE matches SET map = ?2, tickrate = ?3, sample_every = ?4, score_a = ?5,
                                score_b = ?6, roster_a_json = ?7, roster_b_json = ?8
             WHERE id = ?1",
            params![
                id,
                data.map,
                data.tickrate,
                data.ticks.sample_every,
                wins_a,
                wins_b,
                roster_json(&roster_a),
                roster_json(&roster_b),
            ],
        )?;
        insert_match_children(&tx, id, data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn match_file(&self, id: i64) -> Result<Option<MatchFile>, StoreError> {
        let v = self
            .conn
            .query_row(
                "SELECT file_name, file_hash, source_path FROM matches WHERE id = ?1",
                [id],
                |r| {
                    Ok(MatchFile {
                        file_name: r.get(0)?,
                        file_hash: r.get(1)?,
                        source_path: r.get(2)?,
                    })
                },
            )
            .optional()?;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_parser::model::{
        Blind, Kill, MatchData, PlayerMeta, Round, RoundEndReason, Side, TickTable,
    };

    fn round(number: u32, winner: Side, ct: Vec<u64>, t: Vec<u64>) -> Round {
        Round {
            number,
            start_tick: number as i32 * 1000,
            freeze_end_tick: Some(number as i32 * 1000 + 100),
            end_tick: number as i32 * 1000 + 900,
            officially_ended_tick: Some(number as i32 * 1000 + 950),
            winner,
            reason: RoundEndReason::TKilled,
            ct_steamids: ct,
            t_steamids: t,
        }
    }

    fn kill(tick: i32, round: u32, attacker: u64, victim: u64, headshot: bool) -> Kill {
        Kill {
            tick,
            round,
            attacker: Some(attacker),
            victim,
            assister: None,
            weapon: "ak47".into(),
            headshot,
            penetrated: 0,
            thru_smoke: false,
            attacker_blind: false,
            assistedflash: false,
        }
    }

    /// Roster A = {1, 2} (CT round 1), roster B = {3, 4}. A wins 2-1.
    fn sample_match() -> MatchData {
        let mut ticks = TickTable {
            sample_every: 4,
            ..Default::default()
        };
        for (tick, steamid) in [(1100, 1u64), (1100, 3u64), (2100, 1u64)] {
            ticks.tick.push(tick);
            ticks.steamid.push(steamid);
            ticks.x.push(100.0);
            ticks.y.push(-50.0);
            ticks.z.push(0.0);
            ticks.yaw.push(90.0);
            ticks.health.push(100);
            ticks.is_alive.push(true);
            ticks.team_num.push(3);
            ticks.active_weapon.push(Some("weapon_ak47".into()));
            ticks.spotted.push(false);
            ticks.last_place.push(Some("BombsiteA".into()));
            ticks.is_scoped.push(false);
        }
        MatchData {
            map: "de_mirage".into(),
            tickrate: 64.0,
            players: vec![
                PlayerMeta {
                    steamid: 1,
                    name: "alice".into(),
                },
                PlayerMeta {
                    steamid: 2,
                    name: "bob".into(),
                },
                PlayerMeta {
                    steamid: 3,
                    name: "carol".into(),
                },
                PlayerMeta {
                    steamid: 4,
                    name: "dave".into(),
                },
            ],
            rounds: vec![
                round(1, Side::Ct, vec![1, 2], vec![3, 4]),
                round(2, Side::T, vec![3, 4], vec![1, 2]),
                round(3, Side::Ct, vec![3, 4], vec![1, 2]),
            ],
            kills: vec![
                kill(1200, 1, 1, 3, true),
                kill(1300, 1, 1, 4, false),
                kill(2200, 2, 3, 1, false),
            ],
            blinds: vec![Blind {
                tick: 1150,
                victim: 3,
                attacker: Some(1),
                duration: 2.4,
            }],
            grenades: vec![],
            bomb_events: vec![],
            shots: vec![Shot {
                tick: 1190,
                player: 1,
                weapon: "weapon_usp_silencer".into(),
            }],
            hurts: vec![Hurt {
                tick: 1195,
                victim: 3,
                attacker: Some(1),
                dmg_health: 27,
                weapon: "usp_silencer".into(),
                hitgroup: "chest".into(),
            }],
            reloads: vec![Reload {
                tick: 1400,
                player: 1,
            }],
            inventories: vec![InventorySample {
                tick: 2200,
                steamid: 1,
                items: vec!["Flashbang".into(), "Smoke Grenade".into()],
            }],
            ticks,
        }
    }

    fn open_tmp() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("test.db")).unwrap();
        (dir, store)
    }

    /// One saved match, for tests that don't care about its contents beyond
    /// having a valid `match_id` to hang analysis/ledger rows off of. Returns
    /// the `TempDir` guard first so it drops last (fields drop in reverse
    /// declaration order), same as `open_tmp()`'s callers.
    fn one_match() -> (tempfile::TempDir, Store, i64, MatchData) {
        let (dir, mut store) = open_tmp();
        let data = sample_match();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &data)
            .unwrap();
        (dir, store, id, data)
    }

    #[test]
    fn migrations_apply_fresh_and_are_idempotent_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        {
            let store = Store::open(&path).unwrap();
            assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 9);
        }
        // Reopen: migrations must not re-apply / error.
        let store = Store::open(&path).unwrap();
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 9);
    }

    #[test]
    fn save_and_list_roundtrip_with_tracked_stats() {
        let (_dir, mut store) = open_tmp();
        store.set_setting("tracked_steamid", "1").unwrap();
        let id = store
            .save_match("m1.dem", "hash-1", MatchKind::Own, &sample_match())
            .unwrap();
        assert!(id > 0);
        let list = store.list_matches().unwrap();
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!(m.map, "de_mirage");
        assert_eq!(m.rounds, 3);
        assert_eq!((m.score_a, m.score_b), (2, 1));
        assert_eq!(m.tracked_steamid.as_deref(), Some("1"));
        assert_eq!(m.tracked_result.as_deref(), Some("win"));
        assert_eq!(m.tracked_kills, Some(2));
        assert_eq!(m.tracked_deaths, Some(1));
        assert_eq!(m.tracked_hs_pct, Some(50.0));
    }

    #[test]
    fn duplicate_hash_rejected() {
        let (_dir, mut store) = open_tmp();
        store
            .save_match("m1.dem", "hash-1", MatchKind::Own, &sample_match())
            .unwrap();
        let err = store.save_match("m1-copy.dem", "hash-1", MatchKind::Own, &sample_match());
        assert!(matches!(err, Err(StoreError::DuplicateImport)));
        assert_eq!(store.list_matches().unwrap().len(), 1);
    }

    #[test]
    fn settings_roundtrip_and_overwrite() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(store.get_setting("k").unwrap(), None);
        store.set_setting("k", "v1").unwrap();
        store.set_setting("k", "v2").unwrap();
        assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn tracked_falls_back_to_modal_steamid_across_matches() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(store.tracked_steamid().unwrap(), None);
        store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let mut second = sample_match();
        // Second match: only players 1 and 9 — player 1 now appears twice.
        second.players = vec![
            PlayerMeta {
                steamid: 1,
                name: "alice".into(),
            },
            PlayerMeta {
                steamid: 9,
                name: "eve".into(),
            },
        ];
        store
            .save_match("m2.dem", "h2", MatchKind::Own, &second)
            .unwrap();
        assert_eq!(store.tracked_steamid().unwrap().as_deref(), Some("1"));
    }

    #[test]
    fn match_detail_returns_full_read_model() {
        let (_dir, mut store) = open_tmp();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let d = store.match_detail(id).unwrap().expect("detail");
        assert_eq!(d.map, "de_mirage");
        assert_eq!((d.score_a, d.score_b), (2, 1));
        assert_eq!(d.players.len(), 4);
        assert_eq!(d.rounds.len(), 3);
        assert_eq!(d.rounds[0].winner, "CT");
        assert_eq!(d.rounds[0].reason, "t_killed");
        assert_eq!(d.kills.len(), 3);
        assert_eq!(d.kills[0].attacker.as_deref(), Some("1"));
        assert_eq!(d.kills[0].victim, "3");
        // 3 rounds × 4 players per side rows
        assert_eq!(d.round_sides.len(), 12);
        assert!(d
            .round_sides
            .iter()
            .any(|s| s.number == 2 && s.steamid == "1" && s.side == "T"));
    }

    #[test]
    fn match_detail_none_for_unknown_id() {
        let (_dir, store) = open_tmp();
        assert!(store.match_detail(999).unwrap().is_none());
    }

    #[test]
    fn replace_match_data_keeps_the_id_and_swaps_every_child_row() {
        let (_dir, mut store, match_id, mut data) = one_match();
        let before = store.match_detail(match_id).unwrap().unwrap();
        let before_hash: String = store
            .conn
            .query_row(
                "SELECT file_hash FROM matches WHERE id = ?1",
                [match_id],
                |r| r.get(0),
            )
            .unwrap();
        // The previous parse's analysis: a flag, an insight, a death class,
        // a one-round ledger and a review row — all of which must be gone
        // after the replace, atomically with the child-table swap (V1.2b
        // final-review fix wave, #8).
        let analysis = cf_analysis::AnalysisOutput {
            flags: vec![cf_analysis::RuleFlag {
                rule_id: "H2_ISOLATED_DEATH",
                round: 2,
                tick: 2200,
                steamid: 1,
                confidence: 0.75,
                severity: 0.8,
                details: serde_json::json!({}),
                evidence: cf_analysis::EvidenceRef {
                    round: 2,
                    tick_start: 1880,
                    tick_end: 2328,
                    focus_players: vec![1],
                    camera_hint: None,
                },
            }],
            insights: vec![cf_analysis::Insight {
                detector: "H2_ISOLATED_DEATH".into(),
                category: cf_analysis::Category::Deaths,
                severity: 0.8,
                confidence: 0.75,
                round: 0,
                player: 1,
                title_data: serde_json::json!({"count": 1}),
                metrics: serde_json::json!({"count": 1}),
                evidence: vec![],
            }],
            death_classes: vec![cf_analysis::DeathClassRow {
                round: 2,
                tick: 2200,
                victim: 1,
                class_id: 6,
                class_source: "H2_ISOLATED_DEATH".into(),
                secondary_tags: vec![],
                confidence: 0.75,
            }],
            ledger: vec![cf_analysis::play_ledger::RoundLedger {
                round: 1,
                plays: vec![],
                timeline: vec![],
            }],
        };
        store.save_analysis(match_id, &analysis).unwrap();
        store
            .save_round_reviews(
                match_id,
                &[RoundReviewRow {
                    round: 1,
                    impact: 0.1,
                    verdict: "quiet".to_string(),
                    attention: "none".to_string(),
                    selected: false,
                    pivotal_tick: 1200,
                    header_json: "{}".to_string(),
                    moments_json: "[]".to_string(),
                    cfg_fingerprint: cf_analysis::round_review::cfg_fingerprint(
                        &cf_analysis::config::RbrCfg::default(),
                    ),
                }],
            )
            .unwrap();
        store
            .put_coach_cache(
                match_id,
                &CoachCacheRow {
                    kind: "round".to_string(),
                    round: 1,
                    facts_hash: "abc".to_string(),
                    model: "gemini-3.7-flash".to_string(),
                    status: "ok".to_string(),
                    response_json: "{\"round\":1}".to_string(),
                    violations_json: "[]".to_string(),
                },
            )
            .unwrap();
        assert_eq!(store.load_round_plays(match_id).unwrap().len(), 1);
        assert_eq!(store.flags_for_match(match_id).unwrap().len(), 1);
        assert_eq!(store.load_round_reviews(match_id).unwrap().len(), 1);
        assert!(store
            .get_coach_cache(match_id, "round", 1)
            .unwrap()
            .is_some());

        // A re-parse of the "same" demo with one extra kill.
        data.kills.push(cf_parser::model::Kill {
            tick: before.rounds[0].start_tick + 500,
            round: 1,
            attacker: None,
            victim: 1,
            assister: None,
            weapon: "world".to_string(),
            headshot: false,
            penetrated: 0,
            thru_smoke: false,
            attacker_blind: false,
            assistedflash: false,
        });
        store.replace_match_data(match_id, &data).unwrap();
        let after = store.match_detail(match_id).unwrap().unwrap();
        assert_eq!(after.id, match_id);
        assert_eq!(after.kills.len(), before.kills.len() + 1);
        assert_eq!(
            after.rounds.len(),
            before.rounds.len(),
            "rounds re-inserted, not doubled"
        );
        assert_eq!(after.players.len(), before.players.len());
        let f = store.match_file(match_id).unwrap().unwrap();
        assert_eq!(f.file_hash, before_hash);
        assert_eq!(f.source_path, None);
        assert!(
            store.load_round_plays(match_id).unwrap().is_empty(),
            "the old ledger must not survive the re-parse"
        );
        assert!(
            store.flags_for_match(match_id).unwrap().is_empty(),
            "the old flags must not survive the re-parse"
        );
        assert!(store.insights_for_match(match_id).unwrap().is_empty());
        assert!(store.death_classes_for_match(match_id).unwrap().is_empty());
        assert!(store.load_round_reviews(match_id).unwrap().is_empty());
        assert!(
            store
                .get_coach_cache(match_id, "round", 1)
                .unwrap()
                .is_none(),
            "the old coach cache must not survive the re-parse"
        );
    }

    #[test]
    fn round_ticks_returns_only_in_range_rows_sorted() {
        let (_dir, mut store) = open_tmp();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        // Round 1 range: start 1000 → officially_ended 1950. Samples at 1100 (×2 players).
        let rt = store.round_ticks(id, 1).unwrap();
        assert_eq!(rt.tick, vec![1100, 1100]);
        assert_eq!(rt.steamid, vec!["1".to_string(), "3".to_string()]);
        assert_eq!(rt.x[0], 100.0);
        assert!(rt.is_alive[0]);
        // Round 2 range: 2000 → 2950. Sample at 2100 (player 1 only).
        let rt2 = store.round_ticks(id, 2).unwrap();
        assert_eq!(rt2.tick, vec![2100]);
        // Unknown round → empty, not error.
        assert!(store.round_ticks(id, 99).unwrap().tick.is_empty());
    }

    #[test]
    fn save_analysis_roundtrips_and_replaces() {
        use cf_analysis::{
            AnalysisOutput, Category, DeathClassRow, EvidenceRef, Insight, RuleFlag,
        };
        let (_dir, mut store) = open_tmp();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let out = AnalysisOutput {
            flags: vec![RuleFlag {
                rule_id: "H2_ISOLATED_DEATH",
                round: 2,
                tick: 2200,
                steamid: 1,
                confidence: 0.75,
                severity: 0.8,
                details: serde_json::json!({"distance": 1200.0}),
                evidence: EvidenceRef {
                    round: 2,
                    tick_start: 1880,
                    tick_end: 2328,
                    focus_players: vec![1, 3],
                    camera_hint: None,
                },
            }],
            insights: vec![Insight {
                detector: "H2_ISOLATED_DEATH".into(),
                category: Category::Deaths,
                severity: 0.8,
                confidence: 0.75,
                round: 0,
                player: 1,
                title_data: serde_json::json!({"count": 1}),
                metrics: serde_json::json!({"count": 1}),
                evidence: vec![],
            }],
            death_classes: vec![DeathClassRow {
                round: 2,
                tick: 2200,
                victim: 1,
                class_id: 6,
                class_source: "H2_ISOLATED_DEATH".into(),
                secondary_tags: vec!["H3_WASTED_UTILITY".into()],
                confidence: 0.75,
            }],
            ledger: vec![],
        };
        store.save_analysis(id, &out).unwrap();
        let insights = store.insights_for_match(id).unwrap();
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].detector, "H2_ISOLATED_DEATH");
        assert_eq!(insights[0].category, "deaths");
        let classes = store.death_classes_for_match(id).unwrap();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].class_id, 6);
        assert!(classes[0].secondary_tags_json.contains("H3_WASTED_UTILITY"));
        // Re-save replaces, not duplicates.
        store.save_analysis(id, &out).unwrap();
        assert_eq!(store.insights_for_match(id).unwrap().len(), 1);
    }

    #[test]
    fn cross_demo_queries_aggregate_flags_positions_and_rounds() {
        use cf_analysis::{AnalysisOutput, EvidenceRef, RuleFlag};
        let (_dir, mut store) = open_tmp();
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 9);
        store.set_setting("tracked_steamid", "1").unwrap();
        let flag = |round: u32, tick: i32| RuleFlag {
            rule_id: "H2_ISOLATED_DEATH",
            round,
            tick,
            steamid: 1,
            confidence: 0.75,
            severity: 0.8,
            details: serde_json::json!({}),
            evidence: EvidenceRef {
                round,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![1],
                camera_hint: None,
            },
        };
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m1,
                &AnalysisOutput {
                    flags: vec![flag(1, 1200), flag(2, 2200)],
                    ..Default::default()
                },
            )
            .unwrap();
        let m2 = store
            .save_match("m2.dem", "h2", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m2,
                &AnalysisOutput {
                    flags: vec![flag(1, 1300)],
                    ..Default::default()
                },
            )
            .unwrap();

        let counts = store
            .rule_counts_across_matches("1", "H2_ISOLATED_DEATH", 10)
            .unwrap();
        assert_eq!(counts.len(), 2);
        assert_eq!(counts[0].match_id, m2, "newest first");
        assert_eq!(counts[0].count, 1);
        assert_eq!(counts[1].count, 2);
        assert!(counts[1]
            .first_evidence_json
            .as_deref()
            .unwrap()
            .contains("\"round\":1"));
        // The first flag's own round/tick ride along so a caller can rebuild
        // an EvidenceRef when the stored evidence is missing (below).
        assert_eq!(counts[1].first_round, Some(1));
        assert_eq!(counts[1].first_tick, Some(1200));
        // Window truncation.
        assert_eq!(
            store
                .rule_counts_across_matches("1", "H2_ISOLATED_DEATH", 1)
                .unwrap()
                .len(),
            1
        );

        let ids = store.flagged_rule_ids("1").unwrap();
        assert_eq!(ids, vec!["H2_ISOLATED_DEATH".to_string()]);

        // Death positions: sample_match kills player 1 at tick 2200 (round 2);
        // nearest sample at 2100 has x=100, y=-50.
        let pos = store.death_positions("1", 10.0).unwrap();
        assert_eq!(pos.len(), 2, "one death per saved match");
        assert_eq!(pos[0].x, 100.0);
        assert_eq!(pos[0].y, -50.0);
        assert_eq!(pos[0].map, "de_mirage");

        let stats = store.per_round_stats(m1, "1").unwrap();
        assert_eq!(stats.len(), 3);
        assert_eq!(stats[0].winner, "CT");
        assert_eq!(stats[0].tracked_side.as_deref(), Some("CT"));
        assert_eq!(stats[0].kills, 2);
        assert_eq!(stats[0].deaths, 0);
        assert_eq!(stats[1].deaths, 1);
        assert_eq!(stats[0].freeze_end_tick, Some(1100));
    }

    #[test]
    fn death_positions_carry_place_and_bound_lookback_to_10s() {
        let (_dir, mut store) = open_tmp();
        let mut data = sample_match();
        // A second kill of player 1 in round 2 whose only preceding sample is
        // >10 s old (2100 → 5000 is 2900 ticks ≈ 45 s at 64 tick): the stale
        // sample must NOT be used — the death is dropped, not misplaced.
        data.kills.push(kill(5000, 2, 3, 1, false));
        store
            .save_match("a.dem", "hash-a", MatchKind::Own, &data)
            .unwrap();

        let pos = store.death_positions("1", 10.0).unwrap();
        // sample_match kills player 1 at tick 2200 (round 2); nearest sample at
        // 2100 (x=100, y=-50) is 100 ticks ≈ 1.6 s away — inside the bound.
        assert_eq!(
            pos.len(),
            1,
            "stale-sample death excluded, in-bound death kept"
        );
        assert_eq!(pos[0].x, 100.0);
        assert_eq!(pos[0].y, -50.0);
        assert_eq!(pos[0].place.as_deref(), Some("BombsiteA"));
    }

    /// Flags written before migration 3 have a NULL `evidence_json`. The query
    /// must still hand back the first flag's round/tick so `get_habits` can
    /// rebuild an EvidenceRef instead of dropping the replay chip.
    #[test]
    fn rule_counts_expose_round_and_tick_when_evidence_json_is_null() {
        use cf_analysis::{AnalysisOutput, EvidenceRef, RuleFlag};
        let (_dir, mut store) = open_tmp();
        store.set_setting("tracked_steamid", "1").unwrap();
        let flag = |round: u32, tick: i32| RuleFlag {
            rule_id: "H2_FAILED_TRADE",
            round,
            tick,
            steamid: 1,
            confidence: 0.7,
            severity: 0.6,
            details: serde_json::json!({}),
            evidence: EvidenceRef {
                round,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![1],
                camera_hint: None,
            },
        };
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m1,
                &AnalysisOutput {
                    flags: vec![flag(2, 2400), flag(3, 3100)],
                    ..Default::default()
                },
            )
            .unwrap();
        // Simulate pre-migration-3 rows.
        store
            .conn
            .execute("UPDATE rule_flags SET evidence_json = NULL", [])
            .unwrap();

        let counts = store
            .rule_counts_across_matches("1", "H2_FAILED_TRADE", 10)
            .unwrap();
        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].count, 2);
        assert!(
            counts[0].first_evidence_json.is_none(),
            "pre-migration rows carry no evidence"
        );
        // Earliest tick wins, and it is the same row the evidence would be on.
        assert_eq!(counts[0].first_round, Some(2));
        assert_eq!(counts[0].first_tick, Some(2400));
    }

    /// Issue #6 §3: habit clauses say *where* — first-ever reader of
    /// `rule_flags.details_json`. Flags without a place don't count (silence
    /// bias), and results are capped to the top 2 by count.
    #[test]
    fn rule_place_counts_aggregates_details_place() {
        let (_dir, mut store) = open_tmp();
        let data = sample_match();
        let id = store
            .save_match("a.dem", "h1", MatchKind::Own, &data)
            .unwrap();
        let flag = |place: Option<&str>| cf_analysis::RuleFlag {
            rule_id: "H2_ISOLATED_DEATH",
            round: 1,
            tick: 1200,
            steamid: 1,
            confidence: 0.6,
            severity: 0.8,
            details: match place {
                Some(p) => serde_json::json!({ "place": p }),
                None => serde_json::json!({ "place": null }),
            },
            evidence: cf_analysis::EvidenceRef {
                round: 1,
                tick_start: 880,
                tick_end: 1328,
                focus_players: vec![1],
                camera_hint: None,
            },
        };
        let out = cf_analysis::AnalysisOutput {
            flags: vec![
                flag(Some("Catwalk")),
                flag(Some("Catwalk")),
                flag(Some("Underpass")),
                flag(None),
            ],
            insights: vec![],
            death_classes: vec![],
            ledger: vec![],
        };
        store.save_analysis(id, &out).unwrap();

        let places = store
            .rule_place_counts("1", "H2_ISOLATED_DEATH", 10)
            .unwrap();
        assert_eq!(
            places,
            vec![("Catwalk".to_string(), 2), ("Underpass".to_string(), 1)]
        );
    }

    #[test]
    fn migration_2_analysis_tables_and_rule_inputs_persist() {
        let (_dir, mut store) = open_tmp();
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 9);
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let q = |sql: &str| -> u32 { store.conn.query_row(sql, [id], |r| r.get(0)).unwrap() };
        assert_eq!(q("SELECT COUNT(*) FROM shots WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM hurts WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM reloads WHERE match_id = ?1"), 1);
        assert_eq!(q("SELECT COUNT(*) FROM inventories WHERE match_id = ?1"), 1);
        let items: String = store
            .conn
            .query_row(
                "SELECT items_json FROM inventories WHERE match_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(items.contains("Flashbang"));
        let scoped: Option<bool> = store
            .conn
            .query_row(
                "SELECT is_scoped FROM tick_samples WHERE match_id = ?1 LIMIT 1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scoped, Some(false));
    }

    #[test]
    fn corpus_matches_are_invisible_to_tracked_analytics() {
        use cf_analysis::{AnalysisOutput, EvidenceRef, RuleFlag};
        let (_dir, mut store) = open_tmp();
        store.set_setting("tracked_steamid", "1").unwrap();
        let own = store
            .save_match("own.dem", "h-own", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                own,
                &AnalysisOutput {
                    flags: vec![RuleFlag {
                        rule_id: "H2_ISOLATED_DEATH",
                        round: 2,
                        tick: 2200,
                        steamid: 1,
                        confidence: 0.75,
                        severity: 0.8,
                        details: serde_json::json!({}),
                        evidence: EvidenceRef {
                            round: 2,
                            tick_start: 1880,
                            tick_end: 2328,
                            focus_players: vec![1],
                            camera_hint: None,
                        },
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        let habits_before = store
            .rule_counts_across_matches("1", "H2_ISOLATED_DEATH", 10)
            .unwrap()
            .len();
        let deaths_before = store.death_positions("1", 10.0).unwrap().len();

        // Import the same synthetic match as CORPUS (players 1..4 included).
        store
            .save_match("pro.dem", "h-pro", MatchKind::Corpus, &sample_match())
            .unwrap();

        assert_eq!(
            store.list_matches().unwrap().len(),
            1,
            "library shows own matches only"
        );
        assert_eq!(
            store
                .rule_counts_across_matches("1", "H2_ISOLATED_DEATH", 10)
                .unwrap()
                .len(),
            habits_before,
            "habit window unchanged by corpus import"
        );
        assert_eq!(
            store.death_positions("1", 10.0).unwrap().len(),
            deaths_before,
            "death positions unchanged by corpus import"
        );
        assert_eq!(store.tracked_steamid().unwrap().as_deref(), Some("1"));
        let summary = store.corpus_summary().unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].map, "de_mirage");
        assert_eq!(summary[0].demos, 1);

        // A rule flag saved against the corpus match must not leak into the
        // severity/confidence reader either (defense in depth — corpus
        // matches never run analysis in the app, but the query should not
        // rely on that invariant).
        let corpus_id = store.corpus_match_ids("de_mirage").unwrap()[0];
        store
            .save_analysis(
                corpus_id,
                &AnalysisOutput {
                    flags: vec![RuleFlag {
                        rule_id: "H2_ISOLATED_DEATH",
                        round: 1,
                        tick: 1200,
                        steamid: 1,
                        confidence: 0.01,
                        severity: 0.99,
                        details: serde_json::json!({}),
                        evidence: EvidenceRef {
                            round: 1,
                            tick_start: 900,
                            tick_end: 1300,
                            focus_players: vec![1],
                            camera_hint: None,
                        },
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            store
                .rule_severity_confidence("1", "H2_ISOLATED_DEATH")
                .unwrap(),
            Some((0.8, 0.75)),
            "severity/confidence reader ignores corpus-side flags"
        );
    }

    #[test]
    fn tick_samples_persist() {
        let (_dir, mut store) = open_tmp();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let n: u32 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM tick_samples WHERE match_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 3);
    }

    #[test]
    fn grid_blob_roundtrip_and_upsert() {
        let (_dir, mut store) = open_tmp();
        let g = GridRow {
            map: "de_mirage".into(),
            side: "CT".into(),
            phase: "early".into(),
            size: 2,
            counts: vec![0, 1, 7, u32::MAX],
            demos: 8,
            samples: 4321,
        };
        store.save_grids(std::slice::from_ref(&g)).unwrap();
        let loaded = store.load_grids("de_mirage").unwrap();
        assert_eq!(loaded, vec![g.clone()]);
        // Upsert replaces, not duplicates.
        let g2 = GridRow {
            counts: vec![9, 9, 9, 9],
            demos: 9,
            samples: 5000,
            ..g
        };
        store.save_grids(std::slice::from_ref(&g2)).unwrap();
        let loaded = store.load_grids("de_mirage").unwrap();
        assert_eq!(loaded, vec![g2]);
        assert!(store.load_grids("de_dust2").unwrap().is_empty());
        let status = store.grid_status().unwrap();
        assert_eq!(status.len(), 1);
        assert_eq!(
            (status[0].demos, status[0].samples),
            (9, 5000),
            "status mirrors the upserted row"
        );
        assert!(!status[0].built_at.is_empty());
    }

    #[test]
    fn positions_at_takes_nearest_sample_at_or_before_tick() {
        let (_dir, mut store) = open_tmp();
        // sample_match ticks: (1100, sid 1), (1100, sid 3), (2100, sid 1).
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let at_1150 = store.positions_at(id, 1150, 0).unwrap();
        assert_eq!(
            at_1150
                .iter()
                .map(|p| p.steamid.as_str())
                .collect::<Vec<_>>(),
            vec!["1", "3"]
        );
        assert!(at_1150
            .iter()
            .all(|p| p.alive && p.x == 100.0 && p.y == -50.0));
        // Later tick: sid 1 advances to its 2100 sample, sid 3 keeps 1100.
        assert_eq!(store.positions_at(id, 2150, 0).unwrap().len(), 2);
        // Before any sample: nobody has a position yet.
        assert!(store.positions_at(id, 1000, 0).unwrap().is_empty());
        // Round lower bound: sid 3's only sample (1100) predates a round
        // starting at 2000 — a disconnected player must not ghost forward.
        let bounded = store.positions_at(id, 2150, 2000).unwrap();
        assert_eq!(
            bounded
                .iter()
                .map(|p| p.steamid.as_str())
                .collect::<Vec<_>>(),
            vec!["1"]
        );
    }

    #[test]
    fn bomb_plant_tick_scoped_to_round() {
        let (_dir, mut store) = open_tmp();
        let mut md = sample_match();
        // Round 2 spans ticks 2000..2900 (see round()).
        md.bomb_events.push(cf_parser::model::BombEvent {
            tick: 2400,
            kind: "planted".into(),
            player: Some(3),
        });
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &md)
            .unwrap();
        assert_eq!(store.bomb_plant_tick(id, 2).unwrap(), Some(2400));
        assert_eq!(store.bomb_plant_tick(id, 1).unwrap(), None);
    }

    #[test]
    fn corpus_match_ids_and_lean_rounds() {
        let (_dir, mut store) = open_tmp();
        store
            .save_match("own.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let c1 = store
            .save_match("pro1.dem", "h2", MatchKind::Corpus, &sample_match())
            .unwrap();
        let mut other = sample_match();
        other.map = "de_dust2".into();
        store
            .save_match("pro2.dem", "h3", MatchKind::Corpus, &other)
            .unwrap();
        assert_eq!(store.corpus_match_ids("de_mirage").unwrap(), vec![c1]);
        assert_eq!(store.own_match_ids("de_mirage").unwrap(), vec![1]);
        assert!(store.own_match_ids("de_dust2").unwrap().is_empty());
        let rounds = store.rounds_for_match(c1).unwrap();
        assert_eq!(rounds.len(), 3);
        assert_eq!(rounds[0].number, 1);
        assert_eq!(rounds[0].winner, "CT");
        assert_eq!(rounds[0].freeze_end_tick, Some(1100));
        // Lean sides + meta readers used by corpus phase sampling.
        let sides = store.sides_for_round(c1, 1).unwrap();
        assert!(sides.contains(&("1".to_string(), "CT".to_string())));
        assert!(sides.contains(&("3".to_string(), "T".to_string())));
        assert_eq!(
            store.match_map_tickrate(c1).unwrap(),
            Some(("de_mirage".to_string(), 64.0))
        );
        assert_eq!(store.match_map_tickrate(9999).unwrap(), None);
    }

    #[test]
    fn replace_detector_insights_leaves_other_detectors_alone() {
        use cf_analysis::{Category, Insight};
        let (_dir, mut store) = open_tmp();
        let id = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        let mk = |detector: &str| Insight {
            detector: detector.into(),
            category: Category::Positioning,
            severity: 0.5,
            confidence: 0.6,
            round: 0,
            player: 1,
            title_data: serde_json::json!({}),
            metrics: serde_json::json!({}),
            evidence: vec![],
        };
        store
            .replace_detector_insights(id, "OTHER_DETECTOR", &[mk("OTHER_DETECTOR")])
            .unwrap();
        store
            .replace_detector_insights(
                id,
                "D6_UNUSUAL_POSITIONING",
                &[mk("D6_UNUSUAL_POSITIONING"), mk("D6_UNUSUAL_POSITIONING")],
            )
            .unwrap();
        // Re-running replaces D6 rows without duplicating or touching others.
        store
            .replace_detector_insights(
                id,
                "D6_UNUSUAL_POSITIONING",
                &[mk("D6_UNUSUAL_POSITIONING")],
            )
            .unwrap();
        let insights = store.insights_for_match(id).unwrap();
        let d6 = insights
            .iter()
            .filter(|i| i.detector == "D6_UNUSUAL_POSITIONING")
            .count();
        let other = insights
            .iter()
            .filter(|i| i.detector == "OTHER_DETECTOR")
            .count();
        assert_eq!((d6, other), (1, 1));
    }

    #[test]
    fn trend_readers_return_exact_values_and_exclude_corpus() {
        use cf_analysis::{AnalysisOutput, DeathClassRow, EvidenceRef, RuleFlag};
        let (_dir, mut store) = open_tmp();
        store.set_setting("tracked_steamid", "1").unwrap();

        let flag = |rule_id: &'static str, round: u32, tick: i32| RuleFlag {
            rule_id,
            round,
            tick,
            steamid: 1,
            confidence: 0.7,
            severity: 0.6,
            details: serde_json::json!({}),
            evidence: EvidenceRef {
                round,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![1],
                camera_hint: None,
            },
        };
        let dclass = |tick: i32, class_id: u8| DeathClassRow {
            round: 2,
            tick,
            victim: 1,
            class_id,
            class_source: "TEST".into(),
            secondary_tags: vec![],
            confidence: 0.7,
        };

        // m1: 2 death_class rows, 1 class-13 -> 50.0%; two H2_ISOLATED_DEATH flags.
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m1,
                &AnalysisOutput {
                    flags: vec![
                        flag("H2_ISOLATED_DEATH", 1, 1200),
                        flag("H2_ISOLATED_DEATH", 2, 2200),
                    ],
                    death_classes: vec![dclass(2100, 13), dclass(2200, 6)],
                    ..Default::default()
                },
            )
            .unwrap();

        // m2: 4 death_class rows, 1 class-13 -> 25.0%; one flag per rule.
        let m2 = store
            .save_match("m2.dem", "h2", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m2,
                &AnalysisOutput {
                    flags: vec![
                        flag("H2_ISOLATED_DEATH", 1, 1200),
                        flag("H2_FAILED_TRADE", 2, 2200),
                    ],
                    death_classes: vec![
                        dclass(2100, 13),
                        dclass(2200, 6),
                        dclass(2300, 5),
                        dclass(2400, 4),
                    ],
                    ..Default::default()
                },
            )
            .unwrap();

        // m3: no death_class rows -> 0.0%; no flags.
        let m3 = store
            .save_match("m3.dem", "h3", MatchKind::Own, &sample_match())
            .unwrap();

        // Corpus match: flags/death_classes planted here must never surface.
        let corpus = store
            .save_match("pro.dem", "h-pro", MatchKind::Corpus, &sample_match())
            .unwrap();
        store
            .save_analysis(
                corpus,
                &AnalysisOutput {
                    flags: vec![flag("H2_ISOLATED_DEATH", 1, 1200)],
                    death_classes: vec![dclass(2100, 13)],
                    ..Default::default()
                },
            )
            .unwrap();

        let matches = store.trend_matches("1").unwrap();
        assert_eq!(matches.len(), 3, "corpus match excluded");
        assert_eq!(
            matches.iter().map(|m| m.match_id).collect::<Vec<_>>(),
            vec![m1, m2, m3],
            "chronological: imported_at ASC, id ASC"
        );
        assert_eq!(matches[0].deaths, 1);
        assert_eq!(matches[0].class13_pct, 50.0);
        assert_eq!(matches[1].deaths, 1);
        assert_eq!(matches[1].class13_pct, 25.0);
        assert_eq!(matches[2].deaths, 1);
        assert_eq!(matches[2].class13_pct, 0.0, "no death_class rows");

        let cells = store.rule_trend_counts("1").unwrap();
        assert_eq!(cells.len(), 3, "corpus flags excluded");
        assert!(cells
            .iter()
            .any(|c| c.match_id == m1 && c.rule_id == "H2_ISOLATED_DEATH" && c.count == 2));
        assert!(cells
            .iter()
            .any(|c| c.match_id == m2 && c.rule_id == "H2_ISOLATED_DEATH" && c.count == 1));
        assert!(cells
            .iter()
            .any(|c| c.match_id == m2 && c.rule_id == "H2_FAILED_TRADE" && c.count == 1));
        assert!(
            !cells.iter().any(|c| c.match_id == corpus),
            "corpus match must not appear in rule trend cells"
        );
    }

    #[test]
    fn failed_import_leaves_no_rows() {
        // Parse failures can't write rows by construction (save_match runs
        // only after a successful parse); the DB-side failure that CAN
        // happen mid-import is the duplicate-hash rejection — assert it
        // leaves the library untouched.
        let (_dir, mut store) = open_tmp();
        store
            .save_match("a.dem", "same-hash", MatchKind::Own, &sample_match())
            .unwrap();
        let err = store
            .save_match("b.dem", "same-hash", MatchKind::Own, &sample_match())
            .unwrap_err();
        assert!(matches!(err, StoreError::DuplicateImport));
        assert_eq!(store.list_matches().unwrap().len(), 1);
        let rows: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM matches", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1, "failed import must not leave a partial row");
    }

    #[test]
    fn delete_match_cascades_and_frees_the_hash() {
        use cf_analysis::{Category, Insight};
        let (_dir, mut store) = open_tmp();
        let a = store
            .save_match("a.dem", "h-a", MatchKind::Own, &sample_match())
            .unwrap();
        let b = store
            .save_match("b.dem", "h-b", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .replace_detector_insights(
                a,
                "D6_UNUSUAL_POSITIONING",
                &[Insight {
                    detector: "D6_UNUSUAL_POSITIONING".into(),
                    category: Category::Positioning,
                    severity: 0.5,
                    confidence: 0.6,
                    round: 0,
                    player: 1,
                    title_data: serde_json::json!({}),
                    metrics: serde_json::json!({}),
                    evidence: vec![],
                }],
            )
            .unwrap();
        store.delete_match(a).unwrap();
        // Parent gone, children cascaded, hash reusable; b untouched.
        assert_eq!(store.list_matches().unwrap().len(), 1);
        assert!(!store.has_file_hash("h-a").unwrap());
        assert!(store.has_file_hash("h-b").unwrap());
        assert!(store.insights_for_match(a).unwrap().is_empty());
        let orphans: i64 = store
            .conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM kills WHERE match_id = ?1)
                      + (SELECT COUNT(*) FROM rounds WHERE match_id = ?1)
                      + (SELECT COUNT(*) FROM tick_samples WHERE match_id = ?1)
                      + (SELECT COUNT(*) FROM players WHERE match_id = ?1)",
                [a],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans, 0, "cascade left orphan child rows");
        assert!(!store.rounds_for_match(b).unwrap().is_empty());
        // Name lookup survives for players of remaining matches.
        assert_eq!(store.player_name("1").unwrap().as_deref(), Some("alice"));
        assert_eq!(store.player_name("999").unwrap(), None);
    }

    #[test]
    fn flags_for_match_returns_details_and_position() {
        use cf_analysis::{AnalysisOutput, EvidenceRef, RuleFlag};
        let (_dir, mut store) = open_tmp();
        let flag = |round: u32, tick: i32| RuleFlag {
            rule_id: "H2_ISOLATED_DEATH",
            round,
            tick,
            steamid: 1,
            confidence: 0.75,
            severity: 0.8,
            details: serde_json::json!({ "place": "Catwalk" }),
            evidence: EvidenceRef {
                round,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![1],
                camera_hint: None,
            },
        };
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();
        store
            .save_analysis(
                m1,
                &AnalysisOutput {
                    flags: vec![flag(1, 1200), flag(2, 2200)],
                    ..Default::default()
                },
            )
            .unwrap();

        let rows = store.flags_for_match(m1).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rule_id, "H2_ISOLATED_DEATH");
        assert_eq!(rows[0].round, 1);
        assert_eq!(rows[0].tick, 1200);
        assert_eq!(rows[0].steamid, "1");
        assert_eq!(rows[1].round, 2);
        assert_eq!(rows[1].tick, 2200);
        for row in &rows {
            let parsed: serde_json::Value = serde_json::from_str(&row.details_json).unwrap();
            assert_eq!(parsed["place"], serde_json::json!("Catwalk"));
        }
    }

    #[test]
    fn round_reviews_roundtrip_and_replace() {
        let (_dir, mut store) = open_tmp();
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();

        let fp =
            cf_analysis::round_review::cfg_fingerprint(&cf_analysis::config::RbrCfg::default());
        let row = |round: u32, verdict: &str| RoundReviewRow {
            round,
            impact: 0.25,
            verdict: verdict.to_string(),
            attention: "bright".to_string(),
            selected: true,
            pivotal_tick: 1200,
            header_json: serde_json::json!({
                "side": "CT", "won": true, "kills": 1, "deaths": 0, "man_context": "5v5"
            })
            .to_string(),
            moments_json: serde_json::json!([{
                "tick": 1200, "kind": "tracked_kill", "rule_id": null,
                "delta_p": 0.1, "facts": {}
            }])
            .to_string(),
            cfg_fingerprint: fp.clone(),
        };

        store
            .save_round_reviews(m1, &[row(1, "won_it"), row(2, "cost_you")])
            .unwrap();
        let loaded = store.load_round_reviews(m1).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].round, 1, "ordered by round");
        assert_eq!(loaded[0].verdict, "won_it");
        assert_eq!(
            loaded[0].cfg_fingerprint, fp,
            "the fingerprint must round-trip through save/load"
        );
        assert_eq!(loaded[1].round, 2);
        assert_eq!(loaded[1].verdict, "cost_you");
        assert_eq!(loaded, vec![row(1, "won_it"), row(2, "cost_you")]);

        // Replace semantics: saving a different set drops the old rows.
        store.save_round_reviews(m1, &[row(3, "quiet")]).unwrap();
        let replaced = store.load_round_reviews(m1).unwrap();
        assert_eq!(replaced, vec![row(3, "quiet")]);

        // FK cascade: deleting the match drops its round_review rows too.
        store.delete_match(m1).unwrap();
        let count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM round_review", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "delete_match must cascade into round_review");
    }

    /// Store-level slice of Fix #5's mismatch path: a row saved under a
    /// stale fingerprint reads back with that stale value intact — this is
    /// the signal `commands::get_round_review` compares against the current
    /// fingerprint to decide whether to recompute. The recompute call itself
    /// lives in `commands.rs` (needs `AppState`/Tauri wiring this crate
    /// doesn't have); exercised here at the store boundary only.
    #[test]
    fn round_review_stale_fingerprint_is_visible_at_load() {
        let (_dir, mut store) = open_tmp();
        let m1 = store
            .save_match("m1.dem", "h1", MatchKind::Own, &sample_match())
            .unwrap();

        let stale = RoundReviewRow {
            round: 1,
            impact: 0.25,
            verdict: "won_it".to_string(),
            attention: "bright".to_string(),
            selected: true,
            pivotal_tick: 1200,
            header_json: serde_json::json!({
                "side": "CT", "won": true, "kills": 1, "deaths": 0, "man_context": "5v5"
            })
            .to_string(),
            moments_json: "[]".to_string(),
            cfg_fingerprint: "rbr-v0|stale".to_string(),
        };
        store.save_round_reviews(m1, &[stale]).unwrap();

        let loaded = store.load_round_reviews(m1).unwrap();
        let current =
            cf_analysis::round_review::cfg_fingerprint(&cf_analysis::config::RbrCfg::default());
        assert_eq!(
            loaded[0].cfg_fingerprint, "rbr-v0|stale",
            "the stale fingerprint must round-trip unchanged"
        );
        assert_ne!(
            loaded[0].cfg_fingerprint, current,
            "a row saved under an old fingerprint must read back mismatched \
             against the current one"
        );
    }

    #[test]
    fn save_analysis_persists_the_play_ledger_and_source_path_round_trips() {
        let (_dir, mut store, match_id, _data) = one_match();
        let ledger = vec![cf_analysis::play_ledger::RoundLedger {
            round: 1,
            plays: vec![cf_analysis::play_ledger::Play {
                tick: 1320,
                phase: "opening".to_string(),
                kind: "setup".to_string(),
                facts: serde_json::json!({"place": "BombsiteA"}),
                quality: None,
                rule_id: None,
                delta_p: None,
            }],
            timeline: vec![],
        }];
        let out = cf_analysis::AnalysisOutput {
            ledger,
            ..Default::default()
        };
        store.save_analysis(match_id, &out).unwrap();
        let rows = store.load_round_plays(match_id).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].round, 1);
        assert!(rows[0].plays_json.contains("\"kind\":\"setup\""));
        assert_eq!(rows[0].timeline_json, "[]");
        // Replace semantics: a second save with an empty ledger clears it.
        store
            .save_analysis(match_id, &cf_analysis::AnalysisOutput::default())
            .unwrap();
        assert!(store.load_round_plays(match_id).unwrap().is_empty());

        assert_eq!(store.source_path(match_id).unwrap(), None);
        store.set_source_path(match_id, "/demos/a.dem").unwrap();
        assert_eq!(
            store.source_path(match_id).unwrap().as_deref(),
            Some("/demos/a.dem")
        );
    }

    #[test]
    fn distinct_places_lists_every_visited_callout_once_and_skips_blanks() {
        let (_dir, store, match_id, _) = one_match();
        // sample_match() stands everyone in BombsiteA; add a second place,
        // a NULL and an empty string, plus a row from another match id.
        for (tick, steamid, place) in [
            (3100, "1", Some("TopofMid")),
            (3100, "3", Some("TopofMid")),
            (3200, "1", None),
            (3300, "1", Some("")),
        ] {
            store
                .conn
                .execute(
                    "INSERT INTO tick_samples (match_id, steamid, tick, x, y, z, yaw, health,
                       is_alive, team_num, active_weapon, spotted, last_place)
                     VALUES (?1, ?2, ?3, 0, 0, 0, 0, 100, 1, 3, NULL, 0, ?4)",
                    params![match_id, steamid, tick, place],
                )
                .unwrap();
        }
        assert_eq!(
            store.distinct_places(match_id).unwrap(),
            vec!["BombsiteA".to_string(), "TopofMid".to_string()]
        );
        assert!(store.distinct_places(match_id + 1).unwrap().is_empty());
    }

    #[test]
    fn coach_cache_upserts_reads_and_deletes_by_kind_and_round() {
        let (_dir, mut store, match_id, _data) = one_match();
        let row = CoachCacheRow {
            kind: "round".to_string(),
            round: 6,
            facts_hash: "abc".to_string(),
            model: "gemini-3.7-flash".to_string(),
            status: "ok".to_string(),
            response_json: "{\"round\":6}".to_string(),
            violations_json: "[]".to_string(),
        };
        store.put_coach_cache(match_id, &row).unwrap();
        let got = store
            .get_coach_cache(match_id, "round", 6)
            .unwrap()
            .unwrap();
        assert_eq!(got.facts_hash, "abc");
        assert_eq!(got.status, "ok");
        // upsert replaces
        let row2 = CoachCacheRow {
            facts_hash: "def".to_string(),
            status: "fallback".to_string(),
            ..row.clone()
        };
        store.put_coach_cache(match_id, &row2).unwrap();
        assert_eq!(
            store
                .get_coach_cache(match_id, "round", 6)
                .unwrap()
                .unwrap()
                .facts_hash,
            "def"
        );
        assert!(store
            .get_coach_cache(match_id, "round", 7)
            .unwrap()
            .is_none());
        store
            .put_coach_cache(
                match_id,
                &CoachCacheRow {
                    kind: "synthesis".to_string(),
                    round: 0,
                    ..row.clone()
                },
            )
            .unwrap();
        // delete one round
        store
            .delete_coach_cache(match_id, Some("round"), Some(6))
            .unwrap();
        assert!(store
            .get_coach_cache(match_id, "round", 6)
            .unwrap()
            .is_none());
        assert!(store
            .get_coach_cache(match_id, "synthesis", 0)
            .unwrap()
            .is_some());
        // delete everything for the match
        store.delete_coach_cache(match_id, None, None).unwrap();
        assert!(store
            .get_coach_cache(match_id, "synthesis", 0)
            .unwrap()
            .is_none());
        assert_eq!(crate::migrations::current_version(&store.conn).unwrap(), 9);
    }
}
