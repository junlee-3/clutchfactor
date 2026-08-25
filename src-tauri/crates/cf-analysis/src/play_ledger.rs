//! Play ledger (docs/spec/play-ledger-and-coach.md §2): every round, every
//! play the tracked player made, each carrying a number — plus a timeline
//! of everyone's kills and bomb events (the situation). Computed inside
//! `analyze()` because it needs tick samples, blinds, hurts and grenades,
//! which the DB post-pass (`round_review`) deliberately never sees.
//!
//! Rules: a play with no computable number is not emitted; `quality` only
//! when a measure backs it (spec §2 table); `delta_p` is never computed
//! here — commands.rs joins it from the ADR-0008 moments at serve time.
//! Facts keys are a contract read by cf-narrator and V1.3's validator —
//! steamids as strings, callouts RAW, distances whole units, seconds 1 dp.

use std::collections::HashMap;

use cf_parser::model::{Kill, Round, RoundEndReason, Side};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::DetectorConfig;
use crate::context::{AnalysisContext, RoundPhase};
use crate::families::flash_util::{flash_groups, FlashGroup};
use crate::families::h2::{committed, killed_in};
use crate::types::RuleFlag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Good,
    Bad,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Play {
    pub tick: i32,
    /// "freeze" | "opening" | "mid" | "late" | "post_plant" | "unknown"
    pub phase: String,
    /// setup | flash | smoke | he | molotov | rush | rotation | kill | death |
    /// assist | trade | missed_trade | plant | defuse | flag | outcome
    pub kind: String,
    pub facts: Value,
    pub quality: Option<Quality>,
    pub rule_id: Option<String>,
    pub delta_p: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub tick: i32,
    /// "kill" | "plant" | "defuse" | "explode"
    pub kind: String,
    pub actor: Option<String>,
    pub subject: Option<String>,
    /// The actor's side this round, "CT" | "T".
    pub side: Option<String>,
    pub weapon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundLedger {
    pub round: u32,
    pub plays: Vec<Play>,
    pub timeline: Vec<TimelineEvent>,
}

/// Every round the tracked player took part in, in round order.
pub fn build_ledger(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    flags: &[RuleFlag],
) -> Vec<RoundLedger> {
    let tracked = ctx.tracked();
    ctx.data()
        .rounds
        .iter()
        .filter_map(|round| {
            let side = ctx.side_of(tracked, round.number)?;
            let mut plays: Vec<Play> = vec![];
            plays.extend(setup_play(ctx, cfg, round, side));
            plays.extend(engagement_plays(ctx, cfg, round, side));
            plays.extend(utility_plays(ctx, cfg, round, side));
            plays.extend(trade_plays(ctx, cfg, round, side));
            plays.extend(rush_play(ctx, cfg, round));
            plays.extend(rotation_play(ctx, cfg, round, side, flags));
            plays.extend(bomb_plays(ctx, cfg, round));
            plays.extend(outcome_play(ctx, cfg, round, side));
            merge_flags(ctx, cfg, round, tracked, &mut plays, flags);
            finalize_death_quality(&mut plays, &cfg.rbr.exculpatory_rules);
            plays.sort_by(|a, b| {
                a.tick
                    .cmp(&b.tick)
                    .then(kind_rank(&a.kind).cmp(&kind_rank(&b.kind)))
            });
            Some(RoundLedger {
                round: round.number,
                plays,
                timeline: timeline(ctx, round),
            })
        })
        .collect()
}

// ---- shared helpers -------------------------------------------------------

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Ct => "CT",
        Side::T => "T",
    }
}

fn phase_of(ctx: &AnalysisContext, cfg: &DetectorConfig, round: u32, tick: i32) -> String {
    match ctx.phase_at(round, tick, &cfg.phase) {
        Some(RoundPhase::Freeze) => "freeze",
        Some(RoundPhase::Opening) => "opening",
        Some(RoundPhase::Mid) => "mid",
        Some(RoundPhase::Late) => "late",
        Some(RoundPhase::PostPlant) => "post_plant",
        None => "unknown",
    }
    .to_string()
}

fn span_end(round: &Round) -> i32 {
    round.officially_ended_tick.unwrap_or(round.end_tick)
}

/// f64 throughout: rounding in f32 then widening for JSON (`serde_json`
/// stores numbers as f64) reintroduces binary-fraction noise — e.g. 31.3
/// rounded as f32 widens to 31.299999237060547, failing byte-identical
/// facts comparisons. f64 round-tripping matches a `31.3` source literal.
fn secs_1dp(ticks: i32, tickrate: f32) -> f64 {
    (ticks as f64 / tickrate as f64 * 10.0).round() / 10.0
}

fn reason_str(r: &RoundEndReason) -> String {
    match r {
        RoundEndReason::TKilled => "t_killed".to_string(),
        RoundEndReason::CtKilled => "ct_killed".to_string(),
        RoundEndReason::BombDefused => "bomb_defused".to_string(),
        RoundEndReason::BombExploded => "bomb_exploded".to_string(),
        RoundEndReason::TargetSaved => "target_saved".to_string(),
        RoundEndReason::Other(s) => s.to_lowercase(),
    }
}

/// Roster minus everyone killed at or before `tick` — the same state
/// replay `round_review` uses (kill events, never tick samples, so a
/// synthetic or sparsely sampled track can't misreport a death).
fn alive_in(ctx: &AnalysisContext, round: &Round, roster: &[u64], tick: i32) -> usize {
    let dead = ctx
        .data()
        .kills
        .iter()
        .filter(|k| k.round == round.number && k.tick <= tick && roster.contains(&k.victim))
        .count();
    roster.len().saturating_sub(dead)
}

/// "3v5" — my side v their side alive at `tick` (callers pass
/// `kill.tick - 1` so the kill itself is not yet counted).
fn man_context(ctx: &AnalysisContext, round: &Round, side: Side, tick: i32) -> String {
    let (mine, theirs) = match side {
        Side::Ct => (&round.ct_steamids, &round.t_steamids),
        Side::T => (&round.t_steamids, &round.ct_steamids),
    };
    format!(
        "{}v{}",
        alive_in(ctx, round, mine, tick),
        alive_in(ctx, round, theirs, tick)
    )
}

