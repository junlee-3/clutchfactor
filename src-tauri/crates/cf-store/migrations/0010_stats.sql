-- V1.4 coaching-first stats (docs/spec/stats-and-understanding.md §2).
-- Typed columns so the DoD cross-check is plain SQL and Trends is one query.
CREATE TABLE match_stats (
    match_id            INTEGER PRIMARY KEY REFERENCES matches(id) ON DELETE CASCADE,
    rounds_played       INTEGER NOT NULL,
    kills               INTEGER NOT NULL,
    deaths              INTEGER NOT NULL,
    assists             INTEGER NOT NULL,
    damage              INTEGER NOT NULL,
    headshots           INTEGER NOT NULL,
    kast_rounds         INTEGER NOT NULL,
    entry_attempts      INTEGER NOT NULL,
    entry_wins          INTEGER NOT NULL,
    traded_deaths       INTEGER NOT NULL,
    trade_kills         INTEGER NOT NULL,
    trade_opportunities INTEGER NOT NULL,
    clutch_attempts     INTEGER NOT NULL,
    clutch_wins         INTEGER NOT NULL
);

CREATE TABLE round_player_stats (
    match_id  INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round     INTEGER NOT NULL,
    steamid   TEXT NOT NULL,
    side      TEXT NOT NULL,
    kills     INTEGER NOT NULL,
    deaths    INTEGER NOT NULL,
    assists   INTEGER NOT NULL,
    damage    INTEGER NOT NULL,
    headshots INTEGER NOT NULL,
    survived  INTEGER NOT NULL,
    traded    INTEGER NOT NULL,
    entry     TEXT,
    PRIMARY KEY (match_id, round, steamid)
) WITHOUT ROWID;

-- Callout label positions: median world x/y of every last_place value over
-- all imported matches on the map (own + corpus); refreshed after an import
-- or re-analyze of that map.
CREATE TABLE map_callouts (
    map     TEXT NOT NULL,
    place   TEXT NOT NULL,
    x       REAL NOT NULL,
    y       REAL NOT NULL,
    samples INTEGER NOT NULL,
    PRIMARY KEY (map, place)
) WITHOUT ROWID;
