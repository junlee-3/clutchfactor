//! What the coach watches (docs/spec/stats-and-understanding.md §3): every
//! detector, in plain language, with its thresholds rendered from the live
//! config, the taxonomy class it sources, and what the engine cannot see.
//! Static data + a coverage test — the honesty in the code becomes a screen.

pub struct CatalogEntry {
    pub id: &'static str,
    pub family: &'static str,
    pub title: &'static str,
    pub watches_for: &'static str,
    pub thresholds: &'static str,
    pub class_id: Option<u8>,
    pub example: &'static str,
    pub stat_links: &'static [&'static str],
}

pub struct ClassEntry {
    pub id: u8,
    pub name: &'static str,
    pub source: &'static str,
    pub built: bool,
    pub why_not: Option<&'static str>,
}

pub const CANNOT_SEE: &[(&str, &str)] = &[
    ("Economy", "Buys, saves and money are not read — the coach never talks about what you bought."),
    ("Utility lineups", "Where a grenade was aimed from and whether it was a known lineup is not modelled; only where it landed and what it did."),
    ("Comms", "Voice and text are not in the demo. Calls, info and intent are invisible; the engine sees only movement and events."),
    ("Aim mechanics", "Crosshair placement, spray control and reaction time are not measured — only outcomes (kills, headshots, damage)."),
    ("Line of sight", "There is no map geometry or visibility raycast. 'Isolated' means far from teammates, not out of their sight (a same-callout teammate counts as close)."),
    ("Who saw whom", "The demo's spotted flag is per player, not pairwise, so 'seen first' is never claimed."),
];

