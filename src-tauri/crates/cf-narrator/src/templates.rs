//! The v1 template text. **This file is the product's voice** (PROMPT.md §8):
//! specific, actionable, no filler. Rules of the house, enforced by tests:
//!
//! - first sentence = the concrete fact with the numbers behind it;
//! - second sentence = what to do differently, phrased the way a coach says it;
//! - bodies are 1–3 sentences, titles ≤ 60 chars, no exclamation marks;
//! - a missing fact drops its clause — never render `null`, `{}` or a bare 0;
//! - `H2_BAITED_TRADE` never blames the player (death-taxonomy §2 H2): the
//!   player did the right thing, so the caption says team spacing, not fault.
//!
//! Phrasing variants are picked by a stable FNV hash of (detector, round,
//! count) — same insight in, same text out, every run, on every machine.

use cf_analysis::{Category, Insight};
use serde_json::Value;

use crate::{MatchContext, Narration};

/// How many rounds a "— rounds 4, 7, 12 and 2 more" clause lists by name.
const ROUND_LIST_CAP: usize = 4;

pub(crate) fn narrate(insight: &Insight, ctx: &MatchContext) -> Narration {
    let f = Facts {
        td: &insight.title_data,
        m: &insight.metrics,
    };
    let r = insight.round;
    match insight.detector.as_str() {
        "H2_ISOLATED_DEATH" => isolated_death(&f, r),
        "H2_FAILED_TRADE" => failed_trade(&f, r),
        "H2_BAITED_TRADE" => baited_trade(&f, ctx),
        "H3_VULNERABLE_DEATHS" => vulnerable_deaths(&f, r),
        "H3_WASTED_UTILITY" => wasted_utility(&f),
        "H4_KILLED_WITHOUT_CONTACT" => killed_without_contact(&f, r),
        "H4_CAUGHT_IN_CROSSFIRE" => caught_in_crossfire(&f),
        "H16_UTILITY_EXPOSURE" => utility_exposure(&f),
        "D2_FLASH_EFFECTIVENESS" => flash_effectiveness(&f),
        "H6_UTIL_TEAM_DAMAGE" => util_team_damage(&f, ctx),
        "H6_UNUSED_UTIL_AT_ROUND_END" => unused_util(&f),
        "H6_DEAD_TIME_SMOKE" => dead_time_smoke(&f),
        "D4_ENTRY_PROFILE" => entry_profile(&f),
        "D5_TIMING" => timing(&f),
        "D6_UNUSUAL_POSITIONING" => unusual_positioning(&f),
        other => fallback(other, &f),
    }
}

// ---------------------------------------------------------------------------
// H2 — trade spacing
// ---------------------------------------------------------------------------

fn isolated_death(f: &Facts, round: u32) -> Narration {
    let n = f.int("count");
    let where_ = f.round_clause();
    match pick("H2_ISOLATED_DEATH", round, n, 2) {
        0 => Narration {
            title: match n {
                Some(n) => format!("Died isolated {}", times(n)),
                None => "Isolated deaths".to_string(),
            },
            body: sentences(&[
                fact(
                    match n {
                        Some(n) => format!(
                            "You died isolated {} with no teammate close enough to punish the kill",
                            times(n)
                        ),
                        None => {
                            "You died isolated with no teammate close enough to punish the kill"
                                .to_string()
                        }
                    },
                    where_,
                ),
                "Take those duels one angle closer to a teammate: arrive together, or hold \
                 until someone can trade you."
                    .to_string(),
            ]),
        },
        _ => Narration {
            title: match n {
                Some(n) => format!("{} nobody could trade", plural(n, "death")),
                None => "Deaths nobody could trade".to_string(),
            },
            body: sentences(&[
                fact(
                    match n {
                        Some(n) => {
                            format!("Nobody was in range to trade you on {n} of your deaths")
                        }
                        None => "Nobody was in range to trade you when you died".to_string(),
                    },
                    where_,
                ),
                "Before you take the duel, know who re-peeks for you; if the answer is nobody, \
                 hold the angle and make them come to you."
                    .to_string(),
            ]),
        },
    }
}

fn failed_trade(f: &Facts, round: u32) -> Narration {
    let n = f.int("count");
    let where_ = f.round_clause();
    let mut out = match pick("H2_FAILED_TRADE", round, n, 2) {
        0 => Narration {
            title: match n {
                Some(n) => format!("{} you were in range for", plural(n, "trade")),
                None => "Trades you were in range for".to_string(),
            },
            body: sentences(&[
                fact(
                    match n {
                        Some(n) => format!(
                            "A teammate died inside trade range of you {} and you didn't take \
                             the re-peek",
                            times(n)
                        ),
                        None => "A teammate died inside trade range of you and you didn't take \
                                 the re-peek"
                            .to_string(),
                    },
                    where_,
                ),
                "The two seconds after his death are the cheapest kill in the round: move on \
                 the sound, not after it."
                    .to_string(),
            ]),
        },
        _ => Narration {
            title: match n {
                Some(n) => format!("Missed {} in range", plural(n, "trade")),
                None => "Missed trades in range".to_string(),
            },
            body: sentences(&[
                fact(
                    match n {
                        Some(n) => format!(
                            "You were close enough to trade {} and stayed on your angle",
                            plural(n, "teammate death")
                        ),
                        None => "You were close enough to trade a teammate's death and stayed \
                                 on your angle"
                            .to_string(),
                    },
                    where_,
                ),
                "Keep your crosshair where he is fighting so the trade is one step, not a \
                 repositioning job."
                    .to_string(),
            ]),
        },
    };
    if f.flag("team_pattern") {
        out.body.push(' ');
        out.body.push_str(
            "Baited trades are recurring too, so this is a team spacing problem: decide who \
             the second man is before the round, not during it.",
        );
    }
    out
}

