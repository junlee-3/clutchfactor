//! The coach's side effects (spec §3): key resolution, the Gemini call,
//! caching and fallback. The pure half — prompts, schemas, the grounding
//! validator, parsing — lives in `cf_narrator::coach` and is what the
//! adversarial tests exercise.

pub mod gemini;
pub mod key;

use std::collections::{HashMap, HashSet};
use std::fmt;

use cf_narrator::coach::parse::{parse_round_batch, parse_synthesis};
use cf_narrator::coach::prompt::{
    render_round_batch, render_round_block, render_synthesis, round_batch_schema, synthesis_schema,
    DEFAULT_ROUND_MODEL, DEFAULT_SYNTHESIS_MODEL, ROUNDS_PER_CALL, STYLE_VERSION, SYSTEM_PERSONA,
};
use cf_narrator::coach::types::{
    MatchInput, MatchSynthesis, PlayLine, RoundCommentary, RoundDigest, RoundInput, SynthesisInput,
};
use cf_narrator::coach::validate::{
    retry_note, validate_round, validate_synthesis, Grounding, Violation,
};
use cf_store::store::{CoachCacheRow, RoundInfo};
use cf_store::Store;
use sha2::{Digest, Sha256};
use tauri::State;

use crate::commands::{
    assemble_round_reviews, habit_reports, insight_from_row, match_context, AppState,
    CoachRoundsDto, CoachSynthesisDto, MatchSynthesisDto, PlayCommentDto, RoundCommentaryDto,
    RoundReviewDto,
};
use gemini::GeminiClient;
use key::{coach_enabled, resolve_key, SETTING_ROUND_MODEL, SETTING_SYNTHESIS_MODEL};

/// An API key that can be used but not printed. `Debug`/`Display` never
/// show the value; DTOs only ever carry `hint()`.
#[derive(Clone)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(s: String) -> Self {
        SecretKey(s.trim().to_string())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    /// "…ab12" — the last four characters, enough to tell keys apart.
    pub fn hint(&self) -> String {
        let tail: String = self
            .0
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("…{tail}")
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey(…)")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoachError {
    NoKey,
    InvalidKey,
    RateLimited,
    Offline(String),
    Server(u16),
    BadRequest(String),
    BadResponse(String),
}

impl fmt::Display for CoachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoachError::NoKey => write!(f, "No Gemini key set — add one in Settings → Coach."),
            CoachError::InvalidKey => write!(f, "Gemini rejected the key. Check it in Settings → Coach."),
            CoachError::RateLimited => write!(f, "Gemini is rate-limiting this key right now — the coach will retry later; the template captions are shown meanwhile."),
            CoachError::Offline(e) => write!(f, "Couldn't reach Gemini ({e}). The template captions are shown meanwhile."),
            CoachError::Server(s) => write!(f, "Gemini returned a server error ({s}). Try again in a minute."),
            CoachError::BadRequest(m) => write!(f, "Gemini rejected the request: {m}"),
            CoachError::BadResponse(m) => write!(f, "The coach's answer couldn't be used: {m}"),
        }
    }
}

impl std::error::Error for CoachError {}

// ---- V1.3 Task 6: orchestration ----

pub const KIND_ROUND: &str = "round";
pub const KIND_SYNTHESIS: &str = "synthesis";

/// Cache key for one coach answer: the exact facts block the model was
/// shown, the model id, and the prompt style version — any of the three
/// changing busts the row (ADR-0010).
pub fn facts_hash(block: &str, model: &str) -> String {
    let mut h = Sha256::new();
    h.update(block.as_bytes());
    h.update(b"\n");
    h.update(model.as_bytes());
    h.update(b"\n");
    h.update(STYLE_VERSION.as_bytes());
    format!("{:x}", h.finalize())
}

fn clock(tick: i32, round: &RoundInfo, tickrate: f32) -> String {
    let start = round.freeze_end_tick.unwrap_or(round.start_tick);
    let secs = ((tick - start) as f32 / tickrate).round() as i64;
    if secs >= 0 {
        format!("+{secs} s")
    } else {
        format!("{secs} s")
    }
}

