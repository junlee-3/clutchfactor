-- Round-by-round coaching (issue #9 §7). moments_json stores STRUCTURED
-- moments (facts as data, raw callouts); narration renders at serve time
-- in cf-narrator::rail so V1.3's grounding contract feeds on facts.
CREATE TABLE round_review (
    match_id     INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    round        INTEGER NOT NULL,
    impact       REAL NOT NULL,
    verdict      TEXT NOT NULL,
    attention    TEXT NOT NULL,
    selected     INTEGER NOT NULL,
    pivotal_tick INTEGER NOT NULL,
    header_json  TEXT NOT NULL,
    moments_json TEXT NOT NULL,
    PRIMARY KEY (match_id, round)
) WITHOUT ROWID;
