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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Hotspot {
    pub map: String,
    pub center: (f32, f32),
    pub deaths: usize,
    pub matches: usize,
    /// (match_id, round, tick) of each member death.
    pub members: Vec<(i64, u32, i32)>,
}

/// Greedy radius clustering per map (spec H4_REPEAT_HOTSPOT: ≥3 deaths
/// within 250 u across ≥2 demos). Deterministic: seeds iterate in input
/// order; each point joins at most one cluster.
pub fn death_hotspots(points: &[DeathPoint], cfg: &HabitCfg) -> Vec<Hotspot> {
    let mut used = vec![false; points.len()];
    let mut out = vec![];
    for i in 0..points.len() {
        if used[i] {
            continue;
        }
        let seed = &points[i];
        let mut member_idx = vec![];
        for (j, p) in points.iter().enumerate() {
            if used[j] || p.map != seed.map {
                continue;
            }
            let d = ((p.x - seed.x).powi(2) + (p.y - seed.y).powi(2)).sqrt();
            if d <= cfg.hotspot_radius_u {
                member_idx.push(j);
            }
        }
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

    fn pt(match_id: i64, map: &str, x: f32, y: f32) -> DeathPoint {
        DeathPoint {
            match_id,
            map: map.to_string(),
            round: 1,
            tick: 1000,
            x,
            y,
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