fn weapon_short(w: &Option<String>) -> String {
    w.as_deref()
        .map(|w| w.trim_start_matches("weapon_").to_string())
        .unwrap_or_default()
}

/// One narrated line per timeline event, in the model's vocabulary.
fn timeline_line(e: &crate::commands::TimelineDto, round: &RoundInfo, tickrate: f32) -> String {
    let t = clock(e.tick, round, tickrate);
    let actor = e.actor.clone().unwrap_or_else(|| "someone".to_string());
    match e.kind.as_str() {
        "kill" => {
            let victim = e.subject.clone().unwrap_or_else(|| "someone".to_string());
            let w = weapon_short(&e.weapon);
            if w.is_empty() {
                format!("{t} {actor} killed {victim}")
            } else {
                format!("{t} {actor} killed {victim} ({w})")
            }
        }
        "plant" => format!("{t} {actor} planted the bomb"),
        "defuse" => format!("{t} {actor} defused the bomb"),
        "explode" => format!("{t} the bomb exploded"),
        other => format!("{t} {actor} {other}"),
    }
}

pub fn build_round_inputs(
    reviews: &[RoundReviewDto],
    rounds: &[RoundInfo],
    tickrate: f32,
) -> Vec<RoundInput> {
    let by_number: HashMap<u32, &RoundInfo> = rounds.iter().map(|r| (r.number, r)).collect();
    let mut out = vec![];
    let mut digest: Vec<String> = vec![];
    for rv in reviews {
        let Some(info) = by_number.get(&rv.round) else {
            continue;
        };
        out.push(RoundInput {
            round: rv.round,
            side: rv.side.clone(),
            won: rv.won,
            verdict_label: rv.verdict_label.clone(),
            impact_pct: (rv.impact * 100.0).round() as i32,
            man_context: rv.man_context.clone(),
            kills: rv.kills,
            deaths: rv.deaths,
            plays: rv
                .plays
                .iter()
                .map(|p| PlayLine {
                    tick: p.tick,
                    clock: clock(p.tick, info, tickrate),
                    kind: p.kind.clone(),
                    headline: p.headline.clone(),
                    facts: p.facts.clone(),
                    quality: p.quality.clone(),
                })
                .collect(),
            timeline: rv
                .timeline
                .iter()
                .map(|e| timeline_line(e, info, tickrate))
                .collect(),
            prior_digest: digest.clone(),
        });
        // "Round 7", not "R7": only a digit standing alone is grounded, so
        // this spelling is what lets the coach cite an earlier round.
        digest.push(format!(
            "Round {} · {} · {}",
            rv.round,
            rv.verdict_label,
            if rv.won { "won" } else { "lost" }
        ));
    }
    out
}

/// Every callout the match's raw ledger mentions, prettified and deduped —
/// the validator's "known callouts" set (a callout the coach names that the
/// round's facts don't contain is an invention).
pub fn known_callouts(plays_jsons: &[String]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = vec![];
    for json in plays_jsons {
        let Ok(plays) = serde_json::from_str::<Vec<serde_json::Value>>(json) else {
            continue;
        };
        for p in plays {
            for key in ["place", "victim_place", "killer_place", "place_at_plant"] {
                if let Some(raw) = p["facts"][key].as_str() {
                    let pretty = cf_narrator::callouts::callout_name(raw);
                    if seen.insert(pretty.clone()) {
                        out.push(pretty);
                    }
                }
            }
        }
    }
    out
}

pub fn chunk_rounds(rounds: &[u32]) -> Vec<Vec<u32>> {
    rounds.chunks(ROUNDS_PER_CALL).map(|c| c.to_vec()).collect()
}