static ENTRIES: &[CatalogEntry] = &[
    CatalogEntry { id: "H2_ISOLATED_DEATH", family: "H2 Trade spacing", title: "Isolated death",
        watches_for: "You died with no teammate close enough to trade, and nobody punished the killer.",
        thresholds: "no living teammate within {trade.isolation_u} at your death (a teammate in the same callout counts as close) and the killer survives {trade.commit_window_s} after it",
        class_id: Some(6), example: "Isolated at Banana: nearest teammate 1,223 u away.", stat_links: &["trade", "kast"] },
    CatalogEntry { id: "H2_FAILED_TRADE", family: "H2 Trade spacing", title: "Failed trade",
        watches_for: "A teammate died within trade range and you neither fired nor damaged the killer in time.",
        thresholds: "teammate dies within {trade.distance_u} of you; no shot or damage from you within {trade.commit_window_s}; the killer lives through the window",
        class_id: None, example: "Didn't trade Sam — Kit killed them 430 u from you; no shot from you in 2 s.", stat_links: &["trade"] },
    CatalogEntry { id: "H2_BAITED_TRADE", family: "H2 Trade spacing", title: "Baited trade",
        watches_for: "You committed to a trade and died with no teammate following you in.",
        thresholds: "you fire or deal damage within {trade.commit_window_s} of a teammate's death, then die within {trade.window_s} with no teammate within {trade.distance_u}",
        class_id: Some(7), example: "You committed to the trade at Apartments; Sam stayed 1,100 u behind.", stat_links: &["trade"] },
    CatalogEntry { id: "H3_DIED_WITH_NADE_OUT", family: "H3 Utility vulnerability", title: "Died with a grenade out",
        watches_for: "You were killed while holding a grenade instead of a gun.", thresholds: "the last sampled weapon before the death was a grenade",
        class_id: Some(1), example: "Killed with a flashbang out at Connector.", stat_links: &["kd"] },
    CatalogEntry { id: "H3_DIED_MID_SWITCH", family: "H3 Utility vulnerability", title: "Died mid-switch",
        watches_for: "You died just after switching weapons, before the new one was ready.", thresholds: "weapon changed within {h3.switch_window_s} before the death",
        class_id: Some(1), example: "Switched from the smoke to the rifle 0.2 s before dying.", stat_links: &["kd"] },
    CatalogEntry { id: "H3_DIED_RELOADING", family: "H3 Utility vulnerability", title: "Died reloading",
        watches_for: "You died during a reload.", thresholds: "a reload started within {h3.reload_window_s} before the death",
        class_id: Some(4), example: "Reload started 0.9 s before the death at Top Mid.", stat_links: &["kd"] },
    CatalogEntry { id: "H3_DIED_SCOPED_CLOSE", family: "H3 Utility vulnerability", title: "Died scoped at close range",
        watches_for: "You were scoped in while an enemy was already close.", thresholds: "scoped with the killer within {h3.scoped_close_u}",
        class_id: Some(4), example: "Scoped with the killer 310 u away.", stat_links: &["kd"] },
    CatalogEntry { id: "H3_WASTED_UTILITY", family: "H3 Utility vulnerability", title: "Unused utility at death",
        watches_for: "You died with grenades still in your inventory.", thresholds: "any grenade left in the inventory at the death tick",
        class_id: None, example: "Died with a smoke and a flash unused — 3 of your 15 deaths.", stat_links: &["adr"] },
    CatalogEntry { id: "H4_KILLED_WITHOUT_CONTACT", family: "H4 Exposure", title: "Killed without contact",
        watches_for: "You died without ever engaging the killer — through smoke, a wallbang, or with no shot exchanged.",
        thresholds: "no shot fired and no damage exchanged with the killer within {h4.contactless_window_s} before the death (through-smoke and wallbang kills are certain)",
        class_id: Some(5), example: "Killed through the Top Mid smoke without a shot fired.", stat_links: &["kd"] },
    CatalogEntry { id: "H4_CAUGHT_IN_CROSSFIRE", family: "H4 Exposure", title: "Caught in a crossfire",
        watches_for: "A second enemy killed you while you were fighting the first.",
        thresholds: "two enemies you exchanged damage with in the last {h4.crossfire_engage_window_s}, at least {h4.crossfire_min_angle_deg}° apart",
        class_id: Some(9), example: "Fighting Kit at Palace when Sam's teammate hit you from Jungle, 70° apart.", stat_links: &["kd"] },
    CatalogEntry { id: "H5_DIED_FLASHED", family: "H5 Audio-cued misplay", title: "Died flashed",
        watches_for: "You died inside an enemy flash.", thresholds: "an enemy blind of at least {flash.effective_s} covering the death tick",
        class_id: Some(3), example: "Blinded for 1.8 s when the duel started.", stat_links: &["kd"] },
    CatalogEntry { id: "H6_FLASH_SELF_OR_TEAM", family: "H6 Utility usage", title: "Flashed yourself or a teammate",
        watches_for: "Your flash blinded you or a teammate.", thresholds: "a blind of at least {flash.effective_s} on yourself or a teammate from your own flashbang",
        class_id: None, example: "Your flash blinded Sam for 1.4 s at Ramp.", stat_links: &["adr"] },
    CatalogEntry { id: "H6_DEAD_TIME_SMOKE", family: "H6 Utility usage", title: "Smoke after the round was decided",
        watches_for: "A smoke thrown after the round had already been won or lost.", thresholds: "smoke detonates after the round's end tick but before the next round",
        class_id: None, example: "Smoke landed 4 s after the last enemy died.", stat_links: &["adr"] },
    CatalogEntry { id: "H6_UNUSED_UTIL_AT_ROUND_END", family: "H6 Utility usage", title: "Unused utility at round end",
        watches_for: "You survived the round still holding grenades.", thresholds: "alive at the round's end with at least {util.min_unused_nades} grenades in the inventory",
        class_id: None, example: "Round ended with a smoke and a molotov unused.", stat_links: &["adr"] },
    CatalogEntry { id: "H6_UTIL_TEAM_DAMAGE", family: "H6 Utility usage", title: "Utility team damage",
        watches_for: "Your grenade damaged a teammate.", thresholds: "any utility damage dealt to a teammate",
        class_id: None, example: "Your molotov did 22 to Sam at Banana.", stat_links: &["adr"] },
    CatalogEntry { id: "H6_PUSH_WITHOUT_INFO", family: "H6 Utility usage", title: "Push without info",
        watches_for: "An early, deep push into the enemy with no information gathered first.",
        thresholds: "all the early-aggression conditions (below) and no enemy spotted, no damage exchanged and no enemy shot heard since freeze end",
        class_id: Some(11), example: "Pushed 960 u into Mid by 4 s with nothing spotted.", stat_links: &["entry"] },
    CatalogEntry { id: "H11_EARLY_AGGRESSIVE_DEATH", family: "H11 Timing", title: "Early aggressive death",
        watches_for: "You died early, far from spawn, with no teammate close.",
        thresholds: "death within {timing.early_aggression_s} of freeze end, at least {timing.min_spawn_distance_u} from your freeze-end position, no teammate within {trade.distance_u}",
        class_id: None, example: "Died 12 s in, 1,100 u from spawn, nearest teammate 1,500 u.", stat_links: &["entry", "kd"] },
    CatalogEntry { id: "H11_SLOW_ROTATION", family: "H11 Timing", title: "Slow rotation",
        watches_for: "The bomb went down and you never reached the site in time (CT).",
        thresholds: "farther than {timing.rotate_radius_u} from the plant at the plant and still farther {timing.rotate_max_s} later, in a lost round",
        class_id: None, example: "3,000 u from the plant when it went down; never arrived.", stat_links: &["clutch"] },
    CatalogEntry { id: "H14_UNSUPPORTED_ENTRY", family: "H14 Entry structure", title: "Unsupported entry",
        watches_for: "You took the round's opening duel with no teammate close enough to trade you.",
        thresholds: "the round's first kill within {entry.opening_window_s} of freeze end; no living teammate within {entry.support_distance_u} or in the same callout",
        class_id: None, example: "Opened at Palace with Sam 1,200 u behind.", stat_links: &["entry"] },
    CatalogEntry { id: "H16_DIED_TO_UTILITY_NO_DUEL", family: "H16 Utility damage exposure", title: "Died to utility without a duel",
        watches_for: "Grenade or fire damage killed you without a fight.", thresholds: "no shot within {h16.no_shot_window_s} and no enemy contact within {h16.no_contact_window_s} before a utility death",
        class_id: Some(2), example: "Burned out at Banana without firing.", stat_links: &["kd"] },
    CatalogEntry { id: "H16_FIRE_LINGER", family: "H16 Utility damage exposure", title: "Stayed in fire",
        watches_for: "You kept taking fire damage instead of leaving it.", thresholds: "more than {h16.fire_linger_dmg} fire damage over an episode longer than {h16.fire_linger_s}",
        class_id: None, example: "Took 34 fire damage over 2.1 s at Apartments.", stat_links: &["adr"] },
    CatalogEntry { id: "H3_VULNERABLE_DEATHS", family: "H3 Utility vulnerability", title: "Vulnerable deaths (roll-up)",
        watches_for: "How many of your deaths came while you were holding a grenade, switching, reloading or scoped close — the four H3 states together.",
        thresholds: "the H3 rules above; reported as a share of all deaths",
        class_id: None, example: "5 of 15 deaths while vulnerable — 3 reloading, 2 with a grenade out.", stat_links: &["kd"] },
    CatalogEntry { id: "H16_UTILITY_EXPOSURE", family: "H16 Utility damage exposure", title: "Utility exposure (roll-up)",
        watches_for: "How much grenade and fire damage you took across the match, and how much of it came without a duel.",
        thresholds: "the H16 rules above; reported as total utility damage taken",
        class_id: None, example: "Took 118 utility damage, 2 deaths to utility without a duel.", stat_links: &["adr"] },
    CatalogEntry { id: "D2_FLASH_EFFECTIVENESS", family: "D2 Utility effect", title: "Flash effectiveness",
        watches_for: "How many of your flashes blinded enemies, hit teammates, or turned into kills.",
        thresholds: "an enemy counts as blinded at {flash.effective_s} or more; a kill within {flash.conversion_window_s} of the flash counts as converted; reported after {d2.min_flashes} or more flashes",
        class_id: None, example: "9 flashes: 5 effective, 2 team flashes, 3 converted.", stat_links: &["adr"] },
    CatalogEntry { id: "D4_ENTRY_PROFILE", family: "D4 Entry structure", title: "Entry profile",
        watches_for: "How often you take the opening duel, how often you win it, and whether you had support.",
        thresholds: "opening duel = first kill within {entry.opening_window_s} of freeze end; supported = a teammate within {entry.support_distance_u} or in the same callout",
        class_id: None, example: "Entries: 6 attempts, 2 won, 4 unsupported.", stat_links: &["entry"] },
    CatalogEntry { id: "D5_TIMING", family: "D5 Timing", title: "Timing profile",
        watches_for: "Early aggressive deaths and slow rotations across the match.",
        thresholds: "early = within {timing.early_aggression_s} of freeze end; slow rotation = not within {timing.rotate_radius_u} of the plant after {timing.rotate_max_s}",
        class_id: None, example: "3 early deaths, 2 slow rotations.", stat_links: &["entry", "clutch"] },
    CatalogEntry { id: "D6_UNUSUAL_POSITIONING", family: "D6 Positioning vs corpus", title: "Unusual positioning",
        watches_for: "Where you set up compared with reference (pro) demos on the same map — unusual, not wrong.",
        thresholds: "needs {corpus.min_demos_per_map} reference demos per map; a spot is unusual below the {corpus.low_density_pct}th density percentile, reported after {corpus.min_recurrences} rounds",
        class_id: Some(12), example: "Your CT setup at Ticket is unusual: 4 rounds where reference players rarely stand.", stat_links: &[] },
];

