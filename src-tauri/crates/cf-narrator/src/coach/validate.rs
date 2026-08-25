//! Grounding validator (spec §3, ship-blocking): a response may cite only
//! numbers, names, callouts, round numbers and ticks that appear in the
//! text the model was shown. Opinions and advice are never checked. Any
//! number the validator lets through that is not in the facts is a bug.

use std::collections::HashSet;

use super::types::{MatchSynthesis, RoundCommentary};

#[derive(Debug, Clone, PartialEq)]
pub struct Grounding {
    pub numbers: HashSet<String>,
    /// Roster names that appear in the grounding text (case-sensitive).
    pub names: Vec<String>,
    /// Known callouts that appear in the grounding text.
    pub callouts: Vec<String>,
    /// Every roster name, so absent ones can be flagged when cited.
    pub roster: Vec<String>,
    /// Every known callout — everywhere anyone stood this match (the
    /// ledger's places ∪ the position samples) — so absent ones can be
    /// flagged. A callout nobody visited is invisible to the validator.
    pub known_callouts: Vec<String>,
    pub ticks: HashSet<i32>,
    pub round: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    Number,
    Name,
    Callout,
    Tick,
    Voice,
    Empty,
    Round,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    pub field: String,
    pub kind: ViolationKind,
    pub token: String,
}

impl Grounding {
    pub fn for_round(
        block_text: &str,
        roster: &[String],
        known_callouts: &[String],
        ticks: &[i32],
        round: u32,
    ) -> Self {
        let mut numbers: HashSet<String> = number_tokens(block_text).into_iter().collect();
        numbers.insert(round.to_string());
        for t in ticks {
            numbers.insert(t.to_string());
        }
        Grounding {
            numbers,
            names: roster
                .iter()
                .filter(|n| contains_term(block_text, n))
                .cloned()
                .collect(),
            callouts: known_callouts
                .iter()
                .filter(|c| contains_term(block_text, c))
                .cloned()
                .collect(),
            roster: roster.to_vec(),
            known_callouts: known_callouts.to_vec(),
            ticks: ticks.iter().copied().collect(),
            round,
        }
    }

    pub fn for_synthesis(prompt_text: &str, roster: &[String], known_callouts: &[String]) -> Self {
        Grounding {
            numbers: number_tokens(prompt_text).into_iter().collect(),
            names: roster
                .iter()
                .filter(|n| contains_term(prompt_text, n))
                .cloned()
                .collect(),
            callouts: known_callouts
                .iter()
                .filter(|c| contains_term(prompt_text, c))
                .cloned()
                .collect(),
            roster: roster.to_vec(),
            known_callouts: known_callouts.to_vec(),
            ticks: HashSet::new(),
            round: 0,
        }
    }
}

