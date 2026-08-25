//! The rail's voice (issue #9; PROMPT.md §8; V1.2 charter). Turns Task 2's
//! structured `RoundReview`/`Moment` data into short, numbers-first
//! coaching text for the round-by-round rail. Hard rule: this lives here,
//! never in `templates.rs` — the number is the finding, the prose is only
//! its label.
//!
//! Voice: numbers before the label ("1,223 u away at Catwalk", not "you
//! were far from your teammate, 1,223 u away"); no exclamation marks;
//! `Cost you` copy never asserts fault — it states what happened and when,
//! not whose fault it was. Distances render via `fmt_units` (thousands
//! separator + " u"); win-prob deltas render via `fmt_delta_pct` (signed
//! whole percent). Every line is built from the moment's/review's own
//! facts, never a fixed template with blanks filled in: different facts
//! must read as genuinely different sentences (tested).
//!
//! H2_ISOLATED_DEATH's flag `details` don't carry a killer distance today
//! (no LOS/position math backs it) — a "Killer 818 u, from Middle" style
//! line is a V1.2+ addition once that data exists. Do not invent it here.

use cf_analysis::round_review::{Moment, RoundReview, Verdict};

use crate::callouts::callout_name;
use crate::MatchContext;

const ISOLATED: &str = "H2_ISOLATED_DEATH";
const FAILED_TRADE: &str = "H2_FAILED_TRADE";
const BAITED_TRADE: &str = "H2_BAITED_TRADE";
const UNSUPPORTED_ENTRY: &str = "H14_UNSUPPORTED_ENTRY";
const PUSH_WITHOUT_INFO: &str = "H6_PUSH_WITHOUT_INFO";
const EARLY_AGGRESSIVE: &str = "H11_EARLY_AGGRESSIVE_DEATH";
const FLASH_SELF_OR_TEAM: &str = "H6_FLASH_SELF_OR_TEAM";
const UNUSED_UTIL_AT_ROUND_END: &str = "H6_UNUSED_UTIL_AT_ROUND_END";
const DEAD_TIME_SMOKE: &str = "H6_DEAD_TIME_SMOKE";
const SLOW_ROTATION: &str = "H11_SLOW_ROTATION";
const FIRE_LINGER: &str = "H16_FIRE_LINGER";
const PRACTISE_RULES: &[&str] = &[
    ISOLATED,
    FAILED_TRADE,
    UNSUPPORTED_ENTRY,
    PUSH_WITHOUT_INFO,
    EARLY_AGGRESSIVE,
];

/// One narrated moment for the rail: a headline label plus numbers-first
/// fact lines, both derived from the moment's own `facts`.
#[derive(Debug, Clone, PartialEq)]
pub struct MomentText {
    pub clock_tick: i32,
    pub headline: String,
    pub facts: Vec<String>,
    pub rule_id: Option<String>,
}

/// Narrate one structured moment. The number is the finding; the headline
/// is only its label.
pub fn narrate_moment(m: &Moment, ctx: &MatchContext) -> MomentText {
    let (headline, facts) = match m.kind.as_str() {
        "tracked_kill" => (
            kill_headline(&m.rule_id, is_team_kill(m)),
            kill_facts(m, ctx),
        ),
        "tracked_death" => (death_headline(&m.rule_id), death_facts(m, ctx)),
        "plant" => ("Bomb planted".to_string(), delta_only_facts(m)),
        "defuse" => ("Defused".to_string(), delta_only_facts(m)),
        "flag" => (flag_headline(m.rule_id.as_deref()), flag_facts(m, ctx)),
        _ => ("Moment".to_string(), delta_only_facts(m)),
    };
    MomentText {
        clock_tick: m.tick,
        headline,
        facts,
        rule_id: m.rule_id.clone(),
    }
}

/// One line of round consequence, read from the review's own verdict and
/// moments — never a fixed template. `None` when there's nothing to say.
///
/// `CostYou`'s "you were the last event that mattered" and `WonIt`'s "you
/// closed it out" both assert a CAUSAL claim (this specific moment is what
/// decided the round) — so both are gated on that moment actually being the
/// round's `pivotal_tick`, not merely present. When it isn't, a neutral,
/// still numbers-first line reports what happened without the unsupported
/// claim (V1.2 final-review fix wave, finding #2). `Traded`'s claim is
/// already gated by `assign_verdict`'s own `impact > -pivotal_threshold_p`
/// condition, so it's unchanged here.
pub fn why_it_mattered(review: &RoundReview, ctx: &MatchContext) -> Option<String> {
    match review.verdict {
        Verdict::CostYou => {
            let death = review.moments.iter().find(|m| m.kind == "tracked_death")?;
            let secs = num(&death.facts, "round_end_delta_s")?.round() as i64;
            if death.tick == review.pivotal_tick {
                Some(format!(
                    "You were the last event that mattered: the round tipped {secs} s after \
                     your death."
                ))
            } else {
                let pct = fmt_delta_pct(death.delta_p?);
                Some(format!(
                    "Your death cost {pct} win probability; the round ended {secs} s later."
                ))
            }
        }
        Verdict::Traded => {
            let death = review.moments.iter().find(|m| m.kind == "tracked_death")?;
            let secs = num(&death.facts, "round_end_delta_s")?.round() as i64;
            Some(format!(
                "You died, but the trade landed: the round carried on {secs} s after without \
                 hinging on it."
            ))
        }
        Verdict::WonIt => {
            let pct = fmt_delta_pct(review.impact);
            let last_kill = review
                .moments
                .iter()
                .rev()
                .find(|m| m.kind == "tracked_kill");
            let closed_it_out = last_kill.is_some_and(|m| m.tick == review.pivotal_tick);
            let victim = last_kill
                .filter(|_| closed_it_out)
                .and_then(|m| name_of(&m.facts, "victim", ctx));
            Some(match victim {
                Some(name) => {
                    format!("You closed it out on {name}: {pct} win probability, and it held.")
                }
                None => format!("You swung this one: {pct} win probability, and it held."),
            })
        }
        Verdict::NotOnYou | Verdict::Quiet => None,
    }
}