/// Never blames. Spec §2 H2: this rule is the *complement* of the failed
/// trade — the player did the right thing and got left in it.
fn baited_trade(f: &Facts, ctx: &MatchContext) -> Narration {
    let n = f.int("count");
    let where_ = f.round_clause();
    // Spec §2 H2: the caption must NAME the teammate who didn't follow when
    // the insight carries them (steamid strings, resolved to display names).
    let non_followers: Vec<String> = f
        .get("non_following_teammates")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .filter_map(|s| s.parse::<u64>().ok())
                .map(|sid| ctx.name(sid))
                .take(2)
                .collect()
        })
        .unwrap_or_default();
    let opener = match (n, &where_) {
        (Some(n), Some(w)) => format!(
            "You committed to the trade and the follow-up never came — {}, {w}.",
            times(n)
        ),
        (Some(n), None) => format!(
            "You committed to the trade and the follow-up never came — {}.",
            times(n)
        ),
        (None, Some(w)) => {
            format!("You committed to the trade and the follow-up never came — {w}.")
        }
        (None, None) => "You committed to the trade and the follow-up never came.".to_string(),
    };
    let who = match non_followers.as_slice() {
        [] => "You were the only one who re-peeked".to_string(),
        [a] => format!("You were the only one who re-peeked — {a} was nearest and stayed put"),
        [a, b] => {
            format!("You were the only one who re-peeked — {a} and {b} were nearest and stayed put")
        }
        _ => unreachable!("capped at 2 above"),
    };
    let mut body =
        format!("{opener} {who}; that is a team spacing problem, not a reason to stop trading.");
    if f.flag("team_pattern") {
        body.push(' ');
        body.push_str(
            "Failed trades are recurring on your side too — the whole unit is arriving one man \
             at a time.",
        );
    }
    Narration {
        title: "You traded in, nobody followed".to_string(),
        body,
    }
}

// ---------------------------------------------------------------------------
// H3 / H4 / H16 — how the death happened
// ---------------------------------------------------------------------------

fn vulnerable_deaths(f: &Facts, round: u32) -> Narration {
    let n = f.int("vulnerable");
    let total = f.int("total_deaths");
    let share = f
        .float("pct")
        .or_else(|| match (n, total) {
            (Some(n), Some(t)) if t > 0 => Some(n as f64 / t as f64),
            _ => None,
        })
        .map(|p| format!(" ({})", pct(p)))
        .unwrap_or_default();
    let of_total = match (n, total) {
        (Some(n), Some(t)) => format!("{n} of your {}", plural(t, "death")),
        (Some(n), None) => format!("{n} of your deaths"),
        (None, _) => "Some of your deaths".to_string(),
    };
    match pick("H3_VULNERABLE_DEATHS", round, n, 2) {
        0 => Narration {
            title: match (n, total) {
                (Some(n), Some(t)) => {
                    format!("{n} of {} with no way to fight back", plural(t, "death"))
                }
                (Some(n), None) => format!("{} with no way to fight back", plural(n, "death")),
                (None, _) => "Deaths with no way to fight back".to_string(),
            },
            body: format!(
                "{of_total}{share} came while you couldn't fight back — mid-throw, reloading or \
                 swapping weapons. Do that work behind cover: step off the angle first, then \
                 throw or reload."
            ),
        },
        _ => Narration {
            title: match n {
                Some(n) => format!("Caught mid-animation in {}", plural(n, "death")),
                None => "Caught mid-animation".to_string(),
            },
            body: format!(
                "You were mid-animation — throwing, reloading, swapping — for {}{share}. The \
                 nade and the reload each cost you a second: spend it where nobody has a line \
                 on you.",
                of_total.to_lowercase()
            ),
        },
    }
}

fn wasted_utility(f: &Facts) -> Narration {
    let n = f.int("deaths_holding");
    let total = f.int("total_deaths");
    let item = f
        .text("most_common_item")
        .map(|i| {
            let name = item_name(i);
            format!(" — most often {} {name}", article(&name))
        })
        .unwrap_or_default();
    Narration {
        title: match n {
            Some(n) => format!("Died holding utility {}", times(n)),
            None => "Died holding utility".to_string(),
        },
        body: format!(
            "You died with unthrown grenades in {}{item}. Utility you carry into your own death \
             is utility you paid for and never used: throw it into the fight you are already in.",
            match (n, total) {
                (Some(n), Some(t)) => format!("{n} of your {}", plural(t, "death")),
                (Some(n), None) => format!("{n} of your deaths"),
                (None, _) => "several of your deaths".to_string(),
            }
        ),
    }
}

