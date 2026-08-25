//! Template captions for the play ledger (spec §2 "Template captions per
//! kind"): the offline/fallback narration, and V1.3's fallback when the
//! coach is unavailable. Same voice rules as `rail`: numbers first, no
//! exclamation marks, never fault language. Kill/death/plant/defuse/flag
//! plays reuse `rail::narrate_moment` through a `Moment` view (same facts
//! keys) and add the duel numbers the ledger measured.

use cf_analysis::play_ledger::Play;
use cf_analysis::round_review::Moment;

use crate::callouts::callout_name;
use crate::rail::{fmt_units, name_of, narrate_moment, num, text};
use crate::MatchContext;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayText {
    pub headline: String,
    pub facts: Vec<String>,
}

pub fn narrate_play(p: &Play, ctx: &MatchContext) -> PlayText {
    let f = &p.facts;
    match p.kind.as_str() {
        "setup" => {
            let headline = match text(f, "place") {
                Some(place) => format!("Setup at {}", callout_name(&place)),
                None => "Setup".to_string(),
            };
            let mut facts = vec![];
            if let (Some(name), Some(d)) = (
                name_of(f, "nearest_teammate", ctx),
                num(f, "nearest_teammate_dist"),
            ) {
                facts.push(format!("Nearest teammate {name}, {}", fmt_units(d)));
            }
            if let (Some(w), Some(n)) = (
                num(f, "teammates_within_isolation"),
                num(f, "teammates_alive"),
            ) {
                facts.push(format!(
                    "{} of {} teammates within 900 u",
                    w as i64, n as i64
                ));
            }
            PlayText { headline, facts }
        }
        "flash" => {
            let enemies = num(f, "enemies_blinded").unwrap_or(0.0) as i64;
            let mates = num(f, "teammates_blinded").unwrap_or(0.0) as i64;
            let self_blind = f
                .get("self_blind")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let headline = if self_blind {
                "Flashed yourself".to_string()
            } else if mates > 0 {
                "Flash blinded your team".to_string()
            } else if enemies > 0 {
                format!(
                    "Flash: {enemies} {} blinded",
                    plural(enemies, "enemy", "enemies")
                )
            } else {
                "Flash: nobody blinded".to_string()
            };
            let mut facts = vec![];
            if mates > 0 || self_blind {
                if enemies > 0 {
                    facts.push(format!(
                        "{enemies} {} blinded too",
                        plural(enemies, "enemy", "enemies")
                    ));
                }
                for id in f
                    .get("teammate_ids")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                {
                    if let Some(raw) = id.as_str() {
                        let name = raw
                            .parse::<u64>()
                            .map(|i| ctx.name(i))
                            .unwrap_or_else(|_| raw.to_string());
                        facts.push(format!("Blinded {name}"));
                    }
                }
            }
            if f.get("converted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                facts.push("Converted into a kill within 2 s".to_string());
            }
            PlayText { headline, facts }
        }
        "smoke" => {
            let dead = f
                .get("dead_time")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let headline = if dead {
                "Smoke after the round was decided".to_string()
            } else {
                "Smoke".to_string()
            };
            let mut facts = vec![];
            if let Some(place) = text(f, "place") {
                facts.push(format!("Thrown from {}", callout_name(&place)));
            }
            if let Some(s) = num(f, "lifetime_s") {
                facts.push(format!("Lasted {} s", trim(s)));
            }
            PlayText { headline, facts }
        }
        "he" | "molotov" => {
            let enemy = num(f, "enemy_damage").unwrap_or(0.0) as i64;
            let team = num(f, "team_damage").unwrap_or(0.0) as i64;
            let me = num(f, "self_damage").unwrap_or(0.0) as i64;
            let label = if p.kind == "he" { "HE" } else { "Molotov" };
            let headline = match num(f, "burn_s") {
                Some(b) => format!("{label}: {enemy} damage over {} s", trim(b)),
                None => format!("{label}: {enemy} damage"),
            };
            let mut facts = vec![];
            let victims = f
                .get("victims")
                .and_then(|v| v.as_array())
                .map(|v| v.len())
                .unwrap_or(0);
            if victims > 0 {
                facts.push(format!(
                    "{victims} {} hit",
                    plural(victims as i64, "enemy", "enemies")
                ));
            }
            if team > 0 {
                facts.push(format!("{team} damage to teammates"));
            }
            if me > 0 {
                facts.push(format!("{me} damage to yourself"));
            }
            PlayText { headline, facts }
        }
        "rush" => {
            let headline = match (num(f, "distance"), num(f, "seconds_in")) {
                (Some(d), Some(s)) => format!("Rushed {} by {} s", fmt_units(d), trim(s)),
                _ => "Rushed early".to_string(),
            };
            let mut facts = vec![];
            match (
                name_of(f, "nearest_teammate", ctx),
                num(f, "nearest_teammate_dist"),
            ) {
                (Some(name), Some(d)) => {
                    facts.push(format!("Nearest teammate {name}, {}", fmt_units(d)))
                }
                _ => facts.push("No teammate alive nearby".to_string()),
            }
            if f.get("died_in_window")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                facts.push("Died inside the first 20 s".to_string());
            }
            PlayText { headline, facts }
        }
        "rotation" => {
            let at_site = f.get("at_site").and_then(|v| v.as_bool()).unwrap_or(false);
            let died = f
                .get("died_before_arrival")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let headline = if at_site {
                "Held the planted site".to_string()
            } else if let Some(s) = num(f, "arrived_s") {
                format!("Rotated to the plant in {} s", trim(s))
            } else if died {
                "Died before reaching the plant".to_string()
            } else {
                "Never reached the plant".to_string()
            };
            let mut facts = vec![];
            if let Some(d) = num(f, "distance_at_plant") {
                facts.push(format!("{} from the plant when it went down", fmt_units(d)));
            }
            PlayText { headline, facts }
        }
        "kill" | "death" | "plant" | "defuse" | "flag" => {
            let m = as_moment(p);
            let base = narrate_moment(&m, ctx);
            let mut facts = base.facts;
            if p.kind == "kill" || p.kind == "death" {
                let weapon = text(f, "weapon").map(|w| w.trim_start_matches("weapon_").to_string());
                let hs = f.get("headshot").and_then(|v| v.as_bool()).unwrap_or(false);
                if let Some(d) = num(f, "killer_distance") {
                    let mut line = fmt_units(d);
                    if let Some(w) = weapon {
                        line.push_str(&format!(", {w}"));
                    }
                    if hs {
                        line.push_str(", headshot");
                    }
                    facts.insert(0, line);
                }
                if let Some(mc) = text(f, "man_context") {
                    facts.push(format!("{mc} before"));
                }
                if p.kind == "death" {
                    let traded = f.get("traded").and_then(|v| v.as_bool()).unwrap_or(false);
                    facts.push(if traded {
                        "Traded within 2 s".to_string()
                    } else {
                        "Not traded".to_string()
                    });
                }
                if f.get("thru_smoke")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    facts.push("Through smoke".to_string());
                }
                if f.get("wallbang").and_then(|v| v.as_bool()).unwrap_or(false) {
                    facts.push("Through a wall".to_string());
                }
            }
            let headline = match p.kind.as_str() {
                "kill" => match name_of(f, "victim", ctx) {
                    Some(v)
                        if f.get("team_kill")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false) =>
                    {
                        format!("Killed {v} (teammate)")
                    }
                    Some(v) => format!("Killed {v}"),
                    None => base.headline,
                },
                "death" => match name_of(f, "killer", ctx) {
                    Some(k) => format!("Died to {k}"),
                    None => base.headline,
                },
                _ => base.headline,
            };
            PlayText { headline, facts }
        }
        "assist" => {
            let headline = match (name_of(f, "killer", ctx), name_of(f, "victim", ctx)) {
                (Some(k), Some(v)) => format!("Assisted {k} on {v}"),
                (None, Some(v)) => format!("Assist on {v}"),
                _ => "Assist".to_string(),
            };
            let facts = if f
                .get("flash_assist")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                vec!["Your flash set it up".to_string()]
            } else {
                vec![]
            };
            PlayText { headline, facts }
        }
        "trade" => {
            let mate = name_of(f, "teammate", ctx).unwrap_or_else(|| "a teammate".to_string());
            let killer = name_of(f, "killer", ctx).unwrap_or_else(|| "the killer".to_string());
            let mut facts = vec![];
            if let Some(d) = num(f, "distance") {
                facts.push(format!("{} from {mate} when they died", fmt_units(d)));
            }
            PlayText {
                headline: format!("Traded {mate} — killed {killer}"),
                facts,
            }
        }
        "missed_trade" => {
            let mate = name_of(f, "teammate", ctx).unwrap_or_else(|| "a teammate".to_string());
            let killer = name_of(f, "killer", ctx).unwrap_or_else(|| "The killer".to_string());
            let committed = f
                .get("committed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let by_team = f
                .get("traded_by_team")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let window = num(f, "window_s")
                .map(trim)
                .unwrap_or_else(|| "2".to_string());
            let dist = num(f, "distance")
                .map(fmt_units)
                .unwrap_or_else(|| "in range".to_string());
            let (headline, first) = if !committed && !by_team {
                (
                    format!("Didn't trade {mate}"),
                    format!("{killer} killed them {dist} from you; no shot from you in {window} s"),
                )
            } else if by_team {
                (
                    format!("{mate} traded by a teammate"),
                    format!("{killer} killed them {dist} from you"),
                )
            } else {
                (
                    format!("Trade on {mate} missed"),
                    format!("You fired, but {killer} lived {window} s"),
                )
            };
            PlayText {
                headline,
                facts: vec![first],
            }
        }
        "outcome" => {
            let won = f.get("won").and_then(|v| v.as_bool()).unwrap_or(false);
            let reason = text(f, "reason")
                .map(|r| r.replace('_', " "))
                .unwrap_or_default();
            let headline = match (won, reason.is_empty()) {
                (true, true) => "Round won".to_string(),
                (false, true) => "Round lost".to_string(),
                (true, false) => format!("Round won — {reason}"),
                (false, false) => format!("Round lost — {reason}"),
            };
            let mut facts = vec![];
            if let (Some(m), Some(t)) = (num(f, "my_alive"), num(f, "their_alive")) {
                facts.push(format!("{}v{} at the end", m as i64, t as i64));
            }
            if let (Some(k), Some(d)) = (num(f, "kills"), num(f, "damage")) {
                let k = k as i64;
                facts.push(format!(
                    "{k} {}, {} damage",
                    plural(k, "kill", "kills"),
                    d as i64
                ));
            }
            PlayText { headline, facts }
        }
        _ => PlayText {
            headline: "Play".to_string(),
            facts: vec![],
        },
    }
}

