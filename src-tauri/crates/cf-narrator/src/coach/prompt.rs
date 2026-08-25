//! Prompt rendering + response schemas. `render_round_block` is the single
//! grounding source: the validator (validate.rs) builds its allowed sets
//! from exactly this text, so anything the coach may cite must be rendered
//! here, and nothing rendered here may be a guess.

use serde_json::{json, Value};

use super::types::{MatchInput, RoundInput, SynthesisInput};

pub const DEFAULT_ROUND_MODEL: &str = "gemini-3.7-flash";
pub const DEFAULT_SYNTHESIS_MODEL: &str = "gemini-3.7-flash";
/// Bump when the persona, rendering, schema or validator changes — cached
/// responses under an older style are regenerated. v2: word-boundary
/// grounding and "Round N" digests (V1.3 final-review fixes); `ok` rows
/// validated under v1's looser rules must not survive. v3: tick labels
/// confined to the plays array and the match header renders a pretty map
/// name instead of the slug — rows written under v2's prose leaked both.
pub const STYLE_VERSION: &str = "coach-v3";
pub const ROUNDS_PER_CALL: usize = 6;

pub const SYSTEM_PERSONA: &str = "You are ClutchFactor's coach: a calm, experienced CS2 coach reviewing one player's demo with them. \
You are given the FACTS of each round — the player's plays with measured numbers, and a timeline of what everyone did. \
Your job is judgment: read the round like a coach watching the tape, decide what mattered, say what was good, what was a mistake, and what to do instead. \
Use your own CS2 knowledge freely for interpretation and advice (positioning, utility usage, timing, trading, discipline). \
\n\nHard rules:\n\
- Cite ONLY facts that appear in the provided text: numbers, names, callouts, times and events. Never invent a number, a name, a place, or an event that is not there. If the facts do not say, do not claim.\n\
- Put a play's tick number ONLY in the plays array's tick field. In read, why_it_mattered, what_to_practise and focus, never write tick labels — refer to moments by their clock time (for example \"at +40 s\") or by what happened.\n\
- Use place names exactly as they are written in the facts. Never use map codes or internal ids.\n\
- Voice: numbers first, then the fix. Be specific — name the callout, the teammate, the time. No exclamation marks. Never scold; describe what happened and what to change. Positioning that is merely uncommon is 'unusual, not wrong'. No economy or buy advice.\n\
- 'read' is 2 to 4 sentences of live commentary on the round. 'plays' comments only the plays worth a note (good or bad). 'why_it_mattered' and 'what_to_practise' are one sentence each or null. 'focus' is the single most useful takeaway, or null.\n\
- Answer with JSON matching the schema, nothing else.";

/// "de_mirage" → "Mirage". Same rule as `templates::map_name` and
/// `src/lib/mapName.ts` — keep all three in sync. Unlike `templates::map_name`
/// this always returns a `String` (empty in, empty out) since the coach
/// header has no "no map" case to omit.
pub fn map_display_name(slug: &str) -> String {
    let raw = slug.trim();
    if raw.is_empty() {
        return String::new();
    }
    let stripped = raw
        .strip_prefix("de_")
        .or_else(|| raw.strip_prefix("cs_"))
        .or_else(|| raw.strip_prefix("ar_"))
        .unwrap_or(raw);
    let spaced = stripped.replace('_', " ");
    let mut c = spaced.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn render_round_block(m: &MatchInput, r: &RoundInput) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "## Round {} — {} side, {} · verdict: {} · impact {}{}% · you {}-{}{}\n",
        r.round,
        r.side,
        if r.won { "won" } else { "lost" },
        r.verdict_label,
        if r.impact_pct >= 0 { "+" } else { "" },
        r.impact_pct,
        r.kills,
        r.deaths,
        r.man_context
            .as_ref()
            .map(|mc| format!(" · {mc} at the pivotal moment"))
            .unwrap_or_default(),
    ));
    if !r.prior_digest.is_empty() {
        s.push_str("Earlier this match: ");
        s.push_str(&r.prior_digest.join("; "));
        s.push('\n');
    }
    s.push_str(&format!("### {}'s plays\n", m.tracked_name));
    for p in &r.plays {
        s.push_str(&format!("- [tick {}] {} · {}", p.tick, p.clock, p.headline));
        if let Some(q) = &p.quality {
            s.push_str(&format!(" ({q})"));
        }
        for f in &p.facts {
            s.push_str(&format!(" — {f}"));
        }
        s.push('\n');
    }
    if !r.timeline.is_empty() {
        s.push_str("### Timeline (everyone)\n");
        for t in &r.timeline {
            s.push_str(&format!("- {t}\n"));
        }
    }
    s
}