fn play(tick: i32, phase: String, kind: &str, facts: Value, quality: Option<Quality>) -> Play {
    Play {
        tick,
        phase,
        kind: kind.to_string(),
        facts,
        quality,
        rule_id: None,
        delta_p: None,
    }
}

/// Same-tick ordering: what happened first in the tape, for the reader.
fn kind_rank(kind: &str) -> u8 {
    match kind {
        "setup" => 0,
        "flash" | "smoke" | "he" | "molotov" => 1,
        "rush" => 2,
        "trade" | "missed_trade" => 3,
        "kill" => 4,
        "death" => 5,
        "assist" => 6,
        "plant" | "defuse" => 7,
        "rotation" => 8,
        "flag" => 9,
        _ => 10,
    }
}

// ---- core plays -----------------------------------------------------------

fn setup_play(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
) -> Option<Play> {
    let tracked = ctx.tracked();
    let freeze_end = round.freeze_end_tick?;
    let tick = freeze_end + ctx.seconds(cfg.ledger.setup_s);
    if tick > round.end_tick {
        return None;
    }
    let me = ctx.state_at(tracked, tick)?;
    if !me.is_alive || me.tick < round.start_tick {
        return None; // no sample from THIS round yet -> silence
    }
    let z = cfg.general.z_weight;
    let mates = ctx.teammates_alive_at(tracked, round.number, tick);
    let nearest = ctx.nearest_teammate(tracked, round.number, tick, z);
    let within = mates
        .iter()
        .filter(|(_, st)| AnalysisContext::dist(&me, st, z) <= cfg.trade.isolation_u)
        .count();
    Some(play(
        tick,
        phase_of(ctx, cfg, round.number, tick),
        "setup",
        json!({
            "place": me.place,
            "side": side_str(side),
            "nearest_teammate": nearest.map(|(id, _)| id.to_string()),
            "nearest_teammate_dist": nearest.map(|(_, d)| d.round()),
            "teammates_within_isolation": within,
            "teammates_alive": mates.len(),
        }),
        None,
    ))
}

fn engagement_plays(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
) -> Vec<Play> {
    let tracked = ctx.tracked();
    let mut out = vec![];
    for k in ctx.data().kills.iter().filter(|k| k.round == round.number) {
        if k.attacker == Some(tracked) && k.victim != tracked {
            out.push(kill_play(ctx, cfg, round, side, k));
        } else if k.victim == tracked {
            out.push(death_play(ctx, cfg, round, side, k));
        } else if k.assister == Some(tracked) {
            out.push(play(
                k.tick,
                phase_of(ctx, cfg, round.number, k.tick),
                "assist",
                json!({
                    "victim": k.victim.to_string(),
                    "killer": k.attacker.map(|a| a.to_string()),
                    "flash_assist": k.assistedflash,
                }),
                None,
            ));
        }
    }
    out
}

fn kill_play(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
    k: &Kill,
) -> Play {
    let tracked = ctx.tracked();
    let z = cfg.general.z_weight;
    let me = ctx.state_at(tracked, k.tick);
    let victim = ctx.state_at(k.victim, k.tick);
    let distance = match (&me, &victim) {
        (Some(a), Some(b)) => Some(AnalysisContext::dist(a, b, z).round()),
        _ => None,
    };
    let team_kill = ctx.side_of(k.victim, round.number) == Some(side);
    play(
        k.tick,
        phase_of(ctx, cfg, round.number, k.tick),
        "kill",
        json!({
            "victim": k.victim.to_string(),
            "weapon": k.weapon,
            "headshot": k.headshot,
            "killer_distance": distance,
            "place": me.as_ref().and_then(|s| s.place.clone()),
            "victim_place": victim.as_ref().and_then(|s| s.place.clone()),
            "team_kill": team_kill,
            "thru_smoke": k.thru_smoke,
            "wallbang": k.penetrated > 0,
            "while_blind": k.attacker_blind,
            "man_context": man_context(ctx, round, side, k.tick - 1),
        }),
        if team_kill { Some(Quality::Bad) } else { None },
    )
}

fn death_play(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
    k: &Kill,
) -> Play {
    let tracked = ctx.tracked();
    let z = cfg.general.z_weight;
    let commit_w = ctx.seconds(cfg.trade.commit_window_s);
    let me = ctx.state_at(tracked, k.tick);
    let killer = k.attacker.filter(|a| *a != tracked);
    let killer_st = killer.and_then(|a| ctx.state_at(a, k.tick));
    let distance = match (&me, &killer_st) {
        (Some(a), Some(b)) => Some(AnalysisContext::dist(a, b, z).round()),
        _ => None,
    };
    let traded = killer.is_some_and(|a| killed_in(ctx, a, k.tick, k.tick + commit_w));
    let nearest = ctx.nearest_teammate(tracked, round.number, k.tick, z);
    play(
        k.tick,
        phase_of(ctx, cfg, round.number, k.tick),
        "death",
        json!({
            "victim": tracked.to_string(),
            "killer": killer.map(|a| a.to_string()),
            "weapon": k.weapon,
            "headshot": k.headshot,
            "killer_distance": distance,
            "place": me.as_ref().and_then(|s| s.place.clone()),
            "killer_place": killer_st.as_ref().and_then(|s| s.place.clone()),
            "traded": traded,
            "nearest_teammate": nearest.map(|(id, _)| id.to_string()),
            "nearest_teammate_dist": nearest.map(|(_, d)| d.round()),
            "man_context": man_context(ctx, round, side, k.tick - 1),
            "round_end_delta_s": secs_1dp(round.end_tick - k.tick, ctx.data().tickrate),
            "thru_smoke": k.thru_smoke,
            "wallbang": k.penetrated > 0,
        }),
        None, // finalized after flag merge (finalize_death_quality)
    )
}

fn bomb_plays(ctx: &AnalysisContext, cfg: &DetectorConfig, round: &Round) -> Vec<Play> {
    let tracked = ctx.tracked();
    let end = span_end(round);
    ctx.data()
        .bomb_events
        .iter()
        .filter(|b| b.player == Some(tracked) && b.tick >= round.start_tick && b.tick <= end)
        .filter_map(|b| {
            let kind = match b.kind.as_str() {
                "planted" => "plant",
                "defused" => "defuse",
                _ => return None,
            };
            let place = ctx.state_at(tracked, b.tick).and_then(|s| s.place);
            Some(play(
                b.tick,
                phase_of(ctx, cfg, round.number, b.tick),
                kind,
                json!({ "place": place }),
                None,
            ))
        })
        .collect()
}

