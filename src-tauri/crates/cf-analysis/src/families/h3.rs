//! H3 — Utility Vulnerability (spec §2): *your own* utility making *you*
//! vulnerable at the moment of death. Sources taxonomy classes 1 (nade
//! out / mid-switch) and 4 (reloading / scoped close), plus the
//! `H3_WASTED_UTILITY` secondary tag ("died holding utility").
//!
//! Death-anchored flag convention (classify.rs): `tick` = kill tick,
//! `steamid` = victim.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::config::DetectorConfig;
use crate::context::AnalysisContext;
use crate::types::{Category, EvidenceRef, Insight, RuleFlag};
use crate::{evidence_around, Detector};

/// `active_weapon` in the tick table may carry either display names
/// ("Flashbang" — matched against `cfg.util.grenade_items`) or `weapon_`-
/// prefixed code names; these are the grenade fragments of the code names.
const GRENADE_CODE_FRAGMENTS: &[&str] = &[
    "flashbang",
    "smokegrenade",
    "hegrenade",
    "molotov",
    "incgrenade",
    "decoy",
];

/// The rules that mean "you could not shoot back when you died" — the
/// spec's Vulnerable Death % metric counts deaths where any of these fired.
const VULNERABLE_RULES: &[&str] = &[
    "H3_DIED_WITH_NADE_OUT",
    "H3_DIED_MID_SWITCH",
    "H3_DIED_RELOADING",
    "H3_DIED_SCOPED_CLOSE",
];

fn is_grenade_weapon(name: &str, cfg: &DetectorConfig) -> bool {
    cfg.util.grenade_items.iter().any(|g| g == name)
        || (name.starts_with("weapon_") && GRENADE_CODE_FRAGMENTS.iter().any(|f| name.contains(f)))
}

pub struct H3UtilityVulnerability;