pub fn render_round_batch(m: &MatchInput, rounds: &[RoundInput], retry_notes: &[String]) -> String {
    let mut s = format!(
        "# Match: {} · final score {}-{} · reviewing {}{}\nPlayers: {}\n\n",
        map_display_name(&m.map),
        m.score.0,
        m.score.1,
        m.tracked_name,
        m.tracked_result
            .as_ref()
            .map(|r| format!(" ({r})"))
            .unwrap_or_default(),
        m.roster.join(", "),
    );
    for r in rounds {
        s.push_str(&render_round_block(m, r));
        s.push('\n');
    }
    s.push_str(
        "Return JSON: {\"rounds\": [{\"round\", \"read\", \"plays\": [{\"tick\", \"comment\"}], \"why_it_mattered\", \"what_to_practise\", \"focus\"}]} with one entry per round above, in order.\n",
    );
    if !retry_notes.is_empty() {
        s.push_str("\nYour previous answer was rejected because it cited things that are not in the facts. Rewrite those rounds using only the facts above:\n");
        for n in retry_notes {
            s.push_str(n);
            s.push('\n');
        }
    }
    s
}

pub fn render_synthesis(si: &SynthesisInput) -> String {
    let m = &si.match_input;
    let mut s = format!(
        "# Match: {} · final score {}-{} · reviewing {}{}\nPlayers: {}\n\n## Round by round\n",
        map_display_name(&m.map),
        m.score.0,
        m.score.1,
        m.tracked_name,
        m.tracked_result
            .as_ref()
            .map(|r| format!(" ({r})"))
            .unwrap_or_default(),
        m.roster.join(", "),
    );
    // "Round 11", not "R11": the tokenizer only grounds a digit that stands
    // alone, so this spelling is what lets the coach cite a round by number.
    for r in &si.rounds {
        s.push_str(&format!(
            "- Round {} · {} · {}: {}\n",
            r.round,
            r.verdict_label,
            if r.won { "won" } else { "lost" },
            r.read
        ));
    }
    if !si.insights.is_empty() {
        s.push_str("\n## Match-level findings (from the detectors)\n");
        for i in &si.insights {
            s.push_str(&format!("- {i}\n"));
        }
    }
    if !si.habits.is_empty() {
        s.push_str("\n## Habits across recent matches\n");
        for h in &si.habits {
            s.push_str(&format!("- {h}\n"));
        }
    }
    s.push_str(
        "\nReturn JSON: {\"opening\": <3-5 sentences: the coach's opening statement on this match — what decided it for this player, what was good, the one pattern to fix>, \"work_on\": [<1 to 3 short, concrete practice items>]}\n",
    );
    s
}

pub fn round_batch_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "rounds": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "round": { "type": "integer" },
                        "read": { "type": "string" },
                        "plays": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "tick": { "type": "integer" },
                                    "comment": { "type": "string" }
                                },
                                "required": ["tick", "comment"]
                            }
                        },
                        "why_it_mattered": { "type": "string", "nullable": true },
                        "what_to_practise": { "type": "string", "nullable": true },
                        "focus": { "type": "string", "nullable": true }
                    },
                    "required": ["round", "read", "plays"]
                }
            }
        },
        "required": ["rounds"]
    })
}

