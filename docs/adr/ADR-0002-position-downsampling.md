# ADR-0002: Position track downsampling rate

Status: accepted
Date: 2026-08-19

## Context

Per-tick player samples (position, yaw, health, weapon, spotted, place) drive the replay
viewer and detectors. Storing all 64 Hz ticks is wasteful; events always keep exact ticks
regardless (PROMPT.md §4). Measured on `mirage-tie` (24 rounds, ~151k ticks, 10 players),
release build, full two-pass parse + normalize wall time:

| sample_every | Hz | tick rows | parse time | est. SQLite size (~80 B/row) |
|---|---|---|---|---|
| 2 | 32 | 758,610 | 0.94 s | ~61 MB |
| 4 | 16 | 379,300 | 0.65 s | ~30 MB |
| 8 | 8  | 189,650 | 0.56 s | ~15 MB |

## Decision

`sample_every = 4` (16 Hz) — the PROMPT §4 suggestion, confirmed by measurement.

## Consequences

- Replay interpolates 16 Hz → 60 fps over 62.5 ms gaps — visually smooth for player dots;
  scrub-seek precision stays well under the 100 ms budget (§10.4).
- ~30 MB/match keeps a 100-match library ~3 GB. If that bites later, options recorded:
  per-column delta+zstd blobs, or dropping dead-player rows (~15–40 % of rows).
- Detector timing rules quantize to 62.5 ms on tick-table lookups; all *event* timing
  (deaths, blinds, detonates) is exact-tick, so §6.4 windows (2.0 s / 3.0 s) are unaffected.
- `sample_every` is stored per match (`matches.sample_every`), so the rate can change
  without invalidating old imports.