impl Detector for H3UtilityVulnerability {
    fn rule_ids(&self) -> &'static [&'static str] {
        &[
            "H3_DIED_WITH_NADE_OUT",
            "H3_DIED_MID_SWITCH",
            "H3_DIED_RELOADING",
            "H3_DIED_SCOPED_CLOSE",
            "H3_WASTED_UTILITY",
        ]
    }

    fn detect(&self, ctx: &AnalysisContext, cfg: &DetectorConfig) -> Vec<RuleFlag> {
        let mut flags = vec![];
        for kill in ctx.tracked_deaths() {
            let victim = kill.victim;
            let mut focus = vec![victim];
            if let Some(a) = kill.attacker {
                focus.push(a);
            }
            let evidence = || evidence_around(ctx, kill.round, kill.tick, &focus);
            let flag = |rule_id, confidence, severity, details| RuleFlag {
                rule_id,
                round: kill.round,
                tick: kill.tick,
                steamid: victim,
                confidence,
                severity,
                details,
                evidence: evidence(),
            };

            let death_state = ctx.state_at(victim, kill.tick);
            let death_weapon = death_state.as_ref().and_then(|s| s.weapon.clone());

            // H3_DIED_WITH_NADE_OUT (class 1): active weapon at death is a
            // grenade.
            let mut nade_out_fired = false;
            if let Some(w) = &death_weapon {
                if is_grenade_weapon(w, cfg) {
                    nade_out_fired = true;
                    flags.push(flag(
                        "H3_DIED_WITH_NADE_OUT",
                        0.85,
                        cfg.severity.h3_died_with_nade_out,
                        json!({ "weapon": w }),
                    ));
                }
            }

            // H3_DIED_MID_SWITCH (class 1): weapon changed between the sample
            // switch_window_s before death and the death sample. Suppressed
            // when nade-out already claimed this death (no double-anchoring
            // class 1) and when the death-tick weapon is itself a grenade.
            if !nade_out_fired {
                let earlier = ctx.state_at(victim, kill.tick - ctx.seconds(cfg.h3.switch_window_s));
                if let (Some(now_w), Some(prev_w)) = (&death_weapon, earlier.and_then(|s| s.weapon))
                {
                    if prev_w != *now_w && !is_grenade_weapon(now_w, cfg) {
                        flags.push(flag(
                            "H3_DIED_MID_SWITCH",
                            0.7, // 16 Hz sampling makes the switch time approximate
                            cfg.severity.h3_died_mid_switch,
                            json!({ "from": prev_w, "to": now_w }),
                        ));
                    }
                }
            }

            // H3_DIED_RELOADING (class 4): a reload within reload_window_s
            // before death, and no shot between it and the death (a shot
            // means the reload finished).
            let reload_t0 = kill.tick - ctx.seconds(cfg.h3.reload_window_s);
            if let Some(reload) = ctx
                .reloads_by_in(victim, reload_t0, kill.tick)
                .into_iter()
                .max_by_key(|r| r.tick)
            {
                if ctx
                    .shots_by_in(victim, reload.tick + 1, kill.tick)
                    .is_empty()
                {
                    flags.push(flag(
                        "H3_DIED_RELOADING",
                        0.7,
                        cfg.severity.h3_died_reloading,
                        json!({
                            "reload_tick": reload.tick,
                            "seconds_before_death":
                                (kill.tick - reload.tick) as f32 / ctx.data().tickrate,
                        }),
                    ));
                }
            }

            // H3_DIED_SCOPED_CLOSE (class 4): scoped at the death sample with
            // the nearest enemy inside scoped_close_u. Missing is_scoped data
            // reads as false (silence bias).
            if death_state.as_ref().is_some_and(|s| s.is_scoped) {
                if let Some((enemy, dist)) =
                    ctx.nearest_enemy(victim, kill.round, kill.tick, cfg.general.z_weight)
                {
                    if dist < cfg.h3.scoped_close_u {
                        flags.push(flag(
                            "H3_DIED_SCOPED_CLOSE",
                            0.8,
                            cfg.severity.h3_died_scoped_close,
                            json!({ "nearest_enemy": enemy.to_string(), "distance_u": dist }),
                        ));
                    }
                }
            }

            // H3_WASTED_UTILITY (secondary tag, no class): the death-tick
            // inventory holds >=1 grenade. Inventory samples exist only at
            // exact death/round-end ticks — silent when absent.
            if let Some(inv) = ctx.inventory_at(victim, kill.tick) {
                let held: Vec<&str> = inv
                    .items
                    .iter()
                    .filter(|i| cfg.util.grenade_items.iter().any(|g| g == *i))
                    .map(String::as_str)
                    .collect();
                if !held.is_empty() {
                    flags.push(flag(
                        "H3_WASTED_UTILITY",
                        0.9,
                        cfg.severity.h3_wasted_utility,
                        json!({ "held": held, "count": held.len() }),
                    ));
                }
            }
        }
        flags
    }

    fn insights(
        &self,
        ctx: &AnalysisContext,
        cfg: &DetectorConfig,
        flags: &[RuleFlag],
    ) -> Vec<Insight> {
        let mut out = vec![];
        let total_deaths = ctx.tracked_deaths().len();

        // (a) Vulnerable deaths: >=2 deaths where the victim couldn't shoot
        // back (spec H3 headline metric: Vulnerable Death %).
        let mut deaths_seen = BTreeSet::new();
        let mut evidence: Vec<EvidenceRef> = vec![];
        let mut severity: f32 = 0.0;
        let mut confidence: f32 = 1.0;
        for f in flags
            .iter()
            .filter(|f| VULNERABLE_RULES.contains(&f.rule_id))
        {
            if deaths_seen.insert((f.round, f.tick)) {
                evidence.push(f.evidence.clone());
            }
            severity = severity.max(f.severity);
            confidence = confidence.min(f.confidence);
        }
        let vulnerable = deaths_seen.len();
        if vulnerable >= 2 {
            evidence.truncate(8);
            out.push(Insight {
                detector: "H3_VULNERABLE_DEATHS".to_string(),
                category: Category::Deaths,
                severity,
                confidence,
                round: 0,
                player: ctx.tracked(),
                title_data: json!({ "vulnerable": vulnerable, "total_deaths": total_deaths }),
                metrics: json!({
                    "vulnerable": vulnerable,
                    "total_deaths": total_deaths,
                    "pct": vulnerable as f32 / total_deaths.max(1) as f32,
                }),
                evidence,
            });
        }

        // (b) Wasted utility: >=3 deaths while holding unthrown grenades.
        let wasted: Vec<&RuleFlag> = flags
            .iter()
            .filter(|f| f.rule_id == "H3_WASTED_UTILITY")
            .collect();
        if wasted.len() >= 3 {
            let mut item_counts: BTreeMap<String, usize> = BTreeMap::new();
            for f in &wasted {
                for item in f
                    .details
                    .get("held")
                    .and_then(|h| h.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str())
                {
                    *item_counts.entry(item.to_string()).or_insert(0) += 1;
                }
            }
            let most_common_item = item_counts
                .iter()
                .max_by_key(|(_, c)| **c)
                .map(|(name, _)| name.clone())
                .unwrap_or_default();
            out.push(Insight {
                detector: "H3_WASTED_UTILITY".to_string(),
                category: Category::Utility,
                severity: cfg.severity.h3_wasted_utility,
                confidence: 0.9,
                round: 0,
                player: ctx.tracked(),
                title_data: json!({ "deaths_holding": wasted.len(), "total_deaths": total_deaths }),
                metrics: json!({
                    "deaths_holding": wasted.len(),
                    "total_deaths": total_deaths,
                    "most_common_item": most_common_item,
                }),
                evidence: wasted.iter().take(8).map(|f| f.evidence.clone()).collect(),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::Scenario;
    use cf_parser::model::MatchData;

    const TRACKED: u64 = 1;
    const KILLER: u64 = 3;

    fn run(data: &MatchData) -> (Vec<RuleFlag>, Vec<Insight>) {
        let ctx = AnalysisContext::new(data, TRACKED);
        let cfg = DetectorConfig::default();
        let flags = H3UtilityVulnerability.detect(&ctx, &cfg);
        let insights = H3UtilityVulnerability.insights(&ctx, &cfg, &flags);
        (flags, insights)
    }

    fn ids(flags: &[RuleFlag]) -> Vec<&'static str> {
        flags.iter().map(|f| f.rule_id).collect()
    }

    /// One round, killer parked far away (2000 u) so scoped-close can't fire
    /// by accident.
    fn base() -> Scenario {
        Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[KILLER])
            .round(1, 1000, 5000)
            .hold(KILLER, 1000, 5000, 2000.0, 0.0, 0.0)
    }

    /// Victim stationary at origin holding `weapon` across [t0, t1].
    fn hold_weapon(s: Scenario, sid: u64, t0: i32, t1: i32, weapon: &str) -> Scenario {
        s.waypoint_full(
            sid,
            t0,
            0.0,
            0.0,
            0.0,
            0.0,
            100,
            true,
            Some(weapon),
            None,
            false,
        )
        .waypoint_full(
            sid,
            t1,
            0.0,
            0.0,
            0.0,
            0.0,
            100,
            true,
            Some(weapon),
            None,
            false,
        )
    }

    // ---- H3_DIED_WITH_NADE_OUT ----

    #[test]
    fn nade_out_fires_on_display_name_grenade() {
        let data = hold_weapon(base(), TRACKED, 1000, 2000, "Flashbang")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        let f = flags
            .iter()
            .find(|f| f.rule_id == "H3_DIED_WITH_NADE_OUT")
            .expect("nade-out must fire on display-name grenade");
        assert_eq!(f.tick, 2000, "anchored on the kill tick");
        assert_eq!(f.steamid, TRACKED, "anchored on the victim");
        assert_eq!(f.round, 1);
        assert!((f.confidence - 0.85).abs() < 1e-6);
        assert_eq!(
            f.severity,
            DetectorConfig::default().severity.h3_died_with_nade_out
        );
        assert_eq!(f.details["weapon"], "Flashbang");
        // Standard evidence window: 5 s before -> 2 s after, both players.
        assert_eq!(f.evidence.round, 1);
        assert_eq!(f.evidence.tick_start, 2000 - 320);
        assert_eq!(f.evidence.tick_end, 2000 + 128);
        assert!(f.evidence.focus_players.contains(&TRACKED));
        assert!(f.evidence.focus_players.contains(&KILLER));
    }

    #[test]
    fn nade_out_fires_on_weapon_prefixed_code_name() {
        let data = hold_weapon(base(), TRACKED, 1000, 2000, "weapon_smokegrenade")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        assert!(
            ids(&flags).contains(&"H3_DIED_WITH_NADE_OUT"),
            "nade-out must also match weapon_-prefixed code names"
        );
    }

    #[test]
    fn nade_out_suppressed_with_rifle_out() {
        // .hold gives weapon_ak47 throughout.
        let data = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        assert!(
            flags.is_empty(),
            "rifle out, no reload/scope/inventory: nothing may fire, got {:?}",
            ids(&flags)
        );
    }

    // ---- H3_DIED_MID_SWITCH ----

    /// Victim on ak47 until 1988, deagle from 1992 — a switch inside the
    /// 0.3 s window before the 2000 death.
    fn mid_switch_scenario(to_weapon: &str) -> Scenario {
        let s = base();
        let s = hold_weapon(s, TRACKED, 1000, 1988, "weapon_ak47");
        hold_weapon(s, TRACKED, 1992, 2000, to_weapon).kill(KILLER, TRACKED, 1, 2000, "ak47")
    }

    #[test]
    fn mid_switch_fires_on_weapon_change_within_window() {
        let data = mid_switch_scenario("weapon_deagle").build();
        let (flags, _) = run(&data);
        let f = flags
            .iter()
            .find(|f| f.rule_id == "H3_DIED_MID_SWITCH")
            .expect("mid-switch must fire on a weapon change inside the window");
        assert_eq!(f.tick, 2000);
        assert_eq!(f.steamid, TRACKED);
        assert!(
            (f.confidence - 0.7).abs() < 1e-6,
            "16 Hz sampling makes this approximate"
        );
        assert_eq!(f.details["from"], "weapon_ak47");
        assert_eq!(f.details["to"], "weapon_deagle");
        assert!(
            !ids(&flags).contains(&"H3_DIED_WITH_NADE_OUT"),
            "deagle is not a grenade"
        );
    }

    #[test]
    fn mid_switch_suppressed_when_weapon_unchanged() {
        let data = hold_weapon(base(), TRACKED, 1000, 2000, "weapon_deagle")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        assert!(
            !ids(&flags).contains(&"H3_DIED_MID_SWITCH"),
            "same weapon across the window must not flag"
        );
    }

    #[test]
    fn mid_switch_yields_to_nade_out_when_switching_to_grenade() {
        let data = mid_switch_scenario("weapon_flashbang").build();
        let (flags, _) = run(&data);
        assert!(
            ids(&flags).contains(&"H3_DIED_WITH_NADE_OUT"),
            "the switch was TO a grenade: nade-out claims the death"
        );
        assert!(
            !ids(&flags).contains(&"H3_DIED_MID_SWITCH"),
            "class 1 must not be double-anchored"
        );
    }

    // ---- H3_DIED_RELOADING ----

    #[test]
    fn reloading_fires_when_no_shot_after_reload() {
        // Reload 1 s (64 ticks) before death, inside the 2 s window.
        let data = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .reload(TRACKED, 1936)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        let f = flags
            .iter()
            .find(|f| f.rule_id == "H3_DIED_RELOADING")
            .expect("reloading must fire: reload 1 s before death, no shot after");
        assert_eq!(f.tick, 2000);
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.7).abs() < 1e-6);
    }

    #[test]
    fn reloading_suppressed_when_victim_shot_after_reload() {
        let data = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .reload(TRACKED, 1936)
            .shot(TRACKED, 1960, "weapon_ak47")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, _) = run(&data);
        assert!(
            !ids(&flags).contains(&"H3_DIED_RELOADING"),
            "a shot after the reload means the reload finished"
        );
    }

    // ---- H3_DIED_SCOPED_CLOSE ----

    fn scoped_scenario(enemy_x: f32) -> MatchData {
        Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[KILLER])
            .round(1, 1000, 5000)
            .hold(KILLER, 1000, 5000, enemy_x, 0.0, 0.0)
            .waypoint_full(
                TRACKED,
                1000,
                0.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_awp"),
                None,
                true,
            )
            .waypoint_full(
                TRACKED,
                2000,
                0.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_awp"),
                None,
                true,
            )
            .kill(KILLER, TRACKED, 1, 2000, "awp")
            .build()
    }

    #[test]
    fn scoped_close_fires_on_nearby_enemy() {
        let data = scoped_scenario(400.0);
        let (flags, _) = run(&data);
        let f = flags
            .iter()
            .find(|f| f.rule_id == "H3_DIED_SCOPED_CLOSE")
            .expect("scoped at death with an enemy at 400 u must fire");
        assert_eq!(f.tick, 2000);
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.8).abs() < 1e-6);
    }

    #[test]
    fn scoped_close_suppressed_when_enemy_far() {
        let data = scoped_scenario(800.0);
        let (flags, _) = run(&data);
        assert!(
            !ids(&flags).contains(&"H3_DIED_SCOPED_CLOSE"),
            "800 u is outside scoped_close_u (600)"
        );
    }

    // ---- H3_WASTED_UTILITY ----

    #[test]
    fn wasted_utility_uses_pre_death_inventory_sample() {
        // Inventory sample at the exact kill tick -> fires.
        let with_sample = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .inventory(
                TRACKED,
                2000,
                &["Flashbang", "Smoke Grenade", "Desert Eagle"],
            )
            .build();
        let (flags, _) = run(&with_sample);
        let f = flags
            .iter()
            .find(|f| f.rule_id == "H3_WASTED_UTILITY")
            .expect("must fire when the death-tick inventory holds grenades");
        assert_eq!(f.tick, 2000);
        assert_eq!(f.steamid, TRACKED);
        assert!((f.confidence - 0.9).abs() < 1e-6);
        assert_eq!(
            f.severity,
            DetectorConfig::default().severity.h3_wasted_utility
        );
        assert_eq!(f.details["count"], 2, "only grenade items count");
        let held: Vec<String> = f.details["held"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(held, vec!["Flashbang", "Smoke Grenade"]);

        // Pre-death sample shortly before the kill tick -> fires (the parser
        // samples ~0.25 s pre-death because death-tick inventories are
        // already dropped; inventory_at looks back ≤ 0.5 s).
        let pre_death = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .inventory(TRACKED, 1996, &["Flashbang"])
            .build();
        let (flags, _) = run(&pre_death);
        assert!(ids(&flags).contains(&"H3_WASTED_UTILITY"));

        // Sample far before the death (> 0.5 s) -> silent.
        let stale = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .inventory(TRACKED, 1900, &["Flashbang"])
            .build();
        let (flags, _) = run(&stale);
        assert!(!ids(&flags).contains(&"H3_WASTED_UTILITY"));

        // Sample with no grenade items -> silent.
        let no_nades = base()
            .hold(TRACKED, 1000, 2000, 0.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .inventory(TRACKED, 2000, &["Desert Eagle"])
            .build();
        let (flags, _) = run(&no_nades);
        assert!(!ids(&flags).contains(&"H3_WASTED_UTILITY"));
    }

    // ---- insights ----

    #[test]
    fn vulnerable_insight_fires_at_two_vulnerable_deaths() {
        let s = Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[KILLER])
            .round(1, 1000, 3000)
            .round(2, 4000, 6000)
            .hold(KILLER, 1000, 6000, 2000.0, 0.0, 0.0);
        let s = hold_weapon(s, TRACKED, 1000, 2000, "Flashbang");
        let data = hold_weapon(s, TRACKED, 4000, 5000, "Flashbang")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .kill(KILLER, TRACKED, 2, 5000, "ak47")
            .build();
        let (flags, insights) = run(&data);
        assert_eq!(flags.len(), 2, "one nade-out per death");
        assert_eq!(insights.len(), 1);
        let i = &insights[0];
        assert_eq!(i.detector, "H3_VULNERABLE_DEATHS");
        assert_eq!(i.category, Category::Deaths);
        assert_eq!(i.round, 0, "match-level");
        assert_eq!(i.player, TRACKED);
        assert_eq!(i.metrics["vulnerable"], 2);
        assert_eq!(i.metrics["total_deaths"], 2);
        assert!((i.metrics["pct"].as_f64().unwrap() - 1.0).abs() < 1e-6);
        assert_eq!(i.evidence.len(), 2, "one evidence ref per vulnerable death");
    }

    #[test]
    fn vulnerable_insight_suppressed_below_two_deaths() {
        let data = hold_weapon(base(), TRACKED, 1000, 2000, "Flashbang")
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .build();
        let (flags, insights) = run(&data);
        assert_eq!(ids(&flags), vec!["H3_DIED_WITH_NADE_OUT"]);
        assert!(
            insights.is_empty(),
            "one vulnerable death is below the >=2 threshold"
        );
    }

    #[test]
    fn vulnerable_insight_counts_deaths_not_flags() {
        // One death that trips BOTH reloading and scoped-close: two flags,
        // one death -> still below the >=2 deaths threshold.
        let data = Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[KILLER])
            .round(1, 1000, 5000)
            .hold(KILLER, 1000, 5000, 400.0, 0.0, 0.0)
            .waypoint_full(
                TRACKED,
                1000,
                0.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_awp"),
                None,
                true,
            )
            .waypoint_full(
                TRACKED,
                2000,
                0.0,
                0.0,
                0.0,
                0.0,
                100,
                true,
                Some("weapon_awp"),
                None,
                true,
            )
            .reload(TRACKED, 1936)
            .kill(KILLER, TRACKED, 1, 2000, "awp")
            .build();
        let (flags, insights) = run(&data);
        assert!(ids(&flags).contains(&"H3_DIED_RELOADING"));
        assert!(ids(&flags).contains(&"H3_DIED_SCOPED_CLOSE"));
        assert!(
            insights.is_empty(),
            "two flags on ONE death is one vulnerable death, not two"
        );
    }

    fn wasted_scenario(deaths_with_inventory: usize) -> MatchData {
        let mut s = Scenario::new("de_test")
            .players_ct(&[TRACKED])
            .players_t(&[KILLER])
            .round(1, 1000, 3000)
            .round(2, 4000, 6000)
            .round(3, 7000, 9000)
            .hold(TRACKED, 1000, 9000, 0.0, 0.0, 0.0)
            .hold(KILLER, 1000, 9000, 2000.0, 0.0, 0.0)
            .kill(KILLER, TRACKED, 1, 2000, "ak47")
            .kill(KILLER, TRACKED, 2, 5000, "ak47")
            .kill(KILLER, TRACKED, 3, 8000, "ak47");
        let items: [&[&str]; 3] = [
            &["Flashbang", "Smoke Grenade"],
            &["Flashbang"],
            &["Flashbang"],
        ];
        for (i, tick) in [2000, 5000, 8000]
            .iter()
            .take(deaths_with_inventory)
            .enumerate()
        {
            s = s.inventory(TRACKED, *tick, items[i]);
        }
        s.build()
    }

    #[test]
    fn wasted_insight_fires_at_three_deaths_holding_utility() {
        let data = wasted_scenario(3);
        let (flags, insights) = run(&data);
        assert_eq!(
            flags
                .iter()
                .filter(|f| f.rule_id == "H3_WASTED_UTILITY")
                .count(),
            3
        );
        let i = insights
            .iter()
            .find(|i| i.detector == "H3_WASTED_UTILITY")
            .expect("wasted-utility insight must fire at >=3 deaths holding");
        assert_eq!(i.category, Category::Utility);
        assert_eq!(i.player, TRACKED);
        assert_eq!(i.metrics["deaths_holding"], 3);
        assert_eq!(i.metrics["total_deaths"], 3);
        assert_eq!(i.metrics["most_common_item"], "Flashbang");
        assert_eq!(i.evidence.len(), 3);
        // No vulnerable-deaths insight: nothing here was a vulnerability rule.
        assert!(insights
            .iter()
            .all(|i| i.detector != "H3_VULNERABLE_DEATHS"));
    }

    #[test]
    fn wasted_insight_suppressed_below_three_deaths() {
        let data = wasted_scenario(2);
        let (flags, insights) = run(&data);
        assert_eq!(
            flags
                .iter()
                .filter(|f| f.rule_id == "H3_WASTED_UTILITY")
                .count(),
            2
        );
        assert!(
            insights.iter().all(|i| i.detector != "H3_WASTED_UTILITY"),
            "two deaths holding utility is below the >=3 threshold"
        );
    }
}
