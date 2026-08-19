-- Schema v1 (ADR-0003). Cross-demo keys everywhere: (match_id, steamid, map, tick)
-- so M3's death_class / rule-flag tables (migration 2) can join without rework.

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE matches (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name     TEXT NOT NULL,
    file_hash     TEXT NOT NULL UNIQUE,
    map           TEXT NOT NULL,
    tickrate      REAL NOT NULL,
    imported_at   TEXT NOT NULL,
    sample_every  INTEGER NOT NULL,
    score_a       INTEGER NOT NULL,
    score_b       INTEGER NOT NULL,
    roster_a_json TEXT NOT NULL, -- JSON array of steamid strings, CT side of round 1
    roster_b_json TEXT NOT NULL
);

CREATE TABLE players (
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    steamid  TEXT NOT NULL, -- steamid64 as string (JS-safe convention)
    name     TEXT NOT NULL,
    PRIMARY KEY (match_id, steamid)
) WITHOUT ROWID;

CREATE TABLE rounds (
    match_id               INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    number                 INTEGER NOT NULL,
    start_tick             INTEGER NOT NULL,
    freeze_end_tick        INTEGER,
    end_tick               INTEGER NOT NULL,
    officially_ended_tick  INTEGER,
    winner                 TEXT NOT NULL,   -- 'CT' | 'T'
    reason                 TEXT NOT NULL,   -- normalized reason name
    PRIMARY KEY (match_id, number)
) WITHOUT ROWID;

CREATE TABLE round_sides (
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    number   INTEGER NOT NULL,
    steamid  TEXT NOT NULL,
    side     TEXT NOT NULL,               -- 'CT' | 'T'
    PRIMARY KEY (match_id, number, steamid)
) WITHOUT ROWID;

CREATE TABLE kills (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id      INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round         INTEGER NOT NULL,
    tick          INTEGER NOT NULL,
    attacker      TEXT,
    victim        TEXT NOT NULL,
    assister      TEXT,
    weapon        TEXT NOT NULL,
    headshot      INTEGER NOT NULL,
    penetrated    INTEGER NOT NULL,
    thru_smoke    INTEGER NOT NULL,
    attacker_blind INTEGER NOT NULL,
    assistedflash INTEGER NOT NULL
);
CREATE INDEX idx_kills_match_victim ON kills(match_id, victim);
CREATE INDEX idx_kills_match_attacker ON kills(match_id, attacker);

CREATE TABLE blinds (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick     INTEGER NOT NULL,
    victim   TEXT NOT NULL,
    attacker TEXT,
    duration REAL NOT NULL
);
CREATE INDEX idx_blinds_match ON blinds(match_id);

CREATE TABLE grenades (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick     INTEGER NOT NULL,
    kind     TEXT NOT NULL,
    thrower  TEXT,
    x REAL NOT NULL, y REAL NOT NULL, z REAL NOT NULL
);
CREATE INDEX idx_grenades_match ON grenades(match_id);

CREATE TABLE bomb_events (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick     INTEGER NOT NULL,
    kind     TEXT NOT NULL,
    player   TEXT
);
CREATE INDEX idx_bomb_events_match ON bomb_events(match_id);

CREATE TABLE tick_samples (
    match_id    INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    steamid     TEXT NOT NULL,
    tick        INTEGER NOT NULL,
    x REAL NOT NULL, y REAL NOT NULL, z REAL NOT NULL,
    yaw         REAL NOT NULL,
    health      INTEGER NOT NULL,
    is_alive    INTEGER NOT NULL,
    team_num    INTEGER NOT NULL,
    active_weapon TEXT,
    spotted     INTEGER NOT NULL,
    last_place  TEXT,
    PRIMARY KEY (match_id, steamid, tick)
) WITHOUT ROWID;