/// (hits, misses): a hit is a cached row whose hash matches (any status);
/// a miss has no row, a stale hash, or was forced.
pub fn split_cache(
    hashes: &[(u32, String)],
    rows: &[(u32, String, String)],
    force: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    let by_round: HashMap<u32, &(u32, String, String)> = rows.iter().map(|r| (r.0, r)).collect();
    let (mut hits, mut misses) = (vec![], vec![]);
    for (round, hash) in hashes {
        match by_round.get(round) {
            Some(row) if row.1 == *hash && !force.contains(round) => hits.push(*round),
            _ => misses.push(*round),
        }
    }
    (hits, misses)
}

fn commentary_dto(c: &RoundCommentary, model: &str) -> RoundCommentaryDto {
    RoundCommentaryDto {
        round: c.round,
        read: c.read.clone(),
        plays: c
            .plays
            .iter()
            .map(|p| PlayCommentDto {
                tick: p.tick,
                comment: p.comment.clone(),
            })
            .collect(),
        why_it_mattered: c.why_it_mattered.clone(),
        what_to_practise: c.what_to_practise.clone(),
        focus: c.focus.clone(),
        model: model.to_string(),
    }
}

/// `"field:Kind:token"` per violation — what the cache row records for a
/// fallback, so a later look at the DB explains why the coach stayed quiet.
fn violations_json(v: &[Violation]) -> String {
    serde_json::to_string(
        &v.iter()
            .map(|x| format!("{}:{:?}:{}", x.field, x.kind, x.token))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default()
}

fn model_setting(store: &Store, key: &str, default: &str) -> Result<String, String> {
    Ok(store
        .get_setting(key)
        .map_err(|e| e.to_string())?
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| default.to_string()))
}

fn match_input(bundle: &crate::commands::MatchCtxBundle) -> MatchInput {
    MatchInput {
        map: bundle.detail.map.clone(),
        score: (bundle.detail.score_a, bundle.detail.score_b),
        tracked_name: bundle.ctx.name(bundle.ctx.tracked),
        tracked_result: bundle.tracked_result.clone(),
        roster: bundle
            .detail
            .players
            .iter()
            .map(|p| p.name.clone())
            .collect(),
    }
}

/// Everything the round pipeline needs from the store, read under one lock.
struct RoundSession {
    client: GeminiClient,
    model: String,
    match_input: MatchInput,
    inputs: Vec<RoundInput>,
    blocks: Vec<(u32, String, String)>, // (round, block, hash)
    known_callouts: Vec<String>,
    cached: Vec<(u32, String, String, String)>, // (round, hash, status, response_json)
}

fn open_round_session(store: &mut Store, match_id: i64) -> Result<Option<RoundSession>, String> {
    if !coach_enabled(store)? {
        return Ok(None);
    }
    let Some((key, _)) = resolve_key(store)? else {
        return Ok(None);
    };
    let model = model_setting(store, SETTING_ROUND_MODEL, DEFAULT_ROUND_MODEL)?;
    let reviews = assemble_round_reviews(store, match_id)?;
    let Some(bundle) = match_context(store, match_id)? else {
        return Ok(None);
    };
    let rounds = store
        .rounds_for_match(match_id)
        .map_err(|e| e.to_string())?;
    let tickrate = bundle.detail.tickrate;
    let match_input = match_input(&bundle);
    let inputs = build_round_inputs(&reviews, &rounds, tickrate);
    let blocks: Vec<(u32, String, String)> = inputs
        .iter()
        .map(|r| {
            let b = render_round_block(&match_input, r);
            let h = facts_hash(&b, &model);
            (r.round, b, h)
        })
        .collect();
    let plays_jsons: Vec<String> = store
        .load_round_plays(match_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| r.plays_json)
        .collect();
    let known = known_callouts(&plays_jsons);
    let mut cached = vec![];
    for (round, _, _) in &blocks {
        if let Some(row) = store
            .get_coach_cache(match_id, KIND_ROUND, *round)
            .map_err(|e| e.to_string())?
        {
            cached.push((*round, row.facts_hash, row.status, row.response_json));
        }
    }
    Ok(Some(RoundSession {
        client: GeminiClient::new(key).map_err(|e| e.to_string())?,
        model,
        match_input,
        inputs,
        blocks,
        known_callouts: known,
        cached,
    }))
}