fn outcome_play(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
) -> Option<Play> {
    let tracked = ctx.tracked();
    let tick = round.end_tick;
    let survived = ctx.kill_of(tracked, round.number).is_none();
    let (mine, theirs) = match side {
        Side::Ct => (&round.ct_steamids, &round.t_steamids),
        Side::T => (&round.t_steamids, &round.ct_steamids),
    };
    let my_alive = alive_in(ctx, round, mine, span_end(round));
    let their_alive = alive_in(ctx, round, theirs, span_end(round));
    let kills = ctx
        .data()
        .kills
        .iter()
        .filter(|k| {
            k.round == round.number
                && k.attacker == Some(tracked)
                && k.victim != tracked
                && ctx
                    .side_of(k.victim, round.number)
                    .is_some_and(|s| s != side)
        })
        .count();
    let damage: i32 = ctx
        .hurts_dealt_in(tracked, round.start_tick, span_end(round))
        .iter()
        .filter(|h| {
            ctx.side_of(h.victim, round.number)
                .is_some_and(|s| s != side)
        })
        .map(|h| h.dmg_health)
        .sum();
    Some(play(
        tick,
        phase_of(ctx, cfg, round.number, tick),
        "outcome",
        json!({
            "won": round.winner == side,
            "survived": survived,
            "reason": reason_str(&round.reason),
            "my_alive": my_alive,
            "their_alive": their_alive,
            "kills": kills,
            "damage": damage,
            "side": side_str(side),
        }),
        None,
    ))
}

// ---- utility plays --------------------------------------------------------

const FIRE_WEAPONS: &[&str] = &["inferno", "molotov", "incgrenade"];

