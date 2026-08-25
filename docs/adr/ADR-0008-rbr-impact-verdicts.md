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
hand-verification pass, the same way H2's severities were. See
**Calibration** below for that pass's actual numbers and the raised
`attention_threshold_p`.

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
- **Task 9 divergence note:** the replay canvas computes its own nearest
  living teammate from live tick-sampled positions at the current playback
  tick (`src/replay/annotation.ts`'s `nearestLivingTeammate`), independently
  of the rail's `H2_ISOLATED_DEATH` `nearest_teammate` fact fixed at
  analysis time, so in rare ties/edge cases the two may name a DIFFERENT
  teammate — an accepted identity divergence between two independently-
  correct views, not a bug (same ruling as the moment-focus ordering fix:
  V1.2 final-review fix wave, finding #3).

## Calibration (2026-08-22)

The V1.2 §12 hand-verification pass (task 10) evaluated round selection on
all five `fixtures/own/` demos after a lazy-backfill recompute. At the
original default `attention_threshold_p = 0.18`, the 6-round cap saturated
on **all 5/5 matches** — candidate counts (rounds with `|impact| ≥ 0.18`)
were 15, 11, 13, 12, and 11 against a cap of 6, so the cap was doing the
selecting, not the threshold. Root cause: a single opening kill/death at the
common 5v5→5v4 state swings win-prob by ≈0.19–0.23 (table-derived), which
already clears 0.18 on its own — so almost any round with a single early
engagement became a "candidate," flooding the selection.

Raised `attention_threshold_p` to **0.25**. This sits in the second-largest
gap in the real observed impact distribution (0.2427 → 0.2709, a 0.028 gap;
the largest gap, 0.3372 → 0.3767, already sits just under
`pivotal_threshold_p = 0.35`) — i.e. it separates "one common early duel"
swings from swings that compound multiple events or land at a materially
better/worse man-state. Re-run after the change: candidate counts dropped to
7, 4, 4, 2, and 5; selected-round counts became 6, 4, 4, 2, and 5 — only one
match (the closest, most back-and-forth 12-12 tie) still hits the cap, the
other four sit under it. `pivotal_threshold_p`, `max_rounds`, and
`max_moments` are left at their original defaults; only
`attention_threshold_p` moved. Changed in `config.rs`'s
`d_rbr_attention_p()` and its two tests
(`defaults_match_spec_6_4`, `yaml_overrides_merge_over_defaults`); the
`RbrCfg`-default-derived assertions in `round_review.rs`'s
`selection_threshold_and_cap`/`won_it_guarantee_swaps_weakest` tests were
checked by hand and remain valid unchanged (their synthetic impacts already
clear 0.25). `commands.rs`'s `threshold_rows` reads the config live, so the
Settings screen's "Coach rail attention threshold" row picks up the new
value with no code change.

## Final-review fixes (2026-08-25)

Two rulings from the V1.2 final-review fix wave amend this ADR's contract:

1. **Selection must check verdict, not just `|impact|`** (finding #1,
   CONTROLLER RULING). Model point 5's `select_rounds` only ever checked
   `|impact| ≥ attention_threshold_p`, so a `Quiet`-verdict round with a
   large-magnitude impact (positive impact, round lost, no exculpatory rule
   — live: inferno-loss R2, +0.3767) could clear the bar and get selected
   anyway, contradicting "Quiet: nothing notable; summary only." Fixed:
   `select_rounds` now also excludes `verdict == Quiet` from candidacy.
   Verdict is already assigned before selection runs (`review_rounds`
   builds each round's verdict, then passes the assembled candidates —
   verdict included — into `select_rounds`), so no reordering was needed.
   `selection_threshold_and_cap` and `won_it_guarantee_swaps_weakest`
   (§ Calibration, above) were touched again here: their synthetic
   candidates now carry a legitimately non-`Quiet` verdict (`WonIt`/
   `Traded`) instead of `Quiet`, since a `Quiet` candidate is never
   selectable regardless of impact.
2. **Stored reviews need a config fingerprint** (finding #5, CONTROLLER
   RULING). A `round_review` row computed under an old engine version or a
   since-changed `RbrCfg` threshold was served as-is forever. Migration
   0007 adds `cfg_fingerprint` (engine version + digest of `RbrCfg`'s
   tunables, `round_review::cfg_fingerprint`); `get_round_review` recomputes
   via `run_round_review` whenever the stored fingerprint doesn't match the
   current one, the same lazy-backfill path already used for empty rows.
3. **The ledger grades an exculpated death Neutral, whichever rule wins
   `rule_id`** (V1.2b final-review fix wave, #1, CONTROLLER RULING). A
   death carrying `H2_BAITED_TRADE` (0.35) and `H2_ISOLATED_DEATH` (0.8) on
   one tick got verdict `NotOnYou` here but `quality: bad` in the play
   ledger, which read only the winning `rule_id`. `merge_flags` now leaves
   an additive `facts.exculpatory = true` marker whenever any
   `rbr.exculpatory_rules` flag merges into a play, and
   `finalize_death_quality` honours it (live: inferno-loss R6 and R9). Same
   wave, #5: `tracked_death.round_end_delta_s` is clamped at 0 and carries
   `dead_time` for a death after the round was decided — engine version
   `rbr-v3`, so stored rows with negative seconds recompute.
