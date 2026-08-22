//! Round-by-round review engine (issue #9; ADR-0008): scores every round
//! from the tracked player's win-probability perspective and turns that
//! score into the handful of rounds + moments the coaching UI narrates.
//! Pure functions over a narrow input (kill list + bomb events + flags —
//! no tick samples), so it's buildable either from `MatchData` at analysis
//! time or backfilled from stored DB rows.
//!
//! The 8-point model (verbatim from the V1.2 plan / ADR-0008):
//!
//! 1. **State replay per round:** start `(ct = round.ct.len(), t =
//!    round.t.len(), planted = false)`. Events in tick order within
//!    `[start_tick, officially_ended_tick.unwrap_or(end_tick)]`: kills with
//!    that `round` number (skip `round == 0`), plus bomb events in the
//!    span. Kill → decrement the victim's side (roster lookup; victim on
//!    neither roster → skip, silence). Plant → `planted = true`. Defuse →
//!    CT-side P becomes 1.0 (round decided). Explode → CT-side P becomes
//!    0.0.
//! 2. **ΔP per event** = `P_after − P_before` where `P` = `p_ct_win(...)`
//!    if tracked ∈ round.ct else `1 − p_ct_win(...)`; either side `None` →
//!    event unobserved → `delta_p = None`, contributes nothing.
//! 3. **impact** = Σ `delta_p` over PLAYER-ATTRIBUTED events:
//!    `kill.attacker == Some(tracked)`, `kill.victim == tracked`,
//!    `bomb.player == Some(tracked)` (plant/defuse).
//! 4. **pivotal_tick** = tick of max `|delta_p|` over ALL scored events
//!    (the round's turning point, not necessarily the player's).
//! 5. **Selection:** candidates = rounds with `|impact| ≥
//!    attention_threshold_p`, sorted by `|impact|` desc, take `max_rounds`.
//!    **Won-it guarantee:** if a candidate with `verdict == WonIt` was cut
//!    by the cap AND the selection contains a non-WonIt, replace the
//!    lowest-|impact| non-WonIt with the highest-|impact| excluded WonIt.
//! 6. **Attention:** `selected && |impact| ≥ pivotal_threshold_p` →
//!    Bright; `selected` → Dim; else None (a capped-out round shows no dot
//!    — no dot without rail content).
//! 7. **Verdict precedence (the load-bearing order — `NotOnYou` MUST
//!    precede `CostYou`):**
//!    - `WonIt`: `impact ≥ attention_threshold_p && header.won`
//!    - `NotOnYou`: any flag with `steamid == tracked`, this round,
//!      `rule_id ∈ cfg.rbr.exculpatory_rules`
//!    - `Traded`: tracked died this round AND the killer appears as a
//!      victim within `commit_window_s` after the death AND `impact > −
//!      pivotal_threshold_p`
//!    - `CostYou`: `impact ≤ −attention_threshold_p`
//!    - `Quiet`: otherwise
//! 8. **Moments (selected rounds only; unselected rounds get `moments:
//!    vec![]`):** one per tracked kill / tracked death / tracked plant /
//!    tracked defuse (with `delta_p`), plus one `"flag"` moment per
//!    tracked-player flag in the round whose tick doesn't already have a
//!    moment (dedup by tick: a death moment absorbs its death-anchored
//!    flags — merge each such flag's `details` into the death moment's
//!    `facts` and set `rule_id` to the highest-severity fired rule). Death
//!    facts add `"killer"`, `"traded"` (bool), `"round_end_delta_s"`
//!    (`(round end − death tick)/tickrate`, 1 decimal). Tick order, cap
//!    `max_moments` (keep earliest N — the story reads forward).

use std::collections::HashMap;

use cf_parser::model::Side;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::{DetectorConfig, RbrCfg};
use crate::winprob::WinProbTable;

/// Narrow input — buildable from DB rows (the backfill path) or from
/// MatchData. No tick samples: alive counts replay from the kill list.
#[derive(Debug, Clone)]
pub struct RoundReviewInput {
    pub tracked: u64,
    pub tickrate: f32,
    pub rounds: Vec<ReviewRound>,
    pub kills: Vec<ReviewKill>,
    pub bomb_events: Vec<ReviewBomb>, // no round field — attributed by tick span
    pub flags: Vec<ReviewFlag>,
}

