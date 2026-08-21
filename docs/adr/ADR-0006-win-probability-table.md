# ADR-0006: Win-probability table — sourcing and shape

**Status:** accepted · 2026-08-21

## Context
Issue #9 mandates win-probability impact scoring for RBR (man-count heuristic
explicitly rejected); charter §6 makes the table a first-class, versioned,
transparently sourced cf-analysis module shared by RBR impact, stats context,
and any future leak board.

## Options
1. Hand-encode published aggregate numbers — no bomb-state split exists in
   any citable source; partial tables invite invented numbers.
2. Derive from our own fixtures (~18 demos ≈ 400 rounds) — far too sparse
   for 72 live cells; dishonest precision.
3. **Derive from OpenML dataset 43430 (CC0)** — "CSGO-Round-Winner-
   Classification": 122,410 snapshots every 20 s, 700 pro demos (2019–2020,
   Skybox CS:GO AI Challenge); has ct/t players alive, bomb_planted,
   round_winner. Chosen.

## Decision
`cf-analysis::winprob`: embedded versioned YAML (`win_prob_v1`) keyed
(ct_alive, t_alive, bomb_planted) → P(CT win) + sample count, derived by the
committed `derive_winprob` example (deterministic; dataset never enters the
repo or CI). Terminal states clamped in code; unobserved cells return None
(silence bias). No time dimension in v1 (snapshot cadence supports adding
one later); no config knobs (impact threshold/cap live with the V1.2
scorer). Anchor agreement with independent 400k-round published analysis:
5v4 pre-plant 0.706 vs ≈0.68.

One row of the 122,410 (ct=4, t=6, a single-round-start snapshot) is
impossible under 5v5 rules — a source-data glitch, not a parse failure — and
is skipped by the derivation tool rather than folded into the table or
silently dropped; it does not touch any validated anchor cell.

## Consequences
CSGO-era data approximates CS2 (documented in the module header); version
bump + re-derivation when a comparable CS2 dataset exists. ε=0.02 tolerance
in monotonicity tests absorbs low-n corner noise (worst observed violation
0.001). Consumers must handle None and may weight by sample_n.