fn killed_without_contact(f: &Facts, round: u32) -> Narration {
    let smoke = f.int("smoke_deaths").unwrap_or(0);
    let wall = f.int("wallbang_deaths").unwrap_or(0);
    let total = smoke + wall;
    let mut long = vec![];
    let mut split = vec![];
    let mut media = vec![];
    if smoke > 0 {
        long.push(format!("through smoke {}", times(smoke)));
        split.push(format!("{smoke} through smoke"));
        media.push("smoke");
    }
    if wall > 0 {
        long.push(format!("through a wall {}", times(wall)));
        split.push(format!("{wall} through a wall"));
        media.push("a wall");
    }
    if total == 0 {
        return Narration {
            title: "Killed without ever being in the fight".to_string(),
            body: "You were killed by enemies you never traded a shot with. Those are lines the \
                   enemy sprays for free: cross the gap wide, or hold from a spot they don't \
                   pre-fire first."
                .to_string(),
        };
    }
    // With one medium there is no breakdown to give, so the count is stated
    // once rather than restated as a total of itself.
    let single = media.len() == 1;
    match pick("H4_KILLED_WITHOUT_CONTACT", round, Some(total), 2) {
        0 => Narration {
            title: format!("{} without a duel", plural(total, "death")),
            body: format!(
                "{}. Those are lines the enemy sprays for free: cross the gap wide, or hold \
                 from a spot they don't pre-fire first.",
                if single {
                    format!(
                        "You were killed {} without ever getting to fight",
                        list(&long)
                    )
                } else {
                    format!(
                        "You were killed {} — {} where you never got to fight",
                        list(&long),
                        plural(total, "death")
                    )
                }
            ),
        },
        _ => Narration {
            title: match (smoke > 0, wall > 0) {
                (true, true) => format!("Killed through smoke and walls {}", times(total)),
                (true, false) => format!("Killed through smoke {}", times(total)),
                _ => format!("Killed through walls {}", times(total)),
            },
            body: format!(
                "{}. Change where you stand rather than how you aim: step off the common spray \
                 line before you hold it.",
                if single {
                    format!(
                        "{total} of your deaths never became a duel, every one of them through {}",
                        media[0]
                    )
                } else {
                    format!(
                        "{total} of your deaths never became a duel — {}",
                        list(&split)
                    )
                }
            ),
        },
    }
}

fn caught_in_crossfire(f: &Facts) -> Narration {
    let n = f.int("count");
    Narration {
        title: match n {
            Some(n) => format!("Caught in crossfire {}", times(n)),
            None => "Caught in crossfire".to_string(),
        },
        body: format!(
            "You were mid-duel with one enemy and killed by a second from another angle{}. \
             Clear the off-angle before you commit, or take the fight from where only one of \
             them has a line on you.",
            match n {
                Some(n) => format!(" {}", times(n)),
                None => String::new(),
            }
        ),
    }
}

fn utility_exposure(f: &Facts) -> Narration {
    let deaths = f.int("utility_deaths").unwrap_or(0);
    let episodes = f.int("fire_linger_episodes").unwrap_or(0);
    let damage = f.int("total_fire_damage").unwrap_or(0);
    let mut clauses = vec![];
    if deaths > 0 {
        clauses.push(format!(
            "Enemy grenades killed you {} with no duel involved",
            times(deaths)
        ));
    }
    if episodes > 0 {
        clauses.push(if damage > 0 {
            format!(
                "you took {damage} damage standing in fire across {}",
                plural(episodes, "episode")
            )
        } else {
            format!(
                "you stood in fire across {}",
                plural(episodes, "separate episode")
            )
        });
    }
    let opener = if clauses.is_empty() {
        "Enemy utility is taking health off you that you never get to trade for".to_string()
    } else {
        clauses.join(", and ")
    };
    Narration {
        title: if deaths > 0 {
            format!("Enemy utility cost you {}", plural(deaths, "death"))
        } else {
            "You keep standing in fire".to_string()
        },
        body: format!(
            "{}. Move on the first tick of fire damage — the exit is always cheaper than \
             standing in it.",
            capitalize(&opener)
        ),
    }
}

// ---------------------------------------------------------------------------
// Utility usage
// ---------------------------------------------------------------------------

/// A flash rate at or above this, with nothing landing on your own side, is
/// good flashing — say so instead of coaching a habit the player doesn't have.
const FLASH_GOOD_RATE: f64 = 0.6;

