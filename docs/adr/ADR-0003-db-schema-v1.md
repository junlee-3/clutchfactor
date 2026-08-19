# ADR-0003: SQLite schema v1

Status: accepted
Date: 2026-08-19

## Context

M1 needs persistent storage for imported matches (PROMPT.md §13); M3's death taxonomy
(docs/spec/death-taxonomy.md) will add per-death classification and per-rule flags that
must join cheaply across demos.

## Decision

Schema v1 (`cf-store/migrations/0001_schema_v1.sql`), embedded forward-only migrations
recorded in `schema_migrations`, WAL mode, foreign keys ON.

- **Tables:** settings (kv), matches (incl. `file_hash UNIQUE` for import dedup,
  `sample_every`, derived score + round-1 rosters as JSON), players, rounds,
  round_sides (one row per player per round — sides swap at halftime and OT),
  kills / blinds / grenades / bomb_events (exact-tick events), tick_samples
  (downsampled per ADR-0002; `WITHOUT ROWID`, PK `(match_id, steamid, tick)` so the
  replay scan and per-player rule lookups are index-order reads).
- **Steamids are TEXT** (steamid64 as string) in every table — the same convention
  crosses IPC to JS, which cannot represent steamid64 in a Number.
- **Deliberately deferred to migration 2 (M3):** `death_class`, rule-flag, and insight
  tables per §5A. The cross-demo keys they join on (match_id, steamid, map via matches,
  tick) all exist in v1, so adding them is purely additive.
- Tracked-player identity: `settings.tracked_steamid` override, else the steamid
  appearing in the most imported matches.

## Consequences

- Re-import of an identical file is rejected by hash — renamed copies of the same demo
  can't duplicate a match.
- tick_samples dominates size (~30 MB/match at 16 Hz); compression options recorded in
  ADR-0002 if a large library demands it.
- Migration runner is forward-only; no down migrations (a corrupt/experimental DB is
  deleted and re-imported — demos are the source of truth, the DB is a cache).
