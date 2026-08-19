//! CoachingNarrator trait + TemplateNarrator (PROMPT.md §8). Built at M4.
//!
//! The seam the future `ClaudeNarrator` drops into. v1 is deterministic
//! templates — see `templates.rs`, which holds the actual coaching text.

mod templates;

pub use templates::narrate_habit;

use std::collections::HashMap;

/// Everything a template needs about the match that isn't in the insight.
#[derive(Debug, Clone)]
pub struct MatchContext {
    pub map: String,
    pub tracked: u64,
    pub names: HashMap<u64, String>,
    pub score: (u32, u32),
    pub tracked_result: Option<String>,
    pub total_deaths: usize,
    pub class_13_share_pct: f32,
}

impl MatchContext {
    /// Display name for a steamid, falling back to the raw id.
    pub fn name(&self, steamid: u64) -> String {
        self.names
            .get(&steamid)
            .cloned()
            .unwrap_or_else(|| steamid.to_string())
    }
}

/// One piece of user-facing coaching text.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Narration {
    pub title: String,
    pub body: String,
}

pub trait CoachingNarrator {
    /// Turn one insight (+ match context) into user-facing coaching text.
    fn narrate(&self, insight: &cf_analysis::Insight, ctx: &MatchContext) -> Narration;
    /// Optional match-level summary from the full insight set.
    fn summarize(&self, insights: &[cf_analysis::Insight], ctx: &MatchContext)
        -> Option<Narration>;
}

/// v1 narrator: deterministic parameterized templates, no network, no cost.
pub struct TemplateNarrator;

impl CoachingNarrator for TemplateNarrator {
    fn narrate(&self, insight: &cf_analysis::Insight, ctx: &MatchContext) -> Narration {
        templates::narrate(insight, ctx)
    }