/// Per-round commentary for a match: cache hits render immediately; misses
/// are generated in batches of `ROUNDS_PER_CALL`, validated, retried once
/// with the violations listed, then cached (`ok` or `fallback`). A transport
/// error stops generation for this call and is reported once in `error`;
/// un-generated rounds are not cached, so the next open retries them.
///
/// Lock discipline: the store is read once into a `RoundSession`, every
/// `.await` runs with no guard live, and the store is re-locked only to
/// write the new cache rows.
pub async fn round_commentary(
    state: &State<'_, AppState>,
    match_id: i64,
    force: &[u32],
) -> Result<CoachRoundsDto, String> {
    let session = {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        open_round_session(&mut store, match_id)?
    };
    let Some(s) = session else {
        return Ok(CoachRoundsDto {
            rounds: vec![],
            error: None,
        });
    };

    let hashes: Vec<(u32, String)> = s.blocks.iter().map(|(r, _, h)| (*r, h.clone())).collect();
    let rows: Vec<(u32, String, String)> = s
        .cached
        .iter()
        .map(|(r, h, st, _)| (*r, h.clone(), st.clone()))
        .collect();
    let (hits, misses) = split_cache(&hashes, &rows, force);

    let mut out: Vec<RoundCommentaryDto> = vec![];
    for r in &hits {
        if let Some((_, _, status, json)) = s.cached.iter().find(|c| c.0 == *r) {
            if status == "ok" {
                if let Ok(c) = serde_json::from_str::<RoundCommentary>(json) {
                    out.push(commentary_dto(&c, &s.model));
                }
            }
        }
    }

    let mut error: Option<String> = None;
    let mut new_rows: Vec<CoachCacheRow> = vec![];
    let by_round: HashMap<u32, &RoundInput> = s.inputs.iter().map(|r| (r.round, r)).collect();
    let block_of: HashMap<u32, &(u32, String, String)> =
        s.blocks.iter().map(|b| (b.0, b)).collect();

    'chunks: for chunk in chunk_rounds(&misses) {
        let mut pending: Vec<u32> = chunk.clone();
        let mut notes: Vec<String> = vec![];
        for attempt in 0..2 {
            if pending.is_empty() {
                break;
            }
            let inputs: Vec<RoundInput> = pending
                .iter()
                .filter_map(|r| by_round.get(r).map(|x| (*x).clone()))
                .collect();
            let prompt = render_round_batch(&s.match_input, &inputs, &notes);
            let generated = match s
                .client
                .generate_json(&s.model, SYSTEM_PERSONA, &prompt, &round_batch_schema())
                .await
            {
                Ok(g) => g,
                Err(e) => {
                    error = Some(e.to_string());
                    break 'chunks;
                }
            };
            let parsed = match parse_round_batch(&generated.text) {
                Ok(p) => p,
                Err(e) => {
                    notes = pending.iter().map(|r| format!("Round {r}: {e}")).collect();
                    if attempt == 1 {
                        error = Some(e.to_string());
                    }
                    continue;
                }
            };
            let mut still: Vec<u32> = vec![];
            notes.clear();
            for round in &pending {
                let Some((block, hash)) = block_of.get(round).map(|b| (&b.1, &b.2)) else {
                    continue;
                };
                let ticks: Vec<i32> = by_round[round].plays.iter().map(|p| p.tick).collect();
                let g = Grounding::for_round(
                    block,
                    &s.match_input.roster,
                    &s.known_callouts,
                    &ticks,
                    *round,
                );
                let fallback_row = |violations: String| CoachCacheRow {
                    kind: KIND_ROUND.into(),
                    round: *round,
                    facts_hash: hash.clone(),
                    model: s.model.clone(),
                    status: "fallback".into(),
                    response_json: "null".into(),
                    violations_json: violations,
                };
                match parsed.iter().find(|c| c.round == *round) {
                    Some(c) => {
                        let v = validate_round(c, &g);
                        if v.is_empty() {
                            out.push(commentary_dto(c, &s.model));
                            new_rows.push(CoachCacheRow {
                                kind: KIND_ROUND.into(),
                                round: *round,
                                facts_hash: hash.clone(),
                                model: s.model.clone(),
                                status: "ok".into(),
                                response_json: serde_json::to_string(c).unwrap_or_default(),
                                violations_json: "[]".into(),
                            });
                        } else {
                            notes.push(retry_note(*round, &v));
                            if attempt == 1 {
                                new_rows.push(fallback_row(violations_json(&v)));
                            } else {
                                still.push(*round);
                            }
                        }
                    }
                    None => {
                        notes.push(format!("Round {round}: missing from the answer."));
                        if attempt == 1 {
                            new_rows.push(fallback_row("[\"missing\"]".into()));
                        } else {
                            still.push(*round);
                        }
                    }
                }
            }
            pending = still;
        }
    }

    if !new_rows.is_empty() {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        for row in &new_rows {
            store
                .put_coach_cache(match_id, row)
                .map_err(|e| e.to_string())?;
        }
    }
    out.sort_by_key(|r| r.round);
    Ok(CoachRoundsDto { rounds: out, error })
}