/// Every number-like token in `text`, normalized: signs, commas and `%`
/// dropped; `3v5` yields "3v5", "3" and "5"; `0:45` stays one token;
/// decimals keep their point. Order of appearance.
///
/// A token may only START at a digit that is not itself preceded by an
/// ASCII alphanumeric character — otherwise digits embedded in identifiers
/// ("misosoupy3", "ak47", "m4a1", "mp9") would leak invented-looking
/// numbers into the grounding set. Letting such a digit through would be
/// the exact ship-blocking bug this validator exists to prevent.
pub fn number_tokens(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() && (i == 0 || !chars[i - 1].is_ascii_alphanumeric()) {
            let start = i;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || chars[i] == ','
                    || chars[i] == '.'
                    || chars[i] == ':'
                    || chars[i] == 'v')
            {
                // `v` only continues a token when it sits strictly between
                // digits (3v5) — a trailing `v` (or one not preceded by a
                // digit) ends the token instead of being consumed.
                if chars[i] == 'v'
                    && !(i > start
                        && chars[i - 1].is_ascii_digit()
                        && i + 1 < chars.len()
                        && chars[i + 1].is_ascii_digit())
                {
                    break;
                }
                i += 1;
            }
            let raw: String = chars[start..i].iter().collect();
            let raw = raw.trim_end_matches(['.', ',', ':']).to_string();
            if let Some((a, b)) = raw.split_once('v') {
                out.push(raw.clone());
                out.push(a.to_string());
                out.push(b.to_string());
            } else {
                out.push(raw.replace(',', ""));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// `needle` occurs in `hay` as a whole term: the characters immediately
/// before and after the match are not ASCII alphanumeric (start and end of
/// string count as boundaries). Every occurrence is tried, not just the
/// first, so "CT spawn … T spawn" still finds the standalone one. Plain
/// `str::contains` let "CT spawn" ground "T spawn" and "Kitchen" ground a
/// player called "Kit" — on both the grounding and the mention side.
fn contains_term(hay: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut from = 0;
    while let Some(pos) = hay[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        let before_ok = hay[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        let after_ok = hay[end..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        if before_ok && after_ok {
            return true;
        }
        from = start + hay[start..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

fn check_text(field: &str, text: &str, g: &Grounding, out: &mut Vec<Violation>) {
    for n in number_tokens(text) {
        if !g.numbers.contains(&n) {
            out.push(Violation {
                field: field.to_string(),
                kind: ViolationKind::Number,
                token: n,
            });
        }
    }
    for name in &g.roster {
        if contains_term(text, name) && !g.names.contains(name) {
            out.push(Violation {
                field: field.to_string(),
                kind: ViolationKind::Name,
                token: name.clone(),
            });
        }
    }
    for c in &g.known_callouts {
        if contains_term(text, c) && !g.callouts.contains(c) {
            out.push(Violation {
                field: field.to_string(),
                kind: ViolationKind::Callout,
                token: c.clone(),
            });
        }
    }
    if text.contains('!') {
        out.push(Violation {
            field: field.to_string(),
            kind: ViolationKind::Voice,
            token: "!".to_string(),
        });
    }
}

pub fn validate_round(c: &RoundCommentary, g: &Grounding) -> Vec<Violation> {
    let mut out = vec![];
    if c.round != g.round {
        out.push(Violation {
            field: "round".into(),
            kind: ViolationKind::Round,
            token: c.round.to_string(),
        });
    }
    if c.read.trim().is_empty() {
        out.push(Violation {
            field: "read".into(),
            kind: ViolationKind::Empty,
            token: String::new(),
        });
    }
    check_text("read", &c.read, g, &mut out);
    for (i, p) in c.plays.iter().enumerate() {
        if !g.ticks.contains(&p.tick) {
            out.push(Violation {
                field: format!("plays[{i}].tick"),
                kind: ViolationKind::Tick,
                token: p.tick.to_string(),
            });
        }
        check_text(&format!("plays[{i}].comment"), &p.comment, g, &mut out);
    }
    for (field, v) in [
        ("why_it_mattered", &c.why_it_mattered),
        ("what_to_practise", &c.what_to_practise),
        ("focus", &c.focus),
    ] {
        if let Some(t) = v {
            check_text(field, t, g, &mut out);
        }
    }
    out
}

pub fn validate_synthesis(s: &MatchSynthesis, g: &Grounding) -> Vec<Violation> {
    let mut out = vec![];
    if s.opening.trim().is_empty() {
        out.push(Violation {
            field: "opening".into(),
            kind: ViolationKind::Empty,
            token: String::new(),
        });
    }
    check_text("opening", &s.opening, g, &mut out);
    for (i, w) in s.work_on.iter().enumerate() {
        check_text(&format!("work_on[{i}]"), w, g, &mut out);
    }
    out
}

/// One "…is not in the facts" item per violation — shared wording for the
/// round and synthesis retry notes.
fn violation_items(round: u32, v: &[Violation]) -> Vec<String> {
    v.iter()
        .map(|x| match x.kind {
            ViolationKind::Number => {
                format!(
                    "the number {} (in {}) is not in the facts",
                    x.token, x.field
                )
            }
            ViolationKind::Name => format!(
                "the player {} (in {}) is not in this round's facts",
                x.token, x.field
            ),
            ViolationKind::Callout => format!(
                "the callout {} (in {}) is not in the facts",
                x.token, x.field
            ),
            ViolationKind::Tick => {
                format!("tick {} (in {}) is not one of the plays", x.token, x.field)
            }
            ViolationKind::Voice => format!("no exclamation marks (in {})", x.field),
            ViolationKind::Empty => format!("{} must not be empty", x.field),
            ViolationKind::Round => format!("round must be {round}, not {}", x.token),
        })
        .collect()
}

/// "Round 6: the number 1500 (in read) is not in the facts; the callout
/// Apartments (in focus) is not in the facts."
pub fn retry_note(round: u32, v: &[Violation]) -> String {
    format!("Round {round}: {}.", violation_items(round, v).join("; "))
}

/// The synthesis has no round number: "The match read: the number 9 (in
/// opening) is not in the facts." — same items as `retry_note`.
pub fn synthesis_retry_note(v: &[Violation]) -> String {
    format!("The match read: {}.", violation_items(0, v).join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::prompt::render_round_block;
    use crate::coach::types::*;

    fn m() -> MatchInput {
        MatchInput {
            map: "de_inferno".into(),
            score: (8, 13),
            tracked_name: "misosoupy3".into(),
            tracked_result: Some("loss".into()),
            roster: vec![
                "misosoupy3".into(),
                "SirEggsAlot".into(),
                "Konky".into(),
                "MyUnit".into(),
            ],
        }
    }
    fn r() -> RoundInput {
        RoundInput {
            round: 6,
            side: "CT".into(),
            won: false,
            verdict_label: "Not on you".into(),
            impact_pct: -23,
            man_context: Some("3v5".into()),
            kills: 0,
            deaths: 1,
            plays: vec![
                PlayLine {
                    tick: 26752,
                    clock: "+5 s".into(),
                    kind: "setup".into(),
                    headline: "Setup at B Site".into(),
                    facts: vec![
                        "Nearest teammate SirEggsAlot, 159 u".into(),
                        "1 of 4 teammates within 900 u".into(),
                    ],
                    quality: None,
                },
                PlayLine {
                    tick: 27100,
                    clock: "+10 s".into(),
                    kind: "flash".into(),
                    headline: "Flash: 2 enemies blinded".into(),
                    facts: vec!["Converted into a kill within 2 s".into()],
                    quality: Some("good".into()),
                },
                PlayLine {
                    tick: 29000,
                    clock: "+40 s".into(),
                    kind: "death".into(),
                    headline: "Died to Konky".into(),
                    facts: vec![
                        "1,436 u, awp".into(),
                        "Traded — round continued 12 s after".into(),
                        "3v5 before".into(),
                    ],
                    quality: Some("neutral".into()),
                },
            ],
            timeline: vec!["+38 s SirEggsAlot killed MyUnit (m4a1)".into()],
            prior_digest: vec![],
        }
    }
    fn g() -> Grounding {
        let block = render_round_block(&m(), &r());
        Grounding::for_round(
            &block,
            &m().roster,
            &["B Site".into(), "Banana".into(), "Apartments".into()],
            &[26752, 27100, 29000],
            6,
        )
    }
    fn ok() -> RoundCommentary {
        RoundCommentary {
            round: 6,
            read: "You set up at B Site 159 u from SirEggsAlot, which is tight for a 3v5 hold. The flash at +10 s blinded 2 enemies and turned into a kill. Dying to Konky's awp from 1,436 u was traded 12 s later, so this one is not on you.".into(),
            plays: vec![PlayComment { tick: 27100, comment: "Good flash: 2 blinded, converted within 2 s.".into() }],
            why_it_mattered: Some("A 3v5 was already lost before your death.".into()),
            what_to_practise: Some("Hold one step deeper against an awp on B Site.".into()),
            focus: Some("Keep throwing that flash.".into()),
        }
    }

    #[test]
    fn a_grounded_answer_passes() {
        assert!(validate_round(&ok(), &g()).is_empty());
    }

    #[test]
    fn number_tokens_normalize_units_signs_commas_and_man_context() {
        assert_eq!(
            number_tokens("1,436 u, -23%, +12 s, 3v5, 0:45, 31.3"),
            vec!["1436", "23", "12", "3v5", "3", "5", "0:45", "31.3"]
        );
    }

    #[test]
    fn an_invented_number_is_rejected() {
        let mut c = ok();
        c.read = "You were 1,500 u from SirEggsAlot.".into();
        let v = validate_round(&c, &g());
        assert_eq!(v.len(), 1);
        assert!(matches!(v[0].kind, ViolationKind::Number));
        assert_eq!(v[0].token, "1500");
        assert_eq!(v[0].field, "read");
    }

    #[test]
    fn an_invented_name_is_rejected_even_if_on_the_roster() {
        // MyUnit is on the roster and appears in this round's timeline — allowed.
        let mut c = ok();
        c.focus = Some("MyUnit pushed too early.".into());
        assert!(validate_round(&c, &g()).is_empty());
        // A roster name absent from this round's facts is not.
        let mut gr = g();
        gr.names.retain(|n| n != "MyUnit");
        let v = validate_round(&c, &gr);
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Name) && x.token == "MyUnit"));
    }

    #[test]
    fn a_callout_not_in_this_rounds_facts_is_rejected() {
        let mut c = ok();
        c.what_to_practise = Some("Rotate through Apartments earlier.".into());
        let v = validate_round(&c, &g());
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Callout) && x.token == "Apartments"));
        c.what_to_practise = Some("Hold B Site one step deeper.".into());
        assert!(validate_round(&c, &g()).is_empty());
    }

    #[test]
    fn a_tick_that_is_not_a_play_is_rejected() {
        let mut c = ok();
        c.plays.push(PlayComment {
            tick: 28000,
            comment: "Nice.".into(),
        });
        let v = validate_round(&c, &g());
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Tick) && x.token == "28000"));
    }

    #[test]
    fn wrong_round_number_empty_read_and_exclamation_marks_are_rejected() {
        let mut c = ok();
        c.round = 7;
        assert!(validate_round(&c, &g())
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Round)));
        let mut c = ok();
        c.read = "   ".into();
        assert!(validate_round(&c, &g())
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Empty)));
        let mut c = ok();
        c.read = "Great flash!".into();
        assert!(validate_round(&c, &g())
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Voice)));
    }

    #[test]
    fn small_counts_and_the_round_number_are_allowed_because_they_are_in_the_block() {
        let mut c = ok();
        c.read = "Round 6: 1 of 4 teammates within 900 u, 2 enemies blinded, 0 kills.".into();
        assert!(
            validate_round(&c, &g()).is_empty(),
            "{:?}",
            validate_round(&c, &g())
        );
    }

    #[test]
    fn retry_note_lists_the_offending_tokens() {
        let v = vec![
            Violation {
                field: "read".into(),
                kind: ViolationKind::Number,
                token: "1500".into(),
            },
            Violation {
                field: "focus".into(),
                kind: ViolationKind::Callout,
                token: "Apartments".into(),
            },
        ];
        assert_eq!(retry_note(6, &v), "Round 6: the number 1500 (in read) is not in the facts; the callout Apartments (in focus) is not in the facts.");
    }

    #[test]
    fn synthesis_retry_note_lists_the_offending_tokens_without_a_round() {
        let v = vec![
            Violation {
                field: "opening".into(),
                kind: ViolationKind::Number,
                token: "9".into(),
            },
            Violation {
                field: "work_on[0]".into(),
                kind: ViolationKind::Callout,
                token: "Apartments".into(),
            },
        ];
        assert_eq!(
            synthesis_retry_note(&v),
            "The match read: the number 9 (in opening) is not in the facts; the callout Apartments (in work_on[0]) is not in the facts."
        );
    }

    /// V1.3 final-review fix #1: the digest and the synthesis round list
    /// render "Round 5", so the tokenizer grounds the bare "5" and a coach
    /// that says "Round 5 went the same way" is not rejected.
    #[test]
    fn a_prior_round_cited_by_number_is_grounded() {
        let mut r5 = r();
        r5.prior_digest = vec!["Round 5 · Quiet · won".into()];
        let block = render_round_block(&m(), &r5);
        let g = Grounding::for_round(&block, &m().roster, &[], &[26752, 27100, 29000], 6);
        let c = RoundCommentary {
            round: 6,
            read: "Round 5 went the same way.".into(),
            plays: vec![],
            why_it_mattered: None,
            what_to_practise: None,
            focus: None,
        };
        assert_eq!(validate_round(&c, &g), vec![]);
        // …and a round that was never digested is still an invention ("8"
        // appears nowhere in the fixture's facts).
        let mut c = c;
        c.read = "Round 8 went the same way.".into();
        let v = validate_round(&c, &g);
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Number) && x.token == "8"));
    }

    /// V1.3 final-review fix #2: grounding and mentions match whole terms
    /// only. On the owner's Mirage demo "CT spawn" in the facts let an
    /// invented "T spawn" through; a roster name inside a longer word or
    /// name grounded the same way.
    #[test]
    fn callouts_and_names_match_only_at_word_boundaries() {
        let roster: Vec<String> = vec!["misosoupy3".into(), "Kit".into()];
        let known: Vec<String> = vec!["CT spawn".into(), "T spawn".into()];
        let block = "## Round 6 — CT side\n- [tick 1] +5 s · Setup at CT spawn — Nearest teammate Kitchen, 97 u\n";
        let g = Grounding::for_round(block, &roster, &known, &[1], 6);
        assert_eq!(g.callouts, vec!["CT spawn".to_string()]);
        assert!(
            g.names.is_empty(),
            "Kitchen must not ground Kit: {:?}",
            g.names
        );

        let say = |read: &str| RoundCommentary {
            round: 6,
            read: read.into(),
            plays: vec![],
            why_it_mattered: None,
            what_to_practise: None,
            focus: None,
        };
        // An invented "T spawn" is a callout violation…
        let v = validate_round(&say("You died at T spawn."), &g);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(matches!(v[0].kind, ViolationKind::Callout) && v[0].token == "T spawn");
        // …while the legitimate "CT spawn" passes (its inner "T spawn" is not a mention).
        assert_eq!(validate_round(&say("You set up at CT spawn."), &g), vec![]);
        // Mentioning Kitchen is not citing Kit.
        assert_eq!(validate_round(&say("Hold Kitchen longer."), &g), vec![]);
        // The possessive still counts as a mention (the apostrophe is a boundary).
        let v = validate_round(&say("Kit's push was early."), &g);
        assert!(
            v.iter()
                .any(|x| matches!(x.kind, ViolationKind::Name) && x.token == "Kit"),
            "{v:?}"
        );
        // Once Kit is in the facts, the possessive is fine.
        let block2 = format!("{block}- +38 s Kit planted the bomb\n");
        let g2 = Grounding::for_round(&block2, &roster, &known, &[1], 6);
        assert_eq!(g2.names, vec!["Kit".to_string()]);
        assert_eq!(validate_round(&say("Kit's push was early."), &g2), vec![]);
    }

    #[test]
    fn contains_term_boundaries_and_multiple_occurrences() {
        assert!(contains_term("at T spawn", "T spawn"));
        assert!(!contains_term("at CT spawn", "T spawn"));
        assert!(contains_term("CT spawn then T spawn", "T spawn"));
        assert!(contains_term("Kit's", "Kit"));
        assert!(!contains_term("Kitchen", "Kit"));
        assert!(!contains_term("aKit", "Kit"));
        assert!(contains_term("nekoo鸭 killed", "nekoo鸭"));
        assert!(contains_term("(Kit)", "Kit"));
        assert!(!contains_term("anything", ""));
    }

    #[test]
    fn synthesis_is_grounded_against_its_own_prompt() {
        let si = SynthesisInput {
            match_input: m(),
            rounds: vec![RoundDigest {
                round: 6,
                verdict_label: "Not on you".into(),
                won: false,
                read: ok().read,
            }],
            insights: vec!["Isolated deaths: 5 of 12 deaths, most often at Banana".into()],
            habits: vec![],
        };
        let text = crate::coach::prompt::render_synthesis(&si);
        let g =
            Grounding::for_synthesis(&text, &m().roster, &["Banana".into(), "Apartments".into()]);
        let good = MatchSynthesis {
            opening: "5 of 12 deaths were isolated, most at Banana; Round 6 was not on you.".into(),
            work_on: vec!["Arrive at Banana with a teammate.".into()],
        };
        assert!(validate_synthesis(&good, &g).is_empty());
        let bad = MatchSynthesis {
            opening: "9 of 12 deaths were at Apartments.".into(),
            work_on: vec![],
        };
        let v = validate_synthesis(&bad, &g);
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Number) && x.token == "9"));
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Callout) && x.token == "Apartments"));
    }

    #[test]
    fn number_tokens_ignores_digits_embedded_in_identifiers() {
        assert_eq!(
            number_tokens("812 u, ak47 · misosoupy3 killed MyUnit (m4a1), mp9"),
            vec!["812"]
        );
    }

    #[test]
    fn a_number_embedded_in_a_weapon_name_does_not_ground_an_invented_number() {
        // A round whose only digit-bearing fact is "812 u, ak47" — the "47"
        // in the weapon name must never ground an invented "47" elsewhere.
        let weapon_round = RoundInput {
            round: 6,
            side: "CT".into(),
            won: false,
            verdict_label: "Not on you".into(),
            impact_pct: -23,
            man_context: None,
            kills: 0,
            deaths: 1,
            plays: vec![PlayLine {
                tick: 26752,
                clock: "+5 s".into(),
                kind: "death".into(),
                headline: "Died to Konky".into(),
                facts: vec!["812 u, ak47".into()],
                quality: Some("neutral".into()),
            }],
            timeline: vec![],
            prior_digest: vec![],
        };
        let block = render_round_block(&m(), &weapon_round);
        let g2 = Grounding::for_round(&block, &m().roster, &[], &[26752], 6);

        // "812" is a legitimate fact and passes.
        let grounded = RoundCommentary {
            round: 6,
            read: "You were 812 u away.".into(),
            plays: vec![],
            why_it_mattered: None,
            what_to_practise: None,
            focus: None,
        };
        assert!(validate_round(&grounded, &g2).is_empty());

        // "47" only exists inside the weapon name "ak47" and must be rejected.
        let invented = RoundCommentary {
            round: 6,
            read: "You were 47 u away.".into(),
            plays: vec![],
            why_it_mattered: None,
            what_to_practise: None,
            focus: None,
        };
        let v = validate_round(&invented, &g2);
        assert!(v
            .iter()
            .any(|x| matches!(x.kind, ViolationKind::Number) && x.token == "47"));
    }
}