    fn summarize(
        &self,
        insights: &[cf_analysis::Insight],
        ctx: &MatchContext,
    ) -> Option<Narration> {
        templates::summarize(insights, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_analysis::types::{Category, Insight};
    use serde_json::json;

    const TRACKED: u64 = 76561199228328773;

    fn ctx() -> MatchContext {
        MatchContext {
            map: "de_mirage".to_string(),
            tracked: TRACKED,
            names: HashMap::from([
                (TRACKED, "misosoupy3".to_string()),
                (77, "Riku".to_string()),
            ]),
            score: (10, 13),
            tracked_result: Some("loss".to_string()),
            total_deaths: 19,
            class_13_share_pct: 31.6,
        }
    }

    fn ins(detector: &str, title_data: serde_json::Value, metrics: serde_json::Value) -> Insight {
        Insight {
            detector: detector.to_string(),
            category: Category::Deaths,
            severity: 0.6,
            confidence: 0.8,
            round: 0,
            player: TRACKED,
            title_data,
            metrics,
            evidence: vec![],
        }
    }

    fn per_round(rounds: &[u32]) -> serde_json::Value {
        serde_json::Value::Array(
            rounds
                .iter()
                .map(|r| json!({ "round": r, "tick": 1000 }))
                .collect(),
        )
    }

    fn say(i: &Insight) -> Narration {
        TemplateNarrator.narrate(i, &ctx())
    }

    // ---- H2 -------------------------------------------------------------

    #[test]
    fn isolated_death_variant_a_exact() {
        let n = say(&ins(
            "H2_ISOLATED_DEATH",
            json!({ "count": 5, "rule": "H2_ISOLATED_DEATH" }),
            json!({ "count": 5, "per_round": per_round(&[4, 7, 12, 15, 19]) }),
        ));
        assert_eq!(n.title, "Died isolated 5 times");
        assert_eq!(
            n.body,
            "You died isolated 5 times with no teammate close enough to punish the kill — \
             rounds 4, 7, 12, 15 and 1 more. Take those duels one angle closer to a teammate: \
             arrive together, or hold until someone can trade you."
        );
    }

    #[test]
    fn isolated_death_variant_b_exact() {
        let n = say(&ins(
            "H2_ISOLATED_DEATH",
            json!({ "count": 9, "rule": "H2_ISOLATED_DEATH" }),
            json!({ "count": 9, "per_round": per_round(&[2, 6, 11, 14, 17, 19, 21, 23, 24]) }),
        ));
        assert_eq!(n.title, "9 deaths nobody could trade");
        assert_eq!(
            n.body,
            "Nobody was in range to trade you on 9 of your deaths — rounds 2, 6, 11, 14 and 5 \
             more. Before you take the duel, know who re-peeks for you; if the answer is \
             nobody, hold the angle and make them come to you."
        );
    }

    #[test]
    fn isolated_death_without_metrics_skips_clauses() {
        let n = say(&ins("H2_ISOLATED_DEATH", json!({}), json!({})));
        assert!(!n.title.is_empty() && !n.body.is_empty());
        assert!(!n.body.contains("round"), "no round clause: {}", n.body);
        assert!(
            !n.body.contains(" 0 ") && !n.body.contains("null") && !n.body.contains("{}"),
            "no empty-fact leakage: {}",
            n.body
        );
    }

    #[test]
    fn failed_trade_exact() {
        let n = say(&ins(
            "H2_FAILED_TRADE",
            json!({ "count": 2, "rule": "H2_FAILED_TRADE" }),
            json!({ "count": 2, "per_round": per_round(&[5, 9]) }),
        ));
        assert_eq!(n.title, "2 trades you were in range for");
        assert_eq!(
            n.body,
            "A teammate died inside trade range of you twice and you didn't take the re-peek — \
             rounds 5 and 9. The two seconds after his death are the cheapest kill in the \
             round: move on the sound, not after it."
        );
    }

    #[test]
    fn failed_trade_team_pattern_names_the_team() {
        let n = say(&ins(
            "H2_FAILED_TRADE",
            json!({ "count": 3, "rule": "H2_FAILED_TRADE", "team_pattern": true }),
            json!({ "count": 3, "per_round": per_round(&[5, 9, 14]) }),
        ));
        assert!(
            n.body.ends_with(
                "Baited trades are recurring too, so this is a team spacing problem: decide \
                 who the second man is before the round, not during it."
            ),
            "team clause missing: {}",
            n.body
        );
    }

    #[test]
    fn baited_trade_exact_never_blames() {
        let n = say(&ins(
            "H2_BAITED_TRADE",
            json!({ "count": 4, "rule": "H2_BAITED_TRADE" }),
            json!({ "count": 4, "per_round": per_round(&[3, 8, 11, 16]) }),
        ));
        assert_eq!(n.title, "You traded in, nobody followed");
        assert_eq!(
            n.body,
            "You committed to the trade and the follow-up never came — 4 times, rounds 3, 8, \
             11 and 16. You were the only one who re-peeked; that is a team spacing problem, \
             not a reason to stop trading."
        );
        let lower = n.body.to_lowercase();
        for blame in [
            "your fault",
            "you should have",
            "stop trading in",
            "bad play",
        ] {
            assert!(
                !lower.contains(blame),
                "blame word {blame:?} in: {}",
                n.body
            );
        }
        assert!(lower.contains("team"), "must name the team problem");
    }

    #[test]
    fn baited_trade_team_pattern_adds_the_combination() {
        let n = say(&ins(
            "H2_BAITED_TRADE",
            json!({ "count": 4, "rule": "H2_BAITED_TRADE", "team_pattern": true }),
            json!({ "count": 4, "per_round": per_round(&[3, 8, 11, 16]) }),
        ));
        assert!(
            n.body.ends_with(
                "Failed trades are recurring on your side too — the whole unit is arriving one \
                 man at a time."
            ),
            "combination clause missing: {}",
            n.body
        );
    }

    // ---- H3 / H4 / H16 ---------------------------------------------------

    #[test]
    fn vulnerable_deaths_exact() {
        let n = say(&ins(
            "H3_VULNERABLE_DEATHS",
            json!({ "vulnerable": 6, "total_deaths": 19 }),
            json!({ "vulnerable": 6, "total_deaths": 19, "pct": 0.315_789_5 }),
        ));
        assert_eq!(n.title, "6 of 19 deaths with no way to fight back");
        assert_eq!(
            n.body,
            "6 of your 19 deaths (32%) came while you couldn't fight back — mid-throw, \
             reloading or swapping weapons. Do that work behind cover: step off the angle \
             first, then throw or reload."
        );
    }

    #[test]
    fn vulnerable_deaths_second_phrasing_exact() {
        let n = say(&ins(
            "H3_VULNERABLE_DEATHS",
            json!({ "vulnerable": 7, "total_deaths": 19 }),
            json!({ "vulnerable": 7, "total_deaths": 19, "pct": 0.368_421_05 }),
        ));
        assert_eq!(n.title, "Caught mid-animation in 7 deaths");
        assert_eq!(
            n.body,
            "You were mid-animation — throwing, reloading, swapping — for 7 of your 19 deaths \
             (37%). The nade and the reload each cost you a second: spend it where nobody has a \
             line on you."
        );
    }

    #[test]
    fn wasted_utility_exact() {
        let n = say(&ins(
            "H3_WASTED_UTILITY",
            json!({ "deaths_holding": 5, "total_deaths": 19 }),
            json!({ "deaths_holding": 5, "total_deaths": 19, "most_common_item": "Smoke Grenade" }),
        ));
        assert_eq!(n.title, "Died holding utility 5 times");
        assert_eq!(
            n.body,
            "You died with unthrown grenades in 5 of your 19 deaths — most often a smoke. \
             Utility you carry into your own death is utility you paid for and never used: \
             throw it into the fight you are already in."
        );
    }

    #[test]
    fn wasted_utility_gets_the_article_right() {
        let n = say(&ins(
            "H3_WASTED_UTILITY",
            json!({ "deaths_holding": 3, "total_deaths": 19 }),
            json!({ "most_common_item": "High Explosive Grenade" }),
        ));
        assert!(
            n.body.contains("most often an HE grenade"),
            "spoken-sound article: {}",
            n.body
        );
    }

    #[test]
    fn killed_without_contact_exact() {
        let n = say(&ins(
            "H4_KILLED_WITHOUT_CONTACT",
            json!({ "smoke_deaths": 2, "wallbang_deaths": 2 }),
            json!({ "smoke_deaths": 2, "wallbang_deaths": 2, "no_contact_deaths": 1,
                    "total_deaths": 19 }),
        ));
        assert_eq!(n.title, "4 deaths without a duel");
        assert_eq!(
            n.body,
            "You were killed through smoke twice and through a wall twice — 4 deaths where you \
             never got to fight. Those are lines the enemy sprays for free: cross the gap wide, \
             or hold from a spot they don't pre-fire first."
        );
    }

    #[test]
    fn killed_without_contact_second_phrasing_exact() {
        let n = say(&ins(
            "H4_KILLED_WITHOUT_CONTACT",
            json!({ "smoke_deaths": 3, "wallbang_deaths": 2 }),
            json!({ "smoke_deaths": 3, "wallbang_deaths": 2, "no_contact_deaths": 0,
                    "total_deaths": 21 }),
        ));
        assert_eq!(n.title, "Killed through smoke and walls 5 times");
        assert_eq!(
            n.body,
            "5 of your deaths never became a duel — 3 through smoke and 2 through a wall. \
             Change where you stand rather than how you aim: step off the common spray line \
             before you hold it."
        );
    }

    #[test]
    fn crossfire_exact() {
        let n = say(&ins(
            "H4_CAUGHT_IN_CROSSFIRE",
            json!({ "count": 3 }),
            json!({ "count": 3 }),
        ));
        assert_eq!(n.title, "Caught in crossfire 3 times");
        assert_eq!(
            n.body,
            "You were mid-duel with one enemy and killed by a second from another angle 3 \
             times. Clear the off-angle before you commit, or take the fight from where only \
             one of them has a line on you."
        );
    }

    #[test]
    fn utility_exposure_exact() {
        let n = say(&ins(
            "H16_UTILITY_EXPOSURE",
            json!({ "utility_deaths": 2, "fire_linger_episodes": 3 }),
            json!({ "utility_deaths": 2, "fire_linger_episodes": 3, "total_fire_damage": 87 }),
        ));
        assert_eq!(n.title, "Enemy utility cost you 2 deaths");
        assert_eq!(
            n.body,
            "Enemy grenades killed you twice with no duel involved, and you took 87 damage \
             standing in fire across 3 episodes. Move on the first tick of fire damage — the \
             exit is always cheaper than standing in it."
        );
    }

    // ---- utility family --------------------------------------------------

    #[test]
    fn flash_effectiveness_exact() {
        let n = say(&ins(
            "D2_FLASH_EFFECTIVENESS",
            json!({ "flashes": 9, "effective": 4, "team_flashes": 3, "conversions": 2 }),
            json!({ "flashes": 9, "effective_rate": 0.444, "team_flashes": 3,
                    "self_flashes": 1, "conversions": 2 }),
        ));
        assert_eq!(n.title, "9 flashes, 4 blinded an enemy");
        assert_eq!(
            n.body,
            "You threw 9 flashes: 4 blinded an enemy, 3 caught a teammate and 2 led to a kill. \
             Flash for the man entering, not for yourself: throw it over cover from behind him \
             and let him move on the pop."
        );
    }

    #[test]
    fn flash_effectiveness_team_heavy_switches_the_coaching_line() {
        let n = say(&ins(
            "D2_FLASH_EFFECTIVENESS",
            json!({ "flashes": 8, "effective": 2, "team_flashes": 5, "conversions": 1 }),
            json!({ "flashes": 8, "team_flashes": 5, "self_flashes": 2, "conversions": 1 }),
        ));
        assert!(
            n.body.ends_with(
                "More of them landed on your own team than on the enemy — line the flash up \
                 over cover and agree who entries before you throw."
            ),
            "team-flash coaching missing: {}",
            n.body
        );
    }

    #[test]
    fn flash_effectiveness_says_none_rather_than_zero() {
        let n = say(&ins(
            "D2_FLASH_EFFECTIVENESS",
            json!({ "flashes": 6, "effective": 0, "team_flashes": 0, "conversions": 0 }),
            json!({ "flashes": 6 }),
        ));
        assert_eq!(n.title, "6 flashes, none blinded an enemy");
        assert!(
            n.body
                .starts_with("You threw 6 flashes: none of them blinded an enemy."),
            "zero should read as words: {}",
            n.body
        );
    }

    #[test]
    fn flash_effectiveness_reinforces_a_good_rate_instead_of_coaching_a_missing_habit() {
        let n = say(&ins(
            "D2_FLASH_EFFECTIVENESS",
            json!({ "flashes": 8, "effective": 7, "team_flashes": 0, "conversions": 4 }),
            json!({ "flashes": 8, "effective_rate": 0.875, "team_flashes": 0,
                    "self_flashes": 0, "conversions": 4 }),
        ));
        assert_eq!(n.title, "8 flashes, 7 blinded an enemy");
        assert_eq!(
            n.body,
            "You threw 8 flashes: 7 blinded an enemy and 4 led to a kill. That is a rate worth \
             keeping — throw from behind the man entering and make sure someone moves on every \
             pop."
        );
    }

    #[test]
    fn flash_effectiveness_only_coaches_self_flashing_when_it_happened() {
        // Mediocre rate, but nothing landed on the player or his team: the
        // "not for yourself" line would be coaching a habit he doesn't have.
        let n = say(&ins(
            "D2_FLASH_EFFECTIVENESS",
            json!({ "flashes": 9, "effective": 3, "team_flashes": 0, "conversions": 1 }),
            json!({ "flashes": 9, "effective_rate": 0.333, "self_flashes": 0 }),
        ));
        assert!(
            n.body.ends_with(
                "Throw from behind the man entering and over cover, so the flash pops where he \
                 is already looking."
            ),
            "no self-flash jab without self flashes: {}",
            n.body
        );
        assert!(!n.body.contains("not for yourself"), "{}", n.body);
    }

    #[test]
    fn util_team_damage_resolves_the_victim_name() {
        let n = say(&ins(
            "H6_UTIL_TEAM_DAMAGE",
            json!({ "events": 3, "total_damage": 96 }),
            json!({ "events": 3, "total_damage": 96, "victim": "77" }),
        ));
        assert_eq!(n.title, "Your utility hurt teammates 3 times");
        assert_eq!(
            n.body,
            "Your grenades did 96 damage to your own team across 3 throws, most of it on Riku. \
             Call the nade before it leaves your hand and wait for the lane to clear — that HP \
             comes straight out of the next duel."
        );
    }

    #[test]
    fn util_team_damage_without_victim_skips_the_name() {
        let n = say(&ins(
            "H6_UTIL_TEAM_DAMAGE",
            json!({ "events": 3, "total_damage": 96 }),
            json!({ "events": 3, "total_damage": 96 }),
        ));
        assert!(
            n.body.starts_with(
                "Your grenades did 96 damage to your own team across 3 throws. Call the nade"
            ),
            "victim clause should vanish cleanly: {}",
            n.body
        );
    }

    #[test]
    fn unused_util_exact() {
        let n = say(&ins(
            "H6_UNUSED_UTIL_AT_ROUND_END",
            json!({ "rounds": 5, "min_nades": 2 }),
            json!({ "rounds": 5 }),
        ));
        assert_eq!(n.title, "Ended 5 rounds holding utility");
        assert_eq!(
            n.body,
            "You finished 5 rounds alive with 2 or more grenades unthrown. Utility has no value \
             once the round ends: spend the smoke on the timing you already committed to, or the \
             flash on the last angle you take."
        );
    }

    #[test]
    fn dead_time_smoke_exact() {
        let n = say(&ins(
            "H6_DEAD_TIME_SMOKE",
            json!({ "rounds": 3 }),
            json!({ "rounds": 3 }),
        ));
        assert_eq!(n.title, "3 smokes thrown after the round");
        assert_eq!(
            n.body,
            "3 of your smokes went out after the round had already ended. That is utility you \
             paid for and never used — throw it while the round is still live, or keep the money \
             for a rifle."
        );
    }

    // ---- D4 / D5 ---------------------------------------------------------

    #[test]
    fn entry_profile_exact() {
        let n = say(&ins(
            "D4_ENTRY_PROFILE",
            json!({}),
            json!({ "entries": 6, "entry_wins": 2, "supported": 2, "unsupported": 4,
                    "team_entries": 14, "team_entry_wins": 5, "non_trading_on_entries": 3 }),
        ));
        assert_eq!(n.title, "6 entries, 2 won");
        assert_eq!(
            n.body,
            "You took first contact on 6 of your team's 14 entries and won 2 of them. 4 of \
             those went in unsupported and 3 went untraded. Don't take the first duel until the \
             flash or the second man is with you — an entry alone is a coin flip you are paying \
             for."
        );
    }

    #[test]
    fn timing_exact() {
        let n = say(&ins(
            "D5_TIMING",
            json!({}),
            json!({ "early_aggressive_deaths": 4, "slow_rotations": 3, "push_without_info": 2 }),
        ));
        assert_eq!(n.title, "Early deaths and slow rotations");
        assert_eq!(
            n.body,
            "You died on early aggression in 4 rounds, rotated late 3 times and pushed without \
             info twice. Take space after first contact tells you where they aren't — and \
             rotate on the call, not after the site falls."
        );
    }

    #[test]
    fn entry_profile_with_nothing_broken_reinforces_instead_of_scolding() {
        let n = say(&ins(
            "D4_ENTRY_PROFILE",
            json!({}),
            json!({ "entries": 5, "entry_wins": 4, "supported": 5, "unsupported": 0,
                    "team_entries": 12, "non_trading_on_entries": 0 }),
        ));
        assert_eq!(
            n.body,
            "You took first contact on 5 of your team's 12 entries and won 4 of them. Keep the \
             flash and the second man attached to every one of them — an entry alone is a coin \
             flip you are paying for."
        );
    }

    #[test]
    fn timing_only_coaches_the_half_that_fired() {
        let n = say(&ins(
            "D5_TIMING",
            json!({}),
            json!({ "early_aggressive_deaths": 0, "slow_rotations": 3, "push_without_info": 0 }),
        ));
        assert_eq!(n.title, "Rotating late");
        assert_eq!(
            n.body,
            "You rotated late 3 times. Rotate on the call, not after the site falls."
        );
    }

    // ---- fallback --------------------------------------------------------

    #[test]
    fn unknown_detector_falls_back_neutrally() {
        let n = say(&ins(
            "H8_OFF_ANGLE_HABIT",
            json!({ "count": 3 }),
            json!({ "count": 3 }),
        ));
        assert_eq!(n.title, "Off angle habit");
        assert_eq!(n.body, "Flagged 3 times this match.");
    }

    #[test]
    fn unknown_detector_without_count_still_says_something() {
        let n = say(&ins("H11_SOMETHING_NEW", json!({}), json!({})));
        assert_eq!(n.title, "Something new");
        assert_eq!(n.body, "Flagged this match.");
    }

    // ---- determinism -----------------------------------------------------

    #[test]
    fn variants_are_deterministic() {
        let i = ins(
            "H2_ISOLATED_DEATH",
            json!({ "count": 5 }),
            json!({ "count": 5, "per_round": per_round(&[4, 7]) }),
        );
        let a = say(&i);
        let b = say(&i);
        assert_eq!(a.title, b.title);
        assert_eq!(a.body, b.body);
    }

    #[test]
    fn variants_actually_vary_across_rounds_and_counts() {
        let titles: std::collections::BTreeSet<String> = (1..24)
            .map(|c| {
                say(&ins(
                    "H2_ISOLATED_DEATH",
                    json!({ "count": c }),
                    json!({ "count": c }),
                ))
                .title
            })
            .collect();
        assert!(titles.len() >= 2, "expected >=2 phrasings, got {titles:?}");

        let mut by_round = std::collections::BTreeSet::new();
        for r in 1..24u32 {
            let mut i = ins(
                "H4_KILLED_WITHOUT_CONTACT",
                json!({ "smoke_deaths": 2, "wallbang_deaths": 2 }),
                json!({ "smoke_deaths": 2, "wallbang_deaths": 2 }),
            );
            i.round = r;
            by_round.insert(say(&i).title);
        }
        assert!(
            by_round.len() >= 2,
            "round must reach the hash: {by_round:?}"
        );
    }

    // ---- summarize -------------------------------------------------------

    #[test]
    fn summarize_composes_result_class13_and_top_category() {
        let mut set = vec![];
        for _ in 0..3 {
            set.push(ins("H2_ISOLATED_DEATH", json!({}), json!({})));
        }
        let mut util = ins("H3_WASTED_UTILITY", json!({}), json!({}));
        util.category = Category::Utility;
        set.push(util);
        let mut pos = ins("H4_CAUGHT_IN_CROSSFIRE", json!({}), json!({}));
        pos.category = Category::Positioning;
        set.push(pos);

        let n = TemplateNarrator.summarize(&set, &ctx()).expect("summary");
        assert_eq!(n.title, "Mirage, 10-13 loss");
        assert_eq!(
            n.body,
            "You lost 10-13 on Mirage and died 19 times. 32% of your deaths were fair duels you \
             lost on mechanics — the rest had a fixable cause. Deaths are the biggest group at 3 \
             of the 5 insights, so start there."
        );
    }

    #[test]
    fn summarize_is_none_without_insights() {
        assert!(TemplateNarrator.summarize(&[], &ctx()).is_none());
    }

    #[test]
    fn summarize_win_path_exact() {
        let mut c = ctx();
        c.tracked_result = Some("win".to_string());
        c.score = (13, 7);
        c.total_deaths = 13;
        c.class_13_share_pct = 61.5;
        let set = [ins("H2_ISOLATED_DEATH", json!({}), json!({}))];
        let n = TemplateNarrator.summarize(&set, &c).expect("summary");
        assert_eq!(n.title, "Mirage, 13-7 win");
        assert_eq!(
            n.body,
            "You won 13-7 on Mirage and died 13 times. 62% of your deaths were fair duels you \
             lost on mechanics — the rest had a fixable cause. The one insight this match sits \
             in deaths, so start there."
        );
    }

    #[test]
    fn summarize_tie_path_exact() {
        let mut c = ctx();
        c.tracked_result = Some("draw".to_string());
        c.score = (12, 12);
        c.total_deaths = 17;
        c.class_13_share_pct = 40.0;
        let mut util = ins("H3_WASTED_UTILITY", json!({}), json!({}));
        util.category = Category::Utility;
        let set = [util, ins("H2_ISOLATED_DEATH", json!({}), json!({}))];
        let n = TemplateNarrator.summarize(&set, &c).expect("summary");
        assert_eq!(n.title, "Mirage, 12-12 draw");
        assert_eq!(
            n.body,
            "You drew 12-12 on Mirage and died 17 times. 40% of your deaths were fair duels you \
             lost on mechanics — the rest had a fixable cause. Deaths are the biggest group at 1 \
             of the 2 insights, so start there."
        );
    }

    #[test]
    fn summarize_all_fair_duels_does_not_contradict_itself() {
        let mut c = ctx();
        c.class_13_share_pct = 100.0;
        let set = [ins("H2_ISOLATED_DEATH", json!({}), json!({}))];
        let n = TemplateNarrator.summarize(&set, &c).expect("summary");
        assert!(
            n.body.contains("that is the good version of losing"),
            "{}",
            n.body
        );
        assert!(
            !n.body.contains("elsewhere"),
            "must not say the fix is elsewhere then name where to start: {}",
            n.body
        );
    }

    // ---- habits ----------------------------------------------------------

    #[test]
    fn habit_hotspot_names_the_map() {
        let n = narrate_habit(
            "H4_REPEAT_HOTSPOT",
            4,
            5,
            11,
            &json!({ "map": "de_mirage", "deaths": 11, "matches": 4 }),
        );
        assert_eq!(n.title, "Repeat hotspot on Mirage");
        assert_eq!(
            n.body,
            "You have died 11 times at the same spot on Mirage across 4 matches. They know that \
             angle better than you do — hold it from a different position, or stop taking that \
             fight."
        );
    }

    #[test]
    fn habit_isolated_death_has_its_own_phrasing() {
        let n = narrate_habit("H2_ISOLATED_DEATH", 4, 5, 17, &json!({}));
        assert_eq!(n.title, "Habit: isolated deaths");
        assert_eq!(
            n.body,
            "You died isolated in 4 of your last 5 matches — 17 times in all. This is the first \
             habit to fix: pick fights a teammate can re-peek within two seconds."
        );
    }

    /// death-taxonomy §2 H2: the promoted baited habit must read as a team
    /// spacing problem and must never coach the player out of trading. It only
    /// ever promotes alongside H2_FAILED_TRADE, so the caption says so.
    #[test]
    fn habit_baited_trade_blames_the_spacing_not_the_player() {
        let n = narrate_habit("H2_BAITED_TRADE", 3, 5, 9, &json!({}));
        assert_eq!(n.title, "Habit: nobody follows your trade");
        assert_eq!(
            n.body,
            "You were the only one who committed to the trade in 3 of your last 5 matches — 9 \
             times in all. Failed trades are recurring on your side in the same window, so this \
             is a team spacing problem, not a habit to unlearn: keep re-peeking, and fix the \
             timing so the second man leaves with you."
        );
        let lower = n.body.to_lowercase();
        for blame in [
            "your fault",
            "you should have",
            "stop trading",
            "stop re-peeking",
            "bad play",
        ] {
            assert!(
                !lower.contains(blame),
                "blame word {blame:?} in: {}",
                n.body
            );
        }
        assert!(lower.contains("team"), "must name the team problem");
        assert!(
            lower.contains("keep re-peeking"),
            "must protect the trading instinct: {}",
            n.body
        );
    }

    #[test]
    fn habit_generic_fallback_states_the_recurrence() {
        let n = narrate_habit("H11_LATE_ROTATION", 3, 5, 8, &json!({}));
        assert_eq!(n.title, "Habit: late rotation");
        assert_eq!(
            n.body,
            "Late rotation recurred in 3 of your last 5 matches — 8 times in all. A mistake that \
             repeats across matches is a habit: watch the clips together and find what they share."
        );
    }

    #[test]
    fn habit_covers_the_named_rules() {
        for rule in [
            "H2_ISOLATED_DEATH",
            "H2_FAILED_TRADE",
            "H2_BAITED_TRADE",
            "H3_WASTED_UTILITY",
            "H4_KILLED_WITHOUT_CONTACT",
            "H4_REPEAT_HOTSPOT",
        ] {
            let n = narrate_habit(rule, 3, 5, 9, &json!({ "map": "de_nuke", "matches": 3 }));
            assert!(!n.title.is_empty() && !n.body.is_empty(), "{rule}");
            assert!(
                !n.body.contains("recurred in"),
                "{rule} must have a hand-written phrasing, not the generic fallback: {}",
                n.body
            );
        }
    }

    // ---- singular counts -------------------------------------------------

    /// Finds "1 rounds" / "1 episodes" — a count next to a plural noun.
    fn singular_disagreement(text: &str) -> Option<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        for pair in words.windows(2) {
            if pair[0] != "1" {
                continue;
            }
            let noun = pair[1].trim_matches(|c: char| !c.is_alphanumeric());
            if noun.len() > 2 && noun.ends_with('s') && !noun.ends_with("ss") {
                return Some(format!("{} {}", pair[0], pair[1]));
            }
        }
        None
    }

    #[test]
    fn timing_at_the_minimum_gate_says_one_round() {
        // H11 gate is early + slow >= 2, so early=1/slow=1 is reachable.
        let n = say(&ins(
            "D5_TIMING",
            json!({}),
            json!({ "early_aggressive_deaths": 1, "slow_rotations": 1, "push_without_info": 0 }),
        ));
        assert_eq!(
            n.body,
            "You died on early aggression in 1 round and rotated late once. Take space after \
             first contact tells you where they aren't — and rotate on the call, not after the \
             site falls."
        );
    }

    #[test]
    fn utility_exposure_at_the_minimum_gate_says_one_episode() {
        // H16 gate is flags.len() >= 2 → one utility death + one fire episode.
        let n = say(&ins(
            "H16_UTILITY_EXPOSURE",
            json!({ "utility_deaths": 1, "fire_linger_episodes": 1 }),
            json!({ "utility_deaths": 1, "fire_linger_episodes": 1, "total_fire_damage": 22 }),
        ));
        assert_eq!(n.title, "Enemy utility cost you 1 death");
        assert_eq!(
            n.body,
            "Enemy grenades killed you once with no duel involved, and you took 22 damage \
             standing in fire across 1 episode. Move on the first tick of fire damage — the exit \
             is always cheaper than standing in it."
        );
    }

    #[test]
    fn no_template_renders_a_count_against_a_plural_noun() {
        let ones = vec![
            ins(
                "H2_ISOLATED_DEATH",
                json!({ "count": 1 }),
                json!({ "count": 1,
                "per_round": per_round(&[4]) }),
            ),
            ins(
                "H2_FAILED_TRADE",
                json!({ "count": 1 }),
                json!({ "count": 1 }),
            ),
            ins(
                "H2_BAITED_TRADE",
                json!({ "count": 1 }),
                json!({ "count": 1 }),
            ),
            ins(
                "H3_VULNERABLE_DEATHS",
                json!({ "vulnerable": 1, "total_deaths": 1 }),
                json!({ "pct": 1.0 }),
            ),
            ins(
                "H3_WASTED_UTILITY",
                json!({ "deaths_holding": 1, "total_deaths": 1 }),
                json!({ "most_common_item": "Molotov" }),
            ),
            ins(
                "H4_KILLED_WITHOUT_CONTACT",
                json!({ "smoke_deaths": 1, "wallbang_deaths": 0 }),
                json!({}),
            ),
            ins("H4_CAUGHT_IN_CROSSFIRE", json!({ "count": 1 }), json!({})),
            ins(
                "H16_UTILITY_EXPOSURE",
                json!({ "utility_deaths": 1, "fire_linger_episodes": 1 }),
                json!({ "total_fire_damage": 15 }),
            ),
            ins(
                "H16_UTILITY_EXPOSURE",
                json!({ "utility_deaths": 0, "fire_linger_episodes": 1 }),
                json!({}),
            ),
            ins(
                "D2_FLASH_EFFECTIVENESS",
                json!({ "flashes": 1, "effective": 1, "team_flashes": 1, "conversions": 1 }),
                json!({ "self_flashes": 1 }),
            ),
            ins(
                "H6_UTIL_TEAM_DAMAGE",
                json!({ "events": 1, "total_damage": 12 }),
                json!({}),
            ),
            ins(
                "H6_UNUSED_UTIL_AT_ROUND_END",
                json!({ "rounds": 1, "min_nades": 2 }),
                json!({}),
            ),
            ins("H6_DEAD_TIME_SMOKE", json!({ "rounds": 1 }), json!({})),
            ins(
                "D4_ENTRY_PROFILE",
                json!({}),
                json!({ "entries": 1, "entry_wins": 0, "unsupported": 1, "team_entries": 1,
                        "non_trading_on_entries": 1 }),
            ),
            ins(
                "D5_TIMING",
                json!({}),
                json!({ "early_aggressive_deaths": 1, "slow_rotations": 1,
                        "push_without_info": 1 }),
            ),
            ins("Z9_MYSTERY", json!({ "count": 1 }), json!({})),
        ];
        for c in &ones {
            let n = say(c);
            let both = format!("{} {}", n.title, n.body);
            assert!(
                singular_disagreement(&both).is_none(),
                "{} renders {:?} in: {both}",
                c.detector,
                singular_disagreement(&both).unwrap()
            );
        }

        for rule in [
            "H2_ISOLATED_DEATH",
            "H2_BAITED_TRADE",
            "H4_REPEAT_HOTSPOT",
            "H11_LATE_ROTATION",
        ] {
            let n = narrate_habit(
                rule,
                1,
                1,
                1,
                &json!({ "map": "de_nuke", "deaths": 1,
                                                          "matches": 1 }),
            );
            let both = format!("{} {}", n.title, n.body);
            assert!(
                singular_disagreement(&both).is_none(),
                "habit {rule} renders {:?} in: {both}",
                singular_disagreement(&both).unwrap()
            );
        }

        let mut one_ctx = ctx();
        one_ctx.total_deaths = 1;
        let n = TemplateNarrator
            .summarize(&[ins("H2_ISOLATED_DEATH", json!({}), json!({}))], &one_ctx)
            .expect("summary");
        let both = format!("{} {}", n.title, n.body);
        assert!(
            singular_disagreement(&both).is_none(),
            "summary renders {:?} in: {both}",
            singular_disagreement(&both).unwrap()
        );
    }

    // ---- house style sweep ----------------------------------------------

    #[test]
    fn every_template_meets_the_house_style() {
        let cases = vec![
            ins(
                "H2_ISOLATED_DEATH",
                json!({ "count": 5 }),
                json!({ "count": 5, "per_round": per_round(&[4, 7]) }),
            ),
            ins(
                "H2_FAILED_TRADE",
                json!({ "count": 3 }),
                json!({ "count": 3 }),
            ),
            ins(
                "H2_BAITED_TRADE",
                json!({ "count": 4, "team_pattern": true }),
                json!({ "count": 4 }),
            ),
            ins(
                "H3_VULNERABLE_DEATHS",
                json!({ "vulnerable": 7, "total_deaths": 19 }),
                json!({ "pct": 0.37 }),
            ),
            ins(
                "H3_WASTED_UTILITY",
                json!({ "deaths_holding": 5, "total_deaths": 19 }),
                json!({ "most_common_item": "Flashbang" }),
            ),
            ins(
                "H4_KILLED_WITHOUT_CONTACT",
                json!({ "smoke_deaths": 2, "wallbang_deaths": 0 }),
                json!({}),
            ),
            ins("H4_CAUGHT_IN_CROSSFIRE", json!({ "count": 2 }), json!({})),
            ins(
                "H16_UTILITY_EXPOSURE",
                json!({ "utility_deaths": 0, "fire_linger_episodes": 4 }),
                json!({ "total_fire_damage": 120 }),
            ),
            ins(
                "D2_FLASH_EFFECTIVENESS",
                json!({ "flashes": 9, "effective": 4 }),
                json!({}),
            ),
            ins("H6_UTIL_TEAM_DAMAGE", json!({ "events": 2 }), json!({})),
            ins(
                "H6_UNUSED_UTIL_AT_ROUND_END",
                json!({ "rounds": 4 }),
                json!({}),
            ),
            ins("H6_DEAD_TIME_SMOKE", json!({ "rounds": 2 }), json!({})),
            ins("D4_ENTRY_PROFILE", json!({}), json!({ "entries": 5 })),
            ins("D5_TIMING", json!({}), json!({ "slow_rotations": 3 })),
            ins("Z9_MYSTERY", json!({}), json!({})),
        ];
        for c in &cases {
            let n = say(c);
            let both = format!("{} {}", n.title, n.body);
            assert!(!n.title.is_empty(), "{} empty title", c.detector);
            assert!(!n.body.is_empty(), "{} empty body", c.detector);
            assert!(
                n.title.chars().count() <= 60,
                "{} title too long ({}): {}",
                c.detector,
                n.title.chars().count(),
                n.title
            );
            assert!(!both.contains('!'), "{} shouts: {both}", c.detector);
            assert!(
                !both.contains("null") && !both.contains("{}") && !both.contains("  "),
                "{} leaks raw data: {both}",
                c.detector
            );
            let lower = both.to_lowercase();
            for filler in [" just ", " simply ", "try to ", " basically ", " maybe "] {
                assert!(
                    !lower.contains(filler),
                    "{} uses filler {filler:?}: {both}",
                    c.detector
                );
            }
            let sentences = n.body.matches(". ").count() + usize::from(n.body.ends_with('.'));
            assert!(
                (1..=3).contains(&sentences),
                "{} body should be 1-3 sentences, got {sentences}: {}",
                c.detector,
                n.body
            );
        }
    }
}