fn as_moment(p: &Play) -> Moment {
    let kind = match p.kind.as_str() {
        "kill" => "tracked_kill",
        "death" => "tracked_death",
        other => other,
    };
    Moment {
        tick: p.tick,
        kind: kind.to_string(),
        rule_id: p.rule_id.clone(),
        delta_p: p.delta_p,
        facts: p.facts.clone(),
    }
}

fn plural(n: i64, one: &str, many: &str) -> String {
    if n == 1 {
        one.to_string()
    } else {
        many.to_string()
    }
}

/// "4.0" -> "4", "12.5" -> "12.5".
fn trim(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_analysis::play_ledger::Quality;
    use serde_json::json;
    use std::collections::HashMap;

    fn ctx() -> MatchContext {
        MatchContext {
            map: "de_mirage".to_string(),
            tracked: 1,
            names: HashMap::from([
                (1, "me".to_string()),
                (2, "Sam".to_string()),
                (9, "Kit".to_string()),
            ]),
            score: (13, 9),
            tracked_result: Some("win".to_string()),
            total_deaths: 10,
            class_13_share_pct: 20.0,
        }
    }

    fn p(kind: &str, facts: serde_json::Value, quality: Option<Quality>) -> Play {
        Play {
            tick: 3000,
            phase: "mid".to_string(),
            kind: kind.to_string(),
            facts,
            quality,
            rule_id: None,
            delta_p: None,
        }
    }

    #[test]
    fn setup_names_the_place_and_the_nearest_teammate() {
        let t = narrate_play(
            &p(
                "setup",
                json!({"place": "BombsiteA", "nearest_teammate": "2", "nearest_teammate_dist": 512.0, "teammates_within_isolation": 1, "teammates_alive": 4}),
                None,
            ),
            &ctx(),
        );
        assert_eq!(
            t.headline,
            format!("Setup at {}", callout_name("BombsiteA"))
        );
        assert_eq!(
            t.facts,
            vec![
                "Nearest teammate Sam, 512 u".to_string(),
                "1 of 4 teammates within 900 u".to_string()
            ]
        );
    }

    #[test]
    fn flash_headlines_follow_the_measure() {
        let good = narrate_play(
            &p(
                "flash",
                json!({"enemies_blinded": 2, "teammates_blinded": 0, "self_blind": false, "converted": true}),
                Some(Quality::Good),
            ),
            &ctx(),
        );
        assert_eq!(good.headline, "Flash: 2 enemies blinded");
        assert!(good
            .facts
            .contains(&"Converted into a kill within 2 s".to_string()));
        let bad = narrate_play(
            &p(
                "flash",
                json!({"enemies_blinded": 1, "teammates_blinded": 1, "teammate_ids": ["2"], "self_blind": false, "converted": false}),
                Some(Quality::Bad),
            ),
            &ctx(),
        );
        assert_eq!(bad.headline, "Flash blinded your team");
        assert!(bad.facts.contains(&"Blinded Sam".to_string()));
        let dud = narrate_play(
            &p(
                "flash",
                json!({"enemies_blinded": 0, "teammates_blinded": 0, "self_blind": false, "converted": false}),
                Some(Quality::Neutral),
            ),
            &ctx(),
        );
        assert_eq!(dud.headline, "Flash: nobody blinded");
    }

    #[test]
    fn trades_rushes_and_rotations_read_numbers_first() {
        let t = narrate_play(
            &p(
                "missed_trade",
                json!({"teammate": "2", "killer": "9", "distance": 430.0, "committed": false, "traded_by_me": false, "traded_by_team": false, "window_s": 2.0}),
                Some(Quality::Bad),
            ),
            &ctx(),
        );
        assert_eq!(t.headline, "Didn't trade Sam");
        assert_eq!(
            t.facts[0],
            "Kit killed them 430 u from you; no shot from you in 2 s"
        );
        let r = narrate_play(
            &p(
                "rush",
                json!({"seconds_in": 4.0, "distance": 960.0, "nearest_teammate": "2", "nearest_teammate_dist": 1400.0, "died_in_window": true, "place": "TopofMid"}),
                Some(Quality::Bad),
            ),
            &ctx(),
        );
        assert_eq!(r.headline, "Rushed 960 u by 4 s");
        assert_eq!(
            r.facts,
            vec![
                "Nearest teammate Sam, 1,400 u".to_string(),
                "Died inside the first 20 s".to_string()
            ]
        );
        let rot = narrate_play(
            &p(
                "rotation",
                json!({"distance_at_plant": 3000.0, "at_site": false, "arrived_s": 10.0, "died_before_arrival": false, "deadline_s": 25.0, "planter": "9"}),
                None,
            ),
            &ctx(),
        );
        assert_eq!(rot.headline, "Rotated to the plant in 10 s");
        assert_eq!(rot.facts[0], "3,000 u from the plant when it went down");
    }

    #[test]
    fn kill_and_death_reuse_the_rail_voice_and_add_the_duel_numbers() {
        let k = narrate_play(
            &p(
                "kill",
                json!({"victim": "9", "weapon": "weapon_ak47", "headshot": true, "killer_distance": 812.0, "team_kill": false, "thru_smoke": false, "wallbang": false, "while_blind": false, "man_context": "3v4"}),
                None,
            ),
            &ctx(),
        );
        assert_eq!(k.headline, "Killed Kit");
        assert!(k.facts.contains(&"812 u, ak47, headshot".to_string()));
        assert!(k.facts.contains(&"3v4 before".to_string()));
        let d = narrate_play(
            &p(
                "death",
                json!({"victim": "1", "killer": "9", "weapon": "weapon_awp", "headshot": false, "killer_distance": 1500.0, "traded": true, "nearest_teammate": "2", "man_context": "2v2", "round_end_delta_s": 12.0, "thru_smoke": false, "wallbang": false}),
                Some(Quality::Neutral),
            ),
            &ctx(),
        );
        assert_eq!(d.headline, "Died to Kit");
        assert!(d.facts.contains(&"1,500 u, awp".to_string()));
        assert!(d.facts.contains(&"Traded within 2 s".to_string()));
    }

    #[test]
    fn outcome_line() {
        let o = narrate_play(
            &p(
                "outcome",
                json!({"won": false, "survived": false, "reason": "bomb_exploded", "my_alive": 0, "their_alive": 2, "kills": 1, "damage": 143, "side": "CT"}),
                None,
            ),
            &ctx(),
        );
        assert_eq!(o.headline, "Round lost — bomb exploded");
        assert_eq!(
            o.facts,
            vec![
                "0v2 at the end".to_string(),
                "1 kill, 143 damage".to_string()
            ]
        );
    }
}
