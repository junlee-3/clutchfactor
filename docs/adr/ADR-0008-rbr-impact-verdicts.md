# ADR-0008: Round-by-round impact scoring, verdicts, selection & moments

Status: accepted
Date: 2026-08-22

## Context

Issue #9 requires a round-by-round coaching view: for each round, how much did
the tracked player's play move the round's win probability, what's the
one-word verdict on it, which handful of rounds are worth showing, and what
moments back that verdict up. The man-count heuristic is explicitly rejected
(§6) in favor of `cf-analysis::winprob` (ADR-0006) as the single impact
currency. This ADR fixes the scoring model (`cf-analysis::round_review`) so
Tasks 3-5 (narration, replay hookup, DB storage) have a stable contract.

## Decision

The 8-point model, implemented as small pure sub-functions
(`round_events`, `tracked_perspective_p`, `score_round`, `assign_verdict`,
`select_rounds`, `build_moments`) over a narrow `RoundReviewInput`:

1. **State replay per round** from the kill list + bomb events, not tick
   samples: start at `(round.ct.len(), round.t.len(), planted=false)`,
   replay kills/plant/defuse/explode in tick order within
   `[start_tick, officially_ended_tick]`. Chosen over tick-sampled state
   because it's exact (no snapshot-interval slop), trivially unit-testable
   with plain struct literals, and backfillable from already-stored DB rows
   without re-parsing a demo (Task 5's constraint). A defuse/explode forces
   the terminal win-prob and **latches** the round as decided: every event
   after it scores `delta_p: None` unconditionally, so a mop-up/exit-frag
   kill inside the officially-ended tail never re-enters the table as a
   phantom swing.
2. **ΔP per event** = signed win-prob delta from the tracked player's
   perspective; either endpoint `None` → `delta_p: None`, contributing
   nothing (silence bias, PROMPT.md §4: false negative ≫ false positive).
3. **impact** = Σ ΔP over events attributable to the tracked player
   (their kills, their death, their plant/defuse).
4. **pivotal_tick** = the round's single largest `|ΔP|` swing, independent of
   who caused it — the story's turning point, not necessarily the player's.
5. **Selection**: threshold-with-cap (never fixed top-N), `|impact| ≥
   attention_threshold_p`, capped at `max_rounds`. The won-it guarantee
   (a cut `WonIt` round swaps in over the weakest non-`WonIt`) exists so a
   good clutch never silently loses its shelf space to a pile of small
   costly rounds.
6. **Attention**: Bright/Dim/None gates a round's rail dot without
   duplicating the selection cap.
7. **Verdict precedence — praise, then exculpation, then void-the-lesson,
   then measured cost, then quiet**: `WonIt` → `NotOnYou` → `Traded` →
   `CostYou` → `Quiet`. `NotOnYou` strictly precedes `CostYou` — issue #9's
   hard rule: a death the player structurally couldn't prevent (an
   exculpatory flag, e.g. `H2_BAITED_TRADE`) must never render as "cost
   you," even when the round's win-prob swing was large. Getting this order
   backwards produces a coaching app that blames players for their
   teammates' spacing mistakes.
8. **Moments are stored structured, not as strings.** Each moment carries
   `facts` (raw callouts: steamids, distances, place names) and a
   `delta_p`; prose is generated at serve time by the narrator, not baked
   in at analysis time. This deviates from the rest of `cf-analysis`, where
   `Insight`s already carry near-final `title_data` for template rendering.
   The deviation is deliberate: V1.3's grounding/RAG work needs to reason
   over structured facts, not scrape sentences back apart.

Thresholds (`RbrCfg`: `attention_threshold_p=0.18`, `pivotal_threshold_p=0.35`,
`max_rounds=6`, `max_moments=6`) are tunable approximations, not derived
constants — they get calibrated against real matches in the §12
hand-verification pass, the same way H2's severities were.

## Consequences

- `round_review::review_rounds` stays a pure `MatchData`-free function
  (narrow input), so Task 5 can backfill historical matches from SQLite rows
  without re-running the parser.
- Verdict and impact are computed for every round regardless of selection —
  only `moments` is gated on `selected` — so a caller can always ask "was
  this round costly" even for a round the rail never surfaces.
- Because thresholds are shared between `WonIt`'s gate and the selection
  cap, tests that need to force selection without fighting real win-prob
  magnitudes lower `attention_threshold_p` directly rather than special-
  casing the engine — the engine has no test-only branches.
- Round reviews are not part of `AnalysisOutput`/the golden-demo regression
  surface; they get their own contract (this ADR) and their own tests.
