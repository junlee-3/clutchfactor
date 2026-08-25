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

/// Round-phase boundaries (issue #9: distinguishes "on the entry" from "in
/// the retake"). Seconds after freeze end; post-plant overrides the clock.
/// opening_end_s matches timing.early_aggression_s's default — same "early"
/// concept (§6.4); mid_end_s splits the rest where save/last-alive
/// decisions start dominating an MR12 round. Tunable approximations.
#[derive(Debug, Clone, Deserialize)]
pub struct PhaseCfg {
    #[serde(default = "d_phase_opening_end")]
    pub opening_end_s: f32,
    #[serde(default = "d_phase_mid_end")]
    pub mid_end_s: f32,
}
fn d_phase_opening_end() -> f32 {
    20.0
}
fn d_phase_mid_end() -> f32 {
    50.0
}

/// Issue #9 round selection + attention thresholds. Impact is a win-prob
/// delta (0..1 scale) from the tracked player's side. Tunable
/// approximations — calibrated during the V1.2 §12 hand-verification pass.
#[derive(Debug, Clone, Deserialize)]
pub struct RbrCfg {
    /// A round is selected for coaching when |impact| ≥ this (dim dot).
    #[serde(default = "d_rbr_attention_p")]
    pub attention_threshold_p: f32,
    /// Selected rounds at/above this show the bright "pivotal" dot.
    #[serde(default = "d_rbr_pivotal_p")]
    pub pivotal_threshold_p: f32,
    /// Cap on surfaced rounds (threshold-with-cap, never fixed top-N).
    #[serde(default = "d_rbr_max_rounds")]
    pub max_rounds: usize,
    /// Cap on moments per round (kept in tick order).
    #[serde(default = "d_rbr_max_moments")]
    pub max_moments: usize,
    /// Rules that positively establish "Not on you" (issue #9: never
    /// inferred from the absence of flags).
    #[serde(default = "d_rbr_exculpatory")]
    pub exculpatory_rules: Vec<String>,
}
fn d_rbr_attention_p() -> f32 {
    // Calibrated 2026-08-22 (V1.2 §12 hand-verification): 0.18 saturated the
    // 6-round cap on all 5 owner demos (11-15 candidates each — see
    // ADR-0008's "Calibration" amendment). 0.25 sits in a real gap in the
    // observed real-match impact distribution, just above the single-
    // early-duel band (~0.16-0.24), and de-saturates 4/5 matches.
    0.25
}
fn d_rbr_pivotal_p() -> f32 {
    0.35
}
fn d_rbr_max_rounds() -> usize {
    6
}
fn d_rbr_max_moments() -> usize {
    6
}
fn d_rbr_exculpatory() -> Vec<String> {
    vec!["H2_BAITED_TRADE".to_string()]
}

/// V1.2b play ledger (docs/spec/play-ledger-and-coach.md §2). Seconds only.
#[derive(Debug, Clone, Deserialize)]
pub struct LedgerCfg {
    /// Setup checkpoint: positioning is sampled this long after freeze end.
    #[serde(default = "d_ledger_setup_s")]
    pub setup_s: f32,
    /// HE damage is attributed to a detonate within this window after it.
    #[serde(default = "d_ledger_he_window_s")]
    pub he_window_s: f32,
    /// Molotov burn length assumed when no `molotov_expire` event follows.
    #[serde(default = "d_ledger_molotov_burn_s")]
    pub molotov_burn_s: f32,
    /// A flashbang detonate and a blind group this close in time are the
    /// same grenade (detonate and blind ticks are not guaranteed equal).
    #[serde(default = "d_ledger_flash_join_s")]
    pub flash_join_s: f32,
    /// Step used when walking tick samples for rush / rotation checkpoints.
    #[serde(default = "d_ledger_sample_step_s")]
    pub sample_step_s: f32,
}
fn d_ledger_setup_s() -> f32 {
    5.0
}
fn d_ledger_he_window_s() -> f32 {
    0.5
}
fn d_ledger_molotov_burn_s() -> f32 {
    7.0
}
fn d_ledger_flash_join_s() -> f32 {
    0.25
}
fn d_ledger_sample_step_s() -> f32 {
    1.0
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
    /// Bound on the tick-sample lookback when resolving a death's position —
    /// prevents cross-round fallback (issue #6 §4).
    #[serde(default = "d_death_pos_lookback_s")]
    pub death_pos_lookback_s: f32,
}
fn d_death_pos_lookback_s() -> f32 {
    10.0
}