fn flash_effectiveness(f: &Facts) -> Narration {
    let flashes = f.int("flashes");
    let effective = f.int("effective");
    let team = f.int("team_flashes").unwrap_or(0);
    let conversions = f.int("conversions").unwrap_or(0);
    let self_flashes = f.int("self_flashes").unwrap_or(0);
    let rate = f
        .float("effective_rate")
        .or_else(|| match (effective, flashes) {
            (Some(e), Some(all)) if all > 0 => Some(e as f64 / all as f64),
            _ => None,
        });
    let mut clauses = vec![];
    if let Some(e) = effective {
        clauses.push(if e > 0 {
            format!("{e} blinded an enemy")
        } else {
            "none of them blinded an enemy".to_string()
        });
    }
    if team > 0 {
        clauses.push(format!("{team} caught a teammate"));
    }
    if conversions > 0 {
        clauses.push(format!("{conversions} led to a kill"));
    }
    let opener = match (flashes, clauses.is_empty()) {
        (Some(n), false) => format!("You threw {}: {}", plural(n, "flash"), list(&clauses)),
        (Some(n), true) => format!("You threw {} this match", plural(n, "flash")),
        (None, false) => format!("Of the flashes you threw, {}", list(&clauses)),
        (None, true) => "Your flashes aren't earning their place in the round".to_string(),
    };
    // D2 emits at >=3 flashes regardless of quality, so this template sees good
    // flashing as often as bad. Coach only the habit the numbers show.
    let coach = if team > effective.unwrap_or(0) {
        "More of them landed on your own team than on the enemy — line the flash up over cover \
         and agree who entries before you throw."
    } else if self_flashes > 0 {
        "Flash for the man entering, not for yourself: throw it over cover from behind him and \
         let him move on the pop."
    } else if team == 0 && rate.is_some_and(|r| r >= FLASH_GOOD_RATE) {
        "That is a rate worth keeping — throw from behind the man entering and make sure \
         someone moves on every pop."
    } else {
        "Throw from behind the man entering and over cover, so the flash pops where he is \
         already looking."
    };
    Narration {
        title: match (flashes, effective) {
            (Some(n), Some(0)) => format!("{}, none blinded an enemy", plural(n, "flash")),
            (Some(n), Some(e)) => format!("{}, {e} blinded an enemy", plural(n, "flash")),
            (Some(n), None) => format!("{} this match", plural(n, "flash")),
            (None, _) => "Flash effectiveness".to_string(),
        },
        body: format!("{opener}. {coach}"),
    }
}

fn util_team_damage(f: &Facts, ctx: &MatchContext) -> Narration {
    let events = f.int("events");
    let damage = f.int("total_damage");
    let on_whom = f
        .player("victim", ctx)
        .map(|n| format!(", most of it on {n}"))
        .unwrap_or_default();
    let opener = match (damage, events) {
        (Some(d), Some(e)) => format!(
            "Your grenades did {d} damage to your own team across {}{on_whom}",
            plural(e, "throw")
        ),
        (Some(d), None) => format!("Your grenades did {d} damage to your own team{on_whom}"),
        (None, Some(e)) => format!(
            "Your grenades hit your own team on {}{on_whom}",
            plural(e, "throw")
        ),
        (None, None) => format!("Your grenades keep landing on your own team{on_whom}"),
    };
    Narration {
        title: match events {
            Some(e) => format!("Your utility hurt teammates {}", times(e)),
            None => "Your utility hurt teammates".to_string(),
        },
        body: format!(
            "{opener}. Call the nade before it leaves your hand and wait for the lane to clear \
             — that HP comes straight out of the next duel."
        ),
    }
}

fn unused_util(f: &Facts) -> Narration {
    let rounds = f.int("rounds").or_else(|| f.int("count"));
    let min = f.int("min_nades");
    Narration {
        title: match rounds {
            Some(r) => format!("Ended {} holding utility", plural(r, "round")),
            None => "Utility left unthrown".to_string(),
        },
        body: format!(
            "You finished {} alive with {}unthrown. Utility has no value once the round ends: \
             spend the smoke on the timing you already committed to, or the flash on the last \
             angle you take.",
            match rounds {
                Some(r) => plural(r, "round"),
                None => "rounds".to_string(),
            },
            match min {
                Some(m) => format!("{m} or more grenades "),
                None => "grenades still ".to_string(),
            }
        ),
    }
}

fn dead_time_smoke(f: &Facts) -> Narration {
    let n = f.int("rounds").or_else(|| f.int("count"));
    Narration {
        title: match n {
            Some(n) => format!("{} thrown after the round", plural(n, "smoke")),
            None => "Smokes thrown after the round".to_string(),
        },
        body: format!(
            "{} went out after the round had already ended. That is utility you paid for and \
             never used — throw it while the round is still live, or keep the money for a rifle.",
            match n {
                Some(n) => format!("{n} of your smokes"),
                None => "Your smokes".to_string(),
            }
        ),
    }
}

// ---------------------------------------------------------------------------
// D4 / D5
// ---------------------------------------------------------------------------

