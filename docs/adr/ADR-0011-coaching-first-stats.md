# ADR-0011 — Coaching-first stats: computed with the detectors, stored typed, explained by a catalog

**Status:** accepted (V1.4, 2026-08-26)

## Context

PROMPT-V1 §6 asks for stats that serve the coaching, not a stats tracker: every number must link to the rule that explains it, and the DoD demands a raw-SQL cross-check for a real match. We also owed the charter a "What your coach watches" screen and callout labels on the replay map (`docs/spec/stats-and-understanding.md`).

## Decision

1. **Stats are computed inside `analyze()`** by a pure `cf-analysis::stats` module that reuses the detectors' own definitions — H14's opening-duel finder (`round_entries`, both sides), H2's `killed_in` for traded deaths, the play ledger's `trade`/`missed_trade` plays for trade opportunities, and a kill-state replay (ADR-0008's method) for clutch situations. A stat and its rule can never disagree because they are the same code.
2. **Persisted in typed tables** (migration 0010: `match_stats`, `round_player_stats`) written in `save_analysis`'s transaction and cleared on re-analyze, so the cross-check is plain SQL, Trends is one query, and pre-V1.4 imports show a placeholder with a "Re-analyze for stats" action instead of a zero.
3. **Undefined ratios are `None`** (0 kills → HS% "—"), the silence bias applied to numbers. **Damage is the health actually removed:** CS2's `player_hurt.dmg_health` is uncapped (an AWP headshot logs 446), so every round is replayed with 100 HP per player and each hurt credits `min(dmg, hp_left)` — the number the in-game scoreboard would show.
4. **The catalog is static Rust** (`cf-analysis::catalog`) with a coverage test over every emitted rule id, the D-series, the roll-up insight ids and taxonomy classes 1–15 (8, 10, 12 marked not built, with the reason). Threshold sentences carry `{config.path}` placeholders rendered from the live `DetectorConfig`; `config::threshold_values` is now the single source for Settings' threshold table (config-path names).
5. **Callout labels come from the data**: the median position of every `last_place` value across all imported matches on a map (1 Hz samples, ≥ 30 samples), cached in `map_callouts` with the median z, refreshed after every persisted import / re-analyze / corpus import (and lazily on first open of a map analyzed before V1.4), drawn on the radar layer the z selects, only when the radar is ≥ 560 CSS px wide.
6. **Every stat links to `/watches?stat=<key>`**, which filters the catalog to the rules that feed that number.

## Consequences

- Adding a stat means adding a column and a `MatchStats` field — the cross-check script and Trends series grow with it.
- The scoreboard's "Match" tab aggregates the per-round rows client-side (`aggregate`), so it agrees with `match_stats` for the tracked player by construction.
- Settings' threshold table reads more technical than in V1.0; the plain-language explanations live on Watches.
