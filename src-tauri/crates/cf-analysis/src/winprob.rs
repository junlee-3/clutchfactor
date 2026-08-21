//! Win-probability lookup table (charter §6; ADR-0006). The single impact
//! currency for RBR round scoring (issue #9 §6 — the man-count heuristic is
//! explicitly rejected) and any future leak board.
//!
//! Data: embedded `data/win_prob_v1.yaml`, derived by
//! `examples/derive_winprob.rs` from OpenML dataset 43430 (CC0; 122,410
//! snapshots, 700 pro demos 2019-2020). CSGO-era: a documented
//! approximation — man-count/bomb dynamics carry to CS2; re-derive and bump
//! the version when a CS2 dataset of this quality exists.

use std::sync::OnceLock;

use serde::Deserialize;

const TABLE_YAML: &str = include_str!("../data/win_prob_v1.yaml");

#[derive(Debug, Deserialize)]
struct RawTable {
    version: String,
    cells: Vec<RawCell>,
}

#[derive(Debug, Deserialize)]
struct RawCell {
    ct: u8,
    t: u8,
    planted: bool,
    p_ct: f32,
    n: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct Cell {
    p_ct: f32,
    n: u32,
}

pub struct WinProbTable {
    version: String,
    // [ct 0..=5][t 0..=5][planted as usize]
    cells: [[[Option<Cell>; 2]; 6]; 6],
}

impl WinProbTable {
    /// The embedded v1 table. Corrupt embedded data is a build defect, so
    /// this panics rather than propagating an error nobody can handle.
    pub fn v1() -> &'static WinProbTable {
        static TABLE: OnceLock<WinProbTable> = OnceLock::new();
        TABLE.get_or_init(|| {
            let raw: RawTable =
                serde_yaml_ng::from_str(TABLE_YAML).expect("embedded win_prob_v1.yaml is valid");
            let mut cells: [[[Option<Cell>; 2]; 6]; 6] = Default::default();
            for c in raw.cells {
                assert!(c.ct <= 5 && c.t <= 5, "cell out of range: {c:?}");
                cells[c.ct as usize][c.t as usize][usize::from(c.planted)] = Some(Cell {
                    p_ct: c.p_ct,
                    n: c.n,
                });
            }
            WinProbTable {
                version: raw.version,
                cells,
            }
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// P(CT wins the round) given the live state. Terminal states clamp by
    /// rule; unobserved non-terminal states return None — a consumer must
    /// skip scoring that moment, never invent a number (silence bias).
    pub fn p_ct_win(&self, ct_alive: u8, t_alive: u8, planted: bool) -> Option<f32> {
        if ct_alive > 5 || t_alive > 5 {
            return None;
        }
        // All CTs dead: pre-plant the round is over (T win); post-plant
        // nobody can defuse. Either way CT cannot win.
        if ct_alive == 0 {
            return Some(0.0);
        }
        // All Ts dead with no bomb down: round over, CT win. (Planted is
        // NOT terminal — the bomb can still beat a failed defuse — so it
        // stays data-driven.)
        if t_alive == 0 && !planted {
            return Some(1.0);
        }
        self.cells[ct_alive as usize][t_alive as usize][usize::from(planted)].map(|c| c.p_ct)
    }

    /// Snapshot count behind a cell (0 for unobserved/clamped states) —
    /// exposed so consumers can weight confidence by evidence volume.
    pub fn sample_n(&self, ct_alive: u8, t_alive: u8, planted: bool) -> u32 {
        if ct_alive > 5 || t_alive > 5 {
            return 0;
        }
        self.cells[ct_alive as usize][t_alive as usize][usize::from(planted)].map_or(0, |c| c.n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_loads_with_version() {
        let t = WinProbTable::v1();
        assert_eq!(t.version(), "win_prob_v1");
    }

    #[test]
    fn anchors_match_published_reality() {
        let t = WinProbTable::v1();
        // 5v5 pre-plant hovers at a coin flip.
        let p55 = t.p_ct_win(5, 5, false).unwrap();
        assert!((0.45..=0.55).contains(&p55), "5v5 pre-plant: {p55}");
        // A man up pre-plant is a real edge (published 400k-round analyses: ~0.68).
        let p54 = t.p_ct_win(5, 4, false).unwrap();
        assert!((0.65..=0.75).contains(&p54), "5v4 pre-plant: {p54}");
        // The plant flips a full 5v5 hard toward T.
        let p55p = t.p_ct_win(5, 5, true).unwrap();
        assert!(p55p < 0.2, "5v5 planted: {p55p}");
        // Sample counts are exposed for honesty-aware consumers.
        assert!(t.sample_n(5, 5, false) > 50_000);
    }

    #[test]
    fn terminal_states_are_clamped_by_rule() {
        let t = WinProbTable::v1();
        assert_eq!(t.p_ct_win(3, 0, false), Some(1.0)); // all Ts dead, no bomb → CT won
        assert_eq!(t.p_ct_win(0, 3, false), Some(0.0)); // all CTs dead pre-plant → T won
        assert_eq!(t.p_ct_win(0, 3, true), Some(0.0)); // all CTs dead post-plant → nobody defuses
                                                       // Planted with zero Ts alive is NOT terminal (bomb can still win it);
                                                       // it stays data-driven.
        let p50p = t.p_ct_win(5, 0, true).unwrap();
        assert!(p50p > 0.9, "CTs nearly always defuse 5v0: {p50p}");
    }

    #[test]
    fn unobserved_states_stay_silent() {
        let t = WinProbTable::v1();
        // (0,0,planted) is unobservable and non-terminal only in theory —
        // ct=0 clamp covers it; a genuinely absent live cell must be None.
        // 6 is out of range entirely:
        assert_eq!(t.p_ct_win(6, 5, false), None);
    }

    #[test]
    fn monotonicity_holds_within_tolerance() {
        // More teammates never hurts; more enemies never helps; ε absorbs
        // low-n noise in rare corners (validation run: worst violation 0.001).
        const EPS: f32 = 0.02;
        let t = WinProbTable::v1();
        for planted in [false, true] {
            for ct in 1..=5u8 {
                for tt in 1..=5u8 {
                    let here = t.p_ct_win(ct, tt, planted);
                    let more_ct = if ct < 5 {
                        t.p_ct_win(ct + 1, tt, planted)
                    } else {
                        None
                    };
                    if let (Some(a), Some(b)) = (here, more_ct) {
                        assert!(
                            b >= a - EPS,
                            "P(CT) fell adding a CT: {ct}v{tt} planted={planted}: {a} -> {b}"
                        );
                    }
                    let more_t = if tt < 5 {
                        t.p_ct_win(ct, tt + 1, planted)
                    } else {
                        None
                    };
                    if let (Some(a), Some(b)) = (here, more_t) {
                        assert!(
                            b <= a + EPS,
                            "P(CT) rose adding a T: {ct}v{tt} planted={planted}: {a} -> {b}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn plant_always_helps_t_in_every_observed_matchup() {
        const EPS: f32 = 0.02;
        let t = WinProbTable::v1();
        for ct in 1..=5u8 {
            for tt in 1..=5u8 {
                if let (Some(pre), Some(post)) =
                    (t.p_ct_win(ct, tt, false), t.p_ct_win(ct, tt, true))
                {
                    assert!(
                        post <= pre + EPS,
                        "plant helped CT at {ct}v{tt}: {pre} -> {post}"
                    );
                }
            }
        }
    }
}
