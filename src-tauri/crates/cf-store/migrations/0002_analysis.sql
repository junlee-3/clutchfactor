-- Migration 2 (M3): rule-engine inputs + analysis outputs.
-- Old imports keep NULL/absent rows → detectors stay silent (spec §5A bias).

CREATE TABLE shots (
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick     INTEGER NOT NULL,
    player   TEXT NOT NULL,
    weapon   TEXT NOT NULL
);
CREATE INDEX idx_shots_match_player ON shots(match_id, player, tick);

CREATE TABLE hurts (
    match_id   INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick       INTEGER NOT NULL,
    victim     TEXT NOT NULL,
    attacker   TEXT,
    dmg_health INTEGER NOT NULL,
    weapon     TEXT NOT NULL,
    hitgroup   TEXT NOT NULL
);
CREATE INDEX idx_hurts_match ON hurts(match_id, tick);

CREATE TABLE reloads (
    match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick     INTEGER NOT NULL,
    player   TEXT NOT NULL
);
CREATE INDEX idx_reloads_match_player ON reloads(match_id, player, tick);

CREATE TABLE inventories (
    match_id   INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    tick       INTEGER NOT NULL,
    steamid    TEXT NOT NULL,
    items_json TEXT NOT NULL,
    PRIMARY KEY (match_id, tick, steamid)
) WITHOUT ROWID;

-- One row per tracked-player death (spec §1 storage).
CREATE TABLE death_class (
    match_id            INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round               INTEGER NOT NULL,
    tick                INTEGER NOT NULL,
    victim              TEXT NOT NULL,
    class_id            INTEGER NOT NULL,
    class_source        TEXT NOT NULL,
    secondary_tags_json TEXT NOT NULL,
    confidence          REAL NOT NULL,
    PRIMARY KEY (match_id, tick, victim)
) WITHOUT ROWID;

CREATE TABLE rule_flags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id   INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    rule_id    TEXT NOT NULL,
    round      INTEGER NOT NULL,
    tick       INTEGER NOT NULL,
    steamid    TEXT NOT NULL,
    confidence REAL NOT NULL,
    severity   REAL NOT NULL,
    details_json TEXT NOT NULL
);
CREATE INDEX idx_rule_flags_match_rule ON rule_flags(match_id, rule_id);

CREATE TABLE insights (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    match_id       INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    detector       TEXT NOT NULL,
    category       TEXT NOT NULL,
    severity       REAL NOT NULL,
    confidence     REAL NOT NULL,
    round          INTEGER NOT NULL,
    player         TEXT NOT NULL,
    title_data_json TEXT NOT NULL,
    metrics_json   TEXT NOT NULL,
    evidence_json  TEXT NOT NULL
);
CREATE INDEX idx_insights_match ON insights(match_id);

ALTER TABLE tick_samples ADD COLUMN is_scoped INTEGER;