static CLASSES: &[ClassEntry] = &[
    ClassEntry { id: 1, name: "Caught in utility animation", source: "H3", built: true, why_not: None },
    ClassEntry { id: 2, name: "Caught in grenade/incendiary damage (no duel)", source: "H16", built: true, why_not: None },
    ClassEntry { id: 3, name: "Blinded / flashed out", source: "H5", built: true, why_not: None },
    ClassEntry { id: 4, name: "Caught reloading or unscoped", source: "H3", built: true, why_not: None },
    ClassEntry { id: 5, name: "No-engagement death (wallbang / through smoke / never saw the attacker)", source: "H4", built: true, why_not: None },
    ClassEntry { id: 6, name: "Isolated & untradeable", source: "H2", built: true, why_not: None },
    ClassEntry { id: 7, name: "Baited / unsupported trade attempt", source: "H2", built: true, why_not: None },
    ClassEntry { id: 8, name: "Over-peek in man disadvantage", source: "H1", built: false, why_not: Some("Needs peek geometry (who exposed to whom) — the parser gives positions, not lines of sight.") },
    ClassEntry { id: 9, name: "Crossfire death (killed by a second enemy mid-duel)", source: "H4", built: true, why_not: None },
    ClassEntry { id: 10, name: "Lost angle-advantage duel (wide peek)", source: "H4", built: false, why_not: Some("Needs angle-of-exposure geometry against map walls; no raycast data exists in v1.") },
    ClassEntry { id: 11, name: "Pushed without info", source: "H6", built: true, why_not: None },
    ClassEntry { id: 12, name: "Off-angle / repeat-hotspot death", source: "H8", built: false, why_not: Some("Per-death classification needs a 'standard angle' model; hotspots are tracked across matches instead (Habits).") },
    ClassEntry { id: 13, name: "Outaimed in a fair duel", source: "fallback — good to see", built: true, why_not: None },
    ClassEntry { id: 14, name: "Fall damage / self-inflicted", source: "event-derived", built: true, why_not: None },
    ClassEntry { id: 15, name: "Unclassified", source: "fallback", built: true, why_not: None },
];

