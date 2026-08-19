//! Round normalization: a neutral raw event stream → clean `rounds[]`.
//!
//! Handles the §6.2 quirks verified on real demos (docs/plans/M1-ingest.md):
//! MM demos use String winner/reason and duplicate round_officially_ended;
//! GOTV demos use numeric winner/reason, carry no round numbers, and may
//! lack round 1's round_start entirely.

use crate::model::{Round, RoundEndReason, Side};

#[derive(Debug, Clone, PartialEq)]
pub enum RawRoundEvent {
    Start {
        tick: i32,
        round: Option<u32>,
    },
    FreezeEnd {
        tick: i32,
    },
    End {
        tick: i32,
        winner: RawWinner,
        reason: RawReason,
    },
    OfficiallyEnded {
        tick: i32,
    },
    WinPanelMatch {
        tick: i32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawWinner {
    Str(String),
    Num(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawReason {
    Str(String),
    Num(i32),
}

fn decode_winner(w: &RawWinner) -> Option<Side> {
    match w {
        RawWinner::Str(s) if s == "CT" => Some(Side::Ct),
        RawWinner::Str(s) if s == "T" => Some(Side::T),
        RawWinner::Num(3) => Some(Side::Ct),
        RawWinner::Num(2) => Some(Side::T),
        _ => None,
    }
}

fn decode_reason(r: &RawReason) -> RoundEndReason {
    match r {
        RawReason::Str(s) => match s.as_str() {
            "t_killed" => RoundEndReason::TKilled,
            "ct_killed" => RoundEndReason::CtKilled,
            "bomb_defused" => RoundEndReason::BombDefused,
            "bomb_exploded" => RoundEndReason::BombExploded,
            "target_saved" => RoundEndReason::TargetSaved,
            other => RoundEndReason::Other(other.to_string()),
        },
        // Numeric codes name the WINNER'S victory ("Terrorists_Win", 9) while
        // our enum names the eliminated side, matching the MM strings
        // ("t_killed" = Ts were killed, a CT win). Verified on real demos:
        // navi r1 winner=T(2) reason=9 "#SFUI_Notice_Terrorists_Win" → CtKilled.
        RawReason::Num(n) => match n {
            9 => RoundEndReason::CtKilled,
            8 => RoundEndReason::TKilled,
            7 => RoundEndReason::BombDefused,
            1 => RoundEndReason::BombExploded,
            12 => RoundEndReason::TargetSaved,
            other => RoundEndReason::Other(other.to_string()),
        },
    }
}

/// See module docs. Events may arrive unsorted; identical consecutive
/// (variant, tick) duplicates are collapsed; rounds are anchored on End
/// events; numbering is sequence-derived (demo round fields are advisory).
pub fn normalize_rounds(events: &[RawRoundEvent]) -> Vec<Round> {
    let mut evs: Vec<&RawRoundEvent> = events.iter().collect();
    evs.sort_by_key(|e| match e {
        RawRoundEvent::Start { tick, .. }
        | RawRoundEvent::FreezeEnd { tick }
        | RawRoundEvent::End { tick, .. }
        | RawRoundEvent::OfficiallyEnded { tick }
        | RawRoundEvent::WinPanelMatch { tick } => *tick,
    });
    evs.dedup_by(|a, b| a == b);

    let mut rounds: Vec<Round> = vec![];
    // State for the round currently being assembled.
    let mut cur_start: Option<i32> = None;
    let mut cur_freeze_end: Option<i32> = None;
    let mut prev_officially_ended: Option<i32> = None;
    let mut match_over = false;

    for ev in evs {
        match ev {
            RawRoundEvent::WinPanelMatch { .. } => match_over = true,
            _ if match_over => {}
            RawRoundEvent::Start { tick, .. } => {
                cur_start = Some(*tick);
                cur_freeze_end = None;
            }
            RawRoundEvent::FreezeEnd { tick } => cur_freeze_end = Some(*tick),
            RawRoundEvent::OfficiallyEnded { tick } => {
                prev_officially_ended = Some(*tick);
                if let Some(last) = rounds.last_mut() {
                    if last.officially_ended_tick.is_none() {
                        last.officially_ended_tick = Some(*tick);
                    }
                }
            }
            RawRoundEvent::End {
                tick,
                winner,
                reason,
            } => {
                let start_tick = cur_start.or(prev_officially_ended).unwrap_or(0);
                let zero_duration = *tick <= start_tick && start_tick != 0;
                let decoded = decode_winner(winner);
                if !zero_duration {
                    if let Some(side) = decoded {
                        rounds.push(Round {
                            number: rounds.len() as u32 + 1,
                            start_tick,
                            freeze_end_tick: cur_freeze_end,
                            end_tick: *tick,
                            officially_ended_tick: None,
                            winner: side,
                            reason: decode_reason(reason),
                            ct_steamids: vec![],
                            t_steamids: vec![],
                        });
                    }
                }
                cur_start = None;
                cur_freeze_end = None;
            }
        }
    }
    rounds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn end_str(tick: i32, winner: &str, reason: &str) -> RawRoundEvent {
        RawRoundEvent::End {
            tick,
            winner: RawWinner::Str(winner.to_string()),
            reason: RawReason::Str(reason.to_string()),
        }
    }

    fn end_num(tick: i32, winner: i32, reason: i32) -> RawRoundEvent {
        RawRoundEvent::End {
            tick,
            winner: RawWinner::Num(winner),
            reason: RawReason::Num(reason),
        }
    }

    #[test]
    fn clean_mm_stream_with_duplicate_officially_ended() {
        // Two rounds shaped like the mirage-tie probe: round numbers present,
        // string winner/reason, officially_ended fired twice at the same tick.
        let evs = vec![
            RawRoundEvent::Start {
                tick: 65,
                round: Some(1),
            },
            RawRoundEvent::FreezeEnd { tick: 1441 },
            end_str(5120, "CT", "t_killed"),
            RawRoundEvent::OfficiallyEnded { tick: 5568 },
            RawRoundEvent::OfficiallyEnded { tick: 5568 },
            RawRoundEvent::Start {
                tick: 5568,
                round: Some(2),
            },
            RawRoundEvent::FreezeEnd { tick: 6528 },
            end_str(8523, "T", "bomb_exploded"),
            RawRoundEvent::OfficiallyEnded { tick: 8971 },
            RawRoundEvent::OfficiallyEnded { tick: 8971 },
            RawRoundEvent::WinPanelMatch { tick: 9000 },
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 2);
        assert_eq!(rounds[0].number, 1);
        assert_eq!(rounds[0].start_tick, 65);
        assert_eq!(rounds[0].freeze_end_tick, Some(1441));
        assert_eq!(rounds[0].end_tick, 5120);
        assert_eq!(rounds[0].officially_ended_tick, Some(5568));
        assert_eq!(rounds[0].winner, Side::Ct);
        assert_eq!(rounds[0].reason, RoundEndReason::TKilled);
        assert_eq!(rounds[1].number, 2);
        assert_eq!(rounds[1].winner, Side::T);
        assert_eq!(rounds[1].reason, RoundEndReason::BombExploded);
    }

    #[test]
    fn gotv_stream_missing_first_start_numeric_codes() {
        // Shaped like the navi-javelins probe: no round numbers, numeric
        // winner (2=T, 3=CT) and reason codes, round 1 has no Start at all.
        let evs = vec![
            RawRoundEvent::FreezeEnd { tick: 1240 },
            end_num(5478, 2, 9),
            RawRoundEvent::OfficiallyEnded { tick: 5798 },
            RawRoundEvent::Start {
                tick: 5798,
                round: None,
            },
            RawRoundEvent::FreezeEnd { tick: 7078 },
            end_num(18154, 3, 7),
            RawRoundEvent::OfficiallyEnded { tick: 18474 },
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 2);
        assert_eq!(rounds[0].number, 1);
        assert_eq!(rounds[0].start_tick, 0, "missing first Start synthesizes 0");
        assert_eq!(rounds[0].winner, Side::T);
        // Numeric 9 = Terrorists_Win = the CTs were eliminated.
        assert_eq!(rounds[0].reason, RoundEndReason::CtKilled);
        assert_eq!(rounds[1].number, 2);
        assert_eq!(rounds[1].start_tick, 5798);
        assert_eq!(rounds[1].winner, Side::Ct);
        assert_eq!(rounds[1].reason, RoundEndReason::BombDefused);
    }

    #[test]
    fn end_after_win_panel_is_dropped() {
        let evs = vec![
            RawRoundEvent::Start {
                tick: 10,
                round: Some(1),
            },
            end_str(100, "CT", "t_killed"),
            RawRoundEvent::WinPanelMatch { tick: 150 },
            RawRoundEvent::Start {
                tick: 200,
                round: None,
            },
            end_str(300, "T", "t_killed"),
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 1);
    }

    #[test]
    fn zero_duration_round_is_dropped() {
        // Restart artifact: an End at/before its own start.
        let evs = vec![
            RawRoundEvent::Start {
                tick: 10,
                round: Some(1),
            },
            end_str(10, "CT", "t_killed"),
            RawRoundEvent::OfficiallyEnded { tick: 12 },
            RawRoundEvent::Start {
                tick: 12,
                round: Some(1),
            },
            RawRoundEvent::FreezeEnd { tick: 20 },
            end_str(500, "T", "ct_killed"),
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].number, 1);
        assert_eq!(rounds[0].winner, Side::T);
    }

    #[test]
    fn sequence_wins_over_disagreeing_round_field() {
        let evs = vec![
            RawRoundEvent::Start {
                tick: 10,
                round: Some(5),
            }, // lies
            end_str(100, "CT", "t_killed"),
            RawRoundEvent::OfficiallyEnded { tick: 120 },
            RawRoundEvent::Start {
                tick: 120,
                round: Some(9),
            }, // lies again
            end_str(300, "T", "target_saved"),
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 2);
        assert_eq!(rounds[0].number, 1);
        assert_eq!(rounds[1].number, 2);
        assert_eq!(rounds[1].reason, RoundEndReason::TargetSaved);
    }

    #[test]
    fn unknown_codes_map_to_other() {
        let evs = vec![
            RawRoundEvent::Start {
                tick: 10,
                round: Some(1),
            },
            RawRoundEvent::End {
                tick: 100,
                winner: RawWinner::Str("CT".into()),
                reason: RawReason::Num(4),
            },
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds[0].reason, RoundEndReason::Other("4".into()));
    }

    #[test]
    fn unresolvable_winner_drops_round() {
        // A round whose winner can't be decoded is useless downstream.
        let evs = vec![
            RawRoundEvent::Start {
                tick: 10,
                round: Some(1),
            },
            RawRoundEvent::End {
                tick: 100,
                winner: RawWinner::Num(0),
                reason: RawReason::Num(9),
            },
            RawRoundEvent::OfficiallyEnded { tick: 120 },
            RawRoundEvent::Start {
                tick: 120,
                round: Some(2),
            },
            end_str(300, "T", "t_killed"),
        ];
        let rounds = normalize_rounds(&evs);
        assert_eq!(rounds.len(), 1);
        assert_eq!(rounds[0].winner, Side::T);
    }
}