fn entry_profile(f: &Facts) -> Narration {
    let entries = f.int("entries");
    let wins = f.int("entry_wins");
    let team_entries = f.int("team_entries");
    let unsupported = f.int("unsupported").filter(|n| *n > 0);
    let untraded = f.int("non_trading_on_entries").filter(|n| *n > 0);

    let took = match (entries, team_entries) {
        // Taking all of them is the fact; "4 of your team's 4 entries" is not.
        (Some(e), Some(t)) if e >= t => {
            "You took first contact on every one of your team's entries".to_string()
        }
        (Some(e), Some(t)) => format!(
            "You took first contact on {e} of your team's {}",
            plural(t, "entry")
        ),
        (Some(e), None) => format!("You took first contact {}", times(e)),
        (None, Some(t)) => format!("Your team took first contact {} this match", times(t)),
        (None, None) => "You are taking first contact for your team".to_string(),
    };
    let first = match wins {
        // Zero has words in this crate — "won 0 of them" reads like a spreadsheet.
        Some(0) => format!("{took} and won none of them."),
        Some(w) => format!("{took} and won {w} of them."),
        None => format!("{took}."),
    };
    let mut clauses = vec![];
    if let Some(u) = unsupported {
        clauses.push(format!("{u} of those went in unsupported"));
    }
    if let Some(t) = untraded {
        clauses.push(format!("{t} went untraded"));
    }
    // With nothing broken to name, the closing line has to reinforce what is
    // already working instead of scolding a player who entries well.
    let (middle, coach) = if clauses.is_empty() {
        (
            String::new(),
            "Keep the flash and the second man attached to every one of them — an entry alone \
             is a coin flip you are paying for.",
        )
    } else {
        (
            format!("{}. ", capitalize(&list(&clauses))),
            "Don't take the first duel until the flash or the second man is with you — an entry \
             alone is a coin flip you are paying for.",
        )
    };
    Narration {
        title: match (entries, wins) {
            (Some(e), Some(w)) => format!("{}, {w} won", plural(e, "entry")),
            (Some(e), None) => plural(e, "entry duel"),
            (None, _) => "Entry duels".to_string(),
        },
        body: format!("{first} {middle}{coach}"),
    }
}

fn timing(f: &Facts) -> Narration {
    let early = f.int("early_aggressive_deaths").filter(|n| *n > 0);
    let slow = f.int("slow_rotations").filter(|n| *n > 0);
    let blind = f.int("push_without_info").filter(|n| *n > 0);
    let mut clauses = vec![];
    if let Some(n) = early {
        clauses.push(format!(
            "died on early aggression in {}",
            plural(n, "round")
        ));
    }
    if let Some(n) = slow {
        clauses.push(format!("rotated late {}", times(n)));
    }
    if let Some(n) = blind {
        clauses.push(format!("pushed without info {}", times(n)));
    }
    let opener = if clauses.is_empty() {
        "Your round timing is costing you fights before they start".to_string()
    } else {
        format!("You {}", list(&clauses))
    };
    // Only coach the halves that actually fired.
    let coach = match (early.is_some() || blind.is_some(), slow.is_some()) {
        (true, true) => {
            "Take space after first contact tells you where they aren't — and rotate on the \
             call, not after the site falls."
        }
        (true, false) => "Take space after first contact tells you where they aren't, not before.",
        (false, true) => "Rotate on the call, not after the site falls.",
        (false, false) => {
            "Let the round tell you where the space is before you take it, and rotate on the \
             call rather than after it."
        }
    };
    Narration {
        title: match (early.is_some(), slow.is_some(), blind.is_some()) {
            (true, true, _) => "Early deaths and slow rotations".to_string(),
            (true, false, _) => "Dying early in the round".to_string(),
            (false, true, _) => "Rotating late".to_string(),
            (false, false, true) => "Pushing without info".to_string(),
            _ => "Round timing".to_string(),
        },
        body: format!("{opener}. {coach}"),
    }
}

// ---------------------------------------------------------------------------
// D6 — positioning vs the reference corpus. Honesty rule (spec §5): this
// measures unusualness, never wrongness, so the wording must never scold.
// ---------------------------------------------------------------------------

fn unusual_positioning(f: &Facts) -> Narration {
    let count = f.int("count");
    let side = f.text("side");
    let phase = f.text("phase").map(|p| match p {
        "freeze_end" => "freeze end",
        "early" => "the early push",
        "mid" => "mid-round",
        "post_plant" => "post-plant",
        other => other,
    });
    let title = match (side, count) {
        (Some(s), Some(n)) => format!("Unusual {s}-side positioning — {}", plural(n, "round")),
        (Some(s), None) => format!("Unusual {s}-side positioning"),
        (None, Some(n)) => format!("Unusual positioning — {}", plural(n, "round")),
        (None, None) => "Unusual positioning".to_string(),
    };
    let spot = match (phase, side) {
        (Some(p), Some(s)) => format!("the spot you took at {p} on {s}"),
        (Some(p), None) => format!("the spot you took at {p}"),
        (None, Some(s)) => format!("the spot you took on {s}"),
        (None, None) => "the spots you took".to_string(),
    };
    let count_clause = match count {
        Some(n) => format!(" — {} this match", plural(n, "round")),
        None => String::new(),
    };
    Narration {
        title,
        body: format!(
            "Reference players rarely hold {spot}{count_clause}. This measures unusual, not \
             wrong: check the heatmap for where they set up instead."
        ),
    }
}

