//! §5A cross-demo habit promotion + spec H4_REPEAT_HOTSPOT clustering.
//! Pure functions — the store supplies aggregated inputs.

use crate::config::HabitCfg;

/// One rule's per-match occurrence counts, newest match first, already
/// truncated to the promotion window by the caller.
#[derive(Debug, Clone)]
pub struct HabitInput {
    pub rule_id: String,
    pub severity: f32,
    pub confidence: f32,
    pub per_match: Vec<(i64, u32)>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Habit {
    pub rule_id: String,
    pub matches_hit: usize,
    pub window: usize,
    pub total: u32,
    pub score: f32,
}

/// Promote rules that recur across matches (≥ min_matches within the
/// window). Spec H2 rule: H2_BAITED_TRADE is never promoted alone — only
/// when H2_FAILED_TRADE promotes in the same window (the combination is a
/// team pattern; alone it would coach the player to stop trading).
pub fn promote_habits(inputs: &[HabitInput], cfg: &HabitCfg) -> Vec<Habit> {
    let promoted_ids: Vec<&str> = inputs
        .iter()
        .filter(|i| hits(i) >= cfg.min_matches)
        .map(|i| i.rule_id.as_str())
        .collect();
    let failed_trade_promoted = promoted_ids.contains(&"H2_FAILED_TRADE");

    let mut out: Vec<Habit> = inputs
        .iter()
        .filter(|i| hits(i) >= cfg.min_matches)
        .filter(|i| i.rule_id != "H2_BAITED_TRADE" || failed_trade_promoted)
        .map(|i| {
            let matches_hit = hits(i);
            let total: u32 = i.per_match.iter().map(|(_, c)| c).sum();
            Habit {
                rule_id: i.rule_id.clone(),
                matches_hit,
                window: cfg.window_matches,
                total,
                score: i.severity
                    * i.confidence
                    * (matches_hit as f32 / cfg.window_matches.max(1) as f32)
                    * (1.0 + total as f32).ln(),
            }
        })
        .collect();
    out.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.rule_id.cmp(&b.rule_id)));
    out
}