/// One actionable line, only when a rule backs it — the review's earliest
/// moment (tick order) whose `rule_id` names one of the covered rules.
/// Unknown or absent rule → `None`.
pub fn what_to_practise(review: &RoundReview, ctx: &MatchContext) -> Option<String> {
    let moment = review.moments.iter().find(|m| {
        m.rule_id
            .as_deref()
            .is_some_and(|r| PRACTISE_RULES.contains(&r))
    })?;
    match moment.rule_id.as_deref()? {
        ISOLATED => {
            let place = text(&moment.facts, "place")?;
            Some(format!(
                "Before you take a fight at {}, know who is close enough to trade you.",
                callout_name(&place)
            ))
        }
        FAILED_TRADE => {
            let teammate = name_of(&moment.facts, "teammate", ctx)?;
            Some(format!(
                "You were in trade range when {teammate} died — move on the sound, not after it."
            ))
        }
        UNSUPPORTED_ENTRY => {
            let opponent = name_of(&moment.facts, "opponent", ctx);
            Some(match opponent {
                Some(name) => format!(
                    "You took that entry on {name} alone — get the flash or the second man on \
                     you before you commit."
                ),
                None => "Don't take that entry alone — get the flash or the second man on you \
                          before you commit."
                    .to_string(),
            })
        }
        PUSH_WITHOUT_INFO => {
            let dist = num(&moment.facts, "distance_from_spawn")?;
            Some(format!(
                "You pushed {} from spawn with no read on the site — wait for a call before \
                 committing.",
                fmt_units(dist)
            ))
        }
        EARLY_AGGRESSIVE => {
            let secs = num(&moment.facts, "seconds_in")?.round() as i64;
            let dist = num(&moment.facts, "distance_from_spawn")?;
            Some(format!(
                "You died {secs} s into the round, {} from spawn with nobody close enough to \
                 trade — take a slower entry or bring a teammate with you.",
                fmt_units(dist)
            ))
        }
        _ => None,
    }
}

/// UI chip label for a round's verdict.
pub fn verdict_label(v: Verdict) -> &'static str {
    match v {
        Verdict::WonIt => "Won it",
        Verdict::CostYou => "Cost you",
        Verdict::NotOnYou => "Not on you",
        Verdict::Traded => "Traded",
        Verdict::Quiet => "Quiet",
    }
}

// ---- headline builders --------------------------------------------------

/// A teamkill always reads "Teamkill" regardless of any rule tag — labeling
/// it as an "Opening pick"/"Kill" would misreport what actually happened
/// (V1.2 final-review fix wave, minor #6).
fn kill_headline(rule_id: &Option<String>, team_kill: bool) -> String {
    if team_kill {
        "Teamkill".to_string()
    } else if rule_id.as_deref() == Some(UNSUPPORTED_ENTRY) {
        "Opening pick".to_string()
    } else {
        "Kill".to_string()
    }
}