fn synthesis_dto(m: MatchSynthesis, model: &str) -> MatchSynthesisDto {
    MatchSynthesisDto {
        opening: m.opening,
        work_on: m.work_on,
        model: model.to_string(),
    }
}

/// Match synthesis from the validated round reads + template insights +
/// habits. Cached as (match, "synthesis", 0); `force` busts it. Only a
/// definitive outcome is cached — a transport error must not pin a
/// fallback.
pub async fn synthesis(
    state: &State<'_, AppState>,
    match_id: i64,
    force: bool,
) -> Result<CoachSynthesisDto, String> {
    let rounds = round_commentary(state, match_id, &[]).await?;
    let quiet = CoachSynthesisDto {
        synthesis: None,
        error: None,
    };
    let prep = {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        if !coach_enabled(&store)? {
            return Ok(quiet);
        }
        let Some((key, _)) = resolve_key(&store)? else {
            return Ok(quiet);
        };
        let model = model_setting(&store, SETTING_SYNTHESIS_MODEL, DEFAULT_SYNTHESIS_MODEL)?;
        let reviews = assemble_round_reviews(&mut store, match_id)?;
        let Some(bundle) = match_context(&store, match_id)? else {
            return Ok(quiet);
        };
        let match_input = match_input(&bundle);
        let narrator = cf_narrator::TemplateNarrator;
        let insights: Vec<String> = store
            .insights_for_match(match_id)
            .map_err(|e| e.to_string())?
            .iter()
            .filter_map(insight_from_row)
            .map(|i| {
                let n = cf_narrator::CoachingNarrator::narrate(&narrator, &i, &bundle.ctx);
                format!("{}: {}", n.title, n.body)
            })
            .collect();
        let habits: Vec<String> = habit_reports(&store)?
            .iter()
            .map(|h| format!("{}: {}", h.title, h.body))
            .collect();
        let digests: Vec<RoundDigest> = reviews
            .iter()
            .map(|rv| RoundDigest {
                round: rv.round,
                verdict_label: rv.verdict_label.clone(),
                won: rv.won,
                read: rounds
                    .rounds
                    .iter()
                    .find(|c| c.round == rv.round)
                    .map(|c| c.read.clone())
                    .or_else(|| rv.why_it_mattered.clone())
                    .unwrap_or_else(|| "(no coach note)".to_string()),
            })
            .collect();
        let si = SynthesisInput {
            match_input,
            rounds: digests,
            insights,
            habits,
        };
        let prompt = render_synthesis(&si);
        let hash = facts_hash(&prompt, &model);
        let cached = store
            .get_coach_cache(match_id, KIND_SYNTHESIS, 0)
            .map_err(|e| e.to_string())?;
        let plays_jsons: Vec<String> = store
            .load_round_plays(match_id)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|r| r.plays_json)
            .collect();
        (
            GeminiClient::new(key).map_err(|e| e.to_string())?,
            model,
            si,
            prompt,
            hash,
            cached,
            known_callouts(&plays_jsons),
        )
    };
    let (client, model, si, prompt, hash, cached, known) = prep;
    if let Some(row) = &cached {
        if row.facts_hash == hash && !force {
            let synthesis = match row.status.as_str() {
                "ok" => serde_json::from_str::<MatchSynthesis>(&row.response_json)
                    .ok()
                    .map(|m| synthesis_dto(m, &model)),
                _ => None,
            };
            return Ok(CoachSynthesisDto {
                synthesis,
                error: rounds.error,
            });
        }
    }
    let g = Grounding::for_synthesis(&prompt, &si.match_input.roster, &known);
    let mut notes: Vec<String> = vec![];
    let mut result: Option<MatchSynthesis> = None;
    let mut error = rounds.error;
    let mut violations = "[]".to_string();
    for _attempt in 0..2 {
        let user = if notes.is_empty() {
            prompt.clone()
        } else {
            format!(
                "{prompt}\nYour previous answer cited things that are not in the facts. Rewrite it using only the facts above:\n{}\n",
                notes.join("\n")
            )
        };
        match client
            .generate_json(&model, SYSTEM_PERSONA, &user, &synthesis_schema())
            .await
        {
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
            Ok(gen) => match parse_synthesis(&gen.text) {
                Err(e) => {
                    notes = vec![e.to_string()];
                    error = Some(e.to_string());
                }
                Ok(ms) => {
                    let v = validate_synthesis(&ms, &g);
                    if v.is_empty() {
                        result = Some(ms);
                        error = None;
                        break;
                    }
                    notes = vec![retry_note(0, &v)];
                    violations = violations_json(&v);
                }
            },
        }
    }
    let row = CoachCacheRow {
        kind: KIND_SYNTHESIS.into(),
        round: 0,
        facts_hash: hash,
        model: model.clone(),
        status: if result.is_some() { "ok" } else { "fallback" }.into(),
        response_json: result
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .unwrap_or_else(|| "null".into()),
        violations_json: violations,
    };
    // Only cache a definitive outcome: a transport error must not pin a fallback.
    if result.is_some() || error.is_none() {
        let mut store = state.store.lock().map_err(|_| "store lock poisoned")?;
        store
            .put_coach_cache(match_id, &row)
            .map_err(|e| e.to_string())?;
    }
    Ok(CoachSynthesisDto {
        synthesis: result.map(|m| synthesis_dto(m, &model)),
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{PlayDto, RoundReviewDto, TimelineDto};
    use cf_store::store::RoundInfo;

    fn review(round: u32) -> RoundReviewDto {
        RoundReviewDto {
            round,
            impact: -0.234,
            verdict: "not_on_you".into(),
            verdict_label: "Not on you".into(),
            attention: "dim".into(),
            selected: true,
            pivotal_tick: 29000,
            side: "CT".into(),
            won: false,
            kills: 0,
            deaths: 1,
            man_context: Some("3v5".into()),
            moments: vec![],
            plays: vec![PlayDto {
                tick: 26752,
                kind: "setup".into(),
                phase: "opening".into(),
                headline: "Setup at B Site".into(),
                facts: vec!["Nearest teammate Sam, 159 u".into()],
                quality: None,
                rule_id: None,
                delta_p: None,
                focus: vec![],
                killer: None,
            }],
            timeline: vec![
                TimelineDto {
                    tick: 29000,
                    kind: "kill".into(),
                    actor: Some("Kit".into()),
                    subject: Some("misosoupy3".into()),
                    side: Some("T".into()),
                    weapon: Some("weapon_awp".into()),
                },
                TimelineDto {
                    tick: 29500,
                    kind: "plant".into(),
                    actor: Some("Kit".into()),
                    subject: None,
                    side: Some("T".into()),
                    weapon: None,
                },
            ],
            why_it_mattered: None,
            what_to_practise: None,
        }
    }
    fn rounds() -> Vec<RoundInfo> {
        vec![
            RoundInfo {
                number: 5,
                start_tick: 20000,
                freeze_end_tick: Some(21000),
                end_tick: 26000,
                officially_ended_tick: None,
                winner: "CT".into(),
                reason: "t_killed".into(),
            },
            RoundInfo {
                number: 6,
                start_tick: 26000,
                freeze_end_tick: Some(26432),
                end_tick: 31000,
                officially_ended_tick: None,
                winner: "T".into(),
                reason: "ct_killed".into(),
            },
        ]
    }

    #[test]
    fn round_inputs_render_clocks_timeline_lines_and_prior_digest() {
        let reviews = vec![
            RoundReviewDto {
                round: 5,
                won: true,
                verdict_label: "Quiet".into(),
                ..review(5)
            },
            review(6),
        ];
        let inputs = build_round_inputs(&reviews, &rounds(), 64.0);
        assert_eq!(inputs.len(), 2);
        let r6 = &inputs[1];
        assert_eq!(r6.impact_pct, -23);
        assert_eq!(r6.plays[0].clock, "+5 s"); // (26752-26432)/64 = 5.0
        assert_eq!(r6.timeline[0], "+40 s Kit killed misosoupy3 (awp)");
        assert_eq!(r6.timeline[1], "+48 s Kit planted the bomb");
        assert_eq!(r6.prior_digest, vec!["Round 5 · Quiet · won".to_string()]);
        assert!(inputs[0].prior_digest.is_empty());
    }

    #[test]
    fn facts_hash_changes_with_the_block_the_model_or_the_style() {
        let a = facts_hash("block", "m1");
        assert_eq!(a.len(), 64);
        assert_ne!(a, facts_hash("block2", "m1"));
        assert_ne!(a, facts_hash("block", "m2"));
        assert_eq!(a, facts_hash("block", "m1"));
    }

    #[test]
    fn known_callouts_come_from_raw_place_keys_prettified_and_deduped() {
        let plays_json = r#"[{"kind":"death","facts":{"place":"BombsiteB","killer_place":"BombsiteB","weapon":"awp"}},{"kind":"rotation","facts":{"place_at_plant":"CTSpawn"}},{"kind":"kill","facts":{"victim_place":null}}]"#;
        let c = known_callouts(&[plays_json.to_string()]);
        assert_eq!(
            c,
            vec![
                cf_narrator::callouts::callout_name("BombsiteB"),
                cf_narrator::callouts::callout_name("CTSpawn")
            ]
        );
    }

    #[test]
    fn chunks_of_six_and_cache_split() {
        let ids: Vec<u32> = (1..=14).collect();
        let chunks = chunk_rounds(&ids);
        assert_eq!(
            chunks.iter().map(|c| c.len()).collect::<Vec<_>>(),
            vec![6, 6, 2]
        );
        let rows = vec![
            (6u32, "h6".to_string(), "ok".to_string()),
            (7, "stale".into(), "ok".into()),
            (8, "h8".into(), "fallback".into()),
        ];
        let hashes = vec![
            (6u32, "h6".to_string()),
            (7, "h7".into()),
            (8, "h8".into()),
            (9, "h9".into()),
        ];
        let (hits, misses) = split_cache(&hashes, &rows, &[]);
        assert_eq!(hits, vec![6, 8]); // 8 is a cached fallback: a hit that renders as None
        assert_eq!(misses, vec![7, 9]); // 7's hash is stale
        let (_, forced) = split_cache(&hashes, &rows, &[6]);
        assert_eq!(forced, vec![6, 7, 9]);
    }
}