fn hits(i: &HabitInput) -> usize {
    i.per_match.iter().filter(|(_, c)| *c > 0).count()
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeathPoint {
    pub match_id: i64,
    pub map: String,
    pub round: u32,
    pub tick: i32,
    pub x: f32,
    pub y: f32,
    /// Valve's own callout for where the death happened (`last_place_name`).
    /// `None` when the sample carried no place — those points cluster on
    /// geometry alone.
    pub place: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Hotspot {
    pub map: String,
    pub center: (f32, f32),
    pub deaths: usize,
    pub matches: usize,
    /// The callout every member death shares, when they have one.
    pub place: Option<String>,
    /// (match_id, round, tick) of each member death.
    pub members: Vec<(i64, u32, i32)>,
}

fn dist(a: &DeathPoint, b: &DeathPoint) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

/// Clusters repeat deaths per map (spec H4_REPEAT_HOTSPOT: ≥3 deaths within
/// 250 u across ≥2 demos). Deterministic: seeds iterate in input order; each
/// point joins at most one cluster.
///
/// Two constraints keep the "same spot" claim literally true. Both were
/// missing, and together they reported deaths at Ladder, Underpass and
/// Catwalk on Mirage as one spot:
///
/// 1. **Same callout.** Members must share `place`. Valve's own place names
///    are the honest unit of "the same spot"; raw distance alone merges
///    adjacent-but-different areas no player would call one position.
/// 2. **Diameter, not seed-radius.** Every member sits within
///    `hotspot_radius_u` of *every other member*, not merely of the seed —
///    a seed-radius cluster spans up to twice the radius, so 250 u silently
///    permitted a 500 u spread.
pub fn death_hotspots(points: &[DeathPoint], cfg: &HabitCfg) -> Vec<Hotspot> {
    let mut used = vec![false; points.len()];
    let mut out = vec![];
    for i in 0..points.len() {
        if used[i] {
            continue;
        }
        let seed = &points[i];
        let mut candidates: Vec<usize> = (0..points.len())
            .filter(|&j| !used[j])
            .filter(|&j| points[j].map == seed.map && points[j].place == seed.place)
            .collect();
        // Nearest-first, so the tightest available group forms around the seed
        // rather than whichever point happened to come earlier in input order.
        candidates.sort_by(|&a, &b| {
            dist(seed, &points[a])
                .total_cmp(&dist(seed, &points[b]))
                .then(a.cmp(&b))
        });

        let mut member_idx: Vec<usize> = vec![];
        for j in candidates {
            if member_idx
                .iter()
                .all(|&m| dist(&points[m], &points[j]) <= cfg.hotspot_radius_u)
            {
                member_idx.push(j);
            }
        }
        member_idx.sort_unstable();
        let mut match_ids: Vec<i64> = member_idx.iter().map(|j| points[*j].match_id).collect();
        match_ids.sort_unstable();
        match_ids.dedup();
        if member_idx.len() >= cfg.hotspot_min_deaths && match_ids.len() >= cfg.hotspot_min_matches
        {
            for j in &member_idx {
                used[*j] = true;
            }
            let n = member_idx.len() as f32;
            let cx = member_idx.iter().map(|j| points[*j].x).sum::<f32>() / n;
            let cy = member_idx.iter().map(|j| points[*j].y).sum::<f32>() / n;
            out.push(Hotspot {
                map: seed.map.clone(),
                center: (cx, cy),
                deaths: member_idx.len(),
                matches: match_ids.len(),
                place: seed.place.clone(),
                members: member_idx
                    .iter()
                    .map(|j| (points[*j].match_id, points[*j].round, points[*j].tick))
                    .collect(),
            });
        }
    }
    out.sort_by(|a, b| b.deaths.cmp(&a.deaths).then(a.map.cmp(&b.map)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> HabitCfg {
        crate::config::DetectorConfig::default().habit
    }

    fn input(rule_id: &str, per_match: &[(i64, u32)]) -> HabitInput {
        HabitInput {
            rule_id: rule_id.to_string(),
            severity: 0.8,
            confidence: 0.75,
            per_match: per_match.to_vec(),
        }
    }

    #[test]
    fn promotes_at_min_matches_not_below() {
        let three = input("H2_ISOLATED_DEATH", &[(1, 7), (2, 4), (3, 2)]);
        let two = input("H4_KILLED_WITHOUT_CONTACT", &[(1, 6), (2, 5), (3, 0)]);
        let habits = promote_habits(&[three, two], &cfg());
        assert_eq!(habits.len(), 1);
        assert_eq!(habits[0].rule_id, "H2_ISOLATED_DEATH");
        assert_eq!(habits[0].matches_hit, 3);
        assert_eq!(habits[0].total, 13);
    }

    #[test]
    fn baited_alone_never_promotes_but_with_failed_trade_does() {
        let baited = HabitInput {
            severity: 0.35,
            ..input("H2_BAITED_TRADE", &[(1, 2), (2, 1), (3, 1)])
        };
        let alone = promote_habits(std::slice::from_ref(&baited), &cfg());
        assert!(alone.is_empty(), "spec: never promote baited alone");

        let failed = HabitInput {
            severity: 0.6,
            ..input("H2_FAILED_TRADE", &[(1, 5), (2, 3), (3, 2)])
        };
        let both = promote_habits(&[baited, failed], &cfg());
        let ids: Vec<&str> = both.iter().map(|h| h.rule_id.as_str()).collect();
        assert!(ids.contains(&"H2_BAITED_TRADE"));
        assert!(ids.contains(&"H2_FAILED_TRADE"));
    }

    #[test]
    fn score_orders_habits_deterministically() {
        let hot = input("H2_ISOLATED_DEATH", &[(1, 9), (2, 8), (3, 7), (4, 6)]);
        let mild = HabitInput {
            severity: 0.4,
            ..input("H3_WASTED_UTILITY", &[(1, 1), (2, 1), (3, 1)])
        };
        let habits = promote_habits(&[mild, hot], &cfg());
        assert_eq!(habits[0].rule_id, "H2_ISOLATED_DEATH");
        assert!(habits[0].score > habits[1].score);
    }

    /// Points default to one shared callout, so the geometry assertions below
    /// test geometry rather than the place gate.
    fn pt(match_id: i64, map: &str, x: f32, y: f32) -> DeathPoint {
        pt_at(match_id, map, x, y, Some("Palace"))
    }

    fn pt_at(match_id: i64, map: &str, x: f32, y: f32, place: Option<&str>) -> DeathPoint {
        DeathPoint {
            match_id,
            map: map.to_string(),
            round: 1,
            tick: 1000,
            x,
            y,
            place: place.map(str::to_string),
        }
    }

    #[test]
    fn hotspot_needs_min_deaths_across_min_matches() {
        // 3 deaths within 250 u across 2 matches → cluster.
        let good = vec![
            pt(1, "de_mirage", 0.0, 0.0),
            pt(1, "de_mirage", 100.0, 0.0),
            pt(2, "de_mirage", 0.0, 120.0),
        ];
        let hs = death_hotspots(&good, &cfg());
        assert_eq!(hs.len(), 1);
        assert_eq!(hs[0].deaths, 3);
        assert_eq!(hs[0].matches, 2);
        assert_eq!(hs[0].map, "de_mirage");

        // Same 3 deaths all in ONE match → rejected.
        let one_match = vec![
            pt(1, "de_mirage", 0.0, 0.0),
            pt(1, "de_mirage", 100.0, 0.0),
            pt(1, "de_mirage", 0.0, 120.0),
        ];
        assert!(death_hotspots(&one_match, &cfg()).is_empty());
    }

    #[test]
    fn hotspot_radius_boundary_and_map_separation() {
        // Third death 400 u away → not a member; no cluster forms.
        let spread = vec![
            pt(1, "de_mirage", 0.0, 0.0),
            pt(2, "de_mirage", 100.0, 0.0),
            pt(2, "de_mirage", 400.0, 0.0),
        ];
        assert!(death_hotspots(&spread, &cfg()).is_empty());

        // Same coordinates on different maps never cluster together.
        let cross_map = vec![
            pt(1, "de_mirage", 0.0, 0.0),
            pt(2, "de_nuke", 0.0, 0.0),
            pt(1, "de_nuke", 10.0, 0.0),
            pt(2, "de_mirage", 10.0, 0.0),
        ];
        assert!(
            death_hotspots(&cross_map, &cfg()).is_empty(),
            "2 per map < min_deaths"
        );
    }

    #[test]
    fn deaths_at_different_callouts_never_form_one_hotspot() {
        // Regression: the real Mirage bug. Five deaths inside a 250 u
        // seed-radius but spread across Ladder / Underpass / Catwalk were
        // reported as "the same spot". Different callouts must not merge,
        // however close the coordinates are.
        let pts = vec![
            pt_at(3, "de_mirage", -983.0, -72.0, Some("Ladder")),
            pt_at(4, "de_mirage", -916.0, 283.0, Some("Underpass")),
            pt_at(4, "de_mirage", -973.0, 177.0, Some("Underpass")),
            pt_at(4, "de_mirage", -1061.0, 183.0, Some("Catwalk")),
            pt_at(4, "de_mirage", -769.0, 66.0, Some("Catwalk")),
        ];
        assert!(
            death_hotspots(&pts, &cfg()).is_empty(),
            "no single callout reaches 3 deaths across 2 matches"
        );
    }

    #[test]
    fn cluster_diameter_never_exceeds_the_radius() {
        // Three points each within 250 u of the seed but 400 u apart from
        // each other: a seed-radius cluster would take all three and claim a
        // 400 u spread is "the same spot".
        let pts = vec![
            pt(1, "de_mirage", -200.0, 0.0),
            pt(2, "de_mirage", 0.0, 0.0),
            pt(3, "de_mirage", 200.0, 0.0),
        ];
        let hs = death_hotspots(&pts, &cfg());
        assert!(
            hs.is_empty(),
            "third point is 400 u from the first — must not join"
        );

        // Tightened so all three are mutually inside the radius → fires.
        let tight = vec![
            pt(1, "de_mirage", -100.0, 0.0),
            pt(2, "de_mirage", 0.0, 0.0),
            pt(3, "de_mirage", 100.0, 0.0),
        ];
        let hs = death_hotspots(&tight, &cfg());
        assert_eq!(hs.len(), 1);
        assert_eq!(hs[0].deaths, 3);
        assert_eq!(hs[0].place.as_deref(), Some("Palace"));
    }

    #[test]
    fn two_separate_clusters_found_deterministically() {
        let pts = vec![
            pt(1, "de_mirage", 0.0, 0.0),
            pt(2, "de_mirage", 50.0, 0.0),
            pt(3, "de_mirage", 0.0, 50.0),
            pt(1, "de_mirage", 5000.0, 5000.0),
            pt(2, "de_mirage", 5050.0, 5000.0),
            pt(3, "de_mirage", 5000.0, 5050.0),
            pt(1, "de_mirage", 5100.0, 5100.0),
        ];
        let hs = death_hotspots(&pts, &cfg());
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].deaths, 4, "bigger cluster first");
        assert_eq!(hs[1].deaths, 3);
        let again = death_hotspots(&pts, &cfg());
        assert_eq!(hs, again, "deterministic");
    }
}