#[derive(Debug, Clone)]
pub struct ReviewRound {
    pub number: u32,
    pub start_tick: i32,
    pub freeze_end_tick: Option<i32>,
    pub end_tick: i32,
    pub officially_ended_tick: Option<i32>,
    pub winner: Side,
    pub ct: Vec<u64>,
    pub t: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct ReviewKill {
    pub round: u32,
    pub tick: i32,
    pub attacker: Option<u64>,
    pub victim: u64,
    pub weapon: String,
}

#[derive(Debug, Clone)]
pub struct ReviewBomb {
    pub tick: i32,
    pub kind: String, // "planted" | "defused" | "exploded"
    pub player: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ReviewFlag {
    pub rule_id: String,
    pub round: u32,
    pub tick: i32,
    pub steamid: u64,
    pub severity: f32,
    pub confidence: f32,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    WonIt,
    CostYou,
    NotOnYou,
    Traded,
    Quiet,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::WonIt => "won_it",
            Verdict::CostYou => "cost_you",
            Verdict::NotOnYou => "not_on_you",
            Verdict::Traded => "traded",
            Verdict::Quiet => "quiet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attention {
    None,
    Dim,
    Bright,
}

impl Attention {
    pub fn as_str(&self) -> &'static str {
        match self {
            Attention::None => "none",
            Attention::Dim => "dim",
            Attention::Bright => "bright",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundHeader {
    pub side: String, // "CT" | "T" (tracked side this round)
    pub won: bool,    // tracked side won
    pub kills: u32,   // tracked kills on enemies this round
    pub deaths: u32,  // 0 or 1
    /// "3v5" — my-side v their-side alive counts immediately BEFORE the
    /// pivotal event. None when no pivotal event scored.
    pub man_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Moment {
    pub tick: i32,
    /// "tracked_kill" | "tracked_death" | "plant" | "defuse" | "flag"
    pub kind: String,
    pub rule_id: Option<String>,
    /// Signed tracked-side win-prob delta. None = unobserved cell (silence).
    pub delta_p: Option<f32>,
    /// Structured facts, RAW callouts (prettified only at narration):
    /// merged flag `details` plus computed keys — for tracked_death:
    /// "killer", "traded" (bool), "round_end_delta_s"; for tracked_kill:
    /// "victim"; plant/defuse: none extra.
    pub facts: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundReview {
    pub round: u32,
    pub impact: f32,
    pub verdict: Verdict,
    pub attention: Attention,
    pub selected: bool,
    pub pivotal_tick: i32, // 0 when nothing scored
    pub header: RoundHeader,
    pub moments: Vec<Moment>, // tick-ordered, capped at cfg.rbr.max_moments
}

/// One event in a round's kill-list state replay, in tick order.
#[derive(Debug, Clone, Copy)]
enum RoundEvent<'a> {
    Kill(&'a ReviewKill),
    Bomb(&'a ReviewBomb),
}

impl RoundEvent<'_> {
    fn tick(&self) -> i32 {
        match self {
            RoundEvent::Kill(k) => k.tick,
            RoundEvent::Bomb(b) => b.tick,
        }
    }
}

/// A scored event, kept around after state replay so `build_moments` can
/// turn the player-relevant ones into `Moment`s.
#[derive(Debug, Clone)]
struct ScoredEvent {
    tick: i32,
    delta_p: Option<f32>,
    detail: EventDetail,
}

#[derive(Debug, Clone)]
enum EventDetail {
    Kill { attacker: Option<u64>, victim: u64 },
    Plant { player: Option<u64> },
    Defuse { player: Option<u64> },
    Explode,
}

/// The tracked player's death this round, if any, plus whether the killer
/// was punished within the trade commit window (H2's own definition).
#[derive(Debug, Clone, Copy)]
struct TrackedDeath {
    killer: Option<u64>,
    traded: bool,
}

struct RoundScoreCore {
    impact: f32,
    pivotal_tick: i32,
    header: RoundHeader,
    events: Vec<ScoredEvent>,
    tracked_death: Option<TrackedDeath>,
    end_tick: i32,
}

struct RoundCandidate {
    round: u32,
    impact: f32,
    verdict: Verdict,
}

struct RoundSelection {
    round: u32,
    selected: bool,
    attention: Attention,
}

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

/// Gather this round's kills (by `round` number) and bomb events (by tick
/// span), tick-ordered. Model point 1.
fn round_events<'a>(
    round: &ReviewRound,
    kills: &'a [ReviewKill],
    bombs: &'a [ReviewBomb],
) -> Vec<RoundEvent<'a>> {
    let span_end = round.officially_ended_tick.unwrap_or(round.end_tick);
    let mut events: Vec<RoundEvent> = kills
        .iter()
        .filter(|k| {
            k.round == round.number
                && k.round != 0
                && k.tick >= round.start_tick
                && k.tick <= span_end
        })
        .map(RoundEvent::Kill)
        .collect();
    events.extend(
        bombs
            .iter()
            .filter(|b| b.tick >= round.start_tick && b.tick <= span_end)
            .map(RoundEvent::Bomb),
    );
    events.sort_by_key(RoundEvent::tick);
    events
}

/// P(tracked's side wins) at a live state, or None for an unobserved cell
/// (silence bias — the caller must skip scoring, never invent a number).
/// Model point 2.
fn tracked_perspective_p(
    table: &WinProbTable,
    side: Side,
    ct: u8,
    t: u8,
    planted: bool,
) -> Option<f32> {
    let p_ct = table.p_ct_win(ct, t, planted)?;
    Some(match side {
        Side::Ct => p_ct,
        Side::T => 1.0 - p_ct,
    })
}

/// State-replay one round: impact, pivotal tick, header, and the scored
/// event list `build_moments` will turn into moments. Model points 1-4.
fn score_round(
    round: &ReviewRound,
    tracked: u64,
    kills: &[ReviewKill],
    bombs: &[ReviewBomb],
    table: &WinProbTable,
    commit_window_s: f32,
    tickrate: f32,
) -> RoundScoreCore {
    let tracked_side = if round.ct.contains(&tracked) {
        Some(Side::Ct)
    } else if round.t.contains(&tracked) {
        Some(Side::T)
    } else {
        None
    };

    let Some(side) = tracked_side else {
        // Tracked player absent from both rosters this round: nothing to
        // score. Data anomaly, not expected in practice — stay silent.
        return RoundScoreCore {
            impact: 0.0,
            pivotal_tick: 0,
            header: RoundHeader {
                side: side_str(Side::Ct).to_string(),
                won: false,
                kills: 0,
                deaths: 0,
                man_context: None,
            },
            events: vec![],
            tracked_death: None,
            end_tick: round.end_tick,
        };
    };

    let enemy_roster: &[u64] = if side == Side::Ct {
        &round.t
    } else {
        &round.ct
    };

    let mut ct_alive = round.ct.len() as u8;
    let mut t_alive = round.t.len() as u8;
    let mut planted = false;

    let mut impact = 0.0f32;
    let mut kills_count = 0u32;
    let mut deaths_count = 0u32;
    // (tick, |delta|, ct_before, t_before) of the current best pivotal candidate.
    let mut pivotal: Option<(i32, f32, u8, u8)> = None;
    let mut scored: Vec<ScoredEvent> = vec![];

    for ev in round_events(round, kills, bombs) {
        let ct_before = ct_alive;
        let t_before = t_alive;

        match ev {
            RoundEvent::Kill(k) => {
                let on_ct = round.ct.contains(&k.victim);
                let on_t = round.t.contains(&k.victim);
                if !on_ct && !on_t {
                    continue; // victim on neither roster: silence, skip entirely
                }
                let p_before = tracked_perspective_p(table, side, ct_before, t_before, planted);
                if on_ct {
                    ct_alive -= 1;
                } else {
                    t_alive -= 1;
                }
                let p_after = tracked_perspective_p(table, side, ct_alive, t_alive, planted);
                let delta = match (p_before, p_after) {
                    (Some(a), Some(b)) => Some(b - a),
                    _ => None,
                };

                if k.attacker == Some(tracked) {
                    if enemy_roster.contains(&k.victim) {
                        kills_count += 1;
                    }
                    if let Some(d) = delta {
                        impact += d;
                    }
                }
                if k.victim == tracked {
                    deaths_count += 1;
                    if let Some(d) = delta {
                        impact += d;
                    }
                }
                if let Some(d) = delta {
                    record_pivotal(&mut pivotal, k.tick, d, ct_before, t_before);
                }
                scored.push(ScoredEvent {
                    tick: k.tick,
                    delta_p: delta,
                    detail: EventDetail::Kill {
                        attacker: k.attacker,
                        victim: k.victim,
                    },
                });
            }
            RoundEvent::Bomb(b) => {
                let p_before = tracked_perspective_p(table, side, ct_before, t_before, planted);
                let (forced_ct_p, detail) = match b.kind.as_str() {
                    "planted" => {
                        planted = true;
                        (None, EventDetail::Plant { player: b.player })
                    }
                    "defused" => (Some(1.0), EventDetail::Defuse { player: b.player }),
                    "exploded" => (Some(0.0), EventDetail::Explode),
                    _ => continue, // unrecognized bomb event kind: silence
                };
                let p_after = match forced_ct_p {
                    Some(p_ct) => Some(match side {
                        Side::Ct => p_ct,
                        Side::T => 1.0 - p_ct,
                    }),
                    None => tracked_perspective_p(table, side, ct_alive, t_alive, planted),
                };
                let delta = match (p_before, p_after) {
                    (Some(a), Some(b)) => Some(b - a),
                    _ => None,
                };
                let attributed = b.player == Some(tracked)
                    && matches!(
                        detail,
                        EventDetail::Plant { .. } | EventDetail::Defuse { .. }
                    );
                if attributed {
                    if let Some(d) = delta {
                        impact += d;
                    }
                }
                if let Some(d) = delta {
                    record_pivotal(&mut pivotal, b.tick, d, ct_before, t_before);
                }
                scored.push(ScoredEvent {
                    tick: b.tick,
                    delta_p: delta,
                    detail,
                });
            }
        }
    }

    let won = round.winner == side;
    let man_context = pivotal.map(|(_, _, ct_before, t_before)| {
        let (my, their) = match side {
            Side::Ct => (ct_before, t_before),
            Side::T => (t_before, ct_before),
        };
        format!("{my}v{their}")
    });
    let pivotal_tick = pivotal.map_or(0, |(tick, ..)| tick);

    let tracked_death = scored
        .iter()
        .find(|e| matches!(&e.detail, EventDetail::Kill { victim, .. } if *victim == tracked))
        .and_then(|death_ev| {
            let EventDetail::Kill { attacker, .. } = &death_ev.detail else {
                return None;
            };
            let killer = *attacker;
            let commit_ticks = (commit_window_s * tickrate).round() as i32;
            let traded = killer.is_some_and(|k_id| {
                scored.iter().any(|e| {
                    e.tick > death_ev.tick
                        && e.tick <= death_ev.tick + commit_ticks
                        && matches!(&e.detail, EventDetail::Kill { victim, .. } if *victim == k_id)
                })
            });
            Some(TrackedDeath { killer, traded })
        });

    RoundScoreCore {
        impact,
        pivotal_tick,
        header: RoundHeader {
            side: side_str(side).to_string(),
            won,
            kills: kills_count,
            deaths: deaths_count,
            man_context,
        },
        events: scored,
        tracked_death,
        end_tick: round.end_tick,
    }
}

fn record_pivotal(
    pivotal: &mut Option<(i32, f32, u8, u8)>,
    tick: i32,
    delta: f32,
    ct_before: u8,
    t_before: u8,
) {
    let mag = delta.abs();
    let is_new_best = match pivotal {
        Some((_, best, _, _)) => mag > *best,
        None => true,
    };
    if is_new_best {
        *pivotal = Some((tick, mag, ct_before, t_before));
    }
}

/// Verdict precedence (model point 7) — order is load-bearing: `NotOnYou`
/// MUST be checked before `CostYou` (issue #9's hard rule).
fn assign_verdict(
    round: u32,
    impact: f32,
    won: bool,
    tracked_death: &Option<TrackedDeath>,
    flags: &[ReviewFlag],
    tracked: u64,
    cfg: &RbrCfg,
) -> Verdict {
    if impact >= cfg.attention_threshold_p && won {
        return Verdict::WonIt;
    }
    let not_on_you = flags.iter().any(|f| {
        f.round == round
            && f.steamid == tracked
            && cfg.exculpatory_rules.iter().any(|r| r == &f.rule_id)
    });
    if not_on_you {
        return Verdict::NotOnYou;
    }
    if let Some(td) = tracked_death {
        if td.traded && impact > -cfg.pivotal_threshold_p {
            return Verdict::Traded;
        }
    }
    if impact <= -cfg.attention_threshold_p {
        return Verdict::CostYou;
    }
    Verdict::Quiet
}

/// Threshold-with-cap selection + the won-it guarantee + attention level.
/// Model points 5-6.
fn select_rounds(candidates: &[RoundCandidate], cfg: &RbrCfg) -> Vec<RoundSelection> {
    let mut ranked: Vec<&RoundCandidate> = candidates
        .iter()
        .filter(|c| c.impact.abs() >= cfg.attention_threshold_p)
        .collect();
    ranked.sort_by(|a, b| {
        b.impact
            .abs()
            .partial_cmp(&a.impact.abs())
            .expect("impact is always finite")
    });

    let mut selected: Vec<u32> = ranked
        .iter()
        .take(cfg.max_rounds)
        .map(|c| c.round)
        .collect();

    // Won-it guarantee: a cut WonIt candidate replaces the weakest
    // non-WonIt currently in the selection.
    let cut_wonit = ranked
        .iter()
        .skip(cfg.max_rounds)
        .filter(|c| c.verdict == Verdict::WonIt)
        .max_by(|a, b| {
            a.impact
                .abs()
                .partial_cmp(&b.impact.abs())
                .expect("impact is always finite")
        });
    if let Some(wonit) = cut_wonit {
        let weakest_non_wonit = selected
            .iter()
            .enumerate()
            .filter(|(_, &r)| {
                candidates
                    .iter()
                    .find(|c| c.round == r)
                    .is_some_and(|c| c.verdict != Verdict::WonIt)
            })
            .min_by(|(_, &a), (_, &b)| {
                let ia = candidates
                    .iter()
                    .find(|c| c.round == a)
                    .unwrap()
                    .impact
                    .abs();
                let ib = candidates
                    .iter()
                    .find(|c| c.round == b)
                    .unwrap()
                    .impact
                    .abs();
                ia.partial_cmp(&ib).expect("impact is always finite")
            })
            .map(|(idx, _)| idx);
        if let Some(idx) = weakest_non_wonit {
            selected[idx] = wonit.round;
        }
    }

    candidates
        .iter()
        .map(|c| {
            let is_selected = selected.contains(&c.round);
            let attention = if is_selected && c.impact.abs() >= cfg.pivotal_threshold_p {
                Attention::Bright
            } else if is_selected {
                Attention::Dim
            } else {
                Attention::None
            };
            RoundSelection {
                round: c.round,
                selected: is_selected,
                attention,
            }
        })
        .collect()
}

/// Turn a selected round's scored events + flags into the tick-ordered,
/// capped moment list. Model point 8.
fn build_moments(
    tracked: u64,
    events: &[ScoredEvent],
    flags: &[ReviewFlag],
    tracked_death: &Option<TrackedDeath>,
    round_end_tick: i32,
    tickrate: f32,
    max_moments: usize,
) -> Vec<Moment> {
    let mut moments: Vec<Moment> = vec![];

    for ev in events {
        match &ev.detail {
            EventDetail::Kill { attacker, victim } if *attacker == Some(tracked) => {
                moments.push(Moment {
                    tick: ev.tick,
                    kind: "tracked_kill".to_string(),
                    rule_id: None,
                    delta_p: ev.delta_p,
                    facts: json!({ "victim": victim.to_string() }),
                });
            }
            EventDetail::Kill { victim, attacker } if *victim == tracked => {
                let (killer, traded) = match tracked_death {
                    Some(td) => (td.killer, td.traded),
                    None => (*attacker, false),
                };
                let round_end_delta_s =
                    ((round_end_tick - ev.tick) as f32 / tickrate * 10.0).round() / 10.0;
                moments.push(Moment {
                    tick: ev.tick,
                    kind: "tracked_death".to_string(),
                    rule_id: None,
                    delta_p: ev.delta_p,
                    facts: json!({
                        "killer": killer.map(|k| k.to_string()),
                        "traded": traded,
                        "round_end_delta_s": round_end_delta_s,
                    }),
                });
            }
            EventDetail::Plant { player } if *player == Some(tracked) => {
                moments.push(Moment {
                    tick: ev.tick,
                    kind: "plant".to_string(),
                    rule_id: None,
                    delta_p: ev.delta_p,
                    facts: json!({}),
                });
            }
            EventDetail::Defuse { player } if *player == Some(tracked) => {
                moments.push(Moment {
                    tick: ev.tick,
                    kind: "defuse".to_string(),
                    rule_id: None,
                    delta_p: ev.delta_p,
                    facts: json!({}),
                });
            }
            _ => {}
        }
    }

    // Layer in tracked-player flags: dedup by tick (a death moment absorbs
    // its death-anchored flags), keeping the highest-severity rule_id.
    let mut best_severity: HashMap<i32, f32> = HashMap::new();
    for f in flags {
        if f.steamid != tracked {
            continue;
        }
        if let Some(existing) = moments.iter_mut().find(|m| m.tick == f.tick) {
            if let (Some(obj), Some(fobj)) = (existing.facts.as_object_mut(), f.details.as_object())
            {
                for (k, v) in fobj {
                    obj.insert(k.clone(), v.clone());
                }
            }
            let best = best_severity.entry(f.tick).or_insert(f32::MIN);
            if f.severity >= *best {
                *best = f.severity;
                existing.rule_id = Some(f.rule_id.clone());
            }
        } else {
            moments.push(Moment {
                tick: f.tick,
                kind: "flag".to_string(),
                rule_id: Some(f.rule_id.clone()),
                delta_p: None,
                facts: f.details.clone(),
            });
            best_severity.insert(f.tick, f.severity);
        }
    }

    moments.sort_by_key(|m| m.tick);
    moments.truncate(max_moments);
    moments
}

struct RoundAssembly {
    round: u32,
    core: RoundScoreCore,
    verdict: Verdict,
    flags: Vec<ReviewFlag>,
}

pub fn review_rounds(input: &RoundReviewInput, cfg: &DetectorConfig) -> Vec<RoundReview> {
    let table = WinProbTable::v1();

    let assembled: Vec<RoundAssembly> = input
        .rounds
        .iter()
        .map(|r| {
            let round_flags: Vec<ReviewFlag> = input
                .flags
                .iter()
                .filter(|f| f.round == r.number)
                .cloned()
                .collect();
            let core = score_round(
                r,
                input.tracked,
                &input.kills,
                &input.bomb_events,
                table,
                cfg.trade.commit_window_s,
                input.tickrate,
            );
            let verdict = assign_verdict(
                r.number,
                core.impact,
                core.header.won,
                &core.tracked_death,
                &round_flags,
                input.tracked,
                &cfg.rbr,
            );
            RoundAssembly {
                round: r.number,
                core,
                verdict,
                flags: round_flags,
            }
        })
        .collect();

    let candidates: Vec<RoundCandidate> = assembled
        .iter()
        .map(|a| RoundCandidate {
            round: a.round,
            impact: a.core.impact,
            verdict: a.verdict,
        })
        .collect();
    let selections = select_rounds(&candidates, &cfg.rbr);
    let selection_by_round: HashMap<u32, RoundSelection> =
        selections.into_iter().map(|s| (s.round, s)).collect();

    assembled
        .into_iter()
        .map(|a| {
            let (selected, attention) = selection_by_round
                .get(&a.round)
                .map(|s| (s.selected, s.attention))
                .unwrap_or((false, Attention::None));
            let moments = if selected {
                build_moments(
                    input.tracked,
                    &a.core.events,
                    &a.flags,
                    &a.core.tracked_death,
                    a.core.end_tick,
                    input.tickrate,
                    cfg.rbr.max_moments,
                )
            } else {
                vec![]
            };
            RoundReview {
                round: a.round,
                impact: a.core.impact,
                verdict: a.verdict,
                attention,
                selected,
                pivotal_tick: a.core.pivotal_tick,
                header: a.core.header,
                moments,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rr(n: u32, start: i32, end: i32, winner: Side, ct: &[u64], t: &[u64]) -> ReviewRound {
        ReviewRound {
            number: n,
            start_tick: start,
            freeze_end_tick: Some(start),
            end_tick: end,
            officially_ended_tick: Some(end + 128),
            winner,
            ct: ct.to_vec(),
            t: t.to_vec(),
        }
    }

    fn kill(round: u32, tick: i32, att: u64, vic: u64) -> ReviewKill {
        ReviewKill {
            round,
            tick,
            attacker: Some(att),
            victim: vic,
            weapon: "ak47".to_string(),
        }
    }

    fn input(
        rounds: Vec<ReviewRound>,
        kills: Vec<ReviewKill>,
        bombs: Vec<ReviewBomb>,
        flags: Vec<ReviewFlag>,
    ) -> RoundReviewInput {
        RoundReviewInput {
            tracked: 1,
            tickrate: 64.0,
            rounds,
            kills,
            bomb_events: bombs,
            flags,
        }
    }

    #[test]
    fn tracked_entry_kill_scores_positive_delta() {
        let round = rr(1, 0, 5000, Side::Ct, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
        let inp = input(vec![round], vec![kill(1, 2000, 1, 6)], vec![], vec![]);
        let cfg = DetectorConfig::default();
        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        let table = WinProbTable::v1();
        let expected_delta =
            table.p_ct_win(5, 4, false).unwrap() - table.p_ct_win(5, 5, false).unwrap();

        assert!(r1.impact > 0.0, "impact should be positive: {}", r1.impact);
        assert!(
            (r1.impact - expected_delta).abs() < 1e-6,
            "impact {} should equal table-derived delta {}",
            r1.impact,
            expected_delta
        );
        assert_eq!(r1.pivotal_tick, 2000);
        assert_eq!(r1.header.kills, 1);
        assert_eq!(r1.header.deaths, 0);
    }

    #[test]
    fn tracked_death_scores_negative_and_flags_merge() {
        let round = rr(1, 0, 5000, Side::T, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
        let flag = ReviewFlag {
            rule_id: "H2_ISOLATED_DEATH".to_string(),
            round: 1,
            tick: 3000,
            steamid: 1,
            severity: 0.8,
            confidence: 0.75,
            details: json!({
                "nearest_teammate": "2",
                "distance": 1223.0,
                "place": "Catwalk",
            }),
        };
        let inp = input(vec![round], vec![kill(1, 3000, 6, 1)], vec![], vec![flag]);
        let mut cfg = DetectorConfig::default();
        // Force selection so the merge is visible in the moment list; the
        // moment-content assertions below are the point of this test, not
        // the selection threshold.
        cfg.rbr.attention_threshold_p = 0.0;

        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert!(r1.impact < 0.0, "impact should be negative: {}", r1.impact);
        let m = r1
            .moments
            .iter()
            .find(|m| m.kind == "tracked_death")
            .expect("a tracked_death moment must exist");
        assert_eq!(m.facts["distance"], json!(1223.0));
        assert_eq!(m.facts["place"], json!("Catwalk"));
        assert_eq!(m.facts["nearest_teammate"], json!("2"));
        assert_eq!(m.facts["killer"], json!("6"));
        assert_eq!(m.rule_id.as_deref(), Some("H2_ISOLATED_DEATH"));
        assert!(m.delta_p.unwrap() < 0.0);
    }

    #[test]
    fn plant_flips_perspective() {
        // Tracked on T side: rosters swapped relative to the default.
        let round = rr(1, 0, 5000, Side::T, &[6, 7, 8, 9, 10], &[1, 2, 3, 4, 5]);
        let bomb = ReviewBomb {
            tick: 4000,
            kind: "planted".to_string(),
            player: Some(1),
        };
        let inp = input(vec![round], vec![], vec![bomb], vec![]);
        let mut cfg = DetectorConfig::default();
        cfg.rbr.attention_threshold_p = 0.0;

        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert!(
            r1.impact > 0.0,
            "plant should help T's tracked player: {}",
            r1.impact
        );
        let m = r1
            .moments
            .iter()
            .find(|m| m.kind == "plant")
            .expect("a plant moment must exist");
        assert!(m.delta_p.unwrap() > 0.0);
    }

    #[test]
    fn unobserved_cell_contributes_nothing() {
        // 6-player CT roster: ct_alive starts at 6, out of the table's
        // 0..=5 range, so every lookup this round is None.
        let round = rr(
            1,
            0,
            5000,
            Side::Ct,
            &[1, 2, 3, 4, 5, 6],
            &[7, 8, 9, 10, 11],
        );
        let inp = input(vec![round], vec![kill(1, 2000, 1, 7)], vec![], vec![]);
        let mut cfg = DetectorConfig::default();
        cfg.rbr.attention_threshold_p = 0.0;

        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert_eq!(r1.impact, 0.0);
        assert!(
            !r1.moments.is_empty(),
            "expected the tracked kill to surface as a moment"
        );
        assert!(
            r1.moments.iter().all(|m| m.delta_p.is_none()),
            "every moment must carry no delta in an unobserved cell"
        );
    }

    #[test]
    fn traded_verdict_uses_commit_window() {
        let make = |second_kill_tick: i32| {
            let round = rr(1, 0, 5000, Side::Ct, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
            input(
                vec![round],
                vec![kill(1, 3000, 6, 1), kill(1, second_kill_tick, 2, 6)],
                vec![],
                vec![],
            )
        };
        let cfg = DetectorConfig::default();

        // 3000 + 127 ticks = within the 2.0s (128-tick) commit window.
        let traded = review_rounds(&make(3127), &cfg);
        let r1 = traded.iter().find(|r| r.round == 1).unwrap();
        assert_eq!(r1.verdict, Verdict::Traded);

        // 3000 + 129 + 64 ticks = well outside the window.
        let not_traded = review_rounds(&make(3193), &cfg);
        let r1b = not_traded.iter().find(|r| r.round == 1).unwrap();
        assert_ne!(r1b.verdict, Verdict::Traded);
    }

    #[test]
    fn not_on_you_beats_cost_you() {
        let round = rr(1, 0, 5000, Side::T, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
        let flag = ReviewFlag {
            rule_id: "H2_BAITED_TRADE".to_string(),
            round: 1,
            tick: 500,
            steamid: 1,
            severity: 0.35,
            confidence: 0.75,
            details: json!({ "non_following_teammate": "2" }),
        };
        let inp = input(vec![round], vec![kill(1, 500, 6, 1)], vec![], vec![flag]);
        let cfg = DetectorConfig::default();
        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert!(r1.impact < 0.0);
        assert_eq!(
            r1.verdict,
            Verdict::NotOnYou,
            "NotOnYou must precede CostYou"
        );
    }

    #[test]
    fn won_it_requires_win() {
        let round = rr(1, 0, 5000, Side::T, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
        // Two entry kills: a comfortable positive impact, but the round
        // (per `winner`) is lost anyway.
        let inp = input(
            vec![round],
            vec![kill(1, 1000, 1, 6), kill(1, 1500, 1, 7)],
            vec![],
            vec![],
        );
        let cfg = DetectorConfig::default();
        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert!(r1.impact > 0.0, "impact should be positive: {}", r1.impact);
        assert_ne!(r1.verdict, Verdict::WonIt, "a lost round is never WonIt");
        assert_eq!(r1.verdict, Verdict::Quiet);
    }

    #[test]
    fn unselected_round_has_no_moments_and_quiet_ok() {
        let round = rr(1, 0, 5000, Side::T, &[1, 2, 3, 4, 5], &[6, 7, 8, 9, 10]);
        // Neither attacker nor victim is tracked: no impact contribution.
        let inp = input(vec![round], vec![kill(1, 1000, 2, 6)], vec![], vec![]);
        let cfg = DetectorConfig::default();
        let reviews = review_rounds(&inp, &cfg);
        let r1 = reviews.iter().find(|r| r.round == 1).unwrap();

        assert_eq!(r1.impact, 0.0);
        assert!(!r1.selected);
        assert!(r1.moments.is_empty());
        assert_eq!(r1.verdict, Verdict::Quiet);
    }

    #[test]
    fn selection_threshold_and_cap() {
        let cfg = RbrCfg::default();
        let impacts = [0.5, 0.45, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.10, 0.05];
        let candidates: Vec<RoundCandidate> = impacts
            .iter()
            .enumerate()
            .map(|(i, &imp)| RoundCandidate {
                round: (i + 1) as u32,
                impact: imp,
                verdict: Verdict::Quiet,
            })
            .collect();
        let selections = select_rounds(&candidates, &cfg);

        let selected: Vec<u32> = selections
            .iter()
            .filter(|s| s.selected)
            .map(|s| s.round)
            .collect();
        assert_eq!(selected, vec![1, 2, 3, 4, 5, 6]);

        let bright: Vec<u32> = selections
            .iter()
            .filter(|s| s.attention == Attention::Bright)
            .map(|s| s.round)
            .collect();
        assert_eq!(bright, vec![1, 2, 3, 4]);

        let dim: Vec<u32> = selections
            .iter()
            .filter(|s| s.attention == Attention::Dim)
            .map(|s| s.round)
            .collect();
        assert_eq!(dim, vec![5, 6]);

        let none_att: Vec<u32> = selections
            .iter()
            .filter(|s| s.attention == Attention::None)
            .map(|s| s.round)
            .collect();
        assert_eq!(none_att, vec![7, 8, 9, 10]);
    }

    #[test]
    fn won_it_guarantee_swaps_weakest() {
        let cfg = RbrCfg::default(); // max_rounds = 6, attention_threshold_p = 0.18
        let impacts = [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3];
        let candidates: Vec<RoundCandidate> = impacts
            .iter()
            .enumerate()
            .map(|(i, &imp)| RoundCandidate {
                round: (i + 1) as u32,
                impact: imp,
                verdict: if i == 6 {
                    Verdict::WonIt
                } else {
                    Verdict::Quiet
                },
            })
            .collect();
        let selections = select_rounds(&candidates, &cfg);
        let selected: Vec<u32> = selections
            .iter()
            .filter(|s| s.selected)
            .map(|s| s.round)
            .collect();

        assert_eq!(selected.len(), 6);
        assert!(
            selected.contains(&7),
            "the cut WonIt round must be swapped in"
        );
        assert!(
            !selected.contains(&6),
            "the weakest non-WonIt round must be swapped out"
        );
    }

    #[test]
    fn moments_capped_in_tick_order() {
        let tracked = 1u64;
        let events: Vec<ScoredEvent> = (0..8i32)
            .map(|i| ScoredEvent {
                tick: 1000 + i * 100,
                delta_p: Some(0.05),
                detail: if i % 2 == 0 {
                    EventDetail::Kill {
                        attacker: Some(tracked),
                        victim: 100 + i as u64,
                    }
                } else {
                    EventDetail::Kill {
                        attacker: Some(200 + i as u64),
                        victim: tracked,
                    }
                },
            })
            .collect();

        let moments = build_moments(tracked, &events, &[], &None, 5000, 64.0, 6);

        assert_eq!(moments.len(), 6);
        let ticks: Vec<i32> = moments.iter().map(|m| m.tick).collect();
        assert_eq!(ticks, vec![1000, 1100, 1200, 1300, 1400, 1500]);
    }
}