/// D6 / reference corpus (PROMPT.md §5 D6, §6.4).
#[derive(Debug, Clone, Deserialize)]
pub struct CorpusCfg {
    /// Corpus gate: D6 is silent for a map below this many corpus demos.
    #[serde(default = "d_min_demos_per_map")]
    pub min_demos_per_map: usize,
    #[serde(default = "d_grid_size")]
    pub grid_size: usize,
    #[serde(default = "d_freeze_sample_s")]
    pub freeze_sample_s: f32,
    #[serde(default = "d_early_s")]
    pub early_s: f32,
    #[serde(default = "d_mid_s")]
    pub mid_s: f32,
    #[serde(default = "d_post_plant_s")]
    pub post_plant_s: f32,
    /// Percentile (of non-zero pooled densities) below which a position
    /// counts as "rarely held by reference players".
    #[serde(default = "d_low_density_pct")]
    pub low_density_pct: f32,
    /// Rounds with low-density positioning before an insight emits.
    #[serde(default = "d_min_recurrences")]
    pub min_recurrences: usize,
    /// Chebyshev radius of cells pooled around the player's cell.
    #[serde(default = "d_neighborhood")]
    pub neighborhood: usize,
}

fn d_min_demos_per_map() -> usize {
    8
}
fn d_grid_size() -> usize {
    128
}
fn d_freeze_sample_s() -> f32 {
    1.0
}
fn d_early_s() -> f32 {
    10.0
}
fn d_mid_s() -> f32 {
    35.0
}
fn d_post_plant_s() -> f32 {
    5.0
}
fn d_low_density_pct() -> f32 {
    5.0
}
fn d_min_recurrences() -> usize {
    3
}
fn d_neighborhood() -> usize {
    1
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
    CorpusCfg,
    TimingCfg,
    HabitCfg,
    GeneralCfg,
    SeverityCfg,
    PhaseCfg,
    RbrCfg,
    LedgerCfg
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
    pub corpus: CorpusCfg,
    #[serde(default)]
    pub general: GeneralCfg,
    #[serde(default)]
    pub severity: SeverityCfg,
    #[serde(default)]
    pub phase: PhaseCfg,
    #[serde(default)]
    pub rbr: RbrCfg,
    #[serde(default)]
    pub ledger: LedgerCfg,
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
        // V1.2 RBR round selection & attention thresholds (issue #9).
        // attention_threshold_p calibrated 0.18 -> 0.25 2026-08-22 (V1.2 §12
        // hand-verification; ADR-0008 Calibration amendment).
        assert_eq!(c.rbr.attention_threshold_p, 0.25);
        assert_eq!(c.rbr.pivotal_threshold_p, 0.35);
        assert_eq!(c.rbr.max_rounds, 6);
        assert_eq!(c.rbr.max_moments, 6);
        assert_eq!(c.rbr.exculpatory_rules, vec!["H2_BAITED_TRADE"]);
        // V1.2b play ledger (docs/spec/play-ledger-and-coach.md §2).
        assert_eq!(c.ledger.setup_s, 5.0);
        assert_eq!(c.ledger.he_window_s, 0.5);
        assert_eq!(c.ledger.molotov_burn_s, 7.0);
        assert_eq!(c.ledger.flash_join_s, 0.25);
        assert_eq!(c.ledger.sample_step_s, 1.0);
    }

    #[test]
    fn yaml_overrides_merge_over_defaults() {
        let c = DetectorConfig::from_yaml("trade:\n  isolation_u: 1200\n").unwrap();
        assert_eq!(c.trade.isolation_u, 1200.0);
        assert_eq!(c.trade.window_s, 3.0, "untouched fields keep defaults");
        assert_eq!(c.flash.effective_s, 1.1);
        // RBR YAML merge: max_rounds override keeps other fields at defaults.
        let c = DetectorConfig::from_yaml("rbr:\n  max_rounds: 3\n").unwrap();
        assert_eq!(c.rbr.max_rounds, 3);
        assert_eq!(
            c.rbr.attention_threshold_p, 0.25,
            "untouched fields keep defaults"
        );
        assert_eq!(c.rbr.pivotal_threshold_p, 0.35);
        assert_eq!(c.rbr.max_moments, 6);
        assert_eq!(c.rbr.exculpatory_rules, vec!["H2_BAITED_TRADE"]);
    }

    #[test]
    fn grenade_item_names_are_the_verified_display_names() {
        let c = DetectorConfig::default();
        assert!(c.util.grenade_items.iter().any(|s| s == "Smoke Grenade"));
        assert!(c.util.grenade_items.iter().any(|s| s == "Flashbang"));
    }
}