pub fn entries() -> &'static [CatalogEntry] {
    ENTRIES
}

pub fn classes() -> &'static [ClassEntry] {
    CLASSES
}

/// "{trade.isolation_u}" → "900 u" from (name, value, unit) rows. Values
/// print as stored (the threshold rows already format them); a unit is
/// appended when present. Unknown placeholders are left untouched so the
/// coverage test fails loudly.
pub fn render_thresholds(template: &str, values: &[(String, String, String)]) -> String {
    let mut out = template.to_string();
    for (name, value, unit) in values {
        let needle = format!("{{{name}}}");
        let replacement = if unit.is_empty() {
            value.clone()
        } else {
            format!("{value} {unit}")
        };
        out = out.replace(&needle, &replacement);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::families;

    #[test]
    fn every_emitted_rule_and_every_class_has_an_entry() {
        let ids: Vec<&str> = entries().iter().map(|e| e.id).collect();
        for d in families::all() {
            for rule in d.rule_ids() {
                assert!(ids.contains(rule), "catalog lacks {rule}");
            }
        }
        // Roll-up insight ids are `detector` values on Insights, not rule ids — they are on the Report, so they need a card too.
        for d in [
            "D2_FLASH_EFFECTIVENESS",
            "D4_ENTRY_PROFILE",
            "D5_TIMING",
            "D6_UNUSUAL_POSITIONING",
            "H3_VULNERABLE_DEATHS",
            "H16_UTILITY_EXPOSURE",
        ] {
            assert!(ids.contains(&d), "catalog lacks {d}");
        }
        let class_ids: Vec<u8> = classes().iter().map(|c| c.id).collect();
        assert_eq!(class_ids, (1..=15).collect::<Vec<u8>>());
        for c in classes().iter().filter(|c| !c.built) {
            assert!(
                c.why_not.is_some(),
                "class {} not built without a reason",
                c.id
            );
        }
        assert_eq!(
            classes()
                .iter()
                .filter(|c| !c.built)
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![8, 10, 12]
        );
    }

    #[test]
    fn thresholds_render_from_config_values_and_never_leave_a_placeholder() {
        let values = vec![
            (
                "trade.isolation_u".to_string(),
                "900".to_string(),
                "u".to_string(),
            ),
            (
                "trade.commit_window_s".to_string(),
                "2".to_string(),
                "s".to_string(),
            ),
        ];
        assert_eq!(
            render_thresholds(
                "no teammate within {trade.isolation_u} and nobody trades within {trade.commit_window_s}",
                &values
            ),
            "no teammate within 900 u and nobody trades within 2 s"
        );
        for e in entries() {
            let rendered = render_thresholds(
                e.thresholds,
                &crate::config::threshold_values(&crate::DetectorConfig::default()),
            );
            assert!(
                !rendered.contains('{'),
                "{}: unresolved placeholder in {rendered}",
                e.id
            );
            assert!(
                !e.watches_for.contains('!') && !e.example.contains('!'),
                "{}: exclamation mark",
                e.id
            );
        }
    }
}