pub fn synthesis_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "opening": { "type": "string" },
            "work_on": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["opening", "work_on"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::types::*;

    fn m() -> MatchInput {
        MatchInput {
            map: "de_mirage".to_string(),
            score: (13, 9),
            tracked_name: "misosoupy3".to_string(),
            tracked_result: Some("win".to_string()),
            roster: vec!["misosoupy3".into(), "Sam".into(), "Kit".into()],
        }
    }

    fn r() -> RoundInput {
        RoundInput {
            round: 6,
            side: "CT".to_string(),
            won: false,
            verdict_label: "Not on you".to_string(),
            impact_pct: -23,
            man_context: Some("3v5".to_string()),
            kills: 1,
            deaths: 1,
            plays: vec![
                PlayLine {
                    tick: 26752,
                    clock: "+5 s".into(),
                    kind: "setup".into(),
                    headline: "Setup at B Site".into(),
                    facts: vec!["Nearest teammate Sam, 159 u".into()],
                    quality: None,
                },
                PlayLine {
                    tick: 29000,
                    clock: "+40 s".into(),
                    kind: "death".into(),
                    headline: "Died to Kit".into(),
                    facts: vec!["812 u, ak47".into(), "3v5 before".into()],
                    quality: Some("neutral".into()),
                },
            ],
            timeline: vec!["+38 s Sam killed Kit (m4a1)".into()],
            prior_digest: vec!["Round 5 · Quiet · won".into()],
        }
    }

    #[test]
    fn round_block_carries_every_fact_the_validator_will_allow() {
        let block = render_round_block(&m(), &r());
        for needle in [
            "Round 6",
            "CT",
            "lost",
            "Not on you",
            "-23%",
            "3v5",
            "+5 s",
            "Setup at B Site",
            "Nearest teammate Sam, 159 u",
            "[tick 26752]",
            "812 u, ak47",
            "Died to Kit",
            "+38 s Sam killed Kit (m4a1)",
            "Round 5 · Quiet · won",
        ] {
            assert!(block.contains(needle), "missing {needle:?} in:\n{block}");
        }
    }

    #[test]
    fn synthesis_prompt_spells_rounds_out_so_the_number_is_grounded() {
        let si = SynthesisInput {
            match_input: m(),
            rounds: vec![RoundDigest {
                round: 11,
                verdict_label: "Cost you".into(),
                won: false,
                read: "Died first at Connector.".into(),
            }],
            insights: vec![],
            habits: vec![],
        };
        let p = render_synthesis(&si);
        assert!(
            p.contains("- Round 11 · Cost you · lost: Died first at Connector."),
            "{p}"
        );
        assert!(!p.contains("R11"), "{p}");
        assert_eq!(STYLE_VERSION, "coach-v3");
    }

    #[test]
    fn batch_prompt_lists_rounds_in_order_and_appends_retry_notes() {
        let mut r2 = r();
        r2.round = 7;
        let p = render_round_batch(
            &m(),
            &[r(), r2],
            &["Round 6: the number 1,500 is not in the facts.".to_string()],
        );
        let i6 = p.find("## Round 6").unwrap();
        let i7 = p.find("## Round 7").unwrap();
        assert!(i6 < i7);
        assert!(p.contains("Mirage") && p.contains("13-9"));
        assert!(!p.contains("de_mirage"), "{p}");
        assert!(
            p.ends_with("Round 6: the number 1,500 is not in the facts.\n")
                || p.contains("Round 6: the number 1,500 is not in the facts.")
        );
        assert!(p.contains("Return JSON"));
    }

    #[test]
    fn schemas_are_objects_with_required_fields() {
        let s = round_batch_schema();
        assert_eq!(s["type"], "object");
        assert_eq!(s["properties"]["rounds"]["type"], "array");
        let item = &s["properties"]["rounds"]["items"];
        for f in [
            "round",
            "read",
            "plays",
            "why_it_mattered",
            "what_to_practise",
            "focus",
        ] {
            assert!(item["properties"].get(f).is_some(), "missing {f}");
        }
        assert!(item["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "read"));
        let s = synthesis_schema();
        assert_eq!(s["properties"]["work_on"]["type"], "array");
    }

    #[test]
    fn persona_states_the_grounding_rule_and_the_voice_rules() {
        for needle in [
            "only",
            "facts",
            "exclamation",
            "never scold",
            "numbers first",
            "only in the plays array",
            "map codes",
        ] {
            assert!(
                SYSTEM_PERSONA.to_lowercase().contains(needle),
                "persona lacks {needle}"
            );
        }
    }

    #[test]
    fn map_display_name_strips_prefix_and_capitalizes_first_char_only() {
        assert_eq!(map_display_name("de_mirage"), "Mirage");
        assert_eq!(map_display_name("de_dust2"), "Dust2");
        assert_eq!(map_display_name("cs_office"), "Office");
        assert_eq!(map_display_name(""), "");
    }
}