// ---------------------------------------------------------------------------
// Fallback + habits
// ---------------------------------------------------------------------------

/// Unknown detector: name it plainly and say how often it fired. Never empty,
/// never a lie — a new detector ships readable before it ships a template.
fn fallback(detector: &str, f: &Facts) -> Narration {
    let n = f
        .int("count")
        .or_else(|| f.int("events"))
        .or_else(|| f.int("rounds"));
    Narration {
        title: humanize_id(detector),
        body: match n {
            Some(n) => format!("Flagged {} this match.", times(n)),
            None => "Flagged this match.".to_string(),
        },
    }
}

/// Cross-demo habit narration (§5A pattern promotion).
pub fn narrate_habit(
    rule_id: &str,
    matches_hit: usize,
    window: usize,
    total: u32,
    extra: &Value,
) -> Narration {
    let times_in_all = plural(i64::from(total), "time");
    let seen = if window <= 1 {
        format!("in your last match — {times_in_all} in all")
    } else if matches_hit >= window {
        // "in every one of your last 5" lands harder than "in 5 of your last 5".
        format!(
            "in every one of your last {} — {times_in_all} in all",
            plural(window as i64, "match")
        )
    } else {
        format!(
            "in {matches_hit} of your last {} — {times_in_all} in all",
            plural(window as i64, "match")
        )
    };
    match rule_id {
        "H2_ISOLATED_DEATH" => Narration {
            title: "Habit: isolated deaths".to_string(),
            body: format!(
                "You died isolated {seen}. This is the first habit to fix: pick fights a \
                 teammate can re-peek within two seconds."
            ),
        },
        "H2_FAILED_TRADE" => Narration {
            title: "Habit: missed trades".to_string(),
            body: format!(
                "You left trades on the table {seen}. Standing near a teammate is not support; \
                 re-peeking within two seconds of his death is."
            ),
        },
        // Promotion only ever reaches here alongside H2_FAILED_TRADE (see
        // analysis::habits::promote_habits), so the caption leans on the
        // combination: the player keeps being the only one who commits.
        // death-taxonomy §2 H2 — never blame, name the *team* spacing problem,
        // never coach the player out of the trade.
        "H2_BAITED_TRADE" => Narration {
            title: "Habit: nobody follows your trade".to_string(),
            body: format!(
                "You were the only one who committed to the trade {seen}. Failed trades are \
                 recurring on your side in the same window, so this is a team spacing problem, \
                 not a habit to unlearn: keep re-peeking, and fix the timing so the second man \
                 leaves with you."
            ),
        },
        "H3_WASTED_UTILITY" => Narration {
            title: "Habit: dying with utility".to_string(),
            body: format!(
                "You died holding unthrown grenades {seen}. Make it a rule: nades leave your \
                 hand before the fight starts, not once you are in it."
            ),
        },
        "H4_KILLED_WITHOUT_CONTACT" => Narration {
            title: "Habit: killed without a duel".to_string(),
            body: format!(
                "Smoke and wallbang deaths caught you {seen}. You keep holding lines that get \
                 sprayed blind — take one step off the common spot before you set up."
            ),
        },
        "H4_REPEAT_HOTSPOT" => {
            let map = extra.get("map").and_then(Value::as_str).and_then(map_name);
            let place = extra
                .get("place")
                .and_then(Value::as_str)
                .map(crate::callouts::callout_name);
            let deaths = extra
                .get("deaths")
                .and_then(Value::as_i64)
                .unwrap_or(i64::from(total));
            let matches = extra
                .get("matches")
                .and_then(Value::as_i64)
                .unwrap_or(matches_hit as i64);
            let spot = match (&place, &map) {
                (Some(p), Some(m)) => format!("{p} on {m}"),
                (Some(p), None) => p.clone(),
                (None, Some(m)) => format!("the same spot on {m}"),
                (None, None) => "the same spot".to_string(),
            };
            Narration {
                title: match (&place, &map) {
                    (Some(p), Some(m)) => format!("Repeat hotspot: {p} on {m}"),
                    (Some(p), None) => format!("Repeat hotspot: {p}"),
                    (None, Some(m)) => format!("Repeat hotspot on {m}"),
                    (None, None) => "Repeat hotspot".to_string(),
                },
                body: format!(
                    "You have died {} at {spot} across {}. They know that angle better \
                     than you do — hold it from a different position, or stop taking that fight.",
                    times(deaths),
                    plural(matches, "match")
                ),
            }
        }
        other => {
            let label = humanize_id(other);
            Narration {
                title: format!("Habit: {}", label.to_lowercase()),
                body: format!(
                    "{label} recurred {seen}. A mistake that repeats across matches is a habit: \
                     watch the clips together and find what they share."
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Match summary
// ---------------------------------------------------------------------------

pub(crate) fn summarize(insights: &[Insight], ctx: &MatchContext) -> Option<Narration> {
    if insights.is_empty() {
        return None;
    }
    let map = map_name(&ctx.map);
    let score = format!("{}-{}", ctx.score.0, ctx.score.1);
    let verb = match ctx
        .tracked_result
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("win" | "won") => Some(("won", "win")),
        Some("loss" | "lost" | "lose" | "defeat") => Some(("lost", "loss")),
        Some("tie" | "draw" | "drew") => Some(("drew", "draw")),
        _ => None,
    };

    let title = match (&map, verb) {
        (Some(m), Some((_, noun))) => format!("{m}, {score} {noun}"),
        (Some(m), None) => format!("{m}, {score}"),
        (None, Some((_, noun))) => format!("Match report, {score} {noun}"),
        (None, None) => format!("Match report, {score}"),
    };

    let on_map = map.map(|m| format!(" on {m}")).unwrap_or_default();
    let deaths = ctx.total_deaths;
    let mut body = vec![match verb {
        Some((past, _)) if deaths > 0 => format!(
            "You {past} {score}{on_map} and died {}.",
            times(deaths as i64)
        ),
        Some((past, _)) => format!("You {past} {score}{on_map}."),
        None if deaths > 0 => format!(
            "The match finished {score}{on_map}; you died {}.",
            times(deaths as i64)
        ),
        None => format!("The match finished {score}{on_map}."),
    }];

    if deaths > 0 {
        let share = f64::from(ctx.class_13_share_pct);
        body.push(if share >= 99.5 {
            // Must not claim the fixes are "elsewhere" — the next sentence names
            // where to start, and the two would contradict each other.
            "Every one of your deaths was a fair duel you lost on mechanics — that is the good \
             version of losing."
                .to_string()
        } else if share < 0.5 {
            "None of your deaths were fair duels you lost on mechanics — every one of them has \
             a fixable cause."
                .to_string()
        } else {
            format!(
                "{} of your deaths were fair duels you lost on mechanics — the rest had a \
                 fixable cause.",
                pct(share / 100.0)
            )
        });
    }

    let (top, top_n) = top_category(insights);
    let total = insights.len();
    body.push(if total == 1 {
        format!(
            "The one insight this match sits in {}, so start there.",
            top.1
        )
    } else {
        format!(
            "{} are the biggest group at {top_n} of the {}, so start there.",
            top.0,
            plural(total as i64, "insight")
        )
    });

    Some(Narration {
        title,
        body: body.join(" "),
    })
}

/// Most-flagged category, ties broken by the fixed order below so the summary
/// never changes between runs on the same data.
fn top_category(insights: &[Insight]) -> ((&'static str, &'static str), usize) {
    const ORDER: [(Category, &str, &str); 4] = [
        (Category::Deaths, "Deaths", "deaths"),
        (Category::Utility, "Utility", "utility"),
        (Category::Positioning, "Positioning", "positioning"),
        (Category::Timing, "Timing", "timing"),
    ];
    let mut best = ((ORDER[0].1, ORDER[0].2), 0usize);
    for (cat, title, lower) in ORDER {
        let n = insights.iter().filter(|i| i.category == cat).count();
        if n > best.1 {
            best = ((title, lower), n);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Fact access + wording helpers
// ---------------------------------------------------------------------------

/// Reads a fact from `title_data` first, then `metrics`. Missing or null keys
/// come back as `None` so the caller drops the clause instead of rendering it.
struct Facts<'a> {
    td: &'a Value,
    m: &'a Value,
}

impl<'a> Facts<'a> {
    fn get(&self, key: &str) -> Option<&'a Value> {
        self.td
            .get(key)
            .or_else(|| self.m.get(key))
            .filter(|v| !v.is_null())
    }

    fn int(&self, key: &str) -> Option<i64> {
        self.get(key)?.as_i64()
    }

    fn float(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_f64()
    }

    fn text(&self, key: &str) -> Option<&'a str> {
        let s = self.get(key)?.as_str()?.trim();
        (!s.is_empty()).then_some(s)
    }

    fn flag(&self, key: &str) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    /// A steamid fact (string per the §4 serialization rule, or a number)
    /// resolved to a display name.
    fn player(&self, key: &str, ctx: &MatchContext) -> Option<String> {
        let v = self.get(key)?;
        if let Some(id) = v.as_u64() {
            return Some(ctx.name(id));
        }
        let s = v.as_str()?.trim();
        if s.is_empty() {
            return None;
        }
        Some(match s.parse::<u64>() {
            Ok(id) => ctx.name(id),
            Err(_) => s.to_string(),
        })
    }

    /// "rounds 4, 7, 12 and 2 more" from the per-round evidence list.
    fn round_clause(&self) -> Option<String> {
        let rounds: Vec<u64> = self
            .get("per_round")?
            .as_array()?
            .iter()
            .filter_map(|e| e.get("round")?.as_u64())
            .collect();
        if rounds.is_empty() {
            return None;
        }
        let label = if rounds.len() == 1 { "round" } else { "rounds" };
        let shown: Vec<String> = rounds
            .iter()
            .take(ROUND_LIST_CAP)
            .map(u64::to_string)
            .collect();
        let hidden = rounds.len().saturating_sub(ROUND_LIST_CAP);
        let listed = if hidden > 0 {
            format!("{} and {hidden} more", shown.join(", "))
        } else {
            list(&shown)
        };
        Some(format!("{label} {listed}"))
    }
}

/// First sentence: the fact, with the round clause appended when we have one.
fn fact(claim: String, where_: Option<String>) -> String {
    match where_ {
        Some(w) => format!("{claim} — {w}."),
        None => format!("{claim}."),
    }
}

fn sentences(parts: &[String]) -> String {
    parts
        .iter()
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// "a", "a and b", "a, b and c" — the way it is said out loud.
fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        _ => {
            let (last, head) = items.split_last().expect("non-empty");
            format!("{} and {last}", head.join(", "))
        }
    }
}

fn times(n: i64) -> String {
    match n {
        1 => "once".to_string(),
        2 => "twice".to_string(),
        _ => format!("{n} times"),
    }
}

/// "1 round" / "3 rounds". **Use this everywhere a count renders next to its
/// noun** — the detector gates allow 1 far more often than they look like they
/// do (H16 fires on one utility death plus one fire episode; H11 on one early
/// death plus one slow rotation), and "1 rounds" in a coaching line reads like
/// a bug because it is one.
fn plural(n: i64, noun: &str) -> String {
    if n == 1 {
        return format!("{n} {noun}");
    }
    let ends_sibilant = noun.ends_with("ch")
        || noun.ends_with("sh")
        || noun.ends_with('s')
        || noun.ends_with('x')
        || noun.ends_with('z');
    if ends_sibilant {
        return format!("{n} {noun}es");
    }
    // consonant + y → -ies ("entry" → "entries"), vowel + y stays ("plays").
    let consonant_y = noun.ends_with('y')
        && noun
            .chars()
            .rev()
            .nth(1)
            .is_some_and(|c| !matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'));
    if consonant_y {
        return format!("{n} {}ies", &noun[..noun.len() - 1]);
    }
    format!("{n} {noun}s")
}

fn pct(share: f64) -> String {
    format!("{}%", (share * 100.0).round() as i64)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// "H8_OFF_ANGLE_HABIT" → "Off angle habit". Keeps unknown ids readable
/// instead of shouting a rule id at the user.
fn humanize_id(id: &str) -> String {
    let mut parts: Vec<&str> = id.split('_').filter(|p| !p.is_empty()).collect();
    if parts.len() > 1 && is_family_prefix(parts[0]) {
        parts.remove(0);
    }
    let words = parts.join(" ").to_lowercase();
    if words.is_empty() {
        return "Coaching note".to_string();
    }
    capitalize(&words)
}

/// "H2", "D14" — a family prefix, not a word.
fn is_family_prefix(part: &str) -> bool {
    let mut chars = part.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && part.len() > 1
        && part[1..].chars().all(|c| c.is_ascii_digit())
}

/// "de_mirage" → "Mirage".
fn map_name(map: &str) -> Option<String> {
    let raw = map.trim();
    if raw.is_empty() {
        return None;
    }
    let stripped = raw
        .strip_prefix("de_")
        .or_else(|| raw.strip_prefix("cs_"))
        .or_else(|| raw.strip_prefix("ar_"))
        .unwrap_or(raw);
    Some(capitalize(&stripped.replace('_', " ")))
}

/// "a smoke" but "an HE grenade" — spoken sound, not spelling.
fn article(word: &str) -> &'static str {
    let starts_vowel_sound = word.starts_with("HE ")
        || word
            .chars()
            .next()
            .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'));
    if starts_vowel_sound {
        "an"
    } else {
        "a"
    }
}

/// Inventory display names → how a player says it.
fn item_name(raw: &str) -> String {
    match raw.trim().to_lowercase().as_str() {
        "smoke grenade" | "smokegrenade" => "smoke",
        "high explosive grenade" | "hegrenade" => "HE grenade",
        "incendiary grenade" | "incgrenade" => "incendiary",
        "decoy grenade" => "decoy",
        other => return other.to_string(),
    }
    .to_string()
}

/// Deterministic phrasing pick — FNV-1a over (detector, round, count). No
/// randomness anywhere in this crate: same insight in, same text out.
fn pick(detector: &str, round: u32, count: Option<i64>, variants: usize) -> usize {
    let mut h = fnv1a(detector.as_bytes(), 0xcbf2_9ce4_8422_2325);
    h = fnv1a(&round.to_le_bytes(), h);
    h = fnv1a(&count.unwrap_or(0).to_le_bytes(), h);
    (avalanche(h) % variants as u64) as usize
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// splitmix64 finalizer. Without it FNV's low bit tracks the count's low bit,
/// so the phrasing would alternate strictly by parity — deterministic, but
/// visibly mechanical to anyone reading two reports side by side.
fn avalanche(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}