/// Whether a moment's own facts mark it as a teamkill (`round_review`'s
/// `build_moments` stamps `"team_kill"` on every `tracked_kill` moment).
fn is_team_kill(m: &Moment) -> bool {
    m.facts
        .get("team_kill")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn death_headline(rule_id: &Option<String>) -> String {
    match rule_id.as_deref() {
        Some(ISOLATED) => "Died isolated",
        Some(BAITED_TRADE) => "Traded in alone",
        Some(UNSUPPORTED_ENTRY) => "Lost the entry",
        Some(EARLY_AGGRESSIVE) => "Died pushing early",
        Some(PUSH_WITHOUT_INFO) => "Pushed blind",
        _ => "Death",
    }
    .to_string()
}

/// H2_BAITED_TRADE is always death-anchored (h2.rs emits it at the tracked
/// player's own death tick), so `build_moments` always absorbs it into a
/// `tracked_death` moment — it never survives as a standalone `flag` kind.
/// This arm is defensive only; `flag_facts`' generic fallback covers it if
/// that ever changes.
fn flag_headline(rule_id: Option<&str>) -> String {
    match rule_id {
        Some(FAILED_TRADE) => "Missed trade".to_string(),
        Some(UNSUPPORTED_ENTRY) => "Unsupported entry".to_string(),
        Some(EARLY_AGGRESSIVE) => "Early aggression".to_string(),
        Some(PUSH_WITHOUT_INFO) => "Pushed blind".to_string(),
        Some(other) => humanize_rule(other),
        None => "Flagged".to_string(),
    }
}

/// Generic fallback: "H11_LATE_ROTATION" -> "Late Rotation".
fn humanize_rule(id: &str) -> String {
    let rest = id.split_once('_').map_or(id, |(_, rest)| rest);
    rest.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &w[first.len_utf8()..].to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---- fact-line builders ---------------------------------------------------

fn kill_facts(m: &Moment, ctx: &MatchContext) -> Vec<String> {
    let mut out = vec![];
    let team_kill = is_team_kill(m);
    if let Some(name) = name_of(&m.facts, "victim", ctx) {
        out.push(if team_kill {
            format!("{name} down — your side")
        } else {
            format!("{name} down")
        });
    }
    if let Some(d) = m.delta_p {
        out.push(format!("{} win probability", fmt_delta_pct(d)));
    }
    out
}

fn death_facts(m: &Moment, ctx: &MatchContext) -> Vec<String> {
    let mut out = vec![];
    if let Some(name) = name_of(&m.facts, "nearest_teammate", ctx) {
        out.push(format!("Nearest: {name}"));
    }
    match (num(&m.facts, "distance"), text(&m.facts, "place")) {
        (Some(d), Some(p)) => out.push(format!("{} away at {}", fmt_units(d), callout_name(&p))),
        (Some(d), None) => out.push(format!("{} away", fmt_units(d))),
        (None, Some(p)) => out.push(format!("At {}", callout_name(&p))),
        (None, None) => {}
    }
    if let Some(spawn_dist) = num(&m.facts, "distance_from_spawn") {
        out.push(match num(&m.facts, "seconds_in") {
            Some(s) => format!(
                "{} s in, {} from spawn",
                s.round() as i64,
                fmt_units(spawn_dist)
            ),
            None => format!("{} from spawn", fmt_units(spawn_dist)),
        });
    }
    // H2_BAITED_TRADE's schema (always merged in here — see flag_headline's
    // comment). The spec's own requirement: name the teammate who didn't
    // follow, numbers-first, so this reads as "third man in a two-man
    // fight" rather than blame.
    if let (Some(nf), Some(dist)) = (
        name_of(&m.facts, "non_following_teammate", ctx),
        num(&m.facts, "their_distance"),
    ) {
        out.push(match name_of(&m.facts, "dead_teammate", ctx) {
            Some(dead) => format!(
                "{nf} {} back when {dead} went down — never in trade range",
                fmt_units(dist)
            ),
            None => format!("{nf} {} back — never in trade range", fmt_units(dist)),
        });
    }
    if let Some(traded) = m.facts.get("traded").and_then(|v| v.as_bool()) {
        let secs = num(&m.facts, "round_end_delta_s")
            .map(|s| s.round() as i64)
            .unwrap_or(0);
        // Outcome-neutral on purpose: every round is narrated now, won ones
        // included. A death in the post-decision tail carries `dead_time`
        // (ledger + rbr-v3 reviews); rows persisted before the clamp still
        // carry negative seconds, which mean the same thing. 0 s otherwise
        // is the deciding death — the last one standing going down ends
        // the round on that tick.
        let dead_time = m
            .facts
            .get("dead_time")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || secs < 0;
        let prefix = if traded { "Traded" } else { "Not traded" };
        out.push(if dead_time {
            format!("{prefix} — after the round was decided")
        } else if secs == 0 {
            format!("{prefix} — round ended with your death")
        } else if traded {
            format!("Traded — round continued {secs} s after")
        } else {
            format!("Not traded — round ended {secs} s later")
        });
    }
    out
}

/// Standalone flag moments each carry a rule-specific `details` schema
/// (h2.rs/flash_util.rs/h11_timing.rs/h16.rs) — dispatch per rule so every
/// reachable schema renders its own numbers, rather than falling through a
/// generic teammate/distance shape that most of these rules don't have
/// (V1.2 final-review fix wave, finding #4). Any arm that legitimately has
/// nothing to say returns `vec![]`; `commands.rs` suppresses the whole
/// moment when that happens rather than showing a bare label.
fn flag_facts(m: &Moment, ctx: &MatchContext) -> Vec<String> {
    match m.rule_id.as_deref() {
        Some(FLASH_SELF_OR_TEAM) => flash_self_or_team_facts(m, ctx),
        Some(UNUSED_UTIL_AT_ROUND_END) => unused_util_facts(m),
        Some(FIRE_LINGER) => fire_linger_facts(m),
        Some(SLOW_ROTATION) => slow_rotation_facts(m),
        // H6_DEAD_TIME_SMOKE's schema (flash_util.rs's `dead_time_smokes`)
        // carries only the round number, already implied by which round's
        // rail this renders on — nothing left to narrate. Explicit empty
        // arm (not the generic fallback) so the omission reads as a
        // deliberate schema check, not an oversight.
        Some(DEAD_TIME_SMOKE) => vec![],
        _ => generic_flag_facts(m, ctx),
    }
}

/// Fallback for standalone flags without a dedicated arm above — covers
/// `H2_FAILED_TRADE`'s `{teammate, killer, distance}` shape.
fn generic_flag_facts(m: &Moment, ctx: &MatchContext) -> Vec<String> {
    let mut out = vec![];
    if let Some(name) = name_of(&m.facts, "teammate", ctx) {
        out.push(format!("Teammate: {name}"));
    }
    if let Some(d) = num(&m.facts, "distance") {
        out.push(format!("{} away", fmt_units(d)));
    }
    out
}

/// `H6_FLASH_SELF_OR_TEAM`'s schema: `{teammates_blinded: count, victims:
/// [steamid strings — teammates, plus tracked's own id when tracked also
/// self-blinded]}` (`flash_util.rs::flash_self_or_team`).
fn flash_self_or_team_facts(m: &Moment, ctx: &MatchContext) -> Vec<String> {
    let victims: Vec<String> = m
        .facts
        .get("victims")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|raw| match raw.parse::<u64>() {
                    Ok(id) if id == ctx.tracked => "yourself".to_string(),
                    Ok(id) => ctx.name(id),
                    Err(_) => raw.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    if victims.is_empty() {
        return vec![];
    }
    vec![format!("{} blinded: {}", victims.len(), victims.join(", "))]
}

/// `H6_UNUSED_UTIL_AT_ROUND_END`'s schema: `{round, held: [item name
/// strings]}` (`flash_util.rs::unused_util`).
fn unused_util_facts(m: &Moment) -> Vec<String> {
    let held: Vec<String> = m
        .facts
        .get("held")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if held.is_empty() {
        return vec![];
    }
    vec![format!("{} held: {}", held.len(), held.join(", "))]
}

/// `H16_FIRE_LINGER`'s schema: `{total_damage, duration_s, round}`
/// (`h16.rs::detect_fire_linger`).
fn fire_linger_facts(m: &Moment) -> Vec<String> {
    match (num(&m.facts, "total_damage"), num(&m.facts, "duration_s")) {
        (Some(dmg), Some(secs)) => {
            vec![format!(
                "{} dmg, {:.1} s in the fire",
                dmg.round() as i64,
                secs
            )]
        }
        _ => vec![],
    }
}

/// `H11_SLOW_ROTATION`'s schema: `{seconds_late_or_never: null — not
/// computed today, distance_at_plant}` (`h11_timing.rs::detect_slow_rotation`)
/// — only `distance_at_plant` is ever populated; do not invent the other.
fn slow_rotation_facts(m: &Moment) -> Vec<String> {
    match num(&m.facts, "distance_at_plant") {
        Some(d) => vec![format!("{} from the site at the plant", fmt_units(d))],
        None => vec![],
    }
}

fn delta_only_facts(m: &Moment) -> Vec<String> {
    m.delta_p
        .map(|d| vec![format!("{} win probability", fmt_delta_pct(d))])
        .unwrap_or_default()
}

// ---- fact-value helpers ---------------------------------------------------

pub(crate) fn num(facts: &serde_json::Value, key: &str) -> Option<f32> {
    facts.get(key)?.as_f64().map(|v| v as f32)
}

pub(crate) fn text(facts: &serde_json::Value, key: &str) -> Option<String> {
    facts.get(key)?.as_str().map(str::to_string)
}

/// Resolve a facts steamid string (facts carry steamids as JSON strings) to
/// a display name; falls back to the raw string if it isn't numeric.
pub(crate) fn name_of(facts: &serde_json::Value, key: &str, ctx: &MatchContext) -> Option<String> {
    let raw = facts.get(key)?.as_str()?;
    Some(match raw.parse::<u64>() {
        Ok(id) => ctx.name(id),
        Err(_) => raw.to_string(),
    })
}

/// Thousands-separated distance: "1,223 u".
pub(crate) fn fmt_units(v: f32) -> String {
    format!("{} u", group_thousands(v.round() as i64))
}

fn group_thousands(n: i64) -> String {
    let negative = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let grouped: String = grouped.chars().rev().collect();
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

/// Signed whole percent: 0.19 -> "+19%", -0.23 -> "-23%".
pub(crate) fn fmt_delta_pct(d: f32) -> String {
    let pct = (d * 100.0).round() as i64;
    if pct >= 0 {
        format!("+{pct}%")
    } else {
        format!("{pct}%")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_analysis::round_review::{Attention, RoundHeader};
    use serde_json::json;
    use std::collections::HashMap;

    const TAKENOUCHI: u64 = 76561198000000002;
    const UNCLE_BUBBLES: u64 = 76561198000000007;
    const VICTIM8: u64 = 76561198000000008;
    const KANAE: u64 = 76561198000000009;

    fn ctx() -> MatchContext {
        MatchContext {
            map: "de_mirage".to_string(),
            tracked: 76561199228328773,
            names: HashMap::from([
                (TAKENOUCHI, "Takenouchi".to_string()),
                (UNCLE_BUBBLES, "UncleBubbles".to_string()),
                (VICTIM8, "UncleBubbles".to_string()),
                (KANAE, "Kanae".to_string()),
            ]),
            score: (7, 13),
            tracked_result: Some("loss".to_string()),
            total_deaths: 19,
            class_13_share_pct: 31.6,
        }
    }

    fn header(won: bool) -> RoundHeader {
        RoundHeader {
            side: "CT".to_string(),
            won,
            kills: 0,
            deaths: 1,
            man_context: Some("2v3".to_string()),
        }
    }

    fn review(verdict: Verdict, impact: f32, moments: Vec<Moment>) -> RoundReview {
        RoundReview {
            round: 11,
            impact,
            verdict,
            attention: Attention::Bright,
            selected: true,
            pivotal_tick: moments.first().map(|m| m.tick).unwrap_or(0),
            header: header(matches!(verdict, Verdict::WonIt)),
            moments,
        }
    }

    #[test]
    fn isolated_death_moment_reads_like_the_mockup() {
        let m = Moment {
            tick: 128_000,
            kind: "tracked_death".to_string(),
            rule_id: Some("H2_ISOLATED_DEATH".to_string()),
            delta_p: Some(-0.22),
            facts: json!({
                "nearest_teammate": TAKENOUCHI.to_string(),
                "distance": 1223.4,
                "place": "Catwalk",
                "killer": UNCLE_BUBBLES.to_string(),
                "traded": false,
                "round_end_delta_s": 6.2,
            }),
        };
        let t = narrate_moment(&m, &ctx());
        assert_eq!(t.headline, "Died isolated");
        assert_eq!(
            t.facts,
            vec![
                "Nearest: Takenouchi".to_string(),
                "1,223 u away at Catwalk".to_string(),
                "Not traded — round ended 6 s later".to_string(),
            ]
        );
        assert_eq!(t.clock_tick, 128_000);
        assert_eq!(t.rule_id.as_deref(), Some("H2_ISOLATED_DEATH"));
    }

    /// V1.2b final-review fix wave, #3/#5: the traded line is outcome-
    /// neutral (it shows on won rounds too), a tail death reads "after the
    /// round was decided" — via `dead_time`, or via the negative seconds a
    /// pre-clamp stored row still carries — and the deciding death (0 s,
    /// no `dead_time`) says so instead of "0 s later".
    #[test]
    fn traded_line_is_outcome_neutral_and_never_negative_seconds() {
        let death = |facts: serde_json::Value| Moment {
            tick: 64_000,
            kind: "tracked_death".to_string(),
            rule_id: None,
            delta_p: None,
            facts,
        };
        let line = |m: &Moment| narrate_moment(m, &ctx()).facts.last().cloned().unwrap();

        assert_eq!(
            line(&death(json!({ "traded": true, "round_end_delta_s": 12.4 }))),
            "Traded — round continued 12 s after"
        );
        assert_eq!(
            line(&death(json!({ "traded": false, "round_end_delta_s": 9.0 }))),
            "Not traded — round ended 9 s later"
        );
        assert_eq!(
            line(&death(
                json!({ "traded": false, "round_end_delta_s": 0.0, "dead_time": true })
            )),
            "Not traded — after the round was decided"
        );
        assert_eq!(
            line(&death(
                json!({ "traded": true, "round_end_delta_s": 0.0, "dead_time": true })
            )),
            "Traded — after the round was decided"
        );
        assert_eq!(
            line(&death(
                json!({ "traded": false, "round_end_delta_s": -1.4 })
            )),
            "Not traded — after the round was decided",
            "a row stored before the clamp must not read \"-1 s later\""
        );
        assert_eq!(
            line(&death(
                json!({ "traded": false, "round_end_delta_s": 0.0, "dead_time": false })
            )),
            "Not traded — round ended with your death"
        );
        assert_eq!(
            line(&death(json!({ "traded": false, "round_end_delta_s": 0.0 }))),
            "Not traded — round ended with your death",
            "no dead_time key at all (older ledger rows) is the deciding death"
        );
    }

    #[test]
    fn tracked_kill_moment() {
        let base_facts = json!({ "victim": VICTIM8.to_string() });
        let opening = Moment {
            tick: 1000,
            kind: "tracked_kill".to_string(),
            rule_id: Some("H14_UNSUPPORTED_ENTRY".to_string()),
            delta_p: Some(0.19),
            facts: base_facts.clone(),
        };
        let plain = Moment {
            tick: 50_000,
            kind: "tracked_kill".to_string(),
            rule_id: None,
            delta_p: Some(0.19),
            facts: base_facts,
        };
        let c = ctx();
        let opening_text = narrate_moment(&opening, &c);
        let plain_text = narrate_moment(&plain, &c);

        assert_eq!(opening_text.headline, "Opening pick");
        assert_eq!(plain_text.headline, "Kill");
        for t in [&opening_text, &plain_text] {
            assert_eq!(
                t.facts,
                vec![
                    "UncleBubbles down".to_string(),
                    "+19% win probability".to_string(),
                ]
            );
        }
    }

    #[test]
    fn practise_only_when_rule_backed() {
        let with_rule = review(
            Verdict::CostYou,
            -0.3,
            vec![Moment {
                tick: 500,
                kind: "tracked_death".to_string(),
                rule_id: Some("H2_ISOLATED_DEATH".to_string()),
                delta_p: Some(-0.3),
                facts: json!({ "place": "Catwalk" }),
            }],
        );
        let advice = what_to_practise(&with_rule, &ctx()).expect("rule-backed advice");
        assert!(advice.contains("Catwalk"), "{advice}");
        assert!(!advice.contains('!'), "{advice}");

        let without_rule = review(
            Verdict::Quiet,
            0.0,
            vec![Moment {
                tick: 500,
                kind: "tracked_death".to_string(),
                rule_id: None,
                delta_p: Some(-0.1),
                facts: json!({}),
            }],
        );
        assert!(what_to_practise(&without_rule, &ctx()).is_none());
    }

    #[test]
    fn practise_lines_are_rule_specific_and_data_driven() {
        let c = ctx();
        let cases: Vec<(&str, serde_json::Value, &str)> = vec![
            (
                FAILED_TRADE,
                json!({ "teammate": TAKENOUCHI.to_string(), "killer": UNCLE_BUBBLES.to_string(), "distance": 300.0 }),
                "Takenouchi",
            ),
            (
                UNSUPPORTED_ENTRY,
                json!({ "won": true, "opponent": UNCLE_BUBBLES.to_string() }),
                "UncleBubbles",
            ),
            (
                PUSH_WITHOUT_INFO,
                json!({ "seconds_in": 12.0, "distance_from_spawn": 900.0 }),
                "900 u",
            ),
            (
                EARLY_AGGRESSIVE,
                json!({ "seconds_in": 8.0, "distance_from_spawn": 750.0 }),
                "750 u",
            ),
        ];
        for (rule, facts, expect_contains) in cases {
            let r = review(
                Verdict::CostYou,
                -0.2,
                vec![Moment {
                    tick: 1,
                    kind: "tracked_death".to_string(),
                    rule_id: Some(rule.to_string()),
                    delta_p: Some(-0.2),
                    facts,
                }],
            );
            let advice =
                what_to_practise(&r, &c).unwrap_or_else(|| panic!("{rule} must produce advice"));
            assert!(advice.contains(expect_contains), "{rule}: {advice}");
            assert!(!advice.contains('!'), "{rule}: {advice}");
        }
    }

    #[test]
    fn verdict_labels_exact() {
        assert_eq!(verdict_label(Verdict::WonIt), "Won it");
        assert_eq!(verdict_label(Verdict::CostYou), "Cost you");
        assert_eq!(verdict_label(Verdict::NotOnYou), "Not on you");
        assert_eq!(verdict_label(Verdict::Traded), "Traded");
        assert_eq!(verdict_label(Verdict::Quiet), "Quiet");
    }

    #[test]
    fn why_it_mattered_from_stream() {
        let quick = review(
            Verdict::CostYou,
            -0.4,
            vec![Moment {
                tick: 1000,
                kind: "tracked_death".to_string(),
                rule_id: None,
                delta_p: Some(-0.4),
                facts: json!({ "traded": false, "round_end_delta_s": 3.0 }),
            }],
        );
        let slow = review(
            Verdict::CostYou,
            -0.4,
            vec![Moment {
                tick: 1000,
                kind: "tracked_death".to_string(),
                rule_id: None,
                delta_p: Some(-0.4),
                facts: json!({ "traded": false, "round_end_delta_s": 11.0 }),
            }],
        );
        let a = why_it_mattered(&quick, &ctx()).expect("cost-you review must have a line");
        let b = why_it_mattered(&slow, &ctx()).expect("cost-you review must have a line");
        assert_ne!(a, b, "different facts must not produce the same sentence");
        assert!(a.contains("3 s"), "{a}");
        assert!(b.contains("11 s"), "{b}");

        let quiet = review(Verdict::Quiet, 0.0, vec![]);
        assert!(why_it_mattered(&quiet, &ctx()).is_none());
    }

    #[test]
    fn why_it_mattered_won_it_names_the_last_kill() {
        let won = review(
            Verdict::WonIt,
            0.42,
            vec![Moment {
                tick: 2000,
                kind: "tracked_kill".to_string(),
                rule_id: None,
                delta_p: Some(0.42),
                facts: json!({ "victim": VICTIM8.to_string() }),
            }],
        );
        let line = why_it_mattered(&won, &ctx()).expect("won-it review must have a line");
        assert!(line.contains("UncleBubbles"), "{line}");
        assert!(line.contains("+42%"), "{line}");
    }

    /// V1.2 final-review fix wave, finding #2: `CostYou`'s "last event that
    /// mattered" is a causal claim, so it must render only when the death
    /// actually WAS the round's pivotal event (`review.pivotal_tick`); an
    /// ungated death gets the neutral, still numbers-first line instead.
    /// Exact strings for both branches, same facts, only `pivotal_tick`
    /// differs — isolating exactly what the gate changed.
    #[test]
    fn why_it_mattered_cost_you_exact_both_branches() {
        let death = Moment {
            tick: 1000,
            kind: "tracked_death".to_string(),
            rule_id: None,
            delta_p: Some(-0.22),
            facts: json!({ "traded": false, "round_end_delta_s": 6.0 }),
        };

        // Branch 1: the death WAS the round's pivotal event.
        let pivotal = review(Verdict::CostYou, -0.22, vec![death.clone()]);
        assert_eq!(
            pivotal.pivotal_tick, 1000,
            "review() pins pivotal_tick to the only moment's tick"
        );
        assert_eq!(
            why_it_mattered(&pivotal, &ctx()).unwrap(),
            "You were the last event that mattered: the round tipped 6 s after your death."
        );

        // Branch 2: some OTHER event was the round's actual pivotal moment.
        let mut not_pivotal = review(Verdict::CostYou, -0.22, vec![death]);
        not_pivotal.pivotal_tick = 5000;
        assert_eq!(
            why_it_mattered(&not_pivotal, &ctx()).unwrap(),
            "Your death cost -22% win probability; the round ended 6 s later."
        );
    }

    /// Same gate, `WonIt` side: "closed it out on X" only when that kill was
    /// the round's pivotal event; otherwise the existing neutral fallback.
    #[test]
    fn why_it_mattered_won_it_ungated_when_kill_not_pivotal() {
        let kill = Moment {
            tick: 2000,
            kind: "tracked_kill".to_string(),
            rule_id: None,
            delta_p: Some(0.42),
            facts: json!({ "victim": VICTIM8.to_string() }),
        };
        let mut r = review(Verdict::WonIt, 0.42, vec![kill]);
        r.pivotal_tick = 500; // some earlier event actually decided the round
        let line = why_it_mattered(&r, &ctx()).unwrap();
        assert_eq!(
            line,
            "You swung this one: +42% win probability, and it held."
        );
        assert!(
            !line.contains("UncleBubbles"),
            "must not name a kill that wasn't the round's pivotal event: {line}"
        );
    }

    #[test]
    fn plant_and_defuse_and_flag_kinds_do_not_panic() {
        let c = ctx();
        let plant = Moment {
            tick: 10,
            kind: "plant".to_string(),
            rule_id: None,
            delta_p: Some(0.05),
            facts: json!({}),
        };
        let defuse = Moment {
            tick: 20,
            kind: "defuse".to_string(),
            rule_id: None,
            delta_p: Some(0.5),
            facts: json!({}),
        };
        // H2_FAILED_TRADE genuinely stays standalone (teammate-death-anchored,
        // not tracked-death-anchored — see h2.rs:251-264), so this exercises
        // the real standalone-flag path. H2_BAITED_TRADE is NOT used here:
        // it's always death-anchored and always merges into a tracked_death
        // moment (see the real-shape test below), so a standalone-flag
        // H2_BAITED_TRADE moment never occurs in production.
        let flag = Moment {
            tick: 30,
            kind: "flag".to_string(),
            rule_id: Some("H2_FAILED_TRADE".to_string()),
            delta_p: None,
            facts: json!({ "teammate": TAKENOUCHI.to_string(), "distance": 300.0 }),
        };
        assert_eq!(narrate_moment(&plant, &c).headline, "Bomb planted");
        assert_eq!(narrate_moment(&defuse, &c).headline, "Defused");
        assert_eq!(narrate_moment(&flag, &c).headline, "Missed trade");
    }

    /// Real-shape regression: H2_BAITED_TRADE is emitted death-anchored
    /// (h2.rs:303-322, tick = tracked's own death tick), so `build_moments`
    /// always merges it into the `tracked_death` moment, never a standalone
    /// `flag`. The merged facts carry the computed death facts (killer,
    /// traded, round_end_delta_s) alongside the flag's own
    /// non_following_teammate/their_distance/dead_teammate — this must
    /// render a headline and a fact line naming the non-follower, not the
    /// generic "Death" fallback.
    #[test]
    fn baited_trade_merges_into_the_death_moment_and_names_the_non_follower() {
        let m = Moment {
            tick: 64_000,
            kind: "tracked_death".to_string(),
            rule_id: Some("H2_BAITED_TRADE".to_string()),
            delta_p: Some(-0.18),
            facts: json!({
                "killer": UNCLE_BUBBLES.to_string(),
                "traded": false,
                "round_end_delta_s": 9.0,
                "non_following_teammate": TAKENOUCHI.to_string(),
                "their_distance": 1850.0,
                "dead_teammate": KANAE.to_string(),
            }),
        };
        let t = narrate_moment(&m, &ctx());
        assert_eq!(t.headline, "Traded in alone");
        assert_eq!(
            t.facts,
            vec![
                "Takenouchi 1,850 u back when Kanae went down — never in trade range".to_string(),
                "Not traded — round ended 9 s later".to_string(),
            ]
        );
    }

    /// V1.2 final-review fix wave, finding #4: `H16_FIRE_LINGER`'s real
    /// schema (`h16.rs::detect_fire_linger`: `{total_damage, duration_s,
    /// round}`) now gets a dedicated fact arm instead of falling through
    /// the teammate/distance fallback (which would render nothing for it).
    /// Exact string against the real per-episode numbers from the goldens
    /// README's own hand-verified example (75 dmg / 2.8 s).
    #[test]
    fn fire_linger_flag_moment_exact_string() {
        let m = Moment {
            tick: 40_000,
            kind: "flag".to_string(),
            rule_id: Some(FIRE_LINGER.to_string()),
            delta_p: None,
            facts: json!({ "total_damage": 75, "duration_s": 2.8, "round": 9 }),
        };
        let t = narrate_moment(&m, &ctx());
        assert_eq!(t.facts, vec!["75 dmg, 2.8 s in the fire".to_string()]);
    }

    /// The other three fact-arm schemas (finding #4), one exact assertion
    /// each — `H6_DEAD_TIME_SMOKE` is checked separately below since its
    /// arm is deliberately empty.
    #[test]
    fn other_flag_schemas_render_their_own_numbers() {
        let c = ctx();

        let flash = Moment {
            tick: 1,
            kind: "flag".to_string(),
            rule_id: Some(FLASH_SELF_OR_TEAM.to_string()),
            delta_p: None,
            facts: json!({ "teammates_blinded": 1, "victims": [TAKENOUCHI.to_string()] }),
        };
        assert_eq!(
            narrate_moment(&flash, &c).facts,
            vec!["1 blinded: Takenouchi".to_string()]
        );

        let unused = Moment {
            tick: 2,
            kind: "flag".to_string(),
            rule_id: Some(UNUSED_UTIL_AT_ROUND_END.to_string()),
            delta_p: None,
            facts: json!({ "round": 3, "held": ["Flashbang", "Smoke Grenade"] }),
        };
        assert_eq!(
            narrate_moment(&unused, &c).facts,
            vec!["2 held: Flashbang, Smoke Grenade".to_string()]
        );

        let rotation = Moment {
            tick: 3,
            kind: "flag".to_string(),
            rule_id: Some(SLOW_ROTATION.to_string()),
            delta_p: None,
            facts: json!({ "seconds_late_or_never": null, "distance_at_plant": 1800.0 }),
        };
        assert_eq!(
            narrate_moment(&rotation, &c).facts,
            vec!["1,800 u from the site at the plant".to_string()]
        );
    }

    /// `H6_DEAD_TIME_SMOKE`'s schema carries nothing beyond the (redundant)
    /// round number — the arm is deliberately empty, which is exactly the
    /// shape `commands.rs` uses to suppress a fact-less flag moment rather
    /// than showing a bare label.
    #[test]
    fn dead_time_smoke_flag_moment_has_no_facts() {
        let m = Moment {
            tick: 4,
            kind: "flag".to_string(),
            rule_id: Some(DEAD_TIME_SMOKE.to_string()),
            delta_p: None,
            facts: json!({ "round": 7 }),
        };
        assert!(narrate_moment(&m, &ctx()).facts.is_empty());
    }

    /// V1.2 final-review fix wave, minor #6: a teamkill by the tracked
    /// player must be labeled honestly, not reused from the enemy-kill
    /// wording.
    #[test]
    fn team_kill_headline_and_fact_are_honest() {
        let m = Moment {
            tick: 5,
            kind: "tracked_kill".to_string(),
            rule_id: None,
            delta_p: Some(-0.1),
            facts: json!({ "victim": TAKENOUCHI.to_string(), "team_kill": true }),
        };
        let t = narrate_moment(&m, &ctx());
        assert_eq!(t.headline, "Teamkill");
        assert_eq!(
            t.facts,
            vec![
                "Takenouchi down — your side".to_string(),
                "-10% win probability".to_string(),
            ]
        );
    }

    #[test]
    fn fmt_units_thousands_separator() {
        assert_eq!(fmt_units(818.0), "818 u");
        assert_eq!(fmt_units(1223.4), "1,223 u");
        assert_eq!(fmt_units(12_345.0), "12,345 u");
        assert_eq!(
            fmt_units(12_345.6),
            "12,346 u",
            "rounding must apply before grouping across a thousands boundary"
        );
    }

    #[test]
    fn fmt_delta_pct_is_signed() {
        assert_eq!(fmt_delta_pct(0.19), "+19%");
        assert_eq!(fmt_delta_pct(-0.23), "-23%");
        assert_eq!(fmt_delta_pct(0.0), "+0%");
    }
}
