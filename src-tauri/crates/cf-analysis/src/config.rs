//! DetectorConfig: every tunable threshold, in seconds and world units only
//! (PROMPT.md §6.4, spec §4 principles). Defaults are embedded YAML; a user
//! file merges over them field-by-field via serde defaults.

use serde::Deserialize;

fn d_trade_window_s() -> f32 {
    3.0
}
fn d_trade_distance_u() -> f32 {
    700.0
}
fn d_isolation_u() -> f32 {
    900.0
}
fn d_commit_window_s() -> f32 {
    2.0
}
fn d_effective_s() -> f32 {
    1.1
}
fn d_conversion_window_s() -> f32 {
    2.0
}
fn d_switch_window_s() -> f32 {
    0.3
}
fn d_reload_window_s() -> f32 {
    2.0
}
fn d_scoped_close_u() -> f32 {
    600.0
}
fn d_no_shot_window_s() -> f32 {
    3.0
}
fn d_no_contact_window_s() -> f32 {
    2.0
}
fn d_fire_linger_dmg() -> i32 {
    20
}
fn d_fire_linger_s() -> f32 {
    1.0
}
fn d_crossfire_engage_window_s() -> f32 {
    2.0
}
fn d_crossfire_min_angle_deg() -> f32 {
    45.0
}
fn d_contactless_window_s() -> f32 {
    2.0
}
fn d_min_unused_nades() -> usize {
    2
}
fn d_grenade_items() -> Vec<String> {
    // Inventory display names, verified on real demos 2026-08-19.
    [
        "Flashbang",
        "Smoke Grenade",
        "High Explosive Grenade",
        "Molotov",
        "Incendiary Grenade",
        "Decoy Grenade",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
fn d_utility_kill_weapons() -> Vec<String> {
    // Kill/hurt event weapon names (unprefixed).
    ["hegrenade", "inferno", "molotov", "incgrenade"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
fn d_z_weight() -> f32 {
    2.0
}
fn d_support_distance_u() -> f32 {
    700.0
}
fn d_opening_window_s() -> f32 {
    15.0
}
fn d_early_aggression_s() -> f32 {
    20.0
}
fn d_rotate_radius_u() -> f32 {
    800.0
}
fn d_rotate_max_s() -> f32 {
    25.0
}
fn d_min_spawn_distance_u() -> f32 {
    750.0
}
fn d_habit_min_matches() -> usize {
    3
}
fn d_habit_window_matches() -> usize {
    10
}
fn d_hotspot_radius_u() -> f32 {
    250.0
}
fn d_hotspot_min_deaths() -> usize {
    3
}
fn d_hotspot_min_matches() -> usize {
    2
}
fn d_fallthrough_duel_window_s() -> f32 {
    3.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradeCfg {
    #[serde(default = "d_trade_window_s")]
    pub window_s: f32,
    #[serde(default = "d_trade_distance_u")]
    pub distance_u: f32,
    #[serde(default = "d_isolation_u")]
    pub isolation_u: f32,
    #[serde(default = "d_commit_window_s")]
    pub commit_window_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashCfg {
    #[serde(default = "d_effective_s")]
    pub effective_s: f32,
    #[serde(default = "d_conversion_window_s")]
    pub conversion_window_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct H3Cfg {
    #[serde(default = "d_switch_window_s")]
    pub switch_window_s: f32,
    #[serde(default = "d_reload_window_s")]
    pub reload_window_s: f32,
    #[serde(default = "d_scoped_close_u")]
    pub scoped_close_u: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct H16Cfg {
    #[serde(default = "d_no_shot_window_s")]
    pub no_shot_window_s: f32,
    #[serde(default = "d_no_contact_window_s")]
    pub no_contact_window_s: f32,
    #[serde(default = "d_fire_linger_dmg")]
    pub fire_linger_dmg: i32,
    #[serde(default = "d_fire_linger_s")]
    pub fire_linger_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct H4Cfg {
    #[serde(default = "d_crossfire_engage_window_s")]
    pub crossfire_engage_window_s: f32,
    #[serde(default = "d_crossfire_min_angle_deg")]
    pub crossfire_min_angle_deg: f32,
    #[serde(default = "d_contactless_window_s")]
    pub contactless_window_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UtilCfg {
    #[serde(default = "d_min_unused_nades")]
    pub min_unused_nades: usize,
    #[serde(default = "d_grenade_items")]
    pub grenade_items: Vec<String>,
    #[serde(default = "d_utility_kill_weapons")]
    pub utility_kill_weapons: Vec<String>,
}

/// D4 — opening-duel / entry structure.
#[derive(Debug, Clone, Deserialize)]
pub struct EntryCfg {
    #[serde(default = "d_support_distance_u")]
    pub support_distance_u: f32,
    /// The round's first kill counts as an "entry" only within this many
    /// seconds of freeze end (later first-kills are mid-round picks).
    #[serde(default = "d_opening_window_s")]
    pub opening_window_s: f32,
}

/// D5 — timing & rotation.
#[derive(Debug, Clone, Deserialize)]
pub struct TimingCfg {
    /// §6.4: early aggressive death = within 20 s of round_freeze_end.
    #[serde(default = "d_early_aggression_s")]
    pub early_aggression_s: f32,
    #[serde(default = "d_rotate_radius_u")]
    pub rotate_radius_u: f32,
    #[serde(default = "d_rotate_max_s")]
    pub rotate_max_s: f32,
    /// Dying closer than this to your own freeze-end position is not
    /// "aggressive depth".
    #[serde(default = "d_min_spawn_distance_u")]
    pub min_spawn_distance_u: f32,
}

/// §5A cross-demo habit promotion (+ spec H4_REPEAT_HOTSPOT parameters).
#[derive(Debug, Clone, Deserialize)]
pub struct HabitCfg {
    #[serde(default = "d_habit_min_matches")]
    pub min_matches: usize,
    #[serde(default = "d_habit_window_matches")]
    pub window_matches: usize,
    #[serde(default = "d_hotspot_radius_u")]
    pub hotspot_radius_u: f32,
    #[serde(default = "d_hotspot_min_deaths")]
    pub hotspot_min_deaths: usize,
    #[serde(default = "d_hotspot_min_matches")]
    pub hotspot_min_matches: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralCfg {
    /// Vertical distance weight (spec H2 refinement: z-difference matters more).
    #[serde(default = "d_z_weight")]
    pub z_weight: f32,
    /// Class 13 vs 15: a death is a "fair duel lost" only if the victim shot
    /// at or damaged the killer within this window.
    #[serde(default = "d_fallthrough_duel_window_s")]
    pub fallthrough_duel_window_s: f32,
}

/// Per-rule severity (0–1). Baited-trade is capped well below isolated (spec
/// §2 H2: weighting them equally teaches the player to stop trading).
#[derive(Debug, Clone, Deserialize)]
pub struct SeverityCfg {
    #[serde(default = "sev_isolated")]
    pub h2_isolated_death: f32,
    #[serde(default = "sev_failed_trade")]
    pub h2_failed_trade: f32,
    #[serde(default = "sev_baited_trade")]
    pub h2_baited_trade: f32,
    #[serde(default = "sev_default")]
    pub h3_died_with_nade_out: f32,
    #[serde(default = "sev_default")]
    pub h3_died_mid_switch: f32,
    #[serde(default = "sev_default")]
    pub h3_died_reloading: f32,
    #[serde(default = "sev_default")]
    pub h3_died_scoped_close: f32,
    #[serde(default = "sev_low")]
    pub h3_wasted_utility: f32,
    #[serde(default = "sev_default")]
    pub h16_died_to_utility_no_duel: f32,
    #[serde(default = "sev_low")]
    pub h16_fire_linger: f32,
    #[serde(default = "sev_default")]
    pub h4_killed_without_contact: f32,
    #[serde(default = "sev_default")]
    pub h4_caught_in_crossfire: f32,
    #[serde(default = "sev_default")]
    pub h5_died_flashed: f32,
    #[serde(default = "sev_default")]
    pub h6_flash_self_or_team: f32,
    #[serde(default = "sev_low")]
    pub h6_dead_time_smoke: f32,
    #[serde(default = "sev_low")]
    pub h6_unused_util_at_round_end: f32,
    #[serde(default = "sev_default")]
    pub h6_util_team_damage: f32,
    #[serde(default = "sev_default")]
    pub h14_unsupported_entry: f32,
    #[serde(default = "sev_slow_rotation")]
    pub h11_slow_rotation: f32,
    #[serde(default = "sev_default")]
    pub h11_early_aggressive_death: f32,
    #[serde(default = "sev_push_no_info")]
    pub h6_push_without_info: f32,
}

fn sev_slow_rotation() -> f32 {
    0.5
}
fn sev_push_no_info() -> f32 {
    0.7
}

fn sev_isolated() -> f32 {
    0.8
}
fn sev_failed_trade() -> f32 {
    0.6
}
fn sev_baited_trade() -> f32 {
    0.35
}
fn sev_default() -> f32 {
    0.6
}
fn sev_low() -> f32 {
    0.4
}

macro_rules! default_impl {
    ($($t:ty),*) => {$(
        impl Default for $t {
            fn default() -> Self {
                serde_yaml_ng::from_str("{}").expect("defaults")
            }
        }
    )*};
}
default_impl!(
    TradeCfg,
    FlashCfg,
    H3Cfg,
    H16Cfg,
    H4Cfg,
    UtilCfg,
    EntryCfg,
    TimingCfg,
    HabitCfg,
    GeneralCfg,
    SeverityCfg
);

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DetectorConfig {
    #[serde(default)]
    pub trade: TradeCfg,
    #[serde(default)]
    pub flash: FlashCfg,
    #[serde(default)]
    pub h3: H3Cfg,
    #[serde(default)]
    pub h16: H16Cfg,
    #[serde(default)]
    pub h4: H4Cfg,
    #[serde(default)]
    pub util: UtilCfg,
    #[serde(default)]
    pub entry: EntryCfg,
    #[serde(default)]
    pub timing: TimingCfg,
    #[serde(default)]
    pub habit: HabitCfg,
    #[serde(default)]
    pub general: GeneralCfg,
    #[serde(default)]
    pub severity: SeverityCfg,
}

impl DetectorConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml_ng::from_str(yaml).map_err(|e| format!("bad detector config: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec_6_4() {
        let c = DetectorConfig::default();
        assert_eq!(c.trade.window_s, 3.0);
        assert_eq!(c.trade.distance_u, 700.0);
        assert_eq!(c.trade.isolation_u, 900.0);
        assert_eq!(c.flash.effective_s, 1.1);
        assert_eq!(c.flash.conversion_window_s, 2.0);
        assert_eq!(c.h3.switch_window_s, 0.3);
        assert!(c.severity.h2_baited_trade < c.severity.h2_isolated_death / 2.0 + 0.01);
        // M4 additions (§6.4 early aggression + D4/D5/habit defaults).
        assert_eq!(c.timing.early_aggression_s, 20.0);
        assert_eq!(c.entry.support_distance_u, 700.0);
        assert_eq!(c.habit.min_matches, 3);
        assert_eq!(c.habit.hotspot_radius_u, 250.0);
    }

    #[test]
    fn yaml_overrides_merge_over_defaults() {
        let c = DetectorConfig::from_yaml("trade:\n  isolation_u: 1200\n").unwrap();
        assert_eq!(c.trade.isolation_u, 1200.0);
        assert_eq!(c.trade.window_s, 3.0, "untouched fields keep defaults");
        assert_eq!(c.flash.effective_s, 1.1);
    }

    #[test]
    fn grenade_item_names_are_the_verified_display_names() {
        let c = DetectorConfig::default();
        assert!(c.util.grenade_items.iter().any(|s| s == "Smoke Grenade"));
        assert!(c.util.grenade_items.iter().any(|s| s == "Flashbang"));
    }
}