fn ids(v: &[u64]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Enemy / team / self damage dealt by the tracked player with `weapons`
/// in [t0, t1], plus the enemy victims hit.
fn damage_split(
    ctx: &AnalysisContext,
    round: &Round,
    side: Side,
    t0: i32,
    t1: i32,
    weapons: &[&str],
) -> (i32, i32, i32, Vec<String>) {
    let tracked = ctx.tracked();
    let (mut enemy, mut team, mut me) = (0, 0, 0);
    let mut victims: Vec<String> = vec![];
    for h in ctx.hurts_dealt_in(tracked, t0, t1) {
        if h.dmg_health < 1 || !weapons.contains(&h.weapon.as_str()) {
            continue;
        }
        if h.victim == tracked {
            me += h.dmg_health;
            continue;
        }
        match ctx.side_of(h.victim, round.number) {
            Some(s) if s == side => team += h.dmg_health,
            Some(_) => {
                enemy += h.dmg_health;
                let id = h.victim.to_string();
                if !victims.contains(&id) {
                    victims.push(id);
                }
            }
            None => {}
        }
    }
    (enemy, team, me, victims)
}

fn utility_plays(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
) -> Vec<Play> {
    let tracked = ctx.tracked();
    let end = span_end(round);
    let join = ctx.seconds(cfg.ledger.flash_join_s);
    let groups: Vec<FlashGroup> = flash_groups(ctx, cfg)
        .into_iter()
        .filter(|g| g.round == round.number)
        .collect();
    let mut used: Vec<usize> = vec![];
    let mut out = vec![];
    let grenades = &ctx.data().grenades;
    for g in grenades
        .iter()
        .filter(|g| g.thrower == Some(tracked) && g.tick >= round.start_tick && g.tick <= end)
    {
        let phase = phase_of(ctx, cfg, round.number, g.tick);
        let place = ctx.state_at(tracked, g.tick).and_then(|s| s.place);
        match g.kind.as_str() {
            "flashbang" => {
                // Left-join the detonate to its blind group (spec §2: a
                // flash that blinded nobody is a dud, not invisible).
                let grp = groups
                    .iter()
                    .enumerate()
                    .filter(|(i, fg)| !used.contains(i) && (fg.tick - g.tick).abs() <= join)
                    .min_by_key(|(_, fg)| (fg.tick - g.tick).abs());
                let (enemies, mates, self_blind, converted) = match grp {
                    Some((i, fg)) => {
                        used.push(i);
                        (
                            fg.enemies_effective.clone(),
                            fg.teammates_blinded.clone(),
                            fg.self_blind,
                            fg.converted,
                        )
                    }
                    None => (vec![], vec![], false, false),
                };
                let quality = if !mates.is_empty() || self_blind {
                    Quality::Bad
                } else if !enemies.is_empty() {
                    Quality::Good
                } else {
                    Quality::Neutral
                };
                out.push(play(
                    g.tick,
                    phase,
                    "flash",
                    json!({
                        "enemies_blinded": enemies.len(),
                        "enemy_ids": ids(&enemies),
                        "teammates_blinded": mates.len(),
                        "teammate_ids": ids(&mates),
                        "self_blind": self_blind,
                        "converted": converted,
                        "place": place,
                        "x": g.x,
                        "y": g.y,
                    }),
                    Some(quality),
                ));
            }
            "smoke" => {
                let dead_time = g.tick > round.end_tick;
                let lifetime_s = grenades
                    .iter()
                    .find(|e| {
                        e.kind == "smoke_expired"
                            && e.thrower == Some(tracked)
                            && e.tick > g.tick
                            && (e.x - g.x).abs() < 1.0
                            && (e.y - g.y).abs() < 1.0
                    })
                    .map(|e| secs_1dp(e.tick - g.tick, ctx.data().tickrate));
                out.push(play(
                    g.tick,
                    phase,
                    "smoke",
                    json!({ "x": g.x, "y": g.y, "place": place, "dead_time": dead_time, "lifetime_s": lifetime_s }),
                    dead_time.then_some(Quality::Bad),
                ));
            }
            "he" => {
                let t1 = g.tick + ctx.seconds(cfg.ledger.he_window_s);
                let (enemy, team, me, victims) =
                    damage_split(ctx, round, side, g.tick, t1, &["hegrenade"]);
                out.push(play(
                    g.tick,
                    phase,
                    "he",
                    json!({ "enemy_damage": enemy, "team_damage": team, "self_damage": me, "victims": victims, "x": g.x, "y": g.y }),
                    (team > 0).then_some(Quality::Bad),
                ));
            }
            "molotov_start" => {
                let expire = grenades
                    .iter()
                    .find(|e| {
                        e.kind == "molotov_expire" && e.thrower == Some(tracked) && e.tick > g.tick
                    })
                    .map(|e| e.tick)
                    .unwrap_or(g.tick + ctx.seconds(cfg.ledger.molotov_burn_s));
                let (enemy, team, me, victims) =
                    damage_split(ctx, round, side, g.tick, expire, FIRE_WEAPONS);
                out.push(play(
                    g.tick,
                    phase,
                    "molotov",
                    json!({
                        "enemy_damage": enemy, "team_damage": team, "self_damage": me, "victims": victims,
                        "burn_s": secs_1dp(expire - g.tick, ctx.data().tickrate), "x": g.x, "y": g.y,
                    }),
                    (team > 0).then_some(Quality::Bad),
                ));
            }
            _ => {}
        }
    }
    out
}

// ---- trades, rushes, rotations -------------------------------------------

/// One play per teammate death within trade range of a living tracked
/// player: `trade` when the tracked player killed the killer inside the
/// commit window, else `missed_trade` — Bad only under H2_FAILED_TRADE's own
/// conditions (no commit AND the killer lived), Neutral otherwise.
fn trade_plays(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
) -> Vec<Play> {
    let tracked = ctx.tracked();
    let z = cfg.general.z_weight;
    let commit_w = ctx.seconds(cfg.trade.commit_window_s);
    let my_death = ctx.kill_of(tracked, round.number).map(|k| k.tick);
    let mut out = vec![];
    for k in ctx
        .data()
        .kills
        .iter()
        .filter(|k| k.round == round.number && k.victim != tracked)
    {
        if ctx.side_of(k.victim, round.number) != Some(side) {
            continue; // an enemy died, not a teammate
        }
        let Some(killer) = k.attacker else { continue };
        if killer == tracked {
            continue;
        }
        match ctx.side_of(killer, round.number) {
            Some(s) if s != side => {}
            _ => continue, // teamkill / unknown side: not trade spacing (class 14)
        }
        if my_death.is_some_and(|d| d <= k.tick) {
            continue; // already dead
        }
        let (Some(me), Some(mate)) = (
            ctx.state_at(tracked, k.tick),
            ctx.state_at(k.victim, k.tick),
        ) else {
            continue;
        };
        if !me.is_alive {
            continue;
        }
        let distance = AnalysisContext::dist(&me, &mate, z);
        if distance > cfg.trade.distance_u {
            continue;
        }
        let t1 = k.tick + commit_w;
        let traded_by_me = ctx.data().kills.iter().any(|t| {
            t.attacker == Some(tracked) && t.victim == killer && t.tick > k.tick && t.tick <= t1
        });
        let traded_by_team = killed_in(ctx, killer, k.tick, t1);
        let did_commit = committed(ctx, tracked, killer, k.tick, t1);
        let (kind, quality) = if traded_by_me {
            ("trade", Quality::Good)
        } else if did_commit || traded_by_team {
            ("missed_trade", Quality::Neutral)
        } else {
            ("missed_trade", Quality::Bad)
        };
        out.push(play(
            k.tick,
            phase_of(ctx, cfg, round.number, k.tick),
            kind,
            json!({
                "teammate": k.victim.to_string(),
                "killer": killer.to_string(),
                "distance": distance.round(),
                "committed": did_commit,
                "traded_by_me": traded_by_me,
                "traded_by_team": traded_by_team,
                "window_s": cfg.trade.commit_window_s,
            }),
            Some(quality),
        ));
    }
    out
}

/// First 1 s checkpoint inside the early-aggression window at which the
/// tracked player is ≥ `min_spawn_distance_u` (XY, as H11) from their
/// freeze-end position with no teammate within `trade.distance_u`. Moving
/// with the team is not a rush; a rush that died in the window is Bad.
fn rush_play(ctx: &AnalysisContext, cfg: &DetectorConfig, round: &Round) -> Option<Play> {
    let tracked = ctx.tracked();
    let freeze_end = round.freeze_end_tick?;
    let spawn = ctx.state_at(tracked, freeze_end)?;
    if spawn.tick < round.start_tick {
        return None;
    }
    let step = ctx.seconds(cfg.ledger.sample_step_s).max(1);
    let window_end = (freeze_end + ctx.seconds(cfg.timing.early_aggression_s)).min(round.end_tick);
    let mut t = freeze_end + step;
    while t <= window_end {
        let st = ctx.state_at(tracked, t)?;
        if !st.is_alive {
            return None;
        }
        let dx = st.x - spawn.x;
        let dy = st.y - spawn.y;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance >= cfg.timing.min_spawn_distance_u {
            let nearest = ctx.nearest_teammate(tracked, round.number, t, cfg.general.z_weight);
            if nearest.is_some_and(|(_, d)| d <= cfg.trade.distance_u) {
                return None;
            }
            let died_in_window = ctx
                .kill_of(tracked, round.number)
                .is_some_and(|k| k.tick <= window_end);
            return Some(play(
                t,
                phase_of(ctx, cfg, round.number, t),
                "rush",
                json!({
                    "seconds_in": secs_1dp(t - freeze_end, ctx.data().tickrate),
                    "distance": distance.round(),
                    "nearest_teammate": nearest.map(|(id, _)| id.to_string()),
                    "nearest_teammate_dist": nearest.map(|(_, d)| d.round()),
                    "died_in_window": died_in_window,
                    "place": st.place,
                }),
                Some(if died_in_window {
                    Quality::Bad
                } else {
                    Quality::Neutral
                }),
            ));
        }
        t += step;
    }
    None
}

/// CT only, when a plant happened and the tracked player was alive for it:
/// where they were relative to the planter's position (H11's site proxy)
/// and how long the rotation took, sampled at 1 s until the H11 deadline.
/// Bad only when H11_SLOW_ROTATION fired (its flag is absorbed here).
fn rotation_play(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    side: Side,
    flags: &[RuleFlag],
) -> Option<Play> {
    if side != Side::Ct {
        return None;
    }
    let tracked = ctx.tracked();
    let z = cfg.general.z_weight;
    let end = span_end(round);
    let plant = ctx
        .data()
        .bomb_events
        .iter()
        .find(|b| b.kind == "planted" && b.tick >= round.start_tick && b.tick <= end)?;
    let planter = plant.player?;
    let plant_pos = ctx.state_at(planter, plant.tick)?;
    let at_plant = ctx.state_at(tracked, plant.tick)?;
    if !at_plant.is_alive {
        return None;
    }
    let distance_at_plant = AnalysisContext::dist(&at_plant, &plant_pos, z);
    let at_site = distance_at_plant <= cfg.timing.rotate_radius_u;
    let step = ctx.seconds(cfg.ledger.sample_step_s).max(1);
    let deadline = (plant.tick + ctx.seconds(cfg.timing.rotate_max_s)).min(end);
    let mut arrived_s = at_site.then_some(0.0_f64);
    let mut died_before_arrival = false;
    if !at_site {
        let mut t = plant.tick + step;
        while t <= deadline {
            let Some(st) = ctx.state_at(tracked, t) else {
                break;
            };
            if !st.is_alive {
                died_before_arrival = true;
                break;
            }
            if AnalysisContext::dist(&st, &plant_pos, z) <= cfg.timing.rotate_radius_u {
                arrived_s = Some(secs_1dp(t - plant.tick, ctx.data().tickrate));
                break;
            }
            t += step;
        }
    }
    let slow = flags.iter().find(|f| {
        f.round == round.number && f.steamid == tracked && f.rule_id == "H11_SLOW_ROTATION"
    });
    let mut p = play(
        plant.tick,
        phase_of(ctx, cfg, round.number, plant.tick),
        "rotation",
        json!({
            "distance_at_plant": distance_at_plant.round(),
            "at_site": at_site,
            "arrived_s": arrived_s,
            "died_before_arrival": died_before_arrival,
            "deadline_s": cfg.timing.rotate_max_s,
            "planter": planter.to_string(),
            "place_at_plant": at_plant.place,
        }),
        slow.map(|_| Quality::Bad),
    );
    p.rule_id = slow.map(|f| f.rule_id.to_string());
    Some(p)
}

// ---- flags + finalization -------------------------------------------------

/// Layer the tracked player's flags for this round onto the plays: a flag on
/// a play's tick merges its `details` into `facts` (existing keys win) and
/// the highest-severity rule becomes `rule_id`; a flag on a bare tick
/// becomes a `flag` play (Bad, or Neutral when exculpatory) — the `outcome`
/// play (round end tick) never absorbs flags. A rule already
/// carried by some play in the round (Task 8's rotation) is not re-added.
fn merge_flags(
    ctx: &AnalysisContext,
    cfg: &DetectorConfig,
    round: &Round,
    tracked: u64,
    plays: &mut Vec<Play>,
    flags: &[RuleFlag],
) {
    let mut best: HashMap<i32, f32> = HashMap::new();
    for f in flags
        .iter()
        .filter(|f| f.round == round.number && f.steamid == tracked)
    {
        if plays
            .iter()
            .any(|p| p.rule_id.as_deref() == Some(f.rule_id))
        {
            continue;
        }
        let exculpatory = cfg.rbr.exculpatory_rules.iter().any(|e| e == f.rule_id);
        if let Some(p) = plays
            .iter_mut()
            .find(|p| p.tick == f.tick && p.kind != "outcome")
        {
            if let (Some(obj), Some(fobj)) = (p.facts.as_object_mut(), f.details.as_object()) {
                for (k, v) in fobj {
                    obj.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
            let b = best.entry(f.tick).or_insert(f32::MIN);
            if f.severity >= *b {
                *b = f.severity;
                p.rule_id = Some(f.rule_id.to_string());
                if p.kind == "flag" {
                    p.quality = Some(if exculpatory {
                        Quality::Neutral
                    } else {
                        Quality::Bad
                    });
                }
            }
        } else {
            let mut p = play(
                f.tick,
                phase_of(ctx, cfg, round.number, f.tick),
                "flag",
                f.details.clone(),
                Some(if exculpatory {
                    Quality::Neutral
                } else {
                    Quality::Bad
                }),
            );
            p.rule_id = Some(f.rule_id.to_string());
            plays.push(p);
            best.insert(f.tick, f.severity);
        }
    }
}

/// Spec §2: a death is Bad when a (non-exculpatory) rule fired on it,
/// Neutral when exculpatory or traded, ungraded for a fair duel.
fn finalize_death_quality(plays: &mut [Play], exculpatory: &[String]) {
    for p in plays.iter_mut().filter(|p| p.kind == "death") {
        let traded = p
            .facts
            .get("traded")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        p.quality = match &p.rule_id {
            Some(r) if exculpatory.iter().any(|e| e == r) => Some(Quality::Neutral),
            Some(_) => Some(Quality::Bad),
            None if traded => Some(Quality::Neutral),
            None => None,
        };
    }
}

// ---- timeline ---------------------------------------------------------------

fn timeline(ctx: &AnalysisContext, round: &Round) -> Vec<TimelineEvent> {
    let end = span_end(round);
    let side_of = |p: Option<u64>| {
        p.and_then(|p| ctx.side_of(p, round.number))
            .map(|s| side_str(s).to_string())
    };
    let mut out: Vec<TimelineEvent> = ctx
        .data()
        .kills
        .iter()
        .filter(|k| k.round == round.number)
        .map(|k| TimelineEvent {
            tick: k.tick,
            kind: "kill".to_string(),
            actor: k.attacker.map(|a| a.to_string()),
            subject: Some(k.victim.to_string()),
            side: side_of(k.attacker),
            weapon: Some(k.weapon.clone()),
        })
        .collect();
    out.extend(
        ctx.data()
            .bomb_events
            .iter()
            .filter(|b| b.tick >= round.start_tick && b.tick <= end)
            .map(|b| TimelineEvent {
                tick: b.tick,
                kind: match b.kind.as_str() {
                    "planted" => "plant",
                    "defused" => "defuse",
                    "exploded" => "explode",
                    other => other,
                }
                .to_string(),
                actor: b.player.map(|p| p.to_string()),
                subject: None,
                side: side_of(b.player),
                weapon: None,
            }),
    );
    out.sort_by_key(|e| e.tick);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use crate::types::EvidenceRef;
    use cf_parser::model::Side;

    const ME: u64 = 1;
    const MATE: u64 = 2;
    const ENEMY: u64 = 9;

    /// One round, ticks 1000..5000 (freeze_end == start in Scenario), me and
    /// a teammate on CT holding 500 u apart, one enemy far away.
    fn base() -> Scenario {
        Scenario::new("de_mirage")
            .players_ct(&[ME, MATE])
            .players_t(&[ENEMY])
            .round(1, 1000, 5000)
            .hold(ME, 1000, 5000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 5000, 500.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 5000, 4000.0, 0.0, 0.0)
    }

    fn ledger_for(data: &cf_parser::model::MatchData, flags: &[RuleFlag]) -> RoundLedger {
        let ctx = AnalysisContext::new(data, ME);
        let cfg = DetectorConfig::default();
        let mut out = build_ledger(&ctx, &cfg, flags);
        assert_eq!(out.len(), 1);
        out.remove(0)
    }

    fn find_play<'a>(l: &'a RoundLedger, kind: &str) -> &'a Play {
        l.plays
            .iter()
            .find(|p| p.kind == kind)
            .unwrap_or_else(|| panic!("no {kind} play in {:?}", l.plays))
    }

    fn flag(
        rule_id: &'static str,
        tick: i32,
        severity: f32,
        details: serde_json::Value,
    ) -> RuleFlag {
        RuleFlag {
            rule_id,
            round: 1,
            tick,
            steamid: ME,
            confidence: 0.9,
            severity,
            details,
            evidence: EvidenceRef {
                round: 1,
                tick_start: tick - 320,
                tick_end: tick + 128,
                focus_players: vec![ME],
                camera_hint: None,
            },
        }
    }

    #[test]
    fn setup_checkpoint_reads_positioning_five_seconds_in() {
        let data = base().build();
        let l = ledger_for(&data, &[]);
        let s = find_play(&l, "setup");
        assert_eq!(s.tick, 1000 + 320); // 5 s at 64 tick
        assert_eq!(s.phase, "opening");
        assert_eq!(s.facts["nearest_teammate"], "2");
        assert_eq!(s.facts["nearest_teammate_dist"], 500.0);
        assert_eq!(s.facts["teammates_within_isolation"], 1);
        assert_eq!(s.facts["teammates_alive"], 1);
        assert!(s.quality.is_none(), "positioning judgement is the coach's");
    }

    #[test]
    fn kill_play_carries_victim_distance_and_man_context() {
        let data = base().kill(ME, ENEMY, 1, 3000, "weapon_ak47").build();
        let l = ledger_for(&data, &[]);
        let k = find_play(&l, "kill");
        assert_eq!(k.tick, 3000);
        assert_eq!(k.facts["victim"], "9");
        assert_eq!(k.facts["killer_distance"], 4000.0);
        assert_eq!(k.facts["man_context"], "2v1");
        assert_eq!(k.facts["team_kill"], false);
        assert!(k.quality.is_none());
        let o = find_play(&l, "outcome");
        assert_eq!(o.facts["won"], true);
        assert_eq!(o.facts["survived"], true);
        assert_eq!(o.facts["kills"], 1);
        assert_eq!(o.facts["my_alive"], 2);
        assert_eq!(o.facts["their_alive"], 0);
    }

    #[test]
    fn traded_death_is_neutral_untraded_is_unjudged() {
        let traded = base()
            .kill(ENEMY, ME, 1, 3000, "weapon_ak47")
            .kill(MATE, ENEMY, 1, 3060, "weapon_ak47")
            .round_won_by(1, Side::T)
            .build();
        let l = ledger_for(&traded, &[]);
        let d = find_play(&l, "death");
        assert_eq!(d.facts["killer"], "9");
        assert_eq!(d.facts["traded"], true);
        assert_eq!(d.facts["killer_distance"], 4000.0);
        assert_eq!(d.facts["nearest_teammate"], "2");
        assert_eq!(d.facts["round_end_delta_s"], 31.3); // (5000-3000)/64, 1 dp
        assert_eq!(d.quality, Some(Quality::Neutral));
        assert!(
            d.facts.get("distance").is_none(),
            "`distance` belongs to H2's details"
        );

        let untraded = base().kill(ENEMY, ME, 1, 3000, "weapon_ak47").build();
        let l = ledger_for(&untraded, &[]);
        let d = find_play(&l, "death");
        assert_eq!(d.facts["traded"], false);
        assert!(d.quality.is_none(), "a fair, untraded duel is not graded");
        let o = find_play(&l, "outcome");
        assert_eq!(o.facts["survived"], false);
    }

    #[test]
    fn flags_merge_into_the_same_tick_play_without_clobbering() {
        let data = base().kill(ENEMY, ME, 1, 3000, "weapon_ak47").build();
        let flags = vec![
            flag(
                "H2_ISOLATED_DEATH",
                3000,
                0.8,
                serde_json::json!({"distance": 1223.0, "traded": "clobber?"}),
            ),
            flag(
                "H3_DIED_RELOADING",
                3000,
                0.6,
                serde_json::json!({"reload_s": 0.4}),
            ),
        ];
        let l = ledger_for(&data, &flags);
        let d = find_play(&l, "death");
        assert_eq!(
            d.rule_id.as_deref(),
            Some("H2_ISOLATED_DEATH"),
            "highest severity wins"
        );
        assert_eq!(d.facts["distance"], 1223.0, "flag details merge in");
        assert_eq!(d.facts["reload_s"], 0.4);
        assert_eq!(
            d.facts["traded"], false,
            "computed keys are never clobbered"
        );
        assert_eq!(d.quality, Some(Quality::Bad));
        assert_eq!(l.plays.iter().filter(|p| p.kind == "flag").count(), 0);
    }

    #[test]
    fn exculpatory_rule_makes_the_death_neutral_and_bare_flags_become_plays() {
        let data = base().kill(ENEMY, ME, 1, 3000, "weapon_ak47").build();
        let flags = vec![
            flag(
                "H2_BAITED_TRADE",
                3000,
                0.35,
                serde_json::json!({"non_follower": "2"}),
            ),
            flag(
                "H6_UNUSED_UTIL_AT_ROUND_END",
                5000,
                0.4,
                serde_json::json!({"held": ["Flashbang", "Smoke Grenade"]}),
            ),
        ];
        let l = ledger_for(&data, &flags);
        assert_eq!(find_play(&l, "death").quality, Some(Quality::Neutral));
        let f = find_play(&l, "flag");
        assert_eq!(f.tick, 5000);
        assert_eq!(f.rule_id.as_deref(), Some("H6_UNUSED_UTIL_AT_ROUND_END"));
        assert_eq!(f.quality, Some(Quality::Bad));
        assert_eq!(f.facts["held"][0], "Flashbang");
    }

    #[test]
    fn timeline_lists_everyones_kills_and_bomb_events_in_tick_order() {
        let data = base()
            .kill(MATE, ENEMY, 1, 2500, "weapon_m4a1")
            .bomb("planted", ENEMY, 2000)
            .build();
        let l = ledger_for(&data, &[]);
        assert_eq!(l.timeline.len(), 2);
        assert_eq!(l.timeline[0].kind, "plant");
        assert_eq!(l.timeline[0].actor.as_deref(), Some("9"));
        assert_eq!(l.timeline[0].side.as_deref(), Some("T"));
        assert_eq!(l.timeline[1].kind, "kill");
        assert_eq!(l.timeline[1].actor.as_deref(), Some("2"));
        assert_eq!(l.timeline[1].subject.as_deref(), Some("9"));
        assert_eq!(l.timeline[1].weapon.as_deref(), Some("weapon_m4a1"));
        // The teammate's kill is not MY play.
        assert!(l
            .plays
            .iter()
            .all(|p| p.kind != "kill" && p.kind != "death"));
    }

    #[test]
    fn plays_are_tick_ordered_and_a_round_without_the_tracked_player_is_skipped() {
        let data = base()
            .round(2, 6000, 9000)
            .kill(ME, ENEMY, 1, 3000, "weapon_ak47")
            .build();
        let ctx = AnalysisContext::new(&data, ME);
        let out = build_ledger(&ctx, &DetectorConfig::default(), &[]);
        assert_eq!(out.len(), 2);
        let ticks: Vec<i32> = out[0].plays.iter().map(|p| p.tick).collect();
        let mut sorted = ticks.clone();
        sorted.sort();
        assert_eq!(ticks, sorted);
        let spectator = Scenario::new("de_mirage")
            .players_ct(&[MATE])
            .players_t(&[ENEMY])
            .round(1, 1000, 5000)
            .build();
        let ctx = AnalysisContext::new(&spectator, ME);
        assert!(build_ledger(&ctx, &DetectorConfig::default(), &[]).is_empty());
    }

    #[test]
    fn flash_that_blinds_an_enemy_is_good_and_a_dud_is_neutral() {
        let data = base()
            .grenade("flashbang", ME, 2000, 300.0, 0.0)
            .blind(ME, ENEMY, 2002, 2.0)
            .grenade("flashbang", ME, 2600, 300.0, 0.0) // nobody blinded
            .build();
        let l = ledger_for(&data, &[]);
        let flashes: Vec<&Play> = l.plays.iter().filter(|p| p.kind == "flash").collect();
        assert_eq!(flashes.len(), 2);
        assert_eq!(flashes[0].tick, 2000);
        assert_eq!(flashes[0].facts["enemies_blinded"], 1);
        assert_eq!(flashes[0].facts["enemy_ids"][0], "9");
        assert_eq!(flashes[0].facts["teammates_blinded"], 0);
        assert_eq!(flashes[0].quality, Some(Quality::Good));
        assert_eq!(flashes[1].tick, 2600);
        assert_eq!(flashes[1].facts["enemies_blinded"], 0);
        assert_eq!(
            flashes[1].quality,
            Some(Quality::Neutral),
            "a dud is a fact, not invisible"
        );
    }

    #[test]
    fn team_or_self_flash_is_bad_and_conversion_is_recorded() {
        let data = base()
            .grenade("flashbang", ME, 2000, 300.0, 0.0)
            .blind(ME, MATE, 2000, 1.5)
            .blind(ME, ENEMY, 2000, 2.0)
            .kill(ME, ENEMY, 1, 2064, "weapon_ak47")
            .build();
        let l = ledger_for(&data, &[]);
        let f = find_play(&l, "flash");
        assert_eq!(f.facts["teammates_blinded"], 1);
        assert_eq!(f.facts["converted"], true);
        assert_eq!(
            f.quality,
            Some(Quality::Bad),
            "a team flash is bad even when it converts"
        );
    }

    #[test]
    fn smoke_after_the_round_is_decided_is_dead_time() {
        let data = base()
            .grenade("smoke", ME, 1500, 100.0, 100.0)
            .grenade("smoke_expired", ME, 1500 + 64 * 18, 100.0, 100.0)
            .grenade("smoke", ME, 5064, 900.0, 900.0) // end_tick 5000, officially 5128
            .build();
        let l = ledger_for(&data, &[]);
        let smokes: Vec<&Play> = l.plays.iter().filter(|p| p.kind == "smoke").collect();
        assert_eq!(smokes.len(), 2);
        assert_eq!(smokes[0].facts["dead_time"], false);
        assert_eq!(smokes[0].facts["lifetime_s"], 18.0);
        assert!(
            smokes[0].quality.is_none(),
            "a live smoke's worth is the coach's call"
        );
        assert_eq!(smokes[1].facts["dead_time"], true);
        assert_eq!(smokes[1].quality, Some(Quality::Bad));
    }

    #[test]
    fn he_and_molotov_damage_is_split_by_side() {
        let data = base()
            .grenade("he", ME, 2000, 3900.0, 0.0)
            .hurt(ME, ENEMY, 2001, 41, "hegrenade")
            .grenade("molotov_start", ME, 3000, 400.0, 0.0)
            .hurt(ME, MATE, 3010, 12, "inferno")
            .hurt(ME, ENEMY, 3040, 30, "inferno")
            .grenade("molotov_expire", ME, 3000 + 64 * 6, 400.0, 0.0)
            .build();
        let l = ledger_for(&data, &[]);
        let he = find_play(&l, "he");
        assert_eq!(he.facts["enemy_damage"], 41);
        assert_eq!(he.facts["team_damage"], 0);
        assert_eq!(he.facts["victims"][0], "9");
        assert!(he.quality.is_none(), "damage stands alone");
        let m = find_play(&l, "molotov");
        assert_eq!(m.facts["enemy_damage"], 30);
        assert_eq!(m.facts["team_damage"], 12);
        assert_eq!(m.facts["burn_s"], 6.0);
        assert_eq!(m.quality, Some(Quality::Bad));
    }

    #[test]
    fn trading_a_teammate_is_good_not_committing_is_bad_committing_is_neutral() {
        let good = base()
            .kill(ENEMY, MATE, 1, 3000, "weapon_ak47")
            .kill(ME, ENEMY, 1, 3060, "weapon_ak47")
            .build();
        let l = ledger_for(&good, &[]);
        let t = find_play(&l, "trade");
        assert_eq!(t.tick, 3000);
        assert_eq!(t.facts["teammate"], "2");
        assert_eq!(t.facts["killer"], "9");
        assert_eq!(t.facts["distance"], 500.0);
        assert_eq!(t.facts["traded_by_me"], true);
        assert_eq!(t.quality, Some(Quality::Good));

        let bad = base().kill(ENEMY, MATE, 1, 3000, "weapon_ak47").build();
        let l = ledger_for(&bad, &[]);
        let m = find_play(&l, "missed_trade");
        assert_eq!(m.facts["committed"], false);
        assert_eq!(m.quality, Some(Quality::Bad));

        let neutral = base()
            .kill(ENEMY, MATE, 1, 3000, "weapon_ak47")
            .shot(ME, 3020, "weapon_ak47")
            .build();
        let l = ledger_for(&neutral, &[]);
        let m = find_play(&l, "missed_trade");
        assert_eq!(m.facts["committed"], true);
        assert_eq!(m.quality, Some(Quality::Neutral));
    }

    #[test]
    fn a_teammate_dying_out_of_range_or_after_my_death_is_not_a_trade_situation() {
        let far = Scenario::new("de_mirage")
            .players_ct(&[ME, MATE])
            .players_t(&[ENEMY])
            .round(1, 1000, 5000)
            .hold(ME, 1000, 5000, 0.0, 0.0, 0.0)
            .hold(MATE, 1000, 5000, 2500.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 5000, 4000.0, 0.0, 0.0)
            .kill(ENEMY, MATE, 1, 3000, "weapon_ak47")
            .build();
        assert!(ledger_for(&far, &[])
            .plays
            .iter()
            .all(|p| p.kind != "missed_trade"));
        let dead_first = base()
            .kill(ENEMY, ME, 1, 2000, "weapon_ak47")
            .kill(ENEMY, MATE, 1, 3000, "weapon_ak47")
            .build();
        assert!(ledger_for(&dead_first, &[])
            .plays
            .iter()
            .all(|p| p.kind != "missed_trade"));
    }

    #[test]
    fn unsupported_early_push_is_a_rush_play() {
        let data = Scenario::new("de_mirage")
            .players_ct(&[MATE])
            .players_t(&[ME, ENEMY])
            .round(1, 1000, 5000)
            .waypoint(ME, 1000, 0.0, 0.0, 0.0)
            .waypoint(ME, 1320, 1200.0, 0.0, 0.0) // 1,200 u in 5 s
            .waypoint(ME, 5000, 1200.0, 0.0, 0.0)
            .hold(ENEMY, 1000, 5000, 5000.0, 5000.0, 0.0)
            .hold(MATE, 1000, 5000, -5000.0, -5000.0, 0.0)
            .build();
        let l = ledger_for(&data, &[]);
        let r = find_play(&l, "rush");
        assert_eq!(r.tick, 1000 + 64 * 4); // first 1 s checkpoint past 750 u
        assert_eq!(r.facts["seconds_in"], 4.0);
        assert_eq!(r.facts["distance"], 960.0);
        assert_eq!(r.facts["died_in_window"], false);
        assert_eq!(r.quality, Some(Quality::Neutral));
        let supported = base().build(); // holds still: no rush
        assert!(ledger_for(&supported, &[])
            .plays
            .iter()
            .all(|p| p.kind != "rush"));
    }

    #[test]
    fn ct_rotation_after_the_plant_records_arrival_time() {
        let data = Scenario::new("de_mirage")
            .players_ct(&[ME, MATE])
            .players_t(&[ENEMY])
            .round(1, 1000, 5000)
            .hold(ENEMY, 1000, 5000, 3000.0, 0.0, 0.0)
            .hold(MATE, 1000, 5000, 3100.0, 0.0, 0.0)
            .waypoint(ME, 1000, 0.0, 0.0, 0.0)
            .waypoint(ME, 2000, 0.0, 0.0, 0.0)
            .waypoint(ME, 2640, 2400.0, 0.0, 0.0) // within 800 u of the plant 10 s later
            .waypoint(ME, 5000, 2400.0, 0.0, 0.0)
            .bomb("planted", ENEMY, 2000)
            .round_won_by(1, Side::T)
            .build();
        let l = ledger_for(&data, &[]);
        let r = find_play(&l, "rotation");
        assert_eq!(r.tick, 2000);
        assert_eq!(r.facts["distance_at_plant"], 3000.0);
        assert_eq!(r.facts["at_site"], false);
        assert_eq!(r.facts["arrived_s"], 10.0);
        assert!(r.quality.is_none());
        let flags = vec![flag(
            "H11_SLOW_ROTATION",
            2000 + 64 * 25,
            0.5,
            serde_json::json!({"distance_at_plant": 3000.0}),
        )];
        let l = ledger_for(&data, &flags);
        let r = find_play(&l, "rotation");
        assert_eq!(r.quality, Some(Quality::Bad));
        assert_eq!(r.rule_id.as_deref(), Some("H11_SLOW_ROTATION"));
        assert_eq!(
            l.plays.iter().filter(|p| p.kind == "flag").count(),
            0,
            "not re-added as a bare flag"
        );
    }
}
