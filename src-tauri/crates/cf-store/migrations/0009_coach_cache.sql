-- V1.3 coach cache (docs/spec/play-ledger-and-coach.md §3, ADR-0010): one
-- validated (or failed) coach response per (match, kind, round). facts_hash
-- = sha256(rendered facts + model + style version): a changed ledger, model
-- or style regenerates; Regenerate deletes the row. status 'fallback' rows
-- are kept so a failing round is not re-billed on every open.
CREATE TABLE coach_cache (
    match_id        INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,      -- 'round' | 'synthesis'
    round           INTEGER NOT NULL,  -- 0 for synthesis
    facts_hash      TEXT NOT NULL,
    model           TEXT NOT NULL,
    status          TEXT NOT NULL,      -- 'ok' | 'fallback'
    response_json   TEXT NOT NULL,
    violations_json TEXT NOT NULL DEFAULT '[]',
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (match_id, kind, round)
) WITHOUT ROWID;
